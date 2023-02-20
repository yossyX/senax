use anyhow::{ensure, Result};
use askama::Template;
use convert_case::{Case, Casing};
use regex::Regex;
use std::fmt::Write;
use std::{fs, path::Path};

use crate::common::fs_write;

pub fn generate(server_path: &Path, db: &str) -> Result<()> {
    ensure!(
        server_path.exists() && server_path.is_dir(),
        "The crate path does not exist."
    );

    let schema_path = Path::new("./schema");
    fs::create_dir_all(&schema_path)?;

    let file_path = schema_path.join(format!("{}.yml", db));
    if !file_path.exists() {
        let tpl = DbTemplate {};
        println!("{}", file_path.display());
        fs_write(file_path, tpl.render()?)?;

        let file_path = Path::new("./.env");
        if file_path.exists() {
            println!("{}", file_path.display());
            let content = fs::read_to_string(&file_path)?;
            fs_write(file_path, &fix_env(&content, db)?)?;
        }

        let file_path = Path::new("./.env.sample");
        if file_path.exists() {
            println!("{}", file_path.display());
            let content = fs::read_to_string(&file_path)?;
            fs_write(file_path, &fix_env(&content, db)?)?;
        }
    }

    let file_path = server_path.join("Cargo.toml");
    ensure!(file_path.exists(), "Cargo.toml does not exist.");
    let content = fs::read_to_string(&file_path)?;
    let content = content.replace(
        "[dependencies]",
        &format!("[dependencies]\ndb_{} = {{ path = \"../db/{}\" }}", db, db),
    );
    println!("{}", file_path.display());
    fs_write(file_path, &*content)?;

    let file_path = server_path.join("src/db.rs");
    ensure!(file_path.exists(), "src/db.rs does not exist.");
    let content = fs::read_to_string(&file_path)?;
    let tpl = DbStartTemplate { db };
    let content = content.replace("// Do not modify this line. (DbStart)", &tpl.render()?);
    let tpl = DbStopTemplate { db };
    let content = content.replace("// Do not modify this line. (DbStop)", &tpl.render()?);
    println!("{}", file_path.display());
    fs_write(file_path, &*content)?;

    let file_path = server_path.join("src/graphql.rs");
    ensure!(file_path.exists(), "src/graphql.rs does not exist.");
    let content = fs::read_to_string(&file_path)?;
    let content = content.replace(
        "// Do not modify this line. (GqlDbMod)",
        &format!("// Do not modify this line. (GqlDbMod)\n// pub mod {};", db),
    );
    let tpl = QueryRootTemplate { db };
    let content = content.replace("impl QueryRoot {", &tpl.render()?);
    let tpl = MutationRootTemplate { db };
    let content = content.replace("impl MutationRoot {", &tpl.render()?);
    println!("{}", file_path.display());
    fs_write(file_path, &*content)?;

    Ok(())
}

fn fix_env(content: &str, db: &str) -> Result<String> {
    let re = Regex::new(r"RUST_LOG(\s*)=(.+)").unwrap();
    let mut content = if let Some(caps) = re.captures(&content) {
        let sp = caps.get(1).unwrap().as_str();
        let conf = caps.get(2).unwrap().as_str();
        re.replace(&content, format!("RUST_LOG{}={},db_{}=debug", sp, conf, db))
            .to_string()
    } else {
        content.to_owned()
    };
    let upper = db.to_case(Case::Upper);
    write!(
        &mut content,
        r#"
{}_DB_URL=mysql://root:root@localhost/{}
{}_TEST_DB_URL=mysql://root:root@localhost/{}
{}_DB_MAX_CONNECTIONS=50
{}_REPLICA_DB_MAX_CONNECTIONS=50
{}_CACHE_DB_MAX_CONNECTIONS=10
"#,
        upper, db, upper, db, upper, upper, upper
    )?;
    Ok(content)
}

#[derive(Template)]
#[template(path = "db.yml", escape = "none")]
pub struct DbTemplate {}

#[derive(Template)]
#[template(
    source = r###"// Do not modify this line. (DbStart)
    db_@{ db }@::start(
        is_hot_deploy,
        exit_tx.clone(),
        Arc::downgrade(db_guard),
        db_dir,
        linker_port,
        pw,
    )
    .await?;
@{-"\n"}@"###,
    ext = "txt",
    escape = "none"
)]
pub struct DbStartTemplate<'a> {
    pub db: &'a str,
}

#[derive(Template)]
#[template(
    source = r###"// Do not modify this line. (DbStop)
    db_@{ db }@::stop();"###,
    ext = "txt",
    escape = "none"
)]
pub struct DbStopTemplate<'a> {
    pub db: &'a str,
}

#[derive(Template)]
#[template(
    source = r###"impl QueryRoot {
    // async fn @{ db }@(&self) -> @{ db }@::GqiQueryData {
    //     @{ db }@::GqiQueryData
    // }
@{-"\n"}@"###,
    ext = "txt",
    escape = "none"
)]
pub struct QueryRootTemplate<'a> {
    pub db: &'a str,
}

#[derive(Template)]
#[template(
    source = r###"impl MutationRoot {
    // async fn @{ db }@(&self) -> @{ db }@::GqiMutationData {
    //     @{ db }@::GqiMutationData
    // }
@{-"\n"}@"###,
    ext = "txt",
    escape = "none"
)]
pub struct MutationRootTemplate<'a> {
    pub db: &'a str,
}
