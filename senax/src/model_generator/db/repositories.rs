use anyhow::Result;
use askama::Template;
use compact_str::CompactString;
use indexmap::IndexMap;
use regex::Regex;
use std::sync::atomic::AtomicUsize;
use std::{
    collections::{BTreeSet, HashSet},
    ffi::OsString,
    fs,
    path::Path,
    sync::Arc,
};

use crate::common::AtomicLoad as _;
use crate::common::ToCase as _;
use crate::common::{OVERWRITTEN_MSG, fs_write};
use crate::model_generator::REL_START;
use crate::schema::IS_MAIN_GROUP;
use crate::schema::{ConfigDef, GroupsDef, ModelDef, StringOrArray, Timestampable, to_id_name};
use crate::{SEPARATED_BASE_FILES, filters};

mod impl_domain;

#[allow(clippy::too_many_arguments)]
pub fn write_group_files(
    db_repositories_dir: &Path,
    db: &str,
    group: &str,
    groups: &GroupsDef,
    ref_groups: &[String],
    config: &ConfigDef,
    force: bool,
    exclude_from_domain: bool,
    remove_files: &mut HashSet<OsString>,
) -> Result<()> {
    let base_dir = db_repositories_dir.join(group.to_snake());
    let file_path = base_dir.join("Cargo.toml");
    remove_files.remove(file_path.as_os_str());
    let mut content = if force || !file_path.exists() {
        #[derive(Template)]
        #[template(path = "db/repositories/_Cargo.toml", escape = "none")]
        struct Template<'a> {
            db: &'a str,
            group: &'a str,
        }
        Template { db, group }.render()?
    } else {
        fs::read_to_string(&file_path)?.replace("\r\n", "\n")
    };
    let reg = Regex::new(r"(?m)^_repo_\w+\s*=.+\n")?;
    content = reg.replace_all(&content, "").into_owned();
    for group in ref_groups {
        let db = &db.to_snake();
        let group = &group.to_snake();
        content = content.replace(
            "[dependencies]",
            &format!(
                "[dependencies]\n_repo_{}_{} = {{ path = \"../{}\" }}",
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
            pub group: &'a str,
            pub config: &'a ConfigDef,
            pub groups: &'a GroupsDef,
        }

        let tpl = LibTemplate {
            group,
            config,
            groups,
        };
        fs_write(file_path, tpl.render()?)?;
    }

    #[derive(Template)]
    #[template(path = "db/repositories/src/repositories.rs", escape = "none")]
    struct RepositoriesTemplate<'a> {
        pub db: &'a str,
        pub groups: &'a GroupsDef,
        pub ref_groups: &'a [String],
    }

    let file_path = src_dir.join("repositories.rs");
    remove_files.remove(file_path.as_os_str());
    let tpl = RepositoriesTemplate {
        db,
        groups,
        ref_groups,
    };
    fs_write(file_path, tpl.render()?)?;

    #[derive(Template)]
    #[template(path = "db/repositories/src/misc.rs", escape = "none")]
    struct MiscTemplate<'a> {
        pub config: &'a ConfigDef,
    }

    let file_path = src_dir.join("misc.rs");
    remove_files.remove(file_path.as_os_str());
    let tpl = MiscTemplate { config };
    fs_write(file_path, tpl.render()?)?;

    if !exclude_from_domain {
        impl_domain::write_impl_domain_rs(&src_dir, db, group, groups, force, remove_files)?;
    }
    let model_models_dir = src_dir.join("repositories");
    let impl_domain_dir = src_dir.join("impl_domain");
    let base_group_name = group;
    for (group_name, (f, defs)) in groups {
        let is_main_group = f.relaxed_load() == REL_START;
        IS_MAIN_GROUP.relaxed_store(is_main_group);
        let mod_names: BTreeSet<String> = defs
            .iter()
            .filter(|(_, (_, d))| !d.abstract_mode)
            .map(|(_, (_, d))| d.mod_name())
            .collect();
        let entities_mod_names: BTreeSet<(String, &String)> = defs
            .iter()
            .filter(|(_, (_, d))| !d.abstract_mode)
            .map(|(model_name, (_, def))| (def.mod_name(), model_name))
            .collect();

        let file_path = model_models_dir.join(format!("{}.rs", group_name.to_snake()));
        remove_files.remove(file_path.as_os_str());
        let concrete_models = defs
            .iter()
            .filter(|(_k, (_, v))| !v.abstract_mode)
            .collect();

        #[derive(Template)]
        #[template(path = "db/repositories/src/group.rs", escape = "none")]
        struct GroupTemplate<'a> {
            pub group_name: &'a str,
            pub mod_names: &'a BTreeSet<String>,
            pub models: IndexMap<&'a String, &'a (AtomicUsize, Arc<ModelDef>)>,
            pub config: &'a ConfigDef,
            pub is_main_group: bool,
        }

        let tpl = GroupTemplate {
            group_name,
            mod_names: &mod_names,
            models: concrete_models,
            config,
            is_main_group,
        };
        fs_write(file_path, tpl.render()?)?;

        if !exclude_from_domain {
            impl_domain::write_group_rs(
                &impl_domain_dir,
                db,
                base_group_name,
                group_name,
                &entities_mod_names,
                force,
                remove_files,
            )?;
        }

        let mut impl_output = String::new();
        impl_output.push_str(OVERWRITTEN_MSG);

        let mut output = String::new();
        output.push_str(OVERWRITTEN_MSG);

        let model_group_dir = model_models_dir.join(group_name.to_snake());
        let model_group_base_dir = model_group_dir.join("_base");
        for (model_name, (_, def)) in defs {
            let table_name = def.table_name();
            let mod_name = def.mod_name();
            let mod_name = &mod_name;
            if !def.abstract_mode {
                let file_path = model_group_dir.join(format!("{}.rs", mod_name));
                remove_files.remove(file_path.as_os_str());
                if force || !file_path.exists() {
                    #[derive(Template)]
                    #[template(path = "db/repositories/src/group/table.rs", escape = "none")]
                    struct GroupTableTemplate<'a> {
                        pub db: &'a str,
                        pub base_group_name: &'a str,
                        pub group_name: &'a str,
                        pub mod_name: &'a str,
                        pub pascal_name: &'a str,
                        pub id_name: &'a str,
                        pub def: &'a Arc<ModelDef>,
                        pub config: &'a ConfigDef,
                    }

                    let tpl = GroupTableTemplate {
                        db,
                        base_group_name,
                        group_name,
                        mod_name,
                        pascal_name: &model_name.to_pascal(),
                        id_name: &to_id_name(model_name),
                        def,
                        config,
                    };
                    fs_write(file_path, tpl.render()?)?;
                }

                let mut force_indexes = Vec::new();
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

                #[derive(Template)]
                #[template(path = "db/repositories/src/group/base/_table.rs", escape = "none")]
                struct GroupBaseTableTemplate<'a> {
                    pub db: &'a str,
                    pub base_group_name: &'a str,
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
                }

                let tpl = GroupBaseTableTemplate {
                    db,
                    base_group_name,
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
                };
                let ret = tpl.render()?;
                if SEPARATED_BASE_FILES {
                    let file_path = model_group_base_dir.join(format!("_{}.rs", mod_name));
                    remove_files.remove(file_path.as_os_str());
                    fs_write(file_path, format!("{}{}", OVERWRITTEN_MSG, ret))?;
                } else {
                    output.push_str(&format!("pub mod _{} {{\n{}}}\n", mod_name, ret));
                }

                if !exclude_from_domain {
                    impl_output.push_str(&impl_domain::write_entity(
                        &impl_domain_dir,
                        db,
                        config,
                        base_group_name,
                        group_name,
                        mod_name,
                        force,
                        model_name,
                        def,
                        remove_files,
                    )?);
                }
            }
        }
        if !SEPARATED_BASE_FILES {
            let file_path = model_group_dir.join("_base.rs");
            remove_files.remove(file_path.as_os_str());
            fs_write(file_path, output)?;

            let group_dir = impl_domain_dir.join(group_name.to_snake());
            let file_path = group_dir.join("_base.rs");
            remove_files.remove(file_path.as_os_str());
            fs_write(file_path, impl_output)?;
        }
    }
    IS_MAIN_GROUP.relaxed_store(true);
    Ok(())
}
