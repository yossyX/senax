@% if non_snake_case -%@
#![allow(non_snake_case)]
@% endif -%@
#[macro_use]
extern crate log;

use actix_web::dev::Service as _;
use actix_web::web::Data;
use actix_web::{guard, middleware, web, App, HttpMessage, HttpServer};
use anyhow::{ensure, Context, Result};
use async_graphql::{EmptySubscription, Schema};
use clap::{Parser, Subcommand};
use db_session::models::session::session::_SessionStore;
use dotenvy::dotenv;
use mimalloc::MiMalloc;
use once_cell::sync::OnceCell;
use sha2::{Digest, Sha512};
use std::collections::HashMap;
use std::env;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Weak};
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

use crate::auto_api::{MutationRoot, QueryRoot};
use crate::context::Ctx;

mod auto_api;
mod auth;
mod common;
mod context;
mod db;
mod gql_log;
mod response;
mod routes {
    pub mod root;
}
mod tasks;
#[cfg(test)]
mod tests;
mod validator;

const HOST_PORT: &str = "HOST_PORT";
const WORK_DIR: &str = "WORK_DIR";
const DEFAULT_HOST_PORT: &str = "0.0.0.0:8080";
const DEFAULT_WORK_DIR: &str = "temp";
const LINKER_PORT: &str = "LINKER_PORT";
const LINKER_PASSWORD: &str = "LINKER_PASSWORD";
const SECRET_KEY: &str = "SECRET_KEY";
const SESSION_SECRET_KEY: &str = "SESSION_SECRET_KEY";
// for Hot Deploy
const SERVER_STARTER_PORT: &str = "SERVER_STARTER_PORT";
#[cfg(unix)]
const KILL_PARENT: &str = "KILL_PARENT";

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

static SHUTDOWN_GUARD: OnceCell<Weak<mpsc::Sender<u8>>> = OnceCell::new();

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct AppArg {
    #[clap(long)]
    auto_migrate: bool,
    #[clap(long = "pid")]
    /// output pid file
    pid: Option<String>,
    #[clap(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, PartialEq, Clone, Debug)]
enum Command {
    GqlSchema,
    Migrate {
        /// Drop DB before migrating
        #[clap(short, long)]
        clean: bool,
        /// Drop DB before migrating in release environment
        #[clap(long)]
        force_delete_all_db: bool,
        /// Use test environment
        #[clap(short, long)]
        test: bool,
    },
    GenSeedSchema,
    Seed {
        /// Use clean migration
        #[clap(short, long)]
        clean: bool,
        /// Drop DB before migrating in release environment
        #[clap(long)]
        force_delete_all_db: bool,
        /// Use test environment
        #[clap(short, long)]
        test: bool,
    },
    Check {
        /// Use test environment
        #[clap(short, long)]
        test: bool,
    },
    /// generate graphql json schema
    GenGqlSchema {
        #[clap(long)]
        ts_dir: PathBuf,
    },
}

#[actix_web::main]
async fn main() -> Result<()> {
    dotenv().ok();
    #[cfg(feature = "etcd")]
    senax_common::etcd::init().await?;

    let arg: AppArg = AppArg::parse();
    let schema = Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .extension(gql_log::GqlLogger)
        .extension(
            async_graphql::extensions::apollo_persisted_queries::ApolloPersistedQueries::new(
                async_graphql::extensions::apollo_persisted_queries::LruCacheStorage::new(1000),
            ),
        )
        .limit_complexity(auto_api::LIMIT_COMPLEXITY);
    let schema = if cfg!(debug_assertions) || cfg!(feature = "graphiql") {
        schema.finish()
    } else {
        schema
            .disable_introspection()
            .disable_suggestions()
            .finish()
    };
    if let Some(command) = arg.command {
        match command {
            Command::GqlSchema => {
                println!("{}", &schema.sdl());
                return Ok(());
            }
            Command::Migrate {
                clean,
                test,
                force_delete_all_db,
            } => {
                if clean {
                    ensure!(
                        force_delete_all_db || cfg!(debug_assertions),
                        "clean migrate is debug environment only"
                    );
                }
                db::migrate(test, clean || force_delete_all_db, false).await?;
                return Ok(());
            }
            Command::GenSeedSchema => {
                db::gen_seed_schema()?;
                return Ok(());
            }
            Command::Seed {
                clean,
                test,
                force_delete_all_db,
            } => {
                if clean {
                    ensure!(
                        force_delete_all_db || cfg!(debug_assertions),
                        "clean migrate is debug environment only"
                    );
                    db::migrate(test, clean || force_delete_all_db, false).await?;
                }
                db::seed(test).await?;
                return Ok(());
            }
            Command::Check { test } => {
                db::check(test).await?;
                return Ok(());
            }
            Command::GenGqlSchema { ts_dir } => {
                auto_api::gen_json_schema(&ts_dir.join("src").join("gql_query"))?;
                return Ok(());
            }
        }
    }

    let offset_in_sec = chrono::Local::now().offset().local_minus_utc();
    senax_logger::init(
        senax_logger::Rotation::DAILY,
        Some(time::UtcOffset::from_whole_seconds(offset_in_sec)?),
        !cfg!(debug_assertions),
    )?;
    if arg.auto_migrate {
        info!("Starting migration");
        db::migrate(false, false, true).await?;
    }
    let port = env::var(HOST_PORT).unwrap_or_else(|_| DEFAULT_HOST_PORT.to_owned());
    info!("HOST_PORT: {:?}", port);
    let dir = env::var(WORK_DIR).unwrap_or_else(|_| DEFAULT_WORK_DIR.to_owned());
    info!("WORK_DIR: {:?}", dir);
    let is_hot_deploy = env::var(SERVER_STARTER_PORT).is_ok();
    let linker_port = env::var(LINKER_PORT).ok();
    info!("LINKER_PORT: {:?}", linker_port);
    let linker_pw = env::var(LINKER_PASSWORD).ok();
    let secret_key = env::var(SECRET_KEY).with_context(|| format!("{} required", SECRET_KEY))?;
    auth::SECRET
        .set(format!("{}{}", auth::INNER_KEY.as_str(), secret_key))
        .unwrap();

    let (exit_tx, mut exit_rx) = mpsc::channel::<i32>(1);
    let (db_guard_tx, mut db_guard_rx) = mpsc::channel::<u8>(1);
    let db_guard_tx = Arc::new(db_guard_tx);
    let db_guard = Arc::clone(&db_guard_tx);
    let (app_guard_tx, mut app_guard_rx) = mpsc::channel::<u8>(1);
    let app_guard_tx = Arc::new(app_guard_tx);
    SHUTDOWN_GUARD.set(Arc::downgrade(&app_guard_tx)).unwrap();
    tokio::spawn(async move {
        let db_dir = Path::new(&dir);
        db::start(
            is_hot_deploy,
            exit_tx.clone(),
            &db_guard,
            db_dir,
            &linker_port,
            &linker_pw,
        )
        .await
        .unwrap();
    })
    .await?;
    tokio::spawn(async {
        // For rolling update
        tokio::time::sleep(std::time::Duration::from_secs(300)).await;
        db::clear_local_cache().await;
    });

    #[cfg(feature = "v8")]
    {
        let platform = v8::Platform::new(0, false).make_shared();
        v8::V8::initialize_platform(platform);
        v8::V8::initialize();
    }

    let mut listeners = take_listener(&[&port])?;

    tokio::spawn(handle_signals());

    tokio::spawn(async move {
        loop {
            senax_actix_session::SessionMiddleware::gc(_SessionStore, None).await;
            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
        }
    });
    let session_secret_key = env::var(SESSION_SECRET_KEY)
        .map(|v| Sha512::digest(v).to_vec())
        .with_context(|| format!("{} required", SESSION_SECRET_KEY))?;

    let server = HttpServer::new(move || {
        App::new()
            .wrap(
                middleware::Logger::new(
                    r#"%a "%r" %s %b "%{Referer}i" "%{User-Agent}i" %{ctx}xi %{username}xi %T"#,
                )
                .log_target("access_log")
                .custom_request_replace("ctx", |req| {
                    let ctx = Ctx::get(req.request());
                    format!("ctx={}", ctx.ctx_no())
                })
                .custom_request_replace("username", |req| {
                    if let Some(auth) = req.request().extensions().get::<auth::AuthInfo>() {
                        format!("username={}", auth.username())
                    } else {
                        String::new()
                    }
                }),
            )
            .wrap(middleware::Compress::default())
            .wrap_fn(|req, srv| {
                req.extensions_mut().insert(Ctx::new());
                if let Some(auth) = auth::retrieve_auth(req.request()) {
                    req.extensions_mut().insert(auth);
                }
                srv.call(req)
            })
            .wrap(
                senax_actix_session::SessionMiddleware::builder(_SessionStore, &session_secret_key)
                    .cookie_secure(!cfg!(debug_assertions))
                    .build(),
            )
            .configure(routes::root::init)
            // .service(
            //     web::scope("/api")
            //         .app_data(
            //             web::JsonConfig::default().error_handler(response::json_error_handler),
            //         )
            //         .configure(api::init),
            // )
            .service(
                web::resource("/gql")
                    .guard(guard::Post())
                    .app_data(Data::new(schema.clone()))
                    .to(auto_api::graphql),
            )
            .service(
                web::resource("/gql")
                    .guard(guard::fn_guard(|_| {
                        cfg!(debug_assertions) || cfg!(feature = "graphiql")
                    }))
                    .guard(guard::Get())
                    .app_data(Data::new(schema.clone()))
                    .to(auto_api::graphiql),
            )
    })
    .listen(listeners.remove(&port).unwrap())?;

    kill_parent();
    output_pid_file(&arg.pid)?;

    info!("Starting server");
    let exit_code = tokio::select! {
        result = server.run() => {
            match result {
                Ok(_) => 0,
                Err(e) => {
                    error!("{}", e);
                    1
                },
            }
        }
        Some(code) = exit_rx.recv() => {
            code
        }
        else => 0,
    };

    drop(app_guard_tx);
    while let Some(_i) = app_guard_rx.recv().await {}
    sleep(Duration::from_millis(100)).await;
    db::stop();
    drop(db_guard_tx);
    while let Some(_i) = db_guard_rx.recv().await {}

    #[cfg(feature = "v8")]
    {
        unsafe {
            v8::V8::dispose();
        }
        v8::V8::dispose_platform();
    }

    info!("server stopped");
    if exit_code != 0 {
        std::process::exit(exit_code);
    }
    Ok(())
}

pub fn get_shutdown_guard() -> Option<Arc<mpsc::Sender<u8>>> {
    SHUTDOWN_GUARD.wait().upgrade()
}

#[cfg(unix)]
fn take_listener(ports: &[&str]) -> Result<HashMap<String, TcpListener>> {
    use nix::fcntl;
    use std::os::unix::io::{FromRawFd, IntoRawFd};

    let mut results = HashMap::new();
    let mut env_str = Vec::new();
    let starter_port = env::var(SERVER_STARTER_PORT);
    if let Ok(port) = starter_port {
        let list: Vec<&str> = port.split(';').collect();
        for row in list {
            let pair: Vec<&str> = row.split('=').collect();
            if pair.len() == 2 {
                let fd: i32 = pair[1].parse()?;
                let listener = unsafe { TcpListener::from_raw_fd(fd) };
                results.insert(pair[0].to_owned(), listener);
                env_str.push(row.to_string());
            }
        }
    }
    for port in ports {
        if !results.contains_key(*port) {
            let listener = TcpListener::bind(port)?;
            let fd = listener.into_raw_fd();
            fcntl::fcntl(fd, fcntl::FcntlArg::F_SETFD(fcntl::FdFlag::empty()))?;
            let listener = unsafe { TcpListener::from_raw_fd(fd) };
            results.insert(port.to_string(), listener);
            env_str.push(format!("{}={}", port, fd));
        }
    }
    env::set_var(SERVER_STARTER_PORT, env_str.join(";"));
    Ok(results)
}

#[cfg(not(unix))]
fn take_listener(ports: &[&str]) -> Result<HashMap<String, TcpListener>> {
    let mut results = HashMap::new();
    for port in ports {
        let listener = TcpListener::bind(port)?;
        results.insert(port.to_string(), listener);
    }
    Ok(results)
}

#[cfg(unix)]
async fn handle_signals() {
    use futures::stream::StreamExt;
    use nix::unistd;
    use signal_hook::consts::signal::*;
    use signal_hook_tokio::Signals;
    use std::ffi::CString;

    let mut signals = Signals::new([SIGUSR1, SIGUSR2]).unwrap();
    while let Some(signal) = signals.next().await {
        match signal {
            SIGUSR1 => db::clear_whole_cache().await,
            SIGUSR2 => match unsafe { unistd::fork() }.expect("fork failed") {
                unistd::ForkResult::Parent { .. } => {}
                unistd::ForkResult::Child => {
                    let args = env::args()
                        .map(|arg| CString::new(arg).unwrap())
                        .collect::<Vec<CString>>();
                    env::set_var(KILL_PARENT, "true");
                    unistd::execv(&args[0], &args).expect("execution failed.");
                    unreachable!()
                }
            },
            _ => unreachable!(),
        }
    }
}

#[cfg(not(unix))]
async fn handle_signals() {}

#[cfg(unix)]
fn kill_parent() {
    use nix::sys::signal::{self, Signal};
    use nix::unistd;

    if env::var(KILL_PARENT).is_err() {
        return;
    }
    signal::kill(unistd::getppid(), Signal::SIGTERM).unwrap();
}

#[cfg(not(unix))]
fn kill_parent() {}

#[cfg(unix)]
fn output_pid_file(file: &Option<String>) -> Result<()> {
    use nix::unistd;
    use std::fs::File;
    use std::io::Write;

    if let Some(ref file) = file {
        let mut file = File::create(file)?;
        write!(file, "{}", unistd::getpid())?;
        file.flush()?;
    }
    Ok(())
}
#[cfg(not(unix))]
fn output_pid_file(_file: &Option<String>) -> Result<()> {
    Ok(())
}
@{-"\n"}@