use anyhow::{Result, ensure};
use askama::Template;
use indexmap::IndexMap;
use regex::Regex;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use crate::common::fs_write;
use crate::filters;
use crate::schema::{ConfigDef, GroupsDef, ModelDef};

pub fn write_files(
    base_dir: &Path,
    db: &str,
    groups: &GroupsDef,
    config: &ConfigDef,
    force: bool,
    non_snake_case: bool,
) -> Result<()> {
    let file_path = base_dir.join("Cargo.toml");
    if force || !file_path.exists() {
        #[derive(Template)]
        #[template(path = "model/base/_Cargo.toml", escape = "none")]
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

    let src_dir = base_dir.join("src");
    let file_path = src_dir.join("lib.rs");
    if force || !file_path.exists() {
        #[derive(Template)]
        #[template(path = "model/base/src/lib.rs", escape = "none")]
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

    #[derive(Template)]
    #[template(path = "model/base/src/accessor.rs", escape = "none")]
    struct AccessorTemplate {}

    let file_path = src_dir.join("accessor.rs");
    let tpl = AccessorTemplate {};
    fs_write(file_path, tpl.render()?)?;

    #[derive(Template)]
    #[template(path = "model/base/src/cache.rs", escape = "none")]
    struct CacheTemplate {}

    let file_path = src_dir.join("cache.rs");
    if !config.force_disable_cache {
        let tpl = CacheTemplate {};
        fs_write(file_path, tpl.render()?)?;
    } else if file_path.exists() {
        fs::remove_file(&file_path)?;
    }

    #[derive(Template)]
    #[template(path = "model/base/src/misc.rs", escape = "none")]
    struct MiscTemplate<'a> {
        pub config: &'a ConfigDef,
    }

    let file_path = src_dir.join("misc.rs");
    let tpl = MiscTemplate { config: &config };
    fs_write(file_path, tpl.render()?)?;

    #[derive(Template)]
    #[template(path = "model/base/src/connection.rs", escape = "none")]
    struct ConnectionTemplate<'a> {
        pub db: &'a str,
        pub config: &'a ConfigDef,
        pub groups: &'a GroupsDef,
        pub tx_isolation: Option<&'a str>,
        pub read_tx_isolation: Option<&'a str>,
    }

    let file_path = src_dir.join("connection.rs");
    let tpl = ConnectionTemplate {
        db,
        config: &config,
        groups: &groups,
        tx_isolation: config.tx_isolation.map(|v| v.as_str()),
        read_tx_isolation: config.read_tx_isolation.map(|v| v.as_str()),
    };
    fs_write(file_path, tpl.render()?)?;

    let file_path = src_dir.join("models.rs");

    #[derive(Template)]
    #[template(path = "model/base/src/models.rs", escape = "none")]
    struct ModelsTemplate<'a> {
        pub groups: &'a GroupsDef,
        pub config: &'a ConfigDef,
        pub table_names: BTreeSet<String>,
    }

    let mut table_names = BTreeSet::default();
    for (_, (_, defs)) in groups {
        for (_, (_, def)) in defs {
            table_names.insert(def.table_name());
        }
    }
    let tpl = ModelsTemplate {
        groups: &groups,
        config: &config,
        table_names,
    };
    fs_write(file_path, tpl.render()?)?;

    write_impl_domain_rs(&src_dir, groups, force)?;

    Ok(())
}

pub fn write_impl_domain_rs(
    src_dir: &Path,
    groups: &GroupsDef,
    force: bool,
) -> Result<()> {
    let file_path = src_dir.join("impl_domain.rs");
    let content = if force || !file_path.exists() {
        #[derive(Template)]
        #[template(path = "model/base/src/impl_domain.rs", escape = "none")]
        pub struct ImplDomainDbTemplate;

        ImplDomainDbTemplate.render()?
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
