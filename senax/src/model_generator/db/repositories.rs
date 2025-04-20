use crate::common::fs_write;
use crate::filters;
use crate::schema::{ConfigDef, ModelDef, StringOrArray, Timestampable, to_id_name};
use anyhow::Result;
use askama::Template;
use compact_str::CompactString;
use convert_case::{Case, Casing as _};
use indexmap::IndexMap;
use std::{
    collections::{BTreeSet, HashSet},
    ffi::OsString,
    path::Path,
    sync::Arc,
};

mod impl_domain;

// pub fn write_db_rs(
//     model_src_dir: &Path,
//     db: &str,
//     groups: &IndexMap<String, IndexMap<String, Arc<ModelDef>>>,
//     force: bool,
// ) -> Result<()> {
//     let file_path = model_src_dir.join("impl_domain.rs");
//     let content = if force || !file_path.exists() {
//         #[derive(Template)]
//         #[template(path = "model/repositories/src/impl_domain.rs", escape = "none")]
//         pub struct ImplDomainDbTemplate<'a> {
//             pub db: &'a str,
//         }

//         ImplDomainDbTemplate { db }.render()?
//     } else {
//         fs::read_to_string(&file_path)?
//     };

//     let re = Regex::new(r"(?s)// Do not modify below this line. \(ModStart\).+// Do not modify up to this line. \(ModEnd\)").unwrap();
//     ensure!(
//         re.is_match(&content),
//         "File contents are invalid.: {:?}",
//         &file_path
//     );

// #[derive(Template)]
// #[template(
//     source = r###"
// // Do not modify below this line. (ModStart)
// @%- for (name, defs) in groups %@
// pub mod @{ name|snake|to_var_name }@;
// pub static NEW_@{ name|upper }@_REPO: OnceCell<Box<dyn Fn(&Arc<Mutex<DbConn>>) -> Box<dyn _repository::@{ name|snake|to_var_name }@::@{ name|pascal }@Repository> + Send + Sync>> = OnceCell::new();
// pub static NEW_@{ name|upper }@_QS: OnceCell<Box<dyn Fn(&Arc<Mutex<DbConn>>) -> Box<dyn _repository::@{ name|snake|to_var_name }@::@{ name|pascal }@QueryService> + Send + Sync>> = OnceCell::new();
// @%- endfor %@
// // Do not modify up to this line. (ModEnd)"###,
//     ext = "txt",
//     escape = "none"
// )]
// pub struct ModTemplate<'a> {
//     pub groups: &'a IndexMap<String, IndexMap<String, Arc<ModelDef>>>,
// }

//     let tpl = ModTemplate { groups }.render()?;
//     let tpl = tpl.trim_start();
//     let content = re.replace(&content, tpl);

//     let re = Regex::new(r"(?s)// Do not modify below this line. \(RepoStart\).+// Do not modify up to this line. \(RepoEnd\)").unwrap();
//     ensure!(
//         re.is_match(&content),
//         "File contents are invalid.: {:?}",
//         &file_path
//     );

// #[derive(Template)]
// #[template(
//     source = r###"
//     // Do not modify below this line. (RepoStart)
//     @%- for (name, defs) in groups %@
//     get_repo!(@{ name|snake|to_var_name }@, dyn _repository::@{ name|snake|to_var_name }@::@{ name|pascal }@Repository, NEW_@{ name|upper }@_REPO, "The @{ name|pascal }@Repository is not configured.");
//     @%- endfor %@
//     // Do not modify up to this line. (RepoEnd)"###,
//     ext = "txt",
//     escape = "none"
// )]
// pub struct ImplDomainDbRepoTemplate<'a> {
//     pub groups: &'a IndexMap<String, IndexMap<String, Arc<ModelDef>>>,
// }

//     let tpl = ImplDomainDbRepoTemplate { groups }.render()?;
//     let tpl = tpl.trim_start();
//     let content = re.replace(&content, tpl);

//     let re = Regex::new(r"(?s)// Do not modify below this line. \(QueryServiceStart\).+// Do not modify up to this line. \(QueryServiceEnd\)").unwrap();
//     ensure!(
//         re.is_match(&content),
//         "File contents are invalid.: {:?}",
//         &file_path
//     );

// #[derive(Template)]
// #[template(
//     source = r###"
//     // Do not modify below this line. (QueryServiceStart)
//     @%- for (name, defs) in groups %@
//     get_repo!(@{ name|snake|to_var_name }@, dyn _repository::@{ name|snake|to_var_name }@::@{ name|pascal }@QueryService, NEW_@{ name|upper }@_QS, "The @{ name|pascal }@QueryService is not configured.");
//     @%- endfor %@
//     // Do not modify up to this line. (QueryServiceEnd)"###,
//     ext = "txt",
//     escape = "none"
// )]
// pub struct ImplDomainDbQueryServiceTemplate<'a> {
//     pub groups: &'a IndexMap<String, IndexMap<String, Arc<ModelDef>>>,
// }

//     let tpl = ImplDomainDbQueryServiceTemplate { groups }.render()?;
//     let tpl = tpl.trim_start();
//     let content = re.replace(&content, tpl);

//     fs_write(file_path, &*content)?;
//     Ok(())
// }

pub fn write_group_files(
    db_repositories_dir: &Path,
    db: &str,
    group: &str,
    groups: &IndexMap<String, IndexMap<String, Arc<ModelDef>>>,
    config: &ConfigDef,
    force: bool,
    exclude_from_domain: bool,
    remove_files: &mut HashSet<OsString>,
) -> Result<()> {
    let base_dir = db_repositories_dir.join(group.to_case(Case::Snake));
    let file_path = base_dir.join("Cargo.toml");
    remove_files.remove(file_path.as_os_str());
    if force || !file_path.exists() {
        #[derive(Template)]
        #[template(path = "model/repositories/_Cargo.toml", escape = "none")]
        struct Template<'a> {
            db: &'a str,
            group: &'a str,
        }
        let content = Template { db, group }.render()?;
        fs_write(file_path, &*content)?;
    }
    let src_dir = base_dir.join("src");
    let file_path = src_dir.join("lib.rs");
    remove_files.remove(file_path.as_os_str());
    if force || !file_path.exists() {
        #[derive(Template)]
        #[template(path = "model/repositories/src/lib.rs", escape = "none")]
        struct LibTemplate<'a> {
            pub group: &'a str,
            pub config: &'a ConfigDef,
        }

        let tpl = LibTemplate { group, config };
        fs_write(file_path, tpl.render()?)?;
    }

    #[derive(Template)]
    #[template(path = "model/repositories/src/repositories.rs", escape = "none")]
    struct RepositoriesTemplate<'a> {
        pub groups: &'a IndexMap<String, IndexMap<String, Arc<ModelDef>>>,
    }

    let file_path = src_dir.join("repositories.rs");
    remove_files.remove(file_path.as_os_str());
    let tpl = RepositoriesTemplate { groups };
    fs_write(file_path, tpl.render()?)?;

    #[derive(Template)]
    #[template(path = "model/repositories/src/misc.rs", escape = "none")]
    struct MiscTemplate<'a> {
        pub config: &'a ConfigDef,
    }

    let file_path = src_dir.join("misc.rs");
    remove_files.remove(file_path.as_os_str());
    let tpl = MiscTemplate { config };
    fs_write(file_path, tpl.render()?)?;

    if !exclude_from_domain {
        impl_domain::write_impl_domain_rs(&src_dir, db, &groups, force)?;
    }

    for (group_name, defs) in groups {
        let model_models_dir = src_dir.join("repositories");
        let mod_names: BTreeSet<String> = defs
            .iter()
            .filter(|(_, d)| !d.abstract_mode)
            .map(|(_, d)| d.mod_name())
            .collect();
        let entities_mod_names: BTreeSet<(String, &String)> = defs
            .iter()
            .filter(|(_, d)| !d.abstract_mode)
            .map(|(model_name, def)| (def.mod_name(), model_name))
            .collect();

        let file_path = model_models_dir.join(format!("{}.rs", group_name));
        remove_files.remove(file_path.as_os_str());
        let concrete_models = defs.iter().filter(|(_k, v)| !v.abstract_mode).collect();

        #[derive(Template)]
        #[template(path = "model/repositories/src/group.rs", escape = "none")]
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

        let impl_domain_dir = src_dir.join("impl_domain");
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

        let visibility = if config.export_db_layer {
            ""
        } else {
            "(crate)"
        };
        let model_group_dir = model_models_dir.join(group_name);
        let model_group_base_dir = model_group_dir.join("_base");
        for (model_name, def) in defs {
            let table_name = def.table_name();
            let mod_name = def.mod_name();
            let mod_name = &mod_name;
            if !def.abstract_mode {
                let file_path = model_group_dir.join(format!("{}.rs", mod_name));
                remove_files.remove(file_path.as_os_str());
                if force || !file_path.exists() {
                    #[derive(Template)]
                    #[template(path = "model/repositories/src/group/table.rs", escape = "none")]
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
                #[template(path = "model/repositories/src/group/base/_table.rs", escape = "none")]
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
                    version_col: ConfigDef::version(),
                    visibility,
                };
                fs_write(file_path, tpl.render()?)?;

                if !exclude_from_domain {
                    impl_domain::write_entity(
                        &impl_domain_dir,
                        db,
                        &config,
                        group_name,
                        mod_name,
                        force,
                        model_name,
                        def,
                        remove_files,
                    )?;
                }
            }
        }
    }

    Ok(())
}
