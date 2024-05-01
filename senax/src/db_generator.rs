use anyhow::Result;
use askama::Template;
use convert_case::{Case, Casing};
use regex::Regex;
use std::fmt::Write;
use std::{fs, path::Path};

use crate::common::fs_write;
use crate::SCHEMA_PATH;

pub fn list() -> Result<Vec<String>> {
    let schema_path = Path::new(SCHEMA_PATH);
    let mut dbs = Vec::new();
    let re = Regex::new(r"^([a-zA-Z][_a-zA-Z0-9]*)\.yml$").unwrap();
    for entry in fs::read_dir(schema_path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let name = path.file_name().unwrap_or_default();
            let name = name.to_str().unwrap_or_default();
            if let Some(caps) = re.captures(name) {
                let name = caps.get(1).unwrap().as_str();
                if !name.eq("session") {
                    dbs.push(name.to_owned())
                }
            }
        }
    }
    Ok(dbs)
}

pub fn generate(db: &str) -> Result<()> {
    let schema_path = Path::new(SCHEMA_PATH);
    fs::create_dir_all(schema_path)?;

    let file_path = schema_path.join(format!("{}.yml", db));
    if !file_path.exists() {
        let tpl = DbTemplate {
            db_id: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros() as u64,
        };
        fs_write(file_path, tpl.render()?)?;

        let file_path = schema_path.join(db);
        fs::create_dir_all(file_path)?;
    }

    let file_path = Path::new("./.env");
    if file_path.exists() {
        let content = fs::read_to_string(file_path)?;
        fs_write(file_path, fix_env(&content, db)?)?;
    }

    let file_path = Path::new("./.env.example");
    if file_path.exists() {
        let content = fs::read_to_string(file_path)?;
        fs_write(file_path, fix_env(&content, db)?)?;
    }
    Ok(())
}

fn fix_env(content: &str, db: &str) -> Result<String> {
    let re = Regex::new(r"RUST_LOG(\s*)=(.+)").unwrap();
    let mut content = if let Some(caps) = re.captures(content) {
        let sp = caps.get(1).unwrap().as_str();
        let conf = caps.get(2).unwrap().as_str();
        re.replace(
            content,
            format!(
                "RUST_LOG{}={},db_{}=debug",
                sp,
                conf,
                db.to_case(Case::Snake)
            ),
        )
        .to_string()
    } else {
        content.to_owned()
    };
    let upper = db.to_case(Case::UpperSnake);
    write!(
        &mut content,
        r#"
{}_DB_URL=mysql://root:root@db/{}
{}_TEST_DB_URL=mysql://root:root@db/{}_test
{}_DB_MAX_CONNECTIONS_FOR_WRITE=50
{}_DB_MAX_CONNECTIONS_FOR_READ=50
{}_DB_MAX_CONNECTIONS_FOR_CACHE=10
"#,
        upper, db, upper, db, upper, upper, upper
    )?;
    Ok(content)
}

#[derive(Template)]
#[template(path = "db.yml", escape = "none")]
pub struct DbTemplate {
    pub db_id: u64,
}
