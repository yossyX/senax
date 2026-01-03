use crate::common::ToCase as _;
use crate::common::{AtomicLoad as _, OVERWRITTEN_MSG};
use crate::model_generator::analyzer::{self, UnifiedGroup};
use crate::schema::{ConfigDef, GroupsDef};
use crate::{SEPARATED_BASE_FILES, filters};
use crate::{
    common::fs_write,
    schema::{ModelDef, set_domain_mode, to_id_name},
};
use crate::schema::Joinable;
use anyhow::{Result, ensure};
use askama::Template;
use indexmap::IndexMap;
use regex::Regex;
use std::{
    collections::{BTreeSet, HashSet},
    ffi::OsString,
    fs,
    path::Path,
    sync::Arc,
};

#[allow(clippy::regex_creation_in_loops)]
#[allow(clippy::too_many_arguments)]
pub fn write_group_files(
    domain_base_relations_dir: &Path,
    db: &str,
    config: &ConfigDef,
    unified_name: &str,
    groups: &GroupsDef,
    unified_group: &UnifiedGroup,
    unified_groups: &Vec<UnifiedGroup>,
    ref_db: &BTreeSet<(String, String)>,
    force: bool,
    remove_files: &mut HashSet<OsString>,
) -> Result<()> {
    let base_dir = domain_base_relations_dir.join(unified_name);
    let file_path = base_dir.join("Cargo.toml");
    remove_files.remove(file_path.as_os_str());
    let mut content = if force || !file_path.exists() {
        #[derive(Template)]
        #[template(path = "domain/base_relations/groups/_Cargo.toml", escape = "none")]
        struct Template<'a> {
            db: &'a str,
            unified_name: &'a str,
        }
        Template { db, unified_name }.render()?
    } else {
        fs::read_to_string(&file_path)?.replace("\r\n", "\n")
    };
    let reg = Regex::new(r"(?m)^base_relations_\w+\s*=.+\n")?;
    content = reg.replace_all(&content, "").into_owned();
    let mut done = Vec::new();
    for (db, group) in ref_db {
        let db = &db.to_snake();
        let group = &group.to_snake();
        if done.contains(group) {
            continue;
        }
        done.push(group.clone());
        content = content.replace(
            "[dependencies]",
            &format!(
                "[dependencies]\nbase_relations_{} = {{ path = \"../../../{}\" }}",
                db, db
            ),
        );
    }
    for (g, m) in &unified_group.ref_unified_groups {
        let db = &db.to_snake();
        let unified = format!("{}__{}", g.to_snake(), m.to_snake());
        content = content.replace(
            "[dependencies]",
            &format!(
                "[dependencies]\nbase_relations_{}_{} = {{ path = \"../{}\" }}",
                db, unified, unified
            ),
        );
    }
    fs_write(file_path, &*content)?;

    let src_dir = base_dir.join("src");
    let file_path = src_dir.join("lib.rs");
    remove_files.remove(file_path.as_os_str());
    if force || !file_path.exists() {
        #[derive(Template)]
        #[template(path = "domain/base_relations/groups/src/lib.rs", escape = "none")]
        struct Template;
        let content = Template.render()?;
        fs_write(file_path, &*content)?;
    }

    let file_path = src_dir.join("relations.rs");
    remove_files.remove(file_path.as_os_str());
    let content = if force || !file_path.exists() {
        #[derive(Template)]
        #[template(
            path = "domain/base_relations/groups/src/relations.rs",
            escape = "none"
        )]
        struct Template;
        Template.render()?
    } else {
        fs::read_to_string(&file_path)?.replace("\r\n", "\n")
    };
    {
        #[derive(Template)]
        #[template(
            source = r###"
// Do not modify below this line. (ModStart)
@%- for (name, defs) in groups %@
pub mod @{ name|snake|ident }@;
@%- endfor %@
// Do not modify above this line. (ModEnd)"###,
            ext = "txt",
            escape = "none"
        )]
        struct ModTemplate<'a> {
            pub db: &'a str,
            pub groups: &'a GroupsDef,
        }

        let re = Regex::new(r"(?s)// Do not modify below this line. \(ModStart\).+// Do not modify above this line. \(ModEnd\)").unwrap();
        ensure!(
            re.is_match(&content),
            "File contents are invalid.: {:?}",
            &file_path
        );
        let tpl = ModTemplate { db, groups }.render()?;
        let tpl = tpl.trim_start();
        let content = re.replace(&content, tpl);

        fs_write(file_path, &*content)?;
    }

    let repositories_dir = src_dir.join("relations");
    for (group_name, defs) in groups {
        let mod_names: BTreeSet<String> = defs
            .iter()
            .filter(|(_k, v)| !v.abstract_mode)
            .map(|(_, d)| d.mod_name())
            .collect();
        let mod_names = &mod_names;
        let entities_mod_names: BTreeSet<(String, &String)> = defs
            .iter()
            .filter(|(_, d)| !d.abstract_mode)
            .map(|(model_name, def)| (def.mod_name(), model_name))
            .collect();
        let entities_mod_names = &entities_mod_names;
        let file_path = repositories_dir.join(format!("{}.rs", group_name.to_snake()));
        remove_files.remove(file_path.as_os_str());
        let mut output = String::new();
        output.push_str(OVERWRITTEN_MSG);
        for (model_name, def) in defs {
            let group_name = group_name;
            let mod_name = def.mod_name();
            let mod_name = &mod_name;
            if !def.abstract_mode {
                output.push_str(&write_entity(
                    &repositories_dir,
                    db,
                    config,
                    group_name,
                    mod_name,
                    force,
                    model_name,
                    def,
                    remove_files,
                )?);
            }
        }
        if SEPARATED_BASE_FILES {
            #[derive(Template)]
            #[template(
                path = "domain/base_relations/groups/src/relations/group.rs",
                escape = "none"
            )]
            struct Template<'a> {
                pub db: &'a str,
                pub mod_names: &'a BTreeSet<String>,
            }
            output = Template { db, mod_names }.render()?;
        }
        for ((g, m), mark) in &unified_group.nodes {
            if g.eq(group_name) && *mark == analyzer::Mark::Ref {
                let u = UnifiedGroup::unified_name_from_rel(
                    unified_groups,
                    &[g.to_string(), m.to_string()],
                );
                output.push_str(&format!(
                    "pub use base_relations_{}_{}::relations::{}::{};\n",
                    db.to_snake(),
                    u,
                    g.to_snake().to_ident(),
                    m.to_snake().to_ident()
                ));
            }
        }
        fs_write(file_path, output)?;
    }
    Ok(())
}

pub fn write_lib_rs(
    domain_base_relations_src_dir: &Path,
    db: &str,
    groups: &GroupsDef,
    unified_groups: &Vec<UnifiedGroup>,
    force: bool,
) -> Result<()> {
    let file_path = domain_base_relations_src_dir.join("lib.rs");
    let content = if force || !file_path.exists() {
        #[derive(Template)]
        #[template(path = "domain/base_relations/src/lib.rs", escape = "none")]
        pub struct LibTemplate<'a> {
            pub db: &'a str,
        }

        LibTemplate { db }.render()?
    } else {
        fs::read_to_string(&file_path)?.replace("\r\n", "\n")
    };

    #[derive(Template)]
    #[template(
        source = r###"
// Do not modify below this line. (ModStart)
@%- for (group, models) in mods %@
pub mod @{ group|snake|ident }@ {
@%- for (model, unified) in models %@
    pub use base_relations_@{ db|snake }@_@{ unified }@::relations::@{ group|snake|ident }@::@{ model|snake|ident }@ as @{ model|snake|ident }@;
@%- endfor %@
}
@%- endfor %@
// Do not modify above this line. (ModEnd)"###,
        ext = "txt",
        escape = "none"
    )]
    pub struct ModTemplate<'a> {
        pub db: &'a str,
        pub mods: IndexMap<String, IndexMap<String, String>>,
    }

    let mut mods = IndexMap::new();
    for (group_name, defs) in groups {
        let mut models = IndexMap::new();
        for (model_name, _) in defs {
            let unified = unified_groups.iter().find(|v| {
                v.nodes
                    .contains_key(&(group_name.into(), model_name.into()))
            });
            models.insert(model_name.clone(), unified.unwrap().unified_name());
        }
        mods.insert(group_name.clone(), models);
    }

    let re = Regex::new(r"(?s)// Do not modify below this line. \(ModStart\).+// Do not modify above this line. \(ModEnd\)").unwrap();
    ensure!(
        re.is_match(&content),
        "File contents are invalid.: {:?}",
        &file_path
    );
    let tpl = ModTemplate { db, mods }.render()?;
    let tpl = tpl.trim_start();
    let content = re.replace(&content, tpl);

    fs_write(file_path, &*content)?;
    Ok(())
}

pub fn write_cargo_toml(
    domain_base_relations_dir: &Path,
    db: &str,
    unified_groups: &Vec<UnifiedGroup>,
    force: bool,
) -> Result<()> {
    let file_path = domain_base_relations_dir.join("Cargo.toml");
    let mut content = if force || !file_path.exists() {
        #[derive(Template)]
        #[template(path = "domain/base_relations/_Cargo.toml", escape = "none")]
        pub struct CargoTemplate<'a> {
            pub db: &'a str,
        }

        CargoTemplate { db }.render()?
    } else {
        fs::read_to_string(&file_path)?.replace("\r\n", "\n")
    };
    let reg = Regex::new(r"(?m)^base_relations_\w+\s*=.+\n")?;
    content = reg.replace_all(&content, "").into_owned();
    for group in unified_groups.iter().rev() {
        let db = &db.to_snake();
        let group = &group.unified_name();
        content = content.replace(
            "[dependencies]",
            &format!(
                "[dependencies]\nbase_relations_{}_{} = {{ path = \"groups/{}\" }}",
                db, group, group
            ),
        );
    }
    fs_write(file_path, &*content)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn write_entity(
    repositories_dir: &Path,
    db: &str,
    config: &ConfigDef,
    group_name: &str,
    mod_name: &str,
    force: bool,
    model_name: &str,
    def: &Arc<ModelDef>,
    remove_files: &mut HashSet<OsString>,
) -> Result<String, anyhow::Error> {
    set_domain_mode(true);
    let domain_group_dir = repositories_dir.join(group_name.to_snake());
    let pascal_name = &model_name.to_pascal();

    #[derive(Template)]
    #[template(
        path = "domain/base_relations/groups/src/relations/entities/entity.rs",
        escape = "none"
    )]
    pub struct DomainEntityTemplate<'a> {
        pub db: &'a str,
        pub config: &'a ConfigDef,
        pub group_name: &'a str,
        pub mod_name: &'a str,
        pub model_name: &'a str,
        pub pascal_name: &'a str,
        pub def: &'a Arc<ModelDef>,
    }

    let tpl = DomainEntityTemplate {
        db,
        config,
        group_name,
        mod_name,
        model_name,
        pascal_name,
        def,
    };
    let ret = tpl.render()?;
    set_domain_mode(false);
    if SEPARATED_BASE_FILES {
        let file_path = domain_group_dir.join(format!("{}.rs", mod_name));
        remove_files.remove(file_path.as_os_str());
        fs_write(file_path, format!("{}{}", OVERWRITTEN_MSG, ret))?;
        Ok("".to_string())
    } else {
        Ok(format!("pub mod {} {{\n{}}}\n", mod_name, ret))
    }
}
