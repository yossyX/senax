use anyhow::Result;
use askama::Template;
use compact_str::CompactString;
use indexmap::IndexMap;
use regex::Regex;
use std::{
    collections::{BTreeSet, HashMap, HashSet},
    ffi::OsString,
    fs,
    path::Path,
    sync::Arc,
};

use crate::model_generator::analyzer::UnifiedGroup;
use crate::schema::is_mysql_mode;
use crate::schema::{ConfigDef, GroupsDef, ModelDef, StringOrArray, Timestampable, to_id_name};
use crate::{SEPARATED_BASE_FILES, filters};
use crate::{common::ToCase, schema::Joinable};
use crate::{
    common::{OVERWRITTEN_MSG, fs_write},
    model_generator::analyzer,
};

mod impl_domain;

#[allow(clippy::too_many_arguments)]
pub fn write_base_group_files(
    db_repositories_dir: &Path,
    db: &str,
    config: &ConfigDef,
    unified_name: &str,
    groups: &GroupsDef,
    unified_group: &UnifiedGroup,
    unified_groups: &[UnifiedGroup],
    filter_unified_map: HashMap<(String, String), String>,
    filter_unified_names: BTreeSet<String>,
    ref_db: &BTreeSet<(String, String)>,
    remove_files: &mut HashSet<OsString>,
) -> Result<()> {
    let base_dir = db_repositories_dir.join(unified_name);
    let file_path = base_dir.join("Cargo.toml");
    remove_files.remove(file_path.as_os_str());

    #[derive(Template)]
    #[template(path = "db/base_repositories/_Cargo.toml", escape = "none")]
    struct Template<'a> {
        db: &'a str,
        unified_name: &'a str,
    }

    let mut content = Template { db, unified_name }.render()?;
    let mut db_chk = HashSet::new();
    for (db, group) in ref_db {
        let db = &db.to_snake();
        if !db_chk.contains(db) {
            db_chk.insert(db.to_string());
            content = content.replace(
                "[dependencies]",
                &format!(
                    "[dependencies]\ndb_{} = {{ package = \"_db_{}\", path = \"../../../_{}/base\" }}",
                    db, db, db
                ),
            );
        }
        let group = &group.to_snake();
        content = content.replace(
            "[dependencies]",
            &format!(
                "[dependencies]\n_repo_{}_{} = {{ path = \"../../../_{}/repositories/{}\" }}",
                db, group, db, group
            ),
        );
    }
    for (g, m) in &unified_group.ref_unified_groups {
        let db = &db.to_snake();
        let unified = format!("{}__{}", g.to_snake(), m.to_snake());
        content = content.replace(
            "[dependencies]",
            &format!(
                "[dependencies]\n_base_repo_{}_{} = {{ path = \"../{}\" }}",
                db, unified, unified
            ),
        );
    }
    for unified in &filter_unified_names {
        let db = &db.to_snake();
        content = content.replace(
            "[dependencies]",
            &format!(
                "[dependencies]\n_base_filter_{}_{} = {{ path = \"../../base_filters/{}\" }}",
                db, unified, unified
            ),
        );
    }

    fs_write(file_path, &*content)?;

    let src_dir = base_dir.join("src");
    let file_path = src_dir.join("lib.rs");
    remove_files.remove(file_path.as_os_str());

    #[derive(Template)]
    #[template(path = "db/base_repositories/src/lib.rs", escape = "none")]
    struct LibTemplate<'a> {
        pub config: &'a ConfigDef,
        pub groups: &'a GroupsDef,
    }

    let tpl = LibTemplate { config, groups };
    fs_write(file_path, tpl.render()?)?;

    #[derive(Template)]
    #[template(path = "db/base_repositories/src/repositories.rs", escape = "none")]
    struct RepositoriesTemplate<'a> {
        pub groups: &'a GroupsDef,
    }

    let file_path = src_dir.join("repositories.rs");
    remove_files.remove(file_path.as_os_str());
    let tpl = RepositoriesTemplate { groups };
    fs_write(file_path, tpl.render()?)?;

    let model_models_dir = src_dir.join("repositories");
    for (group_name, defs) in groups {
        let mod_names: BTreeSet<String> = defs
            .iter()
            .filter(|(_, d)| !d.abstract_mode)
            .map(|(_, d)| d.mod_name())
            .collect();
        let unified_names: BTreeSet<(String, String)> = unified_group
            .nodes
            .iter()
            .filter(|((g, _), mark)| g.as_str().eq(group_name) && *mark == &analyzer::Mark::Ref)
            .map(|((_, model_name), _)| {
                let u = UnifiedGroup::unified_name_from_rel(
                    unified_groups,
                    &[group_name.to_string(), model_name.to_string()],
                );
                (u, model_name.as_str().to_snake())
            })
            .collect();

        let mut base_output = String::new();
        let model_group_dir = model_models_dir.join(group_name.to_snake());
        let model_group_base_dir = model_group_dir.join("_base");
        for (model_name, def) in defs {
            let table_name = def.table_name();
            let mod_name = def.mod_name();
            let mod_name = &mod_name;
            if !def.abstract_mode {
                let mut force_indexes = Vec::new();
                if is_mysql_mode() {
                    let (_, _, idx_map) = crate::migration_generator::make_table_def(def, config)?;
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
                            let idx: String = format!("{:?}", filters::_to_db_col(idx, true));
                            force_indexes.push((cond.join(" && "), idx));
                        }
                    }
                }

                #[derive(Template)]
                #[template(
                    path = "db/base_repositories/src/group/base/_table.rs",
                    escape = "none"
                )]
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
                    pub is_mysql_str: &'a str,
                    pub unified_group: &'a UnifiedGroup,
                    pub unified_groups: &'a [UnifiedGroup],
                    pub unified_filter_group: &'a str,
                }

                let tpl = GroupBaseTableTemplate {
                    db,
                    group_name,
                    mod_name,
                    model_name,
                    pascal_name: &model_name.to_pascal(),
                    id_name: &to_id_name(model_name),
                    table_name: &table_name,
                    def,
                    force_indexes,
                    config,
                    version_col: ConfigDef::version(),
                    is_mysql_str: if is_mysql_mode() { "true" } else { "false" },
                    unified_group,
                    unified_groups,
                    unified_filter_group: filter_unified_map
                        .get(&(group_name.clone(), model_name.clone()))
                        .unwrap(),
                };
                let ret = tpl.render()?;
                if SEPARATED_BASE_FILES {
                    let file_path = model_group_base_dir.join(format!("_{}.rs", mod_name));
                    remove_files.remove(file_path.as_os_str());
                    fs_write(file_path, format!("{}{}", OVERWRITTEN_MSG, ret))?;
                } else {
                    base_output.push_str(&format!("pub mod _{} {{\n{}}}\n", mod_name, ret));
                }
            }
        }

        for (u, m) in &unified_names {
            base_output.push_str(&format!(
                "pub use _base_repo_{}_{u}::repositories::{}::_base::_{m};\n",
                db.to_snake(),
                group_name.to_snake().to_ident()
            ));
        }

        let file_path = model_models_dir.join(format!("{}.rs", group_name.to_snake()));
        remove_files.remove(file_path.as_os_str());
        let concrete_models: IndexMap<&String, &Arc<ModelDef>> =
            defs.iter().filter(|(_k, v)| !v.abstract_mode).collect();

        #[derive(Template)]
        #[template(path = "db/base_repositories/src/group.rs", escape = "none")]
        struct GroupTemplate<'a> {
            pub db: &'a str,
            pub group_name: &'a str,
            pub mod_names: &'a BTreeSet<String>,
            pub unified_names: &'a BTreeSet<(String, String)>,
            pub models: IndexMap<&'a String, &'a Arc<ModelDef>>,
            pub config: &'a ConfigDef,
            pub base_output: String,
        }

        let tpl = GroupTemplate {
            db,
            group_name,
            mod_names: &mod_names,
            unified_names: &unified_names,
            models: concrete_models,
            config,
            base_output,
        };
        fs_write(file_path, tpl.render()?)?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn write_group_files(
    db_repositories_dir: &Path,
    db: &str,
    config: &ConfigDef,
    group: &str,
    groups: &GroupsDef,
    unified_groups: &[UnifiedGroup],
    unified_joinable_groups: &[UnifiedGroup],
    force: bool,
    exclude_from_domain: bool,
    remove_files: &mut HashSet<OsString>,
) -> Result<()> {
    let base_dir = db_repositories_dir.join(group.to_snake());
    let file_path = base_dir.join("Cargo.toml");
    remove_files.remove(file_path.as_os_str());
    let act_as_session = groups
        .values()
        .any(|g| g.values().any(|m| m.act_as_session()));
    let mut content = if force || !file_path.exists() {
        #[derive(Template)]
        #[template(path = "db/repositories/_Cargo.toml", escape = "none")]
        struct Template<'a> {
            db: &'a str,
            group: &'a str,
            act_as_session: bool,
        }
        Template {
            db,
            group,
            act_as_session,
        }
        .render()?
    } else {
        fs::read_to_string(&file_path)?.replace("\r\n", "\n")
    };
    let reg = Regex::new(r"(?m)^_base_repo_\w+\s*=.+\n")?;
    content = reg.replace_all(&content, "").into_owned();
    let reg = Regex::new(r"(?m)^_base_filter_\w+\s*=.+\n")?;
    content = reg.replace_all(&content, "").into_owned();
    for group in unified_joinable_groups.iter().rev() {
        let db = &db.to_snake();
        let group = &group.unified_name();
        content = content.replace(
            "[dependencies]",
            &format!(
                "[dependencies]\n_base_repo_{}_{} = {{ path = \"../../base_repositories/{}\" }}",
                db, group, group
            ),
        );
    }
    for group in unified_groups.iter().rev() {
        let db = &db.to_snake();
        let group = &group.unified_name();
        content = content.replace(
            "[dependencies]",
            &format!(
                "[dependencies]\n_base_filter_{}_{} = {{ path = \"../../base_filters/{}\" }}",
                db, group, group
            ),
        );
    }
    fs_write(file_path, &*content)?;

    let src_dir = base_dir.join("src");
    let file_path = src_dir.join("lib.rs");
    remove_files.remove(file_path.as_os_str());
    if force || !file_path.exists() {
        #[derive(Template)]
        #[template(path = "db/repositories/src/lib.rs", escape = "none")]
        struct LibTemplate<'a> {
            pub config: &'a ConfigDef,
        }

        let tpl = LibTemplate { config };
        fs_write(file_path, tpl.render()?)?;
    }

    #[derive(Template)]
    #[template(path = "db/repositories/src/repositories.rs", escape = "none")]
    struct RepositoriesTemplate<'a> {
        pub groups: &'a GroupsDef,
    }

    let file_path = src_dir.join("repositories.rs");
    remove_files.remove(file_path.as_os_str());
    let tpl = RepositoriesTemplate { groups };
    fs_write(file_path, tpl.render()?)?;

    if !exclude_from_domain {
        impl_domain::write_impl_domain_rs(&src_dir, db, group, groups, force, remove_files)?;
    }
    let model_models_dir = src_dir.join("repositories");
    let impl_domain_dir = src_dir.join("impl_domain");
    for (group_name, defs) in groups {
        let mod_names: BTreeSet<String> = defs
            .iter()
            .filter(|(_, d)| !d.abstract_mode)
            .map(|(_, d)| d.mod_name())
            .collect();
        let unified_names: BTreeSet<(String, String)> = defs
            .iter()
            .filter(|(_, d)| !d.abstract_mode)
            .map(|(model_name, d)| {
                let u = UnifiedGroup::unified_name_from_rel(
                    unified_joinable_groups,
                    &[group_name.to_string(), model_name.to_string()],
                );
                (u, d.mod_name())
            })
            .collect();
        let entities_mod_names: BTreeSet<(String, &String)> = defs
            .iter()
            .filter(|(_, d)| !d.abstract_mode)
            .map(|(model_name, def)| (def.mod_name(), model_name))
            .collect();

        let file_path = model_models_dir.join(format!("{}.rs", group_name.to_snake()));
        remove_files.remove(file_path.as_os_str());

        #[derive(Template)]
        #[template(path = "db/repositories/src/group.rs", escape = "none")]
        struct GroupTemplate<'a> {
            pub db: &'a str,
            pub group_name: &'a str,
            pub mod_names: &'a BTreeSet<String>,
            pub unified_names: &'a BTreeSet<(String, String)>,
        }

        let tpl = GroupTemplate {
            db,
            group_name,
            mod_names: &mod_names,
            unified_names: &unified_names,
        };
        fs_write(file_path, tpl.render()?)?;

        if !exclude_from_domain {
            impl_domain::write_group_rs(
                &impl_domain_dir,
                db,
                group_name,
                &entities_mod_names,
                force,
                remove_files,
            )?;
        }

        let mut impl_output = String::new();
        impl_output.push_str(OVERWRITTEN_MSG);

        let model_group_dir = model_models_dir.join(group_name.to_snake());
        for (model_name, def) in defs {
            let mod_name = def.mod_name();
            let mod_name = &mod_name;
            let unified = unified_joinable_groups.iter().find(|v| {
                v.nodes
                    .contains_key(&(group_name.into(), model_name.into()))
            });
            let unified_name = unified.unwrap().unified_name();
            if !def.abstract_mode {
                let file_path = model_group_dir.join(format!("{}.rs", mod_name));
                remove_files.remove(file_path.as_os_str());
                if force || !file_path.exists() {
                    #[derive(Template)]
                    #[template(path = "db/repositories/src/group/table.rs", escape = "none")]
                    struct GroupTableTemplate<'a> {
                        pub db: &'a str,
                        pub group_name: &'a str,
                        pub mod_name: &'a str,
                        pub pascal_name: &'a str,
                        pub id_name: &'a str,
                        pub def: &'a Arc<ModelDef>,
                        pub config: &'a ConfigDef,
                        pub unified_name: String,
                    }

                    let tpl = GroupTableTemplate {
                        db,
                        group_name,
                        mod_name,
                        pascal_name: &model_name.to_pascal(),
                        id_name: &to_id_name(model_name),
                        def,
                        config,
                        unified_name,
                    };
                    fs_write(file_path, tpl.render()?)?;
                }

                let mut force_indexes = Vec::new();
                if is_mysql_mode() {
                    let (_, _, idx_map) = crate::migration_generator::make_table_def(def, config)?;
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
                }

                let unified_filter_group = &UnifiedGroup::unified_name_from_rel(
                    unified_groups,
                    &[group_name.to_string(), model_name.to_string()],
                );
                if !exclude_from_domain {
                    impl_output.push_str(&impl_domain::write_entity(
                        &impl_domain_dir,
                        db,
                        config,
                        group_name,
                        mod_name,
                        force,
                        model_name,
                        def,
                        unified_filter_group,
                        remove_files,
                    )?);
                }
            }
        }
        if !SEPARATED_BASE_FILES {
            let group_dir = impl_domain_dir.join(group_name.to_snake());
            let file_path = group_dir.join("_base.rs");
            remove_files.remove(file_path.as_os_str());
            fs_write(file_path, impl_output)?;
        }
    }
    Ok(())
}
