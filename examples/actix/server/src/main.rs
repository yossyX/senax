#[macro_use]
extern crate log;

use actix::{Arbiter, System};
use actix_web::dev::Service as _;
use actix_web::{error, middleware, web, App, HttpMessage, HttpRequest, HttpResponse, HttpServer};
use anyhow::Result;
use clap::Parser;
use db_session::session::session::{_SessionStore, SESSION_SECRET_KEY};
use dotenvy::dotenv;
use futures::stream::StreamExt;
use mimalloc::MiMalloc;
use once_cell::sync::OnceCell;
use std::collections::HashMap;
use std::fs::File;

use std::io::Write;
use std::net::TcpListener;
use std::path::Path;

use std::sync::{Arc, Weak};
use std::{env, thread};
use time::macros::offset;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

use crate::context::Ctx;

mod context;
mod db;
mod response;
mod routes {
    pub mod api;
    pub mod root;
}

const HOST_PORT: &str = "HOST_PORT";
const WORK_DIR: &str = "WORK_DIR";
const DEFAULT_HOST_PORT: &str = "0.0.0.0:8080";
const DEFAULT_WORK_DIR: &str = "temp";
const LINKER_PORT: &str = "LINKER_PORT";
const LINKER_PASSWORD: &str = "LINKER_PASSWORD";

// for Hot Deploy
const SERVER_STARTER_PORT: &str = "SERVER_STARTER_PORT";
const KILL_PARENT: &str = "KILL_PARENT";

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

static SHUTDOWN_GUARD: OnceCell<Weak<mpsc::Sender<u8>>> = OnceCell::new();

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct AppArg {
    #[clap(long = "pid")]
    /// output pid file
    pid: Option<String>,
}

#[actix_web::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let arg: AppArg = AppArg::parse();

    senax_logger::init(Some(offset!(+9)))?;

    let port = env::var(HOST_PORT).unwrap_or_else(|_| DEFAULT_HOST_PORT.to_owned());
    let dir = env::var(WORK_DIR).unwrap_or_else(|_| DEFAULT_WORK_DIR.to_owned());
    let is_hot_deploy = env::var(SERVER_STARTER_PORT).is_ok();
    let linker_port = env::var(LINKER_PORT).ok();
    let linker_pw = env::var(LINKER_PASSWORD).ok();
    let use_linker = linker_port.is_some();

    let (exit_tx, mut exit_rx) = mpsc::channel::<i32>(1);
    let (db_guard_tx, mut db_guard_rx) = mpsc::channel::<u8>(1);
    let db_guard_tx = Arc::new(db_guard_tx);
    let db_guard = Arc::clone(&db_guard_tx);
    let (app_guard_tx, mut app_guard_rx) = mpsc::channel::<u8>(1);
    let app_guard_tx = Arc::new(app_guard_tx);
    SHUTDOWN_GUARD.set(Arc::downgrade(&app_guard_tx)).unwrap();
    // The following will prevent the arbiter from being killed on shutdown.
    let arbiter = thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(move || {
            let system = System::new();
            system.block_on(async move {
                let db_dir = Path::new(&dir);
                let arbiter = Arbiter::new();
                let handle = arbiter.handle();
                db::start(
                    &handle,
                    is_hot_deploy,
                    exit_tx.clone(),
                    &db_guard,
                    db_dir,
                    &linker_port,
                    &linker_pw,
                )
                .await?;
                Ok::<Arbiter, anyhow::Error>(arbiter)
            })
        })?
        .join()
        .unwrap()?;

    let mut listeners = take_listener(&[&port])?;

    tokio::spawn(handle_signals());

    if use_linker {
        sleep(Duration::from_secs(2)).await;
    }
    thread::spawn(|| {
        System::new().block_on(async move {
            loop {
                senax_actix_session::SessionMiddleware::gc(_SessionStore, None).await;
                tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
            }
        })
    });

    let server = HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .wrap_fn(|req, srv| {
                req.extensions_mut().insert(Ctx::new());
                srv.call(req)
            })
            .wrap(
                senax_actix_session::SessionMiddleware::builder(_SessionStore, SESSION_SECRET_KEY)
                    .cookie_secure(!cfg!(debug_assertions))
                    .build(),
            )
            .configure(routes::root::init)
            .service(
                web::scope("/api")
                    .app_data(web::JsonConfig::default().error_handler(json_error_handler))
                    .configure(routes::api::init),
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
    arbiter.stop();
    let _ = arbiter.join();

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
    use nix::unistd;
    use signal_hook::consts::signal::*;
    use signal_hook_tokio::Signals;
    use std::ffi::CString;

    let mut signals = Signals::new(&[SIGUSR2]).unwrap();
    while let Some(signal) = signals.next().await {
        match signal {
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

    if let Some(ref file) = file {
        let mut file = File::create(file)?;
        write!(file, "{}", unistd::getpid())?;
        file.flush()?;
    }
    Ok(())
}
#[cfg(not(unix))]
fn output_pid_file(file: &Option<String>) -> Result<()> {
    Ok(())
}

fn json_error_handler(err: error::JsonPayloadError, _req: &HttpRequest) -> error::Error {
    use actix_web::error::JsonPayloadError;

    let detail = err.to_string();
    let resp = match &err {
        JsonPayloadError::ContentType => HttpResponse::UnsupportedMediaType().body(detail),
        JsonPayloadError::Deserialize(json_err) if json_err.is_data() => {
            HttpResponse::UnprocessableEntity().json(crate::response::BadRequest::new(detail))
        }
        _ => HttpResponse::BadRequest().body(detail),
    };
    error::InternalError::from_response(err, resp).into()
}
