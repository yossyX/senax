use anyhow::{ensure, Context as _, Result};
use askama::Template;
use regex::Regex;
use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::common::fs_write;

pub fn generate(name: &str) -> Result<()> {
    let file_path = Path::new("./Cargo.toml");
    ensure!(file_path.exists(), "Cargo.toml does not exist.");
    let content = fs::read_to_string(&file_path)?;
    let re = Regex::new(r"members\s*=\s*\[([^\]]*)\]").unwrap();
    let caps = re
        .captures(&content)
        .with_context(|| format!("Illegal file content:{}", &file_path.to_string_lossy()))?;
    let members = caps.get(1).unwrap().as_str();
    let content = re.replace(&content, format!("members = [{}, \"{}\"]", members, name));
    println!("{}", file_path.display());
    fs_write(file_path, &*content)?;

    let file_path = Path::new("./.env");
    if file_path.exists() {
        println!("{}", file_path.display());
        let content = fs::read_to_string(&file_path)?;
        fs_write(file_path, &fix_env(&content, name)?)?;
    }

    let file_path = Path::new("./.env.sample");
    if file_path.exists() {
        println!("{}", file_path.display());
        let content = fs::read_to_string(&file_path)?;
        fs_write(file_path, &fix_env(&content, name)?)?;
    }

    let name = sanitize_filename::sanitize(name);
    fs::create_dir_all(&name)?;
    let base_path: PathBuf = name.parse()?;

    let file_path = base_path.join("Cargo.toml");
    let tpl = CargoTemplate { name };
    println!("{}", file_path.display());
    fs_write(file_path, tpl.render()?)?;

    let src_path = base_path.join("src");
    fs::create_dir_all(&src_path)?;

    let file_path = src_path.join("auth.rs");
    let tpl = AuthTemplate {};
    println!("{}", file_path.display());
    fs_write(file_path, tpl.render()?)?;

    let file_path = src_path.join("context.rs");
    let tpl = ContextTemplate {};
    println!("{}", file_path.display());
    fs_write(file_path, tpl.render()?)?;

    let file_path = src_path.join("db.rs");
    let tpl = DbTemplate {};
    println!("{}", file_path.display());
    fs_write(file_path, tpl.render()?)?;

    let file_path = src_path.join("graphql.rs");
    let tpl = GraphqlTemplate {};
    println!("{}", file_path.display());
    fs_write(file_path, tpl.render()?)?;

    let file_path = src_path.join("main.rs");
    let tpl = MainTemplate {};
    println!("{}", file_path.display());
    fs_write(file_path, tpl.render()?)?;

    let file_path = src_path.join("response.rs");
    let tpl = ResponseTemplate {};
    println!("{}", file_path.display());
    fs_write(file_path, tpl.render()?)?;

    let routes_path = src_path.join("routes");
    fs::create_dir_all(&routes_path)?;

    let file_path = routes_path.join("api.rs");
    let tpl = ApiTemplate {};
    println!("{}", file_path.display());
    fs_write(file_path, tpl.render()?)?;

    let file_path = routes_path.join("root.rs");
    let tpl = RootTemplate {};
    println!("{}", file_path.display());
    fs_write(file_path, tpl.render()?)?;

    let root_path = routes_path.join("root");
    fs::create_dir_all(&root_path)?;

    let file_path = root_path.join("index.rs");
    let tpl = IndexTemplate {};
    println!("{}", file_path.display());
    fs_write(file_path, tpl.render()?)?;

    Ok(())
}

fn fix_env(content: &str, name: &str) -> Result<String> {
    let re = Regex::new(r"RUST_LOG(\s*)=(.+)").unwrap();
    let content = if let Some(caps) = re.captures(content) {
        let sp = caps.get(1).unwrap().as_str();
        let conf = caps.get(2).unwrap().as_str();
        re.replace(content, format!("RUST_LOG{}={},{}=debug", sp, conf, name))
            .to_string()
    } else {
        content.to_owned()
    };
    Ok(content)
}

#[derive(Template)]
#[template(path = "new_actix/_Cargo.toml", escape = "none")]
pub struct CargoTemplate {
    pub name: String,
}

#[derive(Template)]
#[template(path = "new_actix/src/auth.rs", escape = "none")]
pub struct AuthTemplate {}

#[derive(Template)]
#[template(path = "new_actix/src/context.rs", escape = "none")]
pub struct ContextTemplate {}

#[derive(Template)]
#[template(path = "new_actix/src/db.rs", escape = "none")]
pub struct DbTemplate {}

#[derive(Template)]
#[template(path = "new_actix/src/graphql.rs", escape = "none")]
pub struct GraphqlTemplate {}

#[derive(Template)]
#[template(path = "new_actix/src/main.rs", escape = "none")]
pub struct MainTemplate {}

#[derive(Template)]
#[template(path = "new_actix/src/response.rs", escape = "none")]
pub struct ResponseTemplate {}

#[derive(Template)]
#[template(path = "new_actix/src/routes/api.rs", escape = "none")]
pub struct ApiTemplate {}

#[derive(Template)]
#[template(path = "new_actix/src/routes/root.rs", escape = "none")]
pub struct RootTemplate {}

#[derive(Template)]
#[template(path = "new_actix/src/routes/root/index.rs", escape = "none")]
pub struct IndexTemplate {}
