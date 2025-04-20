use anyhow::Result;
use askama::Template;
use convert_case::{Case, Casing};
use regex::Regex;
use std::fmt::Write;
use std::{fs, path::Path};

use crate::common::fs_write;
use crate::{DOMAIN_PATH, SCHEMA_PATH};
use crate::filters;

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

pub fn generate(db: &str, exclude_from_domain: bool) -> Result<()> {
    anyhow::ensure!(Path::new("Cargo.toml").exists(), "Incorrect directory.");
    let schema_path = Path::new(SCHEMA_PATH);

    #[derive(Template)]
    #[template(path = "db.yml", escape = "none")]
    struct DbTemplate {
        pub db_id: u64,
        pub exclude_from_domain: bool,
    }
    
    let file_path = schema_path.join(format!("{}.yml", db));
    if !file_path.exists() {
        let tpl = DbTemplate {
            db_id: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros() as u64,
            exclude_from_domain,
        };
        fs_write(file_path, tpl.render()?)?;
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

    if !exclude_from_domain {
        let domain_path = Path::new(DOMAIN_PATH);
        let file_path = domain_path.join("Cargo.toml");
        if file_path.exists() {
            let mut content = fs::read_to_string(&file_path)?;
    
            let db = &db.to_case(Case::Snake);
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
    
        repositories(&domain_path.join("repositories").join(db), db)?;
    }
    Ok(())
}

#[derive(Template)]
#[template(path = "init/domain/db_repositories/src/lib.rs", escape = "none")]
pub struct DomainDbLibTemplate<'a> {
    pub db: &'a str,
}

fn repositories(path: &Path, db: &str) -> Result<()> {

    #[derive(Template)]
    #[template(path = "init/domain/db_repositories/_Cargo.toml", escape = "none")]
    struct DomainCargoTemplate<'a> {
        db: &'a str,
    }

    let file_path = path.join("Cargo.toml");
    let tpl = DomainCargoTemplate { db };
    fs_write(file_path, tpl.render()?)?;

    // #[derive(Template)]
    // #[template(path = "init/domain/db_repositories/src/lib.rs", escape = "none")]
    // struct DomainLibTemplate<'a> {
    //     db: &'a str,
    // }

    let file_path = path.join("src/lib.rs");
    let tpl = DomainDbLibTemplate { db };
    fs_write(file_path, tpl.render()?)?;

    // #[derive(Template)]
    // #[template(path = "init/domain/db_repositories/src/repositories.rs", escape = "none")]
    // struct DomainRepositoriesTemplate;

    // let file_path = path.join("src/repositories.rs");
    // let tpl = DomainRepositoriesTemplate;
    // fs_write(file_path, tpl.render()?)?;

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
