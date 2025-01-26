use anyhow::{Context as _, Result};
use askama::Template;
use convert_case::{Case, Casing};
use rand::{
    distributions::{Alphanumeric, DistString},
    RngCore,
};
use regex::Regex;
use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::common::fs_write;
use crate::schema::CONFIG;

pub fn generate(name: &str, db_list: Vec<&str>, force: bool) -> Result<()> {
    let non_snake_case = crate::common::check_non_snake_case()?;
    for db in &db_list {
        crate::common::check_ascii_name(db);
    }
    let name = crate::common::check_ascii_name(name).to_string();
    fs::create_dir_all(&name)?;
    let base_path: PathBuf = name.parse()?;

    let file_path = Path::new("./Cargo.toml");
    if file_path.exists() {
        let content = fs::read_to_string(file_path)?;
        let re = Regex::new(r"members\s*=\s*\[([^\]]*)\]").unwrap();
        let caps = re
            .captures(&content)
            .with_context(|| format!("Illegal file content:{}", &file_path.to_string_lossy()))?;
        let members = caps.get(1).unwrap().as_str();
        let quoted = format!("\"{}\"", &name);
        if !members.contains(&quoted) {
            let content = re.replace(&content, format!("members = [{}, {}]", members, &quoted));
            fs_write(file_path, &*content)?;
        }
    }

    let mut file_path = PathBuf::from("./.env");
    if !file_path.exists() {
        file_path = base_path.join(".env");
    }
    if file_path.exists() {
        let content = fs::read_to_string(&file_path)?;
        fs_write(file_path, fix_env(&content, &name)?)?;
    }

    let mut file_path = PathBuf::from("./.env.example");
    if !file_path.exists() {
        file_path = base_path.join(".env.example");
    }
    if file_path.exists() {
        let content = fs::read_to_string(&file_path)?;
        fs_write(file_path, fix_env(&content, &name)?)?;
    }

    let file_path = base_path.join("Cargo.toml");
    let mut content = if force || !file_path.exists() {
        CargoTemplate { name }.render()?
    } else {
        fs::read_to_string(&file_path)?
    };
    for db in &db_list {
        let db = &db.to_case(Case::Snake);
        let reg = Regex::new(&format!(r"(?m)^db_{}\s*=", db))?;
        if !reg.is_match(&content) {
            content = content.replace(
                "[dependencies]",
                &format!(
                    "[dependencies]\ndb_{} = {{ path = \"../2_db/{}\" }}",
                    db, db
                ),
            );
        }
    }
    fs_write(file_path, &*content)?;

    let src_path = base_path.join("src");
    fs::create_dir_all(&src_path)?;

    let file_path = src_path.join("auth.rs");
    if force || !file_path.exists() {
        let tpl = AuthTemplate;
        fs_write(file_path, tpl.render()?)?;
    }

    let file_path = src_path.join("context.rs");
    if force || !file_path.exists() {
        let tpl = ContextTemplate;
        fs_write(file_path, tpl.render()?)?;
    }

    let file_path = src_path.join("db.rs");
    let mut content = if force || !file_path.exists() {
        DbTemplate.render()?
    } else {
        fs::read_to_string(&file_path)?
    };
    for db in &db_list {
        let reg = Regex::new(&format!(r"(?m)^\s*db_{}::start", &db.to_case(Case::Snake)))?;
        if !reg.is_match(&content) {
            crate::schema::parse(db, false, false)?;
            let config = CONFIG.read().unwrap().as_ref().unwrap().clone();
            let tpl = DbStartTemplate { db };
            content = content.replace(
                "// Do not modify this line. (DbStart)",
                tpl.render()?.trim_start(),
            );
            let tpl = DbStartTestTemplate { db };
            content = content.replace(
                "// Do not modify this line. (DbStartTest)",
                tpl.render()?.trim_start(),
            );
            let tpl = DbStopTemplate { db };
            content = content.replace(
                "// Do not modify this line. (DbStop)",
                tpl.render()?.trim_start(),
            );
            let tpl = DbClearLocalCacheTemplate { db };
            content = content.replace(
                "// Do not modify this line. (DbClearLocalCache)",
                tpl.render()?.trim_start(),
            );
            let tpl = DbClearCacheTemplate { db };
            content = content.replace(
                "// Do not modify this line. (DbClearCache)",
                tpl.render()?.trim_start(),
            );
            if !config.excluded_from_domain {
                let tpl = DbRepoTemplate { db };
                content = content.replace(
                    "// Do not modify this line. (Repo)",
                    tpl.render()?.trim_start(),
                );
                let tpl = DbRepoNewTemplate { db };
                content = content.replace(
                    "// Do not modify this line. (RepoNew)",
                    tpl.render()?.trim_start(),
                );
                let tpl = DbRepoImplTemplate { db };
                content = content.replace(
                    "// Do not modify this line. (RepoImpl)",
                    tpl.render()?.trim_start(),
                );
                let tpl = DbRepoImplStartTemplate { db };
                content = content.replace(
                    "// Do not modify this line. (RepoImplStart)",
                    tpl.render()?.trim_start(),
                );
                let tpl = DbRepoImplCommitTemplate { db };
                content = content.replace(
                    "// Do not modify this line. (RepoImplCommit)",
                    tpl.render()?.trim_start(),
                );
                let tpl = DbRepoImplRollbackTemplate { db };
                content = content.replace(
                    "// Do not modify this line. (RepoImplRollback)",
                    tpl.render()?.trim_start(),
                );
                let tpl = DbMigrateTemplate { db };
                content = content.replace(
                    "// Do not modify this line. (migrate)",
                    tpl.render()?.trim_start(),
                );
                let tpl = DbGenSeedSchemaTemplate { db };
                content = content.replace(
                    "// Do not modify this line. (gen_seed_schema)",
                    tpl.render()?.trim_start(),
                );
                let tpl = DbSeedTemplate { db };
                content = content.replace(
                    "// Do not modify this line. (seed)",
                    tpl.render()?.trim_start(),
                );
                let tpl = DbCheckTemplate { db };
                content = content.replace(
                    "// Do not modify this line. (check)",
                    tpl.render()?.trim_start(),
                );
            }
        }
    }
    fs_write(file_path, &*content)?;

    let file_path = src_path.join("gql_log.rs");
    if force || !file_path.exists() {
        let tpl = GqlLogTemplate;
        fs_write(&file_path, tpl.render()?)?;
    }

    let file_path = src_path.join("auto_api.rs");
    if force || !file_path.exists() {
        let tpl = AutoApiTemplate;
        fs_write(&file_path, tpl.render()?)?;
    }

    let file_path = src_path.join("main.rs");
    if force || !file_path.exists() {
        let tpl = MainTemplate { non_snake_case };
        fs_write(file_path, tpl.render()?)?;
    }

    let file_path = src_path.join("response.rs");
    if force || !file_path.exists() {
        let tpl = ResponseTemplate;
        fs_write(file_path, tpl.render()?)?;
    }

    let file_path = src_path.join("tasks.rs");
    if !file_path.exists() {
        let tpl = TasksTemplate;
        fs_write(file_path, tpl.render()?)?;
    }

    let file_path = src_path.join("tests.rs");
    if !file_path.exists() {
        let tpl = TestsTemplate;
        fs_write(file_path, tpl.render()?)?;
    }

    let file_path = src_path.join("common.rs");
    if !file_path.exists() {
        let tpl = CommonTemplate;
        fs_write(file_path, tpl.render()?)?;
    }

    let file_path = src_path.join("validator.rs");
    if !file_path.exists() {
        let tpl = ValidatorTemplate;
        fs_write(file_path, tpl.render()?)?;
    }

    let routes_path = src_path.join("routes");
    fs::create_dir_all(&routes_path)?;

    let file_path = routes_path.join("root.rs");
    if force || !file_path.exists() {
        let tpl = RootTemplate;
        fs_write(file_path, tpl.render()?)?;
    }

    let root_path = routes_path.join("root");
    fs::create_dir_all(&root_path)?;

    let file_path = root_path.join("index.rs");
    if force || !file_path.exists() {
        let tpl = IndexTemplate;
        fs_write(file_path, tpl.render()?)?;
    }

    Ok(())
}

fn fix_env(content: &str, name: &str) -> Result<String> {
    let re = Regex::new(r"RUST_LOG(\s*)=(.+)").unwrap();
    let content = if let Some(caps) = re.captures(content) {
        let sp = caps.get(1).unwrap().as_str();
        let conf = caps.get(2).unwrap().as_str();
        if !conf.contains(&format!("{}=", name)) {
            re.replace(content, format!("RUST_LOG{}={},{}=debug", sp, conf, name))
                .to_string()
        } else {
            content.to_owned()
        }
    } else {
        content.to_owned()
    };
    Ok(content)
}

struct Secret;

impl Secret {
    pub fn secret_key(_dummy: usize) -> String {
        let mut rng = rand::thread_rng();
        let len = rng.next_u32() % 10 + 10;
        Alphanumeric.sample_string(&mut rng, len as usize)
    }

    pub fn secret_no(_dummy: usize) -> u64 {
        let mut rng = rand::thread_rng();
        rng.next_u64()
    }
}

#[derive(Template)]
#[template(path = "new_actix/_Cargo.toml", escape = "none")]
pub struct CargoTemplate {
    pub name: String,
}

#[derive(Template)]
#[template(path = "new_actix/src/auth.rs", escape = "none")]
pub struct AuthTemplate;

#[derive(Template)]
#[template(path = "new_actix/src/context.rs", escape = "none")]
pub struct ContextTemplate;

#[derive(Template)]
#[template(path = "new_actix/src/db.rs", escape = "none")]
pub struct DbTemplate;

#[derive(Template)]
#[template(path = "new_actix/src/gql_log.rs", escape = "none")]
pub struct GqlLogTemplate;

#[derive(Template)]
#[template(
    source = r###"
    db_@{ db|snake }@::start(
        is_hot_deploy,
        exit_tx.clone(),
        Arc::downgrade(db_guard),
        db_dir,
        linker_port,
        pw,
        &uuid_node,
    )
    .await?;
    // Do not modify this line. (DbStart)"###,
    ext = "txt",
    escape = "none"
)]
pub struct DbStartTemplate<'a> {
    pub db: &'a str,
}

#[derive(Template)]
#[template(
    source = r###"
    guard.push(db_@{ db|snake }@::start_test().await?);
    // Do not modify this line. (DbStartTest)"###,
    ext = "txt",
    escape = "none"
)]
pub struct DbStartTestTemplate<'a> {
    pub db: &'a str,
}

#[derive(Template)]
#[template(
    source = r###"
    db_@{ db|snake }@::stop();
    // Do not modify this line. (DbStop)"###,
    ext = "txt",
    escape = "none"
)]
pub struct DbStopTemplate<'a> {
    pub db: &'a str,
}

#[derive(Template)]
#[template(
    source = r###"
    db_@{ db|snake }@::clear_local_cache().await;
    // Do not modify this line. (DbClearLocalCache)"###,
    ext = "txt",
    escape = "none"
)]
pub struct DbClearLocalCacheTemplate<'a> {
    pub db: &'a str,
}

#[derive(Template)]
#[template(
    source = r###"
    db_@{ db|snake }@::clear_whole_cache().await;
    // Do not modify this line. (DbClearCache)"###,
    ext = "txt",
    escape = "none"
)]
pub struct DbClearCacheTemplate<'a> {
    pub db: &'a str,
}

#[derive(Template)]
#[template(
    source = r###"
    @{ db|snake|to_var_name }@: Arc<Mutex<db_@{ db|snake }@::DbConn>>,
    // Do not modify this line. (Repo)"###,
    ext = "txt",
    escape = "none"
)]
pub struct DbRepoTemplate<'a> {
    pub db: &'a str,
}

#[derive(Template)]
#[template(
    source = r###"
            @{ db|snake|to_var_name }@: Arc::new(Mutex::new(db_@{ db|snake }@::DbConn::new_with_time(ctx.ctx_no(), ctx.time()))),
            // Do not modify this line. (RepoNew)"###,
    ext = "txt",
    escape = "none"
)]
pub struct DbRepoNewTemplate<'a> {
    pub db: &'a str,
}

#[derive(Template)]
#[template(
    source = r###"
    fn @{ db|snake }@_repository(&self) -> Box<dyn domain::models::@{ db|snake|to_var_name }@::@{ db|pascal }@Repositories> {
        Box::new(db_@{ db|snake }@::impl_domain::@{ db|pascal }@RepositoriesImpl::new(self.@{ db|snake|to_var_name }@.clone()))
    }
    fn @{ db|snake }@_query(&self) -> Box<dyn domain::models::@{ db|snake|to_var_name }@::@{ db|pascal }@Queries> {
        Box::new(db_@{ db|snake }@::impl_domain::@{ db|pascal }@RepositoriesImpl::new(self.@{ db|snake|to_var_name }@.clone()))
    }
    // Do not modify this line. (RepoImpl)"###,
    ext = "txt",
    escape = "none"
)]
pub struct DbRepoImplTemplate<'a> {
    pub db: &'a str,
}

#[derive(Template)]
#[template(
    source = r###"
        self.@{ db|snake|to_var_name }@.lock().await.begin().await?;
        // Do not modify this line. (RepoImplStart)"###,
    ext = "txt",
    escape = "none"
)]
pub struct DbRepoImplStartTemplate<'a> {
    pub db: &'a str,
}

#[derive(Template)]
#[template(
    source = r###"
        self.@{ db|snake|to_var_name }@.lock().await.commit().await?;
        // Do not modify this line. (RepoImplCommit)"###,
    ext = "txt",
    escape = "none"
)]
pub struct DbRepoImplCommitTemplate<'a> {
    pub db: &'a str,
}

#[derive(Template)]
#[template(
    source = r###"
        self.@{ db|snake|to_var_name }@.lock().await.rollback().await?;
        // Do not modify this line. (RepoImplRollback)"###,
    ext = "txt",
    escape = "none"
)]
pub struct DbRepoImplRollbackTemplate<'a> {
    pub db: &'a str,
}

#[derive(Template)]
#[template(
    source = r###"
        db_@{ db|snake }@::migrate(use_test, clean, ignore_missing),
        // Do not modify this line. (migrate)"###,
    ext = "txt",
    escape = "none"
)]
pub struct DbMigrateTemplate<'a> {
    pub db: &'a str,
}

#[derive(Template)]
#[template(
    source = r###"
    db_@{ db|snake }@::seeder::gen_seed_schema()?;
    // Do not modify this line. (gen_seed_schema)"###,
    ext = "txt",
    escape = "none"
)]
pub struct DbGenSeedSchemaTemplate<'a> {
    pub db: &'a str,
}

#[derive(Template)]
#[template(
    source = r###"
        db_@{ db|snake }@::seeder::seed(use_test, None),
        // Do not modify this line. (seed)"###,
    ext = "txt",
    escape = "none"
)]
pub struct DbSeedTemplate<'a> {
    pub db: &'a str,
}

#[derive(Template)]
#[template(
    source = r###"
        db_@{ db|snake }@::check(use_test),
        // Do not modify this line. (check)"###,
    ext = "txt",
    escape = "none"
)]
pub struct DbCheckTemplate<'a> {
    pub db: &'a str,
}

#[derive(Template)]
#[template(path = "new_actix/src/auto_api.rs", escape = "none")]
pub struct AutoApiTemplate;

#[derive(Template)]
#[template(path = "new_actix/src/main.rs", escape = "none")]
pub struct MainTemplate {
    pub non_snake_case: bool,
}

#[derive(Template)]
#[template(path = "new_actix/src/response.rs", escape = "none")]
pub struct ResponseTemplate;

#[derive(Template)]
#[template(path = "new_actix/src/tasks.rs", escape = "none")]
pub struct TasksTemplate;

#[derive(Template)]
#[template(path = "new_actix/src/tests.rs", escape = "none")]
pub struct TestsTemplate;

#[derive(Template)]
#[template(path = "new_actix/src/common.rs", escape = "none")]
pub struct CommonTemplate;

#[derive(Template)]
#[template(path = "new_actix/src/validator.rs", escape = "none")]
pub struct ValidatorTemplate;

#[derive(Template)]
#[template(path = "new_actix/src/routes/root.rs", escape = "none")]
pub struct RootTemplate;

#[derive(Template)]
#[template(path = "new_actix/src/routes/root/index.rs", escape = "none")]
pub struct IndexTemplate;

mod filters {
    use crate::schema::_to_var_name;
    use convert_case::{Case, Casing};

    pub fn to_var_name(s: &str) -> ::askama::Result<String> {
        Ok(_to_var_name(s))
    }
    pub fn pascal(s: &str) -> ::askama::Result<String> {
        Ok(s.to_case(Case::Pascal))
    }
    pub fn snake(s: &str) -> ::askama::Result<String> {
        Ok(s.to_case(Case::Snake))
    }
}
