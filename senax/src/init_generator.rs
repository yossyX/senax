use anyhow::Result;
use askama::Template;
use std::{fs, path::PathBuf};

use crate::common::fs_write;

pub fn generate(name: &Option<String>) -> Result<()> {
    let base_path: PathBuf = if let Some(name) = name {
        let name = sanitize_filename::sanitize(name);
        fs::create_dir_all(&name)?;
        name.parse()?
    } else {
        ".".parse()?
    };

    let file_path = base_path.join("Cargo.toml");
    let tpl = CargoTemplate {};
    println!("{}", file_path.display());
    fs_write(file_path, tpl.render()?)?;

    let file_path = base_path.join(".env.sample");
    let tpl = EnvTemplate {
        tz: std::env::var("TZ").unwrap_or_default(),
    };
    println!("{}", file_path.display());
    fs_write(file_path, tpl.render()?)?;

    let file_path = base_path.join(".gitignore");
    let tpl = GitignoreTemplate {};
    println!("{}", file_path.display());
    fs_write(file_path, tpl.render()?)?;

    let file_path = base_path.join("schema");
    fs::create_dir_all(&file_path)?;

    let file_path = base_path.join("schema/session.yml");
    let tpl = SessionTemplate {};
    println!("{}", file_path.display());
    fs_write(file_path, tpl.render()?)?;

    let file_path = base_path.join("domain/src");
    fs::create_dir_all(&file_path)?;

    let file_path = base_path.join("domain/Cargo.toml");
    let tpl = DomainCargoTemplate {};
    println!("{}", file_path.display());
    fs_write(file_path, tpl.render()?)?;

    let file_path = base_path.join("domain/src/lib.rs");
    let tpl = DomainLibTemplate {};
    println!("{}", file_path.display());
    fs_write(file_path, tpl.render()?)?;

    Ok(())
}

#[derive(Template)]
#[template(path = "init/_Cargo.toml", escape = "none")]
pub struct CargoTemplate {}

#[derive(Template)]
#[template(path = "init/.env.sample", escape = "none")]
pub struct EnvTemplate {
    pub tz: String,
}

#[derive(Template)]
#[template(path = "init/.gitignore", escape = "none")]
pub struct GitignoreTemplate {}

#[derive(Template)]
#[template(path = "init/schema/session.yml", escape = "none")]
pub struct SessionTemplate {}

#[derive(Template)]
#[template(path = "init/domain/_Cargo.toml", escape = "none")]
pub struct DomainCargoTemplate {}

#[derive(Template)]
#[template(path = "init/domain/src/lib.rs", escape = "none")]
pub struct DomainLibTemplate {}
