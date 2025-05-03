use crate::filters;
use crate::schema::GroupsDef;
use crate::{
    common::fs_write,
    schema::{FieldDef, ModelDef, VALUE_OBJECTS, set_domain_mode, to_id_name},
};
use anyhow::{Context, Result, ensure};
use askama::Template;
use convert_case::{Case, Casing as _};
use indexmap::IndexMap;
use regex::Regex;
use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    ffi::OsString,
    fs,
    path::Path,
    sync::Arc,
};

pub fn write_value_objects_rs(
    base_domain_src_dir: &Path,
    remove_files: &mut HashSet<OsString>,
    clean: bool,
    force: bool,
) -> Result<()> {
    let file_path = base_domain_src_dir.join("value_objects.rs");
    let mod_names: BTreeMap<_, _> = VALUE_OBJECTS
        .read()
        .unwrap()
        .as_ref()
        .unwrap()
        .iter()
        .map(|(name, _def)| (name.to_case(Case::Snake), name.to_case(Case::Pascal)))
        .collect();

    #[derive(Template)]
    #[template(
        source = r###"
// Do not modify below this line. (ModStart)
mod _base {
@%- for (mod_name, _) in mod_names %@
    pub mod _@{ mod_name }@;
@%- endfor %@
}
@%- for (mod_name, _) in mod_names %@
mod @{ mod_name|to_var_name }@;
@%- endfor %@
@%- for (mod_name, name) in mod_names %@
pub use @{ mod_name|to_var_name }@::@{ name }@;
@%- endfor %@
// Do not modify up to this line. (ModEnd)"###,
        ext = "txt",
        escape = "none"
    )]
    struct DomainValueObjectModsTemplate {
        pub mod_names: BTreeMap<String, String>,
    }
    let tpl = DomainValueObjectModsTemplate { mod_names }.render()?;
    let tpl = tpl.trim_start();
    if file_path.exists() {
        let content = fs::read_to_string(&file_path)?;
        let re = Regex::new(r"(?s)// Do not modify below this line. \(ModStart\).+// Do not modify up to this line. \(ModEnd\)").unwrap();
        ensure!(
            re.is_match(&content),
            "File contents are invalid.: {:?}",
            &file_path
        );
        let content = re.replace(&content, tpl);
        fs_write(file_path, &*content)?;
    } else {
        fs_write(file_path, tpl)?;
    }

    let value_objects_dir = base_domain_src_dir.join("value_objects");
    let value_objects_base_dir = value_objects_dir.join("_base");
    if clean && value_objects_dir.exists() {
        for entry in glob::glob(&format!("{}/**/*.rs", value_objects_dir.display()))? {
            match entry {
                Ok(path) => remove_files.insert(path.as_os_str().to_owned()),
                Err(_) => false,
            };
        }
    }

    #[allow(dead_code)]
    #[derive(Template)]
    #[template(path = "domain/base_domain/src/value_objects/base.rs", escape = "none")]
    struct DomainValueObjectBaseTemplate<'a> {
        pub mod_name: &'a str,
        pub pascal_name: &'a str,
        pub def: &'a FieldDef,
    }

    #[derive(Template)]
    #[template(
        path = "domain/base_domain/src/value_objects/wrapper.rs",
        escape = "none"
    )]
    struct DomainValueObjectWrapperTemplate<'a> {
        pub mod_name: &'a str,
        pub pascal_name: &'a str,
    }

    for (name, def) in VALUE_OBJECTS.read().unwrap().as_ref().unwrap() {
        let mod_name = name.to_case(Case::Snake);
        let mod_name = &mod_name;
        let file_path = value_objects_base_dir.join(format!("_{}.rs", mod_name));
        remove_files.remove(file_path.as_os_str());
        let tpl = DomainValueObjectBaseTemplate {
            mod_name,
            pascal_name: &name.to_case(Case::Pascal),
            def,
        }
        .render()?;
        fs_write(file_path, tpl)?;

        let file_path = value_objects_dir.join(format!("{}.rs", mod_name));
        remove_files.remove(file_path.as_os_str());
        let tpl = DomainValueObjectWrapperTemplate {
            mod_name,
            pascal_name: &name.to_case(Case::Pascal),
        }
        .render()?;
        if force || !file_path.exists() {
            fs_write(file_path, tpl)?;
        }
    }
    Ok(())
}

pub fn write_models_rs(base_domain_src_dir: &Path, db: &str) -> Result<()> {
    #[derive(Template)]
    #[template(
        source = r###"
pub mod @{ db|snake|to_var_name }@;
// Do not modify this line. (Mod:@{ all }@)"###,
        ext = "txt",
        escape = "none"
    )]
    pub struct ModTemplate<'a> {
        pub all: String,
        pub db: &'a str,
    }

    let file_path = base_domain_src_dir.join("models.rs");
    let mut content = if !file_path.exists() {
        #[derive(Template)]
        #[template(path = "domain/base_domain/src/models.rs", escape = "none")]
        struct DomainModelsTemplate;

        DomainModelsTemplate.render()?
    } else {
        fs::read_to_string(&file_path)?
    };
    let re = Regex::new(r"// Do not modify this line\. \(Mod:([_a-zA-Z0-9,]*)\)").unwrap();
    let caps = re
        .captures(&content)
        .with_context(|| format!("Illegal file content:{}", &file_path.to_string_lossy()))?;
    let mut all: BTreeSet<String> = caps
        .get(1)
        .unwrap()
        .as_str()
        .split(',')
        .filter(|v| !v.is_empty())
        .map(|v| v.to_string())
        .collect();

    if !all.contains(db) {
        all.insert(db.to_string());
        let all = all.iter().cloned().collect::<Vec<_>>().join(",");
        let tpl = ModTemplate { all, db }.render()?;
        content = re.replace(&content, tpl.trim_start()).to_string();
    }

    fs_write(&file_path, &*content)?;
    Ok(())
}

pub fn write_models_db_rs(
    domain_models_dir: &Path,
    db: &str,
    groups: &GroupsDef,
    force: bool,
) -> Result<()> {
    let file_path = domain_models_dir.join(format!("{}.rs", &db.to_case(Case::Snake)));
    let content = if force || !file_path.exists() {
        #[derive(Template)]
        #[template(path = "domain/base_domain/src/models/db.rs", escape = "none")]
        pub struct DomainDbTemplate;

        DomainDbTemplate.render()?
    } else {
        fs::read_to_string(&file_path)?
    };

    let re = Regex::new(r"(?s)// Do not modify below this line. \(ModStart\).+// Do not modify up to this line. \(ModEnd\)").unwrap();
    ensure!(
        re.is_match(&content),
        "File contents are invalid.: {:?}",
        &file_path
    );

    #[derive(Template)]
    #[template(
        source = r###"
// Do not modify below this line. (ModStart)
@%- for (name, (_, defs)) in groups %@
pub mod @{ name|snake|to_var_name }@;
@%- endfor %@
// Do not modify up to this line. (ModEnd)"###,
        ext = "txt",
        escape = "none"
    )]
    pub struct ModTemplate<'a> {
        pub groups: &'a GroupsDef,
    }

    let tpl = ModTemplate { groups }.render()?;
    let tpl = tpl.trim_start();
    let content = re.replace(&content, tpl);

    fs_write(file_path, &*content)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn write_abstract(
    domain_db_dir: &Path,
    db: &str,
    group_name: &String,
    mod_name: &str,
    force: bool,
    model_name: &String,
    def: &Arc<ModelDef>,
    remove_files: &mut HashSet<OsString>,
) -> Result<(), anyhow::Error> {
    set_domain_mode(true);
    let domain_group_dir = domain_db_dir.join(group_name.to_case(Case::Snake));
    let file_path = domain_group_dir.join(format!("{}.rs", mod_name));
    remove_files.remove(file_path.as_os_str());
    let pascal_name = &model_name.to_case(Case::Pascal);
    if force || !file_path.exists() {
        #[derive(Template)]
        #[template(
            path = "domain/base_domain/src/models/entities/abstract.rs",
            escape = "none"
        )]
        pub struct DomainAbstractTemplate<'a> {
            pub mod_name: &'a str,
            pub pascal_name: &'a str,
            pub def: &'a Arc<ModelDef>,
        }

        let tpl = DomainAbstractTemplate {
            mod_name,
            pascal_name,
            def,
        };
        fs_write(file_path, tpl.render()?)?;
    }
    let domain_group_base_dir = domain_group_dir.join("_base");
    let file_path = domain_group_base_dir.join(format!("_{}.rs", mod_name));
    remove_files.remove(file_path.as_os_str());

    #[derive(Template)]
    #[template(
        path = "domain/base_domain/src/models/entities/base/_abstract.rs",
        escape = "none"
    )]
    pub struct DomainBaseAbstractTemplate<'a> {
        pub db: &'a str,
        pub group_name: &'a str,
        pub mod_name: &'a str,
        pub pascal_name: &'a str,
        pub def: &'a Arc<ModelDef>,
    }

    let tpl = DomainBaseAbstractTemplate {
        db,
        group_name,
        mod_name,
        pascal_name,
        def,
    };
    fs_write(file_path, tpl.render()?)?;
    set_domain_mode(false);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn write_entity(
    domain_db_dir: &Path,
    db: &str,
    group_name: &String,
    mod_name: &str,
    force: bool,
    model_name: &String,
    def: &Arc<ModelDef>,
    remove_files: &mut HashSet<OsString>,
) -> Result<(), anyhow::Error> {
    set_domain_mode(true);
    let domain_group_dir = domain_db_dir.join(group_name.to_case(Case::Snake));
    let file_path = domain_group_dir.join(format!("{}.rs", mod_name));
    remove_files.remove(file_path.as_os_str());
    let pascal_name = &model_name.to_case(Case::Pascal);
    let id_name = &to_id_name(model_name);
    let model_id: u64 = if let Some(model_id) = def.model_id {
        model_id
    } else {
        use crc::{CRC_64_ECMA_182, Crc};
        pub const CRC64: Crc<u64> = Crc::<u64>::new(&CRC_64_ECMA_182);
        CRC64.checksum(format!("{db}:{group_name}:{mod_name}").as_bytes())
    };

    #[derive(Template)]
    #[template(
        path = "domain/base_domain/src/models/entities/entity.rs",
        escape = "none"
    )]
    pub struct DomainEntityTemplate<'a> {
        pub db: &'a str,
        pub group_name: &'a str,
        pub mod_name: &'a str,
        pub pascal_name: &'a str,
        pub id_name: &'a str,
        pub def: &'a Arc<ModelDef>,
        pub model_id: u64,
    }

    if force || !file_path.exists() {
        let tpl = DomainEntityTemplate {
            db,
            group_name,
            mod_name,
            pascal_name,
            id_name,
            def,
            model_id,
        };
        fs_write(file_path, tpl.render()?)?;
    }
    let domain_group_base_dir = domain_group_dir.join("_base");
    let file_path = domain_group_base_dir.join(format!("_{}.rs", mod_name));
    remove_files.remove(file_path.as_os_str());

    #[derive(Template)]
    #[template(
        path = "domain/base_domain/src/models/entities/base/_entity.rs",
        escape = "none"
    )]
    pub struct DomainBaseEntityTemplate<'a> {
        pub db: &'a str,
        pub group_name: &'a str,
        pub mod_name: &'a str,
        pub model_name: &'a str,
        pub pascal_name: &'a str,
        pub id_name: &'a str,
        pub def: &'a Arc<ModelDef>,
    }

    let tpl = DomainBaseEntityTemplate {
        db,
        group_name,
        mod_name,
        model_name,
        pascal_name,
        id_name,
        def,
    };
    fs_write(file_path, tpl.render()?)?;
    set_domain_mode(false);
    Ok(())
}

pub fn write_group_rs(
    domain_db_dir: &Path,
    group_name: &String,
    entities_mod_names: &BTreeSet<(String, &String)>,
    mod_names: &BTreeSet<String>,
    force: bool,
    remove_files: &mut HashSet<OsString>,
) -> Result<()> {
    let file_path = domain_db_dir.join(format!("{}.rs", group_name.to_case(Case::Snake)));
    remove_files.remove(file_path.as_os_str());
    let content = if force || !file_path.exists() {
        #[derive(Template)]
        #[template(path = "domain/base_domain/src/models/group.rs", escape = "none")]
        struct DomainGroupTemplate<'a> {
            pub group_name: &'a str,
        }

        DomainGroupTemplate { group_name }.render()?
    } else {
        fs::read_to_string(&file_path)?
    };

    let re = Regex::new(r"(?s)// Do not modify below this line. \(ModStart\).+// Do not modify up to this line. \(ModEnd\)").unwrap();
    ensure!(
        re.is_match(&content),
        "File contents are invalid.: {:?}",
        &file_path
    );

    #[derive(Template)]
    #[template(
        source = r###"
// Do not modify below this line. (ModStart)
pub mod _base {
@%- for mod_name in mod_names %@
    pub mod _@{ mod_name }@;
@%- endfor %@
}
@%- for mod_name in mod_names %@
pub mod @{ mod_name|to_var_name }@;
@%- endfor %@
// Do not modify up to this line. (ModEnd)"###,
        ext = "txt",
        escape = "none"
    )]
    pub struct DomainGroupModTemplate<'a> {
        pub mod_names: &'a BTreeSet<String>,
    }

    let tpl = DomainGroupModTemplate { mod_names }.render()?;
    let tpl = tpl.trim_start();
    let content = re.replace(&content, tpl);

    // let re = Regex::new(r"(?s)// Do not modify below this line. \(RepoStart\).+// Do not modify up to this line. \(RepoEnd\)").unwrap();
    // ensure!(
    //     re.is_match(&content),
    //     "File contents are invalid.: {:?}",
    //     &file_path
    // );
    // let tpl = template::DomainGroupRepoTemplate {
    //     mod_names: entities_mod_names,
    // }
    // .render()?;
    // let tpl = tpl.trim_start();
    // let content = re.replace(&content, tpl);

    // let re = Regex::new(r"(?s)// Do not modify below this line. \(QueryServiceStart\).+// Do not modify up to this line. \(QueryServiceEnd\)").unwrap();
    // ensure!(
    //     re.is_match(&content),
    //     "File contents are invalid.: {:?}",
    //     &file_path
    // );
    // let tpl = template::DomainGroupQueryServiceTemplate {
    //     mod_names: entities_mod_names,
    // }
    // .render()?;
    // let tpl = tpl.trim_start();
    // let content = re.replace(&content, tpl);

    // let re = Regex::new(r"(?s)// Do not modify below this line. \(EmuRepoStart\).+// Do not modify up to this line. \(EmuRepoEnd\)").unwrap();
    // ensure!(
    //     re.is_match(&content),
    //     "File contents are invalid.: {:?}",
    //     &file_path
    // );
    // let tpl = template::DomainGroupEmuRepoTemplate {
    //     mod_names: entities_mod_names,
    // }
    // .render()?;
    // let tpl = tpl.trim_start();
    // let content = re.replace(&content, tpl);

    // let re = Regex::new(r"(?s)// Do not modify below this line. \(EmuQueryServiceStart\).+// Do not modify up to this line. \(EmuQueryServiceEnd\)").unwrap();
    // ensure!(
    //     re.is_match(&content),
    //     "File contents are invalid.: {:?}",
    //     &file_path
    // );
    // let tpl = template::DomainGroupEmuQueryServiceTemplate {
    //     mod_names: entities_mod_names,
    // }
    // .render()?;
    // let tpl = tpl.trim_start();
    // let content = re.replace(&content, tpl);

    fs_write(file_path, &*content)?;
    Ok(())
}
