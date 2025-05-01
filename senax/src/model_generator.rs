use anyhow::{Context, Result, ensure};
use askama::Template;
use compact_str::CompactString;
use convert_case::{Case, Casing};
use indexmap::IndexMap;
use regex::Regex;
use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::Path;
use std::sync::Arc;

use crate::filters;
use crate::schema::{ConfigDef, StringOrArray, Timestampable};
use crate::{BASE_DOMAIN_PATH, DOMAIN_REPOSITORIES_PATH};
use crate::{
    DB_PATH, DOMAIN_PATH,
    common::fs_write,
    schema::{self, CONFIG, GROUPS, ModelDef, to_id_name},
};

mod db;
mod domain;

pub fn generate(db: &str, force: bool, clean: bool, skip_version_check: bool) -> Result<()> {
    if !skip_version_check {
        check_version(db)?;
    }
    let non_snake_case = crate::common::check_non_snake_case()?;
    schema::parse(db, false, false)?;

    let config = CONFIG.read().unwrap().as_ref().unwrap().clone();
    let exclude_from_domain = config.exclude_from_domain;
    let groups = GROUPS.read().unwrap().as_ref().unwrap().clone();
    let model_dir = Path::new(DB_PATH).join(db.to_case(Case::Snake));
    let db_repositories_dir = model_dir.join("repositories");
    let domain_src_dir = Path::new(DOMAIN_PATH).join("src");
    let base_domain_src_dir = Path::new(BASE_DOMAIN_PATH).join("src");
    let domain_repositories_dir = Path::new(DOMAIN_REPOSITORIES_PATH).join(db.to_case(Case::Snake));
    let domain_repositories_src_dir = domain_repositories_dir.join("src");

    let file_path = model_dir.join("Cargo.toml");
    if force || !file_path.exists() {
        #[derive(Template)]
        #[template(path = "model/_Cargo.toml", escape = "none")]
        struct CargoTemplate<'a> {
            pub db: &'a str,
            pub config: &'a ConfigDef,
        }

        let tpl = CargoTemplate {
            db,
            config: &config,
        };
        fs_write(file_path, tpl.render()?)?;
    }

    let file_path = model_dir.join("build.rs");
    if force || !file_path.exists() {
        #[derive(Template)]
        #[template(path = "model/build.rs", escape = "none")]
        struct BuildTemplate {}

        let tpl = BuildTemplate {};
        fs_write(file_path, tpl.render()?)?;
    }

    let base_dir = model_dir.join("base");
    let base_src_dir = base_dir.join("src");
    let model_src_dir = model_dir.join("src");
    let file_path = model_src_dir.join("lib.rs");
    if force || !file_path.exists() {
        #[derive(Template)]
        #[template(path = "model/src/lib.rs", escape = "none")]
        struct LibTemplate<'a> {
            pub db: &'a str,
            pub config: &'a ConfigDef,
            pub non_snake_case: bool,
        }

        let tpl = LibTemplate {
            db,
            config: &config,
            non_snake_case,
        };
        fs_write(file_path, tpl.render()?)?;
    }

    let file_path = model_src_dir.join("models.rs");

    #[derive(Template)]
    #[template(path = "model/src/models.rs", escape = "none")]
    struct ModelsTemplate<'a> {
        pub groups: &'a IndexMap<String, IndexMap<String, Arc<ModelDef>>>,
        pub config: &'a ConfigDef,
    }

    let tpl = ModelsTemplate {
        groups: &groups,
        config: &config,
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
        db::impl_domain::write_impl_domain_rs(&model_src_dir, db, &groups, force)?;
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
        domain::base_domain::write_models_db_rs(&domain_models_dir, db, &groups, force)?;
        domain::repositories::write_lib_rs(&domain_repositories_src_dir, db, &groups, force)?;
        domain::repositories::write_cargo_toml(&domain_repositories_dir, db, &groups, force)?;
    }

    db::base::write_files(
        &base_dir,
        db,
        &groups,
        &config,
        force,
        non_snake_case,
    )?;

    let file_path = model_src_dir.join("main.rs");
    if force || !file_path.exists() {
        #[derive(Template)]
        #[template(path = "model/src/main.rs", escape = "none")]
        struct MainTemplate<'a> {
            pub db: &'a str,
        }

        let tpl = MainTemplate { db };
        fs_write(file_path, tpl.render()?)?;
    }

    #[derive(Template)]
    #[template(path = "model/src/seeder.rs", escape = "none")]
    struct SeederTemplate<'a> {
        pub groups: &'a IndexMap<String, IndexMap<String, Arc<ModelDef>>>,
    }

    let file_path = model_src_dir.join("seeder.rs");
    let tpl = SeederTemplate { groups: &groups };
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

    let domain_db_dir = domain_models_dir.join(db.to_case(Case::Snake));
    if clean && domain_db_dir.exists() {
        for entry in glob::glob(&format!("{}/**/*.rs", domain_db_dir.display()))? {
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
    let domain_repositories_dir = domain_repositories_dir.join("groups");
    if clean && domain_repositories_dir.exists() {
        for entry in glob::glob(&format!("{}/**/*.*", domain_repositories_dir.display()))? {
            match entry {
                Ok(path) => remove_files.insert(path.as_os_str().to_owned()),
                Err(_) => false,
            };
        }
    }

    for (group_name, defs) in &groups {
        let mod_names: BTreeSet<String> = defs.iter().map(|(_, d)| d.mod_name()).collect();
        let entities_mod_names: BTreeSet<(String, &String)> = defs
            .iter()
            .filter(|(_, d)| !d.abstract_mode)
            .map(|(model_name, def)| (def.mod_name(), model_name))
            .collect();

        let model_group_dir = model_models_dir.join(group_name.to_case(Case::Snake));
        let model_group_base_dir = model_group_dir.join("_base");
        let file_path = model_models_dir.join(format!("{}.rs", group_name.to_case(Case::Snake)));
        remove_files.remove(file_path.as_os_str());
        let concrete_models = defs.iter().filter(|(_k, v)| !v.abstract_mode).collect();

        #[derive(Template)]
        #[template(path = "model/base/src/group.rs", escape = "none")]
        struct GroupTemplate<'a> {
            pub group_name: &'a str,
            pub mod_names: &'a BTreeSet<String>,
            pub models: IndexMap<&'a String, &'a Arc<ModelDef>>,
            pub config: &'a ConfigDef,
        }

        let tpl = GroupTemplate {
            group_name,
            mod_names: &mod_names,
            models: concrete_models,
            config: &config,
        };
        fs_write(file_path, tpl.render()?)?;

        db::repositories::write_group_files(
            &db_repositories_dir,
            db,
            group_name,
            &groups,
            &config,
            force,
            clean,
            exclude_from_domain,
            &mut remove_files,
        )?;

        if !exclude_from_domain {
            domain::base_domain::write_group_rs(
                &domain_db_dir,
                group_name,
                &entities_mod_names,
                &mod_names,
                force,
                &mut remove_files,
            )?;
            domain::repositories::write_group_files(
                &domain_repositories_dir,
                db,
                group_name,
                &groups,
                force,
                &mut remove_files,
            )?;
            db::impl_domain::write_group_rs(
                &impl_domain_dir,
                db,
                group_name,
                &entities_mod_names,
                force,
                &mut remove_files,
            )?;
        }
        let visibility = if config.export_db_layer {
            ""
        } else {
            "(crate)"
        };

        for (model_name, def) in defs {
            let table_name = def.table_name();
            let mod_name = def.mod_name();
            let mod_name = &mod_name;
            if def.abstract_mode {
                let file_path = model_group_dir.join(format!("{}.rs", mod_name));
                remove_files.remove(file_path.as_os_str());
                if force || !file_path.exists() {
                    #[derive(Template)]
                    #[template(path = "model/base/src/group/abstract.rs", escape = "none")]
                    struct GroupAbstractTemplate<'a> {
                        pub db: &'a str,
                        pub group_name: &'a str,
                        pub mod_name: &'a str,
                        pub name: &'a str,
                        pub pascal_name: &'a str,
                        pub def: &'a Arc<ModelDef>,
                        pub config: &'a ConfigDef,
                        pub visibility: &'a str,
                    }

                    let tpl = GroupAbstractTemplate {
                        db,
                        group_name,
                        mod_name,
                        name: model_name,
                        pascal_name: &model_name.to_case(Case::Pascal),
                        def,
                        config: &config,
                        visibility,
                    };
                    fs_write(file_path, tpl.render()?)?;
                }

                let file_path = model_group_base_dir.join(format!("_{}.rs", mod_name));
                remove_files.remove(file_path.as_os_str());

                #[derive(Template)]
                #[template(path = "model/base/src/group/base/_abstract.rs", escape = "none")]
                struct GroupBaseAbstractTemplate<'a> {
                    pub db: &'a str,
                    pub group_name: &'a str,
                    pub mod_name: &'a str,
                    pub name: &'a str,
                    pub pascal_name: &'a str,
                    pub id_name: &'a str,
                    pub table_name: &'a str,
                    pub def: &'a Arc<ModelDef>,
                    pub config: &'a ConfigDef,
                    pub visibility: &'a str,
                }

                let tpl = GroupBaseAbstractTemplate {
                    db,
                    group_name,
                    mod_name,
                    name: model_name,
                    pascal_name: &model_name.to_case(Case::Pascal),
                    id_name: &to_id_name(model_name),
                    table_name: &table_name,
                    def,
                    config: &config,
                    visibility,
                };
                fs_write(file_path, tpl.render()?)?;

                if !exclude_from_domain {
                    domain::base_domain::write_abstract(
                        &domain_db_dir,
                        db,
                        group_name,
                        mod_name,
                        force,
                        model_name,
                        def,
                        &mut remove_files,
                    )?;
                }
            } else {
                let file_path = model_group_dir.join(format!("{}.rs", mod_name));
                remove_files.remove(file_path.as_os_str());
                if force || !file_path.exists() {
                    #[derive(Template)]
                    #[template(path = "model/base/src/group/table.rs", escape = "none")]
                    struct GroupTableTemplate<'a> {
                        pub db: &'a str,
                        pub group_name: &'a str,
                        pub mod_name: &'a str,
                        pub model_name: &'a str,
                        pub pascal_name: &'a str,
                        pub id_name: &'a str,
                        pub def: &'a Arc<ModelDef>,
                        pub config: &'a ConfigDef,
                        pub visibility: &'a str,
                    }

                    let tpl = GroupTableTemplate {
                        db,
                        group_name,
                        mod_name,
                        model_name,
                        pascal_name: &model_name.to_case(Case::Pascal),
                        id_name: &to_id_name(model_name),
                        def,
                        config: &config,
                        visibility,
                    };
                    fs_write(file_path, tpl.render()?)?;
                }

                let file_path = model_group_base_dir.join(format!("_{}.rs", mod_name));
                remove_files.remove(file_path.as_os_str());
                let mut force_indexes = Vec::new();
                let (_, _, idx_map) = crate::migration_generator::make_table_def(def, &config)?;
                for (index_name, index_def) in &def.merged_indexes {
                    for (force_index_name, force_index_def) in &index_def.force_index_on {
                        let force_index_def = force_index_def.clone().unwrap_or_default();
                        let includes = force_index_def
                            .includes
                            .unwrap_or(StringOrArray::One(force_index_name.clone()));
                        let mut cond: Vec<_> = includes
                            .to_vec()
                            .iter()
                            .map(|v| format!("filter_digest.contains({:?})", v))
                            .collect();
                        let excludes = force_index_def
                            .excludes
                            .unwrap_or(StringOrArray::Many(vec![]));
                        for v in excludes.to_vec() {
                            cond.push(format!("!filter_digest.contains({:?})", v));
                        }
                        let idx = idx_map.get(index_name).unwrap();
                        let idx = format!("{:?}", filters::_to_db_col(idx, true));
                        force_indexes.push((cond.join(" && "), idx));
                    }
                }

                #[derive(Template)]
                #[template(path = "model/base/src/group/base/_table.rs", escape = "none")]
                struct GroupBaseTableTemplate<'a> {
                    pub db: &'a str,
                    pub group_name: &'a str,
                    pub mod_name: &'a str,
                    pub model_name: &'a str,
                    pub pascal_name: &'a str,
                    pub id_name: &'a str,
                    pub table_name: &'a str,
                    pub def: &'a Arc<ModelDef>,
                    pub force_indexes: Vec<(String, String)>,
                    pub config: &'a ConfigDef,
                    pub version_col: CompactString,
                    pub visibility: &'a str,
                }

                let tpl = GroupBaseTableTemplate {
                    db,
                    group_name,
                    mod_name,
                    model_name,
                    pascal_name: &model_name.to_case(Case::Pascal),
                    id_name: &to_id_name(model_name),
                    table_name: &table_name,
                    def,
                    force_indexes,
                    config: &config,
                    version_col: schema::ConfigDef::version(),
                    visibility,
                };
                fs_write(file_path, tpl.render()?)?;

                if !exclude_from_domain {
                    domain::base_domain::write_entity(
                        &domain_db_dir,
                        db,
                        group_name,
                        mod_name,
                        force,
                        model_name,
                        def,
                        &mut remove_files,
                    )?;
                    db::impl_domain::write_entity(
                        &impl_domain_dir,
                        db,
                        &config,
                        group_name,
                        mod_name,
                        force,
                        model_name,
                        def,
                        &mut remove_files,
                    )?;
                }
            }
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
    let model_dir = Path::new(DB_PATH).join(db.to_case(Case::Snake));
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
