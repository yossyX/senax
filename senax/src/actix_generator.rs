use anyhow::{Context as _, Result};
use askama::Template;
use rand::{
    RngCore,
    distr::{Alphanumeric, SampleString},
};
use regex::Regex;
use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::filters;
use crate::{API_SCHEMA_PATH, SCHEMA_PATH, common::ToCase as _, common::fs_write};
use crate::{api_generator::template::DbConfigTemplate, schema::CONFIG};

pub fn generate(
    name: &str,
    db_list: &[&str],
    force: bool,
    db_for_api: bool,
    session: bool,
) -> Result<()> {
    anyhow::ensure!(Path::new("Cargo.toml").exists(), "Incorrect directory.");
    let non_snake_case = crate::common::check_non_snake_case()?;
    for db in db_list {
        crate::common::check_ascii_name(db);
        let path = Path::new(SCHEMA_PATH).join(format!("{db}.yml"));
        anyhow::ensure!(path.exists(), "{} DB is not found.", db);
    }
    crate::common::check_ascii_name(name);
    let base_path: PathBuf = name.parse()?;

    let file_path = Path::new("./Cargo.toml");
    if file_path.exists() {
        let content = fs::read_to_string(file_path)?.replace("\r\n", "\n");
        let re = Regex::new(r"members\s*=\s*\[([^\]]*)\]").unwrap();
        let caps = re
            .captures(&content)
            .with_context(|| format!("Illegal file content:{}", &file_path.to_string_lossy()))?;
        let members = caps.get(1).unwrap().as_str();
        let quoted = format!("\"{}\"", name);
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
        let content = fs::read_to_string(&file_path)?.replace("\r\n", "\n");
        fs_write(file_path, fix_env(&content, name)?)?;
    }

    let mut file_path = PathBuf::from("./.env.example");
    if !file_path.exists() {
        file_path = base_path.join(".env.example");
    }
    if file_path.exists() {
        let content = fs::read_to_string(&file_path)?.replace("\r\n", "\n");
        fs_write(file_path, fix_env(&content, name)?)?;
    }

    let file_path = Path::new("./build.sh");
    if file_path.exists() {
        let content = fs::read_to_string(file_path)?.replace("\r\n", "\n");
        fs_write(file_path, fix_build_sh(&content, name)?)?;
    }

    write_base_files(&base_path, name, db_list, force)?;

    #[derive(Template)]
    #[template(path = "new_actix/_Cargo.toml", escape = "none")]
    pub struct CargoTemplate<'a> {
        pub name: &'a str,
    }

    let file_path = base_path.join("Cargo.toml");
    let mut content = if force || !file_path.exists() {
        CargoTemplate { name }.render()?
    } else {
        fs::read_to_string(&file_path)?.replace("\r\n", "\n")
    };
    for db in db_list.iter().rev() {
        let db = &db.to_snake();
        let reg = Regex::new(&format!(r"(?m)^db_{}\s*=", db))?;
        if !reg.is_match(&content) {
            content = content.replace(
                "[dependencies]",
                &format!(
                    "[dependencies]\ndb_{} = {{ path = \"../2_db/_{}\" }}",
                    db, db
                ),
            );
        }
    }
    fs_write(file_path, &*content)?;

    #[derive(Template)]
    #[template(path = "api/_config.yml", escape = "none")]
    pub struct ConfigTemplate;

    let schema_dir = base_path.join(API_SCHEMA_PATH);
    let config_path = schema_dir.join("_config.yml");
    if !config_path.exists() {
        let tpl = ConfigTemplate;
        fs_write(&config_path, tpl.render()?)?;
    }
    if db_for_api {
        for db in db_list {
            let db_config_path = schema_dir.join(format!("{db}.yml"));
            if !db_config_path.exists() {
                let tpl = DbConfigTemplate;
                fs_write(&db_config_path, tpl.render()?)?;
            }
        }
    }

    let src_path = base_path.join("src");

    #[derive(Template)]
    #[template(path = "new_actix/src/db.rs", escape = "none")]
    pub struct DbTemplate;

    let file_path = src_path.join("db.rs");
    let mut content = if force || !file_path.exists() {
        DbTemplate.render()?
    } else {
        fs::read_to_string(&file_path)?.replace("\r\n", "\n")
    };
    for db in db_list {
        let reg = Regex::new(&format!(r"(?m)^\s*db_{}::init\(\);", &db.to_snake()))?;
        if !reg.is_match(&content) {
            crate::schema::parse(db, false, false)?;
            let config = CONFIG.read().unwrap().as_ref().unwrap().clone();
            let tpl = DbInitTemplate {
                db,
                exclude_from_domain: config.exclude_from_domain,
            };
            content = content.replace(
                "// Do not modify this line. (DbInit)",
                tpl.render()?.trim_start(),
            );
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
            let tpl = DbMigrateTemplate { db };
            content = content.replace(
                "// Do not modify this line. (migrate)",
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
    fs_write(file_path, &*content)?;

    #[derive(Template)]
    #[template(path = "new_actix/src/gql_log.rs", escape = "none")]
    pub struct GqlLogTemplate;

    let file_path = src_path.join("gql_log.rs");
    if force || !file_path.exists() {
        let tpl = GqlLogTemplate;
        fs_write(&file_path, tpl.render()?)?;
    }

    #[derive(Template)]
    #[template(path = "new_actix/src/auto_api.rs", escape = "none")]
    pub struct AutoApiTemplate;

    let file_path = src_path.join("auto_api.rs");
    let mut content = if force || !file_path.exists() {
        AutoApiTemplate.render()?
    } else {
        fs::read_to_string(&file_path)?.replace("\r\n", "\n")
    };
    if session && let Some(db) = db_list.first() {
        content = content.replace("db_session::", &format!("db_{}::", db.to_snake()));
    }
    fs_write(file_path, &*content)?;

    #[derive(Template)]
    #[template(path = "new_actix/src/main.rs", escape = "none")]
    pub struct MainTemplate<'a> {
        pub name: &'a str,
        pub non_snake_case: bool,
    }

    let file_path = src_path.join("main.rs");
    let mut content = if force || !file_path.exists() {
        MainTemplate {
            name,
            non_snake_case,
        }
        .render()?
    } else {
        fs::read_to_string(&file_path)?.replace("\r\n", "\n")
    };
    if session && let Some(db) = db_list.first() {
        content = content.replace("db_session::", &format!("db_{}::", db.to_snake()));
    }
    fs_write(file_path, &*content)?;

    #[derive(Template)]
    #[template(path = "new_actix/src/tasks.rs", escape = "none")]
    pub struct TasksTemplate;

    let file_path = src_path.join("tasks.rs");
    if !file_path.exists() {
        let tpl = TasksTemplate;
        fs_write(file_path, tpl.render()?)?;
    }

    #[derive(Template)]
    #[template(path = "new_actix/src/tests.rs", escape = "none")]
    pub struct TestsTemplate;

    let file_path = src_path.join("tests.rs");
    if !file_path.exists() {
        let tpl = TestsTemplate;
        fs_write(file_path, tpl.render()?)?;
    }

    #[derive(Template)]
    #[template(path = "new_actix/src/routes/root.rs", escape = "none")]
    pub struct RootTemplate;

    let routes_path = src_path.join("routes");
    let file_path = routes_path.join("root.rs");
    if force || !file_path.exists() {
        let tpl = RootTemplate;
        fs_write(file_path, tpl.render()?)?;
    }

    #[derive(Template)]
    #[template(path = "new_actix/src/routes/root/index.rs", escape = "none")]
    pub struct IndexTemplate;

    let root_path = routes_path.join("root");
    let file_path = root_path.join("index.rs");
    if force || !file_path.exists() {
        let tpl = IndexTemplate;
        fs_write(file_path, tpl.render()?)?;
    }

    Ok(())
}

pub fn write_base_files(base_path: &Path, name: &str, db_list: &[&str], force: bool) -> Result<()> {
    let non_snake_case = crate::common::check_non_snake_case()?;
    let base_path = base_path.join("base");

    #[derive(Template)]
    #[template(path = "new_actix/base/_Cargo.toml", escape = "none")]
    pub struct CargoTemplate<'a> {
        pub name: &'a str,
    }

    let file_path = base_path.join("Cargo.toml");
    let mut content = if force || !file_path.exists() {
        CargoTemplate { name }.render()?
    } else {
        fs::read_to_string(&file_path)?.replace("\r\n", "\n")
    };
    for db in db_list.iter().rev() {
        let db = &db.to_snake();
        let reg = Regex::new(&format!(r"(?m)^_db_{}\s*=", db))?;
        if !reg.is_match(&content) {
            content = content.replace(
                "[dependencies]",
                &format!(
                    "[dependencies]\n_db_{} = {{ path = \"../../2_db/_{}/base\" }}",
                    db, db
                ),
            );
        }
    }
    fs_write(file_path, &*content)?;

    #[derive(Template)]
    #[template(path = "new_actix/base/src/auth.rs", escape = "none")]
    pub struct AuthTemplate;

    let src_path = base_path.join("src");
    let file_path = src_path.join("auth.rs");
    if force || !file_path.exists() {
        let tpl = AuthTemplate;
        fs_write(file_path, tpl.render()?)?;
    }

    #[derive(Template)]
    #[template(path = "new_actix/base/src/context.rs", escape = "none")]
    pub struct ContextTemplate;

    let file_path = src_path.join("context.rs");
    if force || !file_path.exists() {
        let tpl = ContextTemplate;
        fs_write(file_path, tpl.render()?)?;
    }

    #[derive(Template)]
    #[template(path = "new_actix/base/src/db.rs", escape = "none")]
    pub struct DbTemplate;

    let file_path = src_path.join("db.rs");
    let mut content = if force || !file_path.exists() {
        DbTemplate.render()?
    } else {
        fs::read_to_string(&file_path)?.replace("\r\n", "\n")
    };
    for db in db_list {
        let chk = format!("_db_{}::clear_local_cache().await;", &db.to_snake());
        if !content.contains(&chk) {
            crate::schema::parse(db, false, false)?;
            let config = CONFIG.read().unwrap().as_ref().unwrap().clone();
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
            if !config.exclude_from_domain {
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
                let tpl = DbRepoStaticTemplate { db };
                content = content.replace(
                    "// Do not modify this line. (RepoStatic)",
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
            }
        }
    }
    fs_write(file_path, &*content)?;

    #[derive(Template)]
    #[template(path = "new_actix/base/src/auto_api.rs", escape = "none")]
    pub struct AutoApiTemplate;

    let file_path = src_path.join("auto_api.rs");
    if force || !file_path.exists() {
        fs_write(&file_path, AutoApiTemplate.render()?)?;
    }

    #[derive(Template)]
    #[template(path = "new_actix/base/src/lib.rs", escape = "none")]
    pub struct LibTemplate {
        pub non_snake_case: bool,
    }

    let file_path = src_path.join("lib.rs");
    if force || !file_path.exists() {
        let tpl = LibTemplate { non_snake_case };
        fs_write(file_path, tpl.render()?)?;
    }

    #[derive(Template)]
    #[template(path = "new_actix/base/src/response.rs", escape = "none")]
    pub struct ResponseTemplate;

    let file_path = src_path.join("response.rs");
    if force || !file_path.exists() {
        let tpl = ResponseTemplate;
        fs_write(file_path, tpl.render()?)?;
    }

    #[derive(Template)]
    #[template(path = "new_actix/base/src/validator.rs", escape = "none")]
    pub struct ValidatorTemplate;

    let file_path = src_path.join("validator.rs");
    if !file_path.exists() {
        let tpl = ValidatorTemplate;
        fs_write(file_path, tpl.render()?)?;
    }

    #[derive(Template)]
    #[template(path = "new_actix/base/src/maybe_undefined.rs", escape = "none")]
    pub struct MaybeUndefinedTemplate;

    let file_path = src_path.join("maybe_undefined.rs");
    if !file_path.exists() {
        let tpl = MaybeUndefinedTemplate;
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

fn fix_build_sh(content: &str, name: &str) -> Result<String> {
    let mut content = content.to_owned();
    if !content.contains(&format!("senax actix api {name}")) {
        content = content.replace(
            "# Do not modify this line. (Api)",
            &format!(
                "senax actix api {name} --auto-fix -c -f  ${name}_client\n# Do not modify this line. (Api)",
            ),
        );
    }
    if !content.contains(&format!("schema {name} \"${name}_client\"")) {
        content = content.replace(
            "# Do not modify this line. (Schema)",
            &format!("schema {name} \"${name}_client\"\n# Do not modify this line. (Schema)",),
        );
    }
    Ok(content)
}

struct Secret;

impl Secret {
    pub fn secret_key(_dummy: usize) -> String {
        let mut rng = rand::rng();
        let len = rng.next_u32() % 10 + 10;
        Alphanumeric.sample_string(&mut rng, len as usize)
    }

    pub fn secret_no(_dummy: usize) -> u64 {
        let mut rng = rand::rng();
        rng.next_u64()
    }
}

#[derive(Template)]
#[template(
    source = r###"
    db_@{ db|snake }@::init();
    @%- if !exclude_from_domain %@
    let _ = crate::_base::db::@{ db|upper_snake }@_REPO.set(Box::new(|conn| Box::new(db_@{ db|snake }@::impl_domain::@{ db|pascal }@RepositoryImpl::new(conn.clone()))));
    let _ = crate::_base::db::@{ db|upper_snake }@_QS.set(Box::new(|conn| Box::new(db_@{ db|snake }@::impl_domain::@{ db|pascal }@RepositoryImpl::new(conn.clone()))));
    @%- endif %@
    // Do not modify this line. (DbInit)"###,
    ext = "txt",
    escape = "none"
)]
pub struct DbInitTemplate<'a> {
    pub db: &'a str,
    pub exclude_from_domain: bool,
}

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
    _db_@{ db|snake }@::clear_local_cache().await;
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
    _db_@{ db|snake }@::clear_all_cache().await;
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
    @{ db|snake|ident }@: Arc<Mutex<_db_@{ db|snake }@::DbConn>>,
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
            @{ db|snake|ident }@: Arc::new(Mutex::new(_db_@{ db|snake }@::DbConn::new_with_time(ctx.ctx_no(), ctx.time()))),
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
pub static @{ db|upper_snake }@_REPO: OnceCell<Box<dyn Fn(&Arc<Mutex<_db_@{ db|snake }@::DbConn>>) -> Box<dyn domain::repository::@{ db|snake|ident }@::@{ db|pascal }@Repository> + Send + Sync>> = OnceCell::new();
pub static @{ db|upper_snake }@_QS: OnceCell<Box<dyn Fn(&Arc<Mutex<_db_@{ db|snake }@::DbConn>>) -> Box<dyn domain::repository::@{ db|snake|ident }@::@{ db|pascal }@QueryService> + Send + Sync>> = OnceCell::new();
// Do not modify this line. (RepoStatic)"###,
    ext = "txt",
    escape = "none"
)]
pub struct DbRepoStaticTemplate<'a> {
    pub db: &'a str,
}

#[derive(Template)]
#[template(
    source = r###"
    fn @{ db|snake }@_repository(&self) -> Box<dyn domain::repository::@{ db|snake|ident }@::@{ db|pascal }@Repository> {
        @{ db|upper_snake }@_REPO.get().expect("No @{ db|upper_snake }@_REPO")(&self.@{ db|snake|ident }@)
    }
    fn @{ db|snake }@_query(&self) -> Box<dyn domain::repository::@{ db|snake|ident }@::@{ db|pascal }@QueryService> {
        @{ db|upper_snake }@_QS.get().expect("No @{ db|upper_snake }@_QS")(&self.@{ db|snake|ident }@)
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
        self.@{ db|snake|ident }@.lock().await.begin().await?;
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
        self.@{ db|snake|ident }@.lock().await.commit().await?;
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
        self.@{ db|snake|ident }@.lock().await.rollback().await?;
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
    if db.is_none() || db == Some("@{ db }@") {
        join_set.spawn_local(db_@{ db|snake }@::migrate(use_test, clean, ignore_missing, remove_missing));
    }
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
    db_@{ db|snake }@::seeder::seed(_use_test, None).await?;
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
