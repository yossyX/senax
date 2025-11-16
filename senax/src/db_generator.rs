use anyhow::Result;
use askama::Template;
use regex::Regex;
use std::fmt::Write;
use std::{fs, path::Path};

use crate::common::ToCase as _;
use crate::common::fs_write;
use crate::schema::DbType;
use crate::{DOMAIN_PATH, SCHEMA_PATH};
use crate::{DOMAIN_REPOSITORIES_PATH, filters};

pub fn db_list(dir_type_only: bool) -> Result<Vec<String>> {
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
                if !dir_type_only || schema_path.join(name).exists() {
                    dbs.push(name.to_owned())
                }
            }
        }
    }
    Ok(dbs)
}

pub fn generate(db_type: DbType, db: &str, exclude_from_domain: bool) -> Result<()> {
    anyhow::ensure!(Path::new("Cargo.toml").exists(), "Incorrect directory.");
    let schema_path = Path::new(SCHEMA_PATH);

    #[derive(Template)]
    #[template(path = "db.yml", escape = "none")]
    struct DbTemplate {
        pub db_id: u64,
        pub db_type: DbType,
        pub exclude_from_domain: bool,
    }

    let file_path = schema_path.join(format!("{}.yml", db));
    if !file_path.exists() {
        let tpl = DbTemplate {
            db_id: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros() as u64,
            db_type,
            exclude_from_domain,
        };
        fs_write(file_path, tpl.render()?)?;
    }

    let file_path = Path::new("./.env");
    if file_path.exists() {
        let content = fs::read_to_string(file_path)?;
        fs_write(file_path, fix_env(&content, db, db_type)?)?;
    }

    let file_path = Path::new("./.env.example");
    if file_path.exists() {
        let content = fs::read_to_string(file_path)?;
        fs_write(file_path, fix_env(&content, db, db_type)?)?;
    }

    if !exclude_from_domain {
        let domain_path = Path::new(DOMAIN_PATH);
        let file_path = domain_path.join("Cargo.toml");
        if file_path.exists() {
            let mut content = fs::read_to_string(&file_path)?.replace("\r\n", "\n");

            let db = &db.to_snake();
            content = content.replace(
                "\"mockall\"",
                &format!("\"mockall\",\"repository_{}/mock\"", db),
            );
            content = content.replace(
                "[dependencies]",
                &format!(
                    "[dependencies]\nrepository_{} = {{ path = \"repositories/{}\" }}",
                    db, db
                ),
            );
            content = content.replace(
                "[dev-dependencies]",
                &format!(
                    "[dev-dependencies]\nrepository_{} = {{ path = \"repositories/{}\", features = [\"mock\"] }}",
                    db, db
                ),
            );
            fs_write(file_path, &*content)?;
        }

        repositories(&domain_path.join(DOMAIN_REPOSITORIES_PATH).join(db), db)?;
    }
    Ok(())
}

#[derive(Template)]
#[template(path = "domain/db_repositories/_Cargo.toml", escape = "none")]
pub struct DomainCargoTemplate<'a> {
    pub db: &'a str,
}

#[derive(Template)]
#[template(path = "domain/db_repositories/src/lib.rs", escape = "none")]
pub struct DomainDbLibTemplate<'a> {
    pub db: &'a str,
}

fn repositories(path: &Path, db: &str) -> Result<()> {
    let file_path = path.join("Cargo.toml");
    let tpl = DomainCargoTemplate { db };
    fs_write(file_path, tpl.render()?)?;

    let file_path = path.join("src/lib.rs");
    let tpl = DomainDbLibTemplate { db };
    fs_write(file_path, tpl.render()?)?;

    Ok(())
}

fn fix_env(content: &str, db: &str, db_type: DbType) -> Result<String> {
    let re = Regex::new(r"RUST_LOG(\s*)=(.+)").unwrap();
    let mut content = if let Some(caps) = re.captures(content) {
        let sp = caps.get(1).unwrap().as_str();
        let conf = caps.get(2).unwrap().as_str();
        re.replace(
            content,
            format!("RUST_LOG{}={},db_{}=debug", sp, conf, db.to_snake()),
        )
        .to_string()
    } else {
        content.to_owned()
    };
    let upper = db.to_upper_snake();
    let (user, pw) = match db_type {
        DbType::Mysql => ("root", "root"),
        DbType::Postgres => ("postgres", "postgres"),
    };
    write!(
        &mut content,
        r#"
{upper}_DB_URL={db_type}://{user}:{pw}@db/{db}
{upper}_TEST_DB_URL={db_type}://{user}:{pw}@db/{db}_test
{upper}_DB_MAX_CONNECTIONS_FOR_WRITE=50
{upper}_DB_MAX_CONNECTIONS_FOR_READ=50
{upper}_DB_MAX_CONNECTIONS_FOR_CACHE=10
"#
    )?;
    Ok(content)
}
