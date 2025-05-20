use anyhow::Result;
use askama::Template;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use crate::common::fs_write;
use crate::filters;
use crate::schema::{ConfigDef, GroupsDef};

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
        #[template(path = "db/base/_Cargo.toml", escape = "none")]
        struct CargoTemplate<'a> {
            pub db: &'a str,
            pub config: &'a ConfigDef,
        }

        let tpl = CargoTemplate {
            db,
            config,
        };
        fs_write(file_path, tpl.render()?)?;
    }

    let src_dir = base_dir.join("src");
    let file_path = src_dir.join("lib.rs");
    if force || !file_path.exists() {
        #[derive(Template)]
        #[template(path = "db/base/src/lib.rs", escape = "none")]
        struct LibTemplate<'a> {
            pub db: &'a str,
            pub config: &'a ConfigDef,
            pub non_snake_case: bool,
        }

        let tpl = LibTemplate {
            db,
            config,
            non_snake_case,
        };
        fs_write(file_path, tpl.render()?)?;
    }

    #[derive(Template)]
    #[template(path = "db/base/src/accessor.rs", escape = "none")]
    struct AccessorTemplate {}

    let file_path = src_dir.join("accessor.rs");
    let tpl = AccessorTemplate {};
    fs_write(file_path, tpl.render()?)?;

    #[derive(Template)]
    #[template(path = "db/base/src/cache.rs", escape = "none")]
    struct CacheTemplate {}

    let file_path = src_dir.join("cache.rs");
    if !config.force_disable_cache {
        let tpl = CacheTemplate {};
        fs_write(file_path, tpl.render()?)?;
    } else if file_path.exists() {
        fs::remove_file(&file_path)?;
    }

    #[derive(Template)]
    #[template(path = "db/base/src/misc.rs", escape = "none")]
    struct MiscTemplate<'a> {
        pub config: &'a ConfigDef,
    }

    let file_path = src_dir.join("misc.rs");
    let tpl = MiscTemplate { config };
    fs_write(file_path, tpl.render()?)?;

    #[derive(Template)]
    #[template(path = "db/base/src/connection.rs", escape = "none")]
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
        config,
        groups,
        tx_isolation: config.tx_isolation.map(|v| v.as_str()),
        read_tx_isolation: config.read_tx_isolation.map(|v| v.as_str()),
    };
    fs_write(file_path, tpl.render()?)?;

    let file_path = src_dir.join("models.rs");

    #[derive(Template)]
    #[template(path = "db/base/src/models.rs", escape = "none")]
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
        groups,
        config,
        table_names,
    };
    fs_write(file_path, tpl.render()?)?;

    write_impl_domain_rs(&src_dir, groups)?;

    Ok(())
}

pub fn write_impl_domain_rs(src_dir: &Path, groups: &GroupsDef) -> Result<()> {
    let file_path = src_dir.join("impl_domain.rs");

    #[derive(Template)]
    #[template(path = "db/base/src/impl_domain.rs", escape = "none")]
    pub struct ImplDomainDbTemplate<'a> {
        pub groups: &'a GroupsDef,
    }

    let content = ImplDomainDbTemplate { groups }.render()?;

    fs_write(file_path, &*content)?;
    Ok(())
}
