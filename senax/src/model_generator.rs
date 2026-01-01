use anyhow::{Context, Result, ensure};
use askama::Template;
use compact_str::CompactString;
use indexmap::{IndexMap, IndexSet};
use regex::Regex;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;

use crate::common::ToCase as _;
use crate::common::{AtomicLoad as _, OVERWRITTEN_MSG};
use crate::schema::{_to_ident_name, ConfigDef, GroupsDef, Timestampable};
use crate::{BASE_DOMAIN_PATH, DOMAIN_BASE_RELATIONS_PATH, DOMAIN_REPOSITORIES_PATH};
use crate::{
    DB_PATH, DOMAIN_PATH,
    common::fs_write,
    schema::{self, CONFIG, GROUPS, ModelDef, to_id_name},
};
use crate::{SEPARATED_BASE_FILES, filters};

mod analyzer;
mod db;
mod domain;

pub fn generate(
    db: &str,
    force: bool,
    clean: bool,
    skip_version_check: bool,
    model_split_size: Option<usize>,
) -> Result<()> {
    if !skip_version_check {
        check_version(db)?;
    }
    let non_snake_case = crate::common::check_non_snake_case()?;
    schema::parse(db, false, false)?;

    let config = CONFIG.read().unwrap().as_ref().unwrap().clone();
    let exclude_from_domain = config.exclude_from_domain;
    let group_lock = GROUPS.read().unwrap();
    let groups = group_lock.as_ref().unwrap();
    let model_dir = Path::new(DB_PATH).join(db.to_snake());
    let base_repositories_dir = model_dir.join("base_repositories");
    let db_repositories_dir = model_dir.join("repositories");
    let domain_src_dir = Path::new(DOMAIN_PATH).join("src");
    let base_domain_src_dir = Path::new(DOMAIN_PATH).join(BASE_DOMAIN_PATH).join("src");
    let domain_base_relations_dir = Path::new(DOMAIN_PATH)
        .join(DOMAIN_BASE_RELATIONS_PATH)
        .join(db.to_snake());
    let domain_base_relations_src_dir = domain_base_relations_dir.join("src");
    let domain_repositories_dir = Path::new(DOMAIN_PATH)
        .join(DOMAIN_REPOSITORIES_PATH)
        .join(db.to_snake());
    let domain_repositories_src_dir = domain_repositories_dir.join("src");

    let mut analyzer = analyzer::GraphAnalyzer::new();
    for (group_name, group) in groups {
        let mut nodes = Vec::new();
        for (model_name, model) in group.iter() {
            let edges = model
                .merged_relations
                .iter()
                .filter(|(_, r)| r.db.is_none())
                .map(|(_, r)| analyzer::EdgeTarget {
                    group: Some(r.get_group_name()),
                    node: r.get_foreign_model_name(),
                })
                .collect();

            nodes.push(analyzer::Node {
                name: model_name.clone(),
                edges,
            });
        }
        analyzer.add_group(analyzer::Group {
            name: group_name.clone(),
            nodes,
        });
    }
    let unified_groups = analyzer.unify_all_nodes();
    let unified_groups =
        analyzer.merge_unified_groups(unified_groups, model_split_size.unwrap_or(10));

    let file_path = model_dir.join("Cargo.toml");
    let mut content = if force || !file_path.exists() {
        #[derive(Template)]
        #[template(path = "db/_Cargo.toml", escape = "none")]
        struct CargoTemplate<'a> {
            pub db: &'a str,
            pub config: &'a ConfigDef,
        }

        CargoTemplate {
            db,
            config: &config,
        }
        .render()?
    } else {
        fs::read_to_string(&file_path)?.replace("\r\n", "\n")
    };
    let reg = Regex::new(r"(?m)^_base_repo_\w+\s*=.+\n")?;
    content = reg.replace_all(&content, "").into_owned();
    let reg = Regex::new(r"(?m)^_repo_\w+\s*=.+\n")?;
    content = reg.replace_all(&content, "").into_owned();
    for (group, _) in groups.iter().rev() {
        let db = &db.to_snake();
        let group = &group.to_snake();
        content = content.replace(
            "[dependencies]",
            &format!(
                "[dependencies]\n_repo_{} = {{ package = \"_repo_{}_{}\", path = \"repositories/{}\" }}",
                group, db, group, group
            ),
        );
    }
    for unified_group in unified_groups.iter().rev() {
        let db = &db.to_snake();
        let unified_name = unified_group.unified_name();
        content = content.replace(
            "[dependencies]",
            &format!(
                "[dependencies]\n_base_repo_{} = {{ package = \"_base_repo_{}_{}\", path = \"base_repositories/{}\" }}",
                unified_name, db, unified_name, unified_name
            ),
        );
    }
    let reg = Regex::new(r"(?m)^db_\w+\s*=.+\n")?;
    content = reg.replace_all(&content, "").into_owned();
    for db in config.outer_db().iter().rev() {
        let db = &db.to_snake();
        content = content.replace(
            "[dependencies]",
            &format!("[dependencies]\ndb_{} = {{ path = \"../{}\" }}", db, db),
        );
    }
    fs_write(file_path, &*content)?;

    let file_path = model_dir.join("build.rs");
    if force || !file_path.exists() {
        #[derive(Template)]
        #[template(path = "db/build.rs", escape = "none")]
        struct BuildTemplate {}

        let tpl = BuildTemplate {};
        fs_write(file_path, tpl.render()?)?;
    }

    let base_dir = model_dir.join("base");
    let base_src_dir = base_dir.join("src");
    let model_src_dir = model_dir.join("src");

    let file_path = model_src_dir.join("lib.rs");
    let mut content = if force || !file_path.exists() {
        #[derive(Template)]
        #[template(path = "db/src/lib.rs", escape = "none")]
        struct LibTemplate<'a> {
            pub config: &'a ConfigDef,
            pub non_snake_case: bool,
        }

        LibTemplate {
            config: &config,
            non_snake_case,
        }
        .render()?
    } else {
        fs::read_to_string(&file_path)?.replace("\r\n", "\n")
    };

    let reg = Regex::new(r"(?m)^[ \t]*pub use _repo_\w+::repositories::[#\w]+;\n")?;
    content = reg.replace_all(&content, "").into_owned();
    let reg = Regex::new(
        r"(?m)^[ \t]*let _ = _base::models::\w+_HANDLER.set\(Box::new\(_base_repo_\w+::repositories::CacheHandler\)\);\n",
    )?;
    content = reg.replace_all(&content, "").into_owned();
    for (group, _) in groups.iter().rev() {
        let group = &group.to_snake();
        content = content.replace(
            "pub mod repositories {",
            &format!(
                "pub mod repositories {{\n    pub use _repo_{}::repositories::{};",
                group,
                _to_ident_name(group)
            ),
        );
    }
    for unified_group in unified_groups.iter().rev() {
        let unified_name = unified_group.unified_name();
        content = content.replace(
            "pub fn init() {",
            &format!(
                "pub fn init() {{\n    let _ = _base::models::{}_HANDLER.set(Box::new(_base_repo_{}::repositories::CacheHandler));",
                unified_name.to_upper(), unified_name
            ),
        );
    }
    fs_write(file_path, &*content)?;

    let file_path = model_src_dir.join("models.rs");

    #[derive(Template)]
    #[template(path = "db/src/models.rs", escape = "none")]
    struct ModelsTemplate<'a> {
        pub config: &'a ConfigDef,
        pub groups: &'a GroupsDef,
        pub unified: &'a Vec<String>,
    }

    let unified = unified_groups.iter().map(|g| g.unified_name()).collect();
    let tpl = ModelsTemplate {
        config: &config,
        groups,
        unified: &unified,
    };
    fs_write(file_path, tpl.render()?)?;

    let mut remove_files = HashSet::new();
    domain::base_domain::write_value_objects_rs(
        &base_domain_src_dir,
        &mut remove_files,
        clean,
        force,
    )?;

    if !exclude_from_domain {
        db::impl_domain::write_impl_domain_rs(&model_src_dir, db, groups, force)?;
    }
    let domain_models_dir = base_domain_src_dir.join("models");
    let impl_domain_dir = base_src_dir.join("impl_domain");
    if clean && impl_domain_dir.exists() {
        for entry in glob::glob(&format!("{}/**/*.rs", impl_domain_dir.display()))? {
            match entry {
                Ok(path) => remove_files.insert(path.as_os_str().to_owned()),
                Err(_) => false,
            };
        }
    }
    if !exclude_from_domain {
        domain::base_domain::write_models_db_rs(&domain_models_dir, db, groups, force)?;
        domain::base_relations::write_cargo_toml(
            &domain_base_relations_dir,
            db,
            &unified_groups,
            force,
        )?;
        domain::base_relations::write_lib_rs(
            &domain_base_relations_src_dir,
            db,
            groups,
            &unified_groups,
            force,
        )?;
        domain::repositories::write_cargo_toml(&domain_repositories_dir, db, groups, force)?;
        domain::repositories::write_lib_rs(&domain_repositories_src_dir, db, groups, force)?;
    }

    db::base::write_files(
        &base_dir,
        db,
        groups,
        &config,
        &unified,
        force,
        non_snake_case,
    )?;

    #[derive(Template)]
    #[template(path = "db/src/seeder.rs", escape = "none")]
    struct SeederTemplate<'a> {
        pub config: &'a ConfigDef,
        pub groups: &'a GroupsDef,
        pub unified: &'a Vec<String>,
    }

    let file_path = model_src_dir.join("seeder.rs");
    let tpl = SeederTemplate {
        config: &config,
        groups,
        unified: &unified,
    };
    fs_write(file_path, tpl.render()?)?;

    let path = model_dir.join("migrations");
    if !path.exists() {
        let file_path = path.join(".gitkeep");
        fs_write(file_path, "")?;
    }

    let path = model_dir.join("seeds");
    if !path.exists() {
        let file_path = path.join(".gitkeep");
        fs_write(file_path, "")?;
    }

    let model_models_dir = base_src_dir.join("models");
    if clean && model_models_dir.exists() {
        for entry in glob::glob(&format!("{}/**/*.rs", model_models_dir.display()))? {
            match entry {
                Ok(path) => remove_files.insert(path.as_os_str().to_owned()),
                Err(_) => false,
            };
        }
    }

    let domain_db_dir = domain_models_dir.join(db.to_snake());
    if clean && domain_db_dir.exists() {
        for entry in glob::glob(&format!("{}/**/*.rs", domain_db_dir.display()))? {
            match entry {
                Ok(path) => remove_files.insert(path.as_os_str().to_owned()),
                Err(_) => false,
            };
        }
    }
    if base_repositories_dir.exists() {
        for entry in glob::glob(&format!("{}/**/*.*", base_repositories_dir.display()))? {
            match entry {
                Ok(path) => remove_files.insert(path.as_os_str().to_owned()),
                Err(_) => false,
            };
        }
    }
    if clean && db_repositories_dir.exists() {
        for entry in glob::glob(&format!("{}/**/*.*", db_repositories_dir.display()))? {
            match entry {
                Ok(path) => remove_files.insert(path.as_os_str().to_owned()),
                Err(_) => false,
            };
        }
    }
    let domain_base_relations_dir = domain_base_relations_dir.join("groups");
    if clean && domain_base_relations_dir.exists() {
        for entry in glob::glob(&format!("{}/**/*.*", domain_base_relations_dir.display()))? {
            match entry {
                Ok(path) => remove_files.insert(path.as_os_str().to_owned()),
                Err(_) => false,
            };
        }
    }
    let domain_repositories_dir = domain_repositories_dir.join("groups");
    if clean && domain_repositories_dir.exists() {
        for entry in glob::glob(&format!("{}/**/*.*", domain_repositories_dir.display()))? {
            match entry {
                Ok(path) => remove_files.insert(path.as_os_str().to_owned()),
                Err(_) => false,
            };
        }
    }

    for (group_name, defs) in groups {
        let mod_names: BTreeSet<String> = defs.iter().map(|(_, d)| d.mod_name()).collect();
        let entities_mod_names: BTreeSet<(String, &String)> = defs
            .iter()
            .filter(|(_, d)| !d.abstract_mode)
            .map(|(model_name, def)| (def.mod_name(), model_name))
            .collect();

        let model_group_dir = model_models_dir.join(group_name.to_snake());

        let mut table_output = String::new();
        let mut base_domain_output = String::new();
        let mut impl_domain_output = String::new();

        for (model_name, def) in defs {
            let mod_name = def.mod_name();
            let mod_name = &mod_name;
            base_domain_output.push_str(&format!("pub mod {}", _to_ident_name(mod_name)));
            if def.abstract_mode {
                #[derive(Template)]
                #[template(path = "db/base/src/group/abstract.rs", escape = "none")]
                struct GroupAbstractTemplate<'a> {
                    pub pascal_name: &'a str,
                    pub def: &'a Arc<ModelDef>,
                    pub config: &'a ConfigDef,
                }

                let tpl = GroupAbstractTemplate {
                    pascal_name: &model_name.to_pascal(),
                    def,
                    config: &config,
                };
                let output = tpl.render()?;
                if SEPARATED_BASE_FILES {
                    let file_path = model_group_dir.join(format!("{}.rs", mod_name));
                    remove_files.remove(file_path.as_os_str());
                    fs_write(file_path, format!("{}{}", OVERWRITTEN_MSG, output))?;
                } else {
                    table_output.push_str(&format!(
                        "pub mod {} {{\n{}}}\n",
                        _to_ident_name(mod_name),
                        output
                    ));
                }

                if !exclude_from_domain {
                    base_domain_output.push_str(&domain::base_domain::write_abstract(
                        &domain_db_dir,
                        db,
                        &config,
                        group_name,
                        mod_name,
                        model_name,
                        def,
                        &mut remove_files,
                    )?);
                }
            } else {
                #[derive(Template)]
                #[template(path = "db/base/src/group/table.rs", escape = "none")]
                struct GroupTableTemplate<'a> {
                    pub group_name: &'a str,
                    pub model_name: &'a str,
                    pub pascal_name: &'a str,
                    pub id_name: &'a str,
                    pub def: &'a Arc<ModelDef>,
                    pub config: &'a ConfigDef,
                    pub version_col: CompactString,
                }

                let tpl = GroupTableTemplate {
                    group_name,
                    model_name,
                    pascal_name: &model_name.to_pascal(),
                    id_name: &to_id_name(model_name),
                    def,
                    config: &config,
                    version_col: schema::ConfigDef::version(),
                };
                let output = tpl.render()?;
                if SEPARATED_BASE_FILES {
                    let file_path = model_group_dir.join(format!("{}.rs", mod_name));
                    remove_files.remove(file_path.as_os_str());
                    fs_write(file_path, format!("{}{}", OVERWRITTEN_MSG, output))?;
                } else {
                    table_output.push_str(&format!(
                        "pub mod {} {{\n{}}}\n",
                        _to_ident_name(mod_name),
                        output
                    ));
                }

                if !exclude_from_domain {
                    base_domain_output.push_str(&domain::base_domain::write_entity(
                        &domain_db_dir,
                        db,
                        &config,
                        group_name,
                        mod_name,
                        model_name,
                        def,
                        &mut remove_files,
                    )?);
                    impl_domain_output.push_str(&db::impl_domain::write_entity(
                        &impl_domain_dir,
                        db,
                        &config,
                        group_name,
                        mod_name,
                        model_name,
                        def,
                        &mut remove_files,
                    )?);
                }
            }
        }
        let file_path = model_models_dir.join(format!("{}.rs", group_name.to_snake()));
        remove_files.remove(file_path.as_os_str());
        let concrete_models = defs.iter().filter(|(_k, v)| !v.abstract_mode).collect();

        #[derive(Template)]
        #[template(path = "db/base/src/group.rs", escape = "none")]
        struct GroupTemplate<'a> {
            pub group_name: &'a str,
            pub mod_names: &'a BTreeSet<String>,
            pub models: IndexMap<&'a String, &'a Arc<ModelDef>>,
            pub table_output: String,
        }

        let tpl = GroupTemplate {
            group_name,
            mod_names: &mod_names,
            models: concrete_models,
            table_output,
        };
        fs_write(file_path, tpl.render()?)?;

        if !exclude_from_domain {
            domain::base_domain::write_group_rs(
                &domain_db_dir,
                group_name,
                base_domain_output,
                &mut remove_files,
            )?;
            db::impl_domain::write_group_rs(
                &impl_domain_dir,
                group_name,
                &entities_mod_names,
                impl_domain_output,
                &mut remove_files,
            )?;
        }
    }
    for unified_group in &unified_groups {
        let include_groups = unified_group.include_groups();
        let repo_include_groups: GroupsDef = groups
            .iter()
            .map(|(group_name, models)| {
                let v2: IndexMap<String, Arc<ModelDef>> = models
                    .iter()
                    .filter(|(model_name, _)| {
                        unified_group
                            .nodes
                            .get(&(group_name.into(), (*model_name).into()))
                            == Some(&analyzer::Mark::Include)
                    })
                    .map(|(n, v)| (n.to_string(), v.clone()))
                    .collect();
                (group_name.to_string(), v2)
            })
            .collect();
        let ref_db: BTreeSet<(String, String)> = groups
            .iter()
            .filter(|(n, _)| include_groups.contains(n.as_str()))
            .flat_map(|(group_name, models)| {
                models
                    .iter()
                    .filter(|(model_name, _)| {
                        unified_group
                            .nodes
                            .get(&(group_name.into(), (*model_name).into()))
                            == Some(&analyzer::Mark::Include)
                    })
                    .flat_map(|(_, v)| {
                        v.belongs_to_outer_db()
                            .iter()
                            .map(|v| (v.1.db().to_string(), v.1.get_group_name()))
                            .collect::<Vec<_>>()
                    })
            })
            .collect();
        let unified_name = unified_group.unified_name();
        db::repositories::write_base_group_files(
            &base_repositories_dir,
            db,
            &config,
            &unified_name,
            &repo_include_groups,
            &unified_group,
            &unified_groups,
            &ref_db,
            force,
            &mut remove_files,
        )?;

        if !exclude_from_domain {
            domain::base_relations::write_group_files(
                &domain_base_relations_dir,
                db,
                &config,
                &unified_name,
                &repo_include_groups,
                &unified_group,
                &unified_groups,
                &ref_db,
                force,
                &mut remove_files,
            )?;
        }
    }
    for (group_name, group) in groups {
        let mut groups = GroupsDef::new();
        groups.insert(group_name.clone(), group.clone());
        db::repositories::write_group_files(
            &db_repositories_dir,
            db,
            &config,
            group_name,
            &groups,
            &unified_groups,
            force,
            exclude_from_domain,
            &mut remove_files,
        )?;

        if !exclude_from_domain {
            domain::repositories::write_group_files(
                &domain_repositories_dir,
                db,
                &config,
                group_name,
                &groups,
                force,
                &mut remove_files,
            )?;
        }
    }
    if !exclude_from_domain {
        domain::base_domain::write_models_rs(&base_domain_src_dir, db)?;
        domain::write_repositories_rs(&domain_src_dir, db)?;
    }
    for file in &remove_files {
        println!("REMOVE:{}", file.to_string_lossy());
        fs::remove_file(file)?;
        let ancestors = Path::new(file).ancestors();
        for ancestor in ancestors {
            if let Ok(dir) = ancestor.read_dir() {
                if dir.count() == 0 {
                    fs::remove_dir(ancestor)?;
                } else {
                    break;
                }
            }
        }
    }
    Ok(())
}

pub fn check_version(db: &str) -> Result<()> {
    let model_dir = Path::new(DB_PATH).join(db.to_snake());
    let model_src_dir = model_dir.join("src");
    let file_path = model_src_dir.join("models.rs");
    if file_path.exists() {
        let content = fs::read_to_string(&file_path)?;
        let re = Regex::new(r"(?m)^// Senax v(.+)$").unwrap();
        let caps = re
            .captures(&content)
            .with_context(|| format!("Illegal file content:{}", &file_path.to_string_lossy()))?;
        let ver = caps.get(1).unwrap().as_str();
        let req = semver::VersionReq::parse(ver)?;
        let version = semver::Version::parse(crate::VERSION)?;
        ensure!(req.matches(&version), "Use {} version of Senax.", ver);
    }
    Ok(())
}
