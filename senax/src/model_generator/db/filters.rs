use anyhow::Result;
use askama::Template;
use std::{
    collections::{BTreeSet, HashSet},
    ffi::OsString,
    path::Path,
    sync::Arc,
};

use crate::model_generator::analyzer::UnifiedGroup;
use crate::schema::Joinable;
use crate::schema::is_mysql_mode;
use crate::schema::{ConfigDef, GroupsDef, ModelDef, StringOrArray};
use crate::{SEPARATED_BASE_FILES, filters};
use crate::{
    common::{OVERWRITTEN_MSG, ToCase as _, fs_write},
    model_generator::analyzer,
};

#[allow(clippy::too_many_arguments)]
pub fn write_base_group_files(
    db_repositories_dir: &Path,
    db: &str,
    config: &ConfigDef,
    unified_name: &str,
    groups: &GroupsDef,
    unified_group: &UnifiedGroup,
    unified_groups: &[UnifiedGroup],
    ref_db: &BTreeSet<(String, String)>,
    remove_files: &mut HashSet<OsString>,
) -> Result<()> {
    let base_dir = db_repositories_dir.join(unified_name);
    let file_path = base_dir.join("Cargo.toml");
    remove_files.remove(file_path.as_os_str());

    #[derive(Template)]
    #[template(path = "db/base_filters/_Cargo.toml", escape = "none")]
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
                "[dependencies]\n_base_filter_{}_{} = {{ path = \"../{}\" }}",
                db, unified, unified
            ),
        );
    }
    fs_write(file_path, &*content)?;

    let src_dir = base_dir.join("src");
    let file_path = src_dir.join("lib.rs");
    remove_files.remove(file_path.as_os_str());
    #[derive(Template)]
    #[template(path = "db/base_filters/src/lib.rs", escape = "none")]
    struct LibTemplate<'a> {
        pub config: &'a ConfigDef,
        pub groups: &'a GroupsDef,
    }

    let tpl = LibTemplate { config, groups };
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
                            let idx = format!("{:?}", filters::_to_db_col(idx, true));
                            force_indexes.push((cond.join(" && "), idx));
                        }
                    }
                }

                #[derive(Template)]
                #[template(path = "db/base_filters/src/group/base/_table.rs", escape = "none")]
                struct GroupBaseTableTemplate<'a> {
                    pub db: &'a str,
                    pub group_name: &'a str,
                    pub mod_name: &'a str,
                    pub model_name: &'a str,
                    pub pascal_name: &'a str,
                    pub table_name: &'a str,
                    pub def: &'a Arc<ModelDef>,
                    pub config: &'a ConfigDef,
                    pub unified_group: &'a UnifiedGroup,
                    pub unified_groups: &'a [UnifiedGroup],
                }

                let tpl = GroupBaseTableTemplate {
                    db,
                    group_name,
                    mod_name,
                    model_name,
                    pascal_name: &model_name.to_pascal(),
                    table_name: &table_name,
                    def,
                    config,
                    unified_group,
                    unified_groups,
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

        let file_path = model_models_dir.join(format!("{}.rs", group_name.to_snake()));
        remove_files.remove(file_path.as_os_str());

        #[derive(Template)]
        #[template(path = "db/base_filters/src/group.rs", escape = "none")]
        struct GroupTemplate<'a> {
            pub db: &'a str,
            pub group_name: &'a str,
            pub mod_names: &'a BTreeSet<String>,
            pub unified_names: &'a BTreeSet<(String, String)>,
            pub base_output: String,
        }

        let tpl = GroupTemplate {
            db,
            group_name,
            mod_names: &mod_names,
            unified_names: &unified_names,
            base_output,
        };
        fs_write(file_path, tpl.render()?)?;
    }
    Ok(())
}
