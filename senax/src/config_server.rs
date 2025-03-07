use anyhow::{ensure, Result};
use axum::extract::Path as AxumPath;
use axum::http::header::{ACCEPT_ENCODING, CONTENT_ENCODING, CONTENT_TYPE};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::response::Response;
use axum::routing::delete;
use axum::routing::put;
use axum::{
    routing::{get, post},
    Json, Router,
};
use chrono::Local;
use derive_more::Display;
use http::header::{IF_MODIFIED_SINCE, LAST_MODIFIED};
use httpdate::HttpDate;
use includedir::Compression;
use indexmap::IndexMap;
use mime_guess::MimeGuess;
use once_cell::sync::Lazy;
use serde::Serialize;
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::atomic::Ordering;
use std::time::SystemTime;
use tokio::sync::Semaphore;
use tower_http::compression::CompressionLayer;
use validator::Validate;

use crate::api_generator::schema::{
    ApiConfigDef, ApiConfigJson, ApiDbDef, ApiDbJson, ApiModelDef, ApiModelJson,
};
use crate::common::{
    parse_yml, parse_yml_file, read_api_yml, read_group_yml, read_simple_vo_yml, set_api_config,
    simplify_yml, write_api_yml, write_group_yml, write_simple_vo_yml, BACKUP, READ_ONLY,
};
use crate::schema::{self, ConfigDef, ConfigJson, FieldDef, ModelDef, ModelJson, ValueObjectJson};
use crate::{API_SCHEMA_PATH, SCHEMA_PATH};

static SEMAPHORE: Semaphore = Semaphore::const_new(1);

pub async fn start(
    host: &Option<String>,
    port: Option<u16>,
    open: bool,
    backup: &Option<PathBuf>,
    read_only: bool,
) -> anyhow::Result<()> {
    if let Some(backup) = backup.clone() {
        ensure!(backup.is_dir(), "Specify a directory for backup");
        crate::common::BACKUP.set(backup).unwrap();
    }
    crate::common::READ_ONLY.store(read_only, std::sync::atomic::Ordering::SeqCst);

    let compression_layer: CompressionLayer = CompressionLayer::new()
        .br(true)
        .deflate(true)
        .gzip(true)
        .zstd(true);

    let app = Router::new()
        .route("/", get(root_handler))
        .nest("/api", api_routes())
        .route("/*path", get(file_handler))
        .layer(compression_layer);

    if std::env::var("AWS_LAMBDA_RUNTIME_API").is_ok() {
        let app = tower::ServiceBuilder::new()
            .layer(axum_aws_lambda::LambdaLayer::default())
            .service(app);
        lambda_http::run(app).await.unwrap();
    } else {
        let host = host.as_deref().unwrap_or(crate::DEFAULT_CONFIG_HOST);
        let port = port.unwrap_or(crate::DEFAULT_CONFIG_PORT);
        let host_port = format!("{host}:{port}");
        let addr: std::net::SocketAddr = host_port.parse()?;
        let url = format!("http://localhost:{port}/");
        use std::io::Write;
        use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
        let mut stdout = StandardStream::stdout(ColorChoice::Auto);
        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Blue)))?;
        write!(&mut stdout, "{url}")?;
        stdout.reset()?;
        writeln!(&mut stdout)?;
        if open {
            let _ = webbrowser::open(&url);
        }

        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await
            .unwrap();
        writeln!(&mut stdout, "stop")?;
    }
    Ok(())
}

async fn shutdown_signal() {
    use tokio::signal;
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

pub fn api_routes() -> Router {
    Router::new()
        .route("/db", get(get_db))
        .route("/db/:db", get(get_db_config))
        .route("/db/:db", post(save_db_config))
        .route("/config_schema", get(get_config_schema))
        .route("/model_schema", get(get_model_schema))
        .route("/vo_schema/simple", get(get_vo_schema))
        .route("/api_config_schema", get(get_api_config_schema))
        .route("/api_db_schema", get(get_api_db_schema))
        .route("/api_schema", get(get_api_schema))
        .route("/model_names/:db", get(get_model_names))
        .route("/models/:db/:group", get(get_models))
        .route("/merged_models/:db/:group", get(get_merged_models))
        .route("/models/:db/:group", post(create_model))
        .route("/models/:db/:group/:model", put(save_model))
        .route("/models/:db/:group/:model", delete(delete_model))
        .route("/models/:db/:group", put(save_models))
        .route("/vo/simple", get(get_simple_vo))
        .route("/vo/simple", put(save_simple_vo_list))
        .route("/vo/simple", post(create_simple_vo))
        .route("/vo/simple/:vo", put(save_simple_vo))
        .route("/vo/simple/:vo", delete(delete_simple_vo))
        .route("/api_server", get(get_api_server))
        .route("/api_server/:server/_db", get(get_api_server_db))
        .route("/api_server/:server/_config", get(get_api_server_config))
        .route("/api_server/:server/_config", post(save_api_server_config))
        .route(
            "/api_server/:server/:db/_groups",
            get(get_api_server_groups),
        )
        .route(
            "/api_server/:server/:db/_config",
            get(get_api_server_db_config),
        )
        .route(
            "/api_server/:server/:db/_config",
            post(save_api_server_db_config),
        )
        .route(
            "/api_server/:server/:db/:group/_models",
            get(get_api_server_models),
        )
        .route(
            "/clean_api_server/:server/:db/:group",
            get(clean_api_server_models),
        )
        .route(
            "/api_server/:server/:db/:group",
            get(get_api_server_model_paths),
        )
        .route(
            "/api_server/:server/:db/:group",
            put(save_api_server_models),
        )
        .route(
            "/api_server/:server/:db/:group",
            post(create_api_server_model),
        )
        .route(
            "/api_server/:server/:db/:group/:model",
            put(update_api_server_model),
        )
        .route(
            "/api_server/:server/:db/:group/:model",
            delete(delete_api_server_model),
        )
        .route("/build/exec", post(build_exec))
        .route("/build/result", get(build_result))
        .route("/git/exec/:cmd", post(git_exec))
        .route("/git/result", get(git_result))
}

async fn root_handler(headers: HeaderMap) -> impl IntoResponse {
    if let Ok(tpl) = crate::CONFIG_APP.get_raw("config-app/dist/index.html") {
        file_response(tpl, mime_guess::from_path("index.html"), &headers)
    } else {
        panic!("index.html not found")
    }
}
async fn file_handler(
    AxumPath(mut path): AxumPath<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if path.is_empty() || path.ends_with('/') {
        path.push_str("index.html");
    }
    if let Ok(tpl) = crate::CONFIG_APP.get_raw(&format!("config-app/dist/{}", path)) {
        file_response(tpl, mime_guess::from_path(path), &headers)
    } else if let Ok(tpl) = crate::CONFIG_APP.get_raw("config-app/dist/index.html") {
        file_response(tpl, mime_guess::from_path("index.html"), &headers)
    } else {
        panic!("index.html not found")
    }
}
fn file_response(
    tpl: (Compression, Cow<'static, [u8]>),
    mime: MimeGuess,
    req_headers: &HeaderMap,
) -> impl IntoResponse {
    static TIME: Lazy<HttpDate> = Lazy::new(|| HttpDate::from(SystemTime::now()));

    let is_gzip_request = req_headers
        .get(&ACCEPT_ENCODING)
        .map(|v| {
            v.to_str()
                .map(|v| v.split(',').map(|v| v.trim()).any(|v| v.eq("gzip")))
                .unwrap_or_default()
        })
        .unwrap_or_default();
    let mut res_headers = HeaderMap::new();
    if let Some(mime) = mime.first() {
        res_headers.insert(CONTENT_TYPE, mime.essence_str().parse().unwrap());
    }

    if let Some(time) = req_headers.get(&IF_MODIFIED_SINCE) {
        if let Ok(Ok(t)) = time.to_str().map(HttpDate::from_str) {
            if t.eq(&TIME) {
                return (StatusCode::NOT_MODIFIED, res_headers, Vec::new().into());
            }
        }
    }

    res_headers.insert(LAST_MODIFIED, TIME.to_string().parse().unwrap());
    match tpl.0 {
        Compression::None => {}
        Compression::Gzip => {
            if is_gzip_request {
                res_headers.insert(CONTENT_ENCODING, "gzip".parse().unwrap());
            } else {
                use std::io::Read;
                let mut dec = flate2::read::GzDecoder::new(tpl.1.as_ref());
                let mut vec = Vec::new();
                dec.read_to_end(&mut vec).unwrap();
                return (StatusCode::OK, res_headers, vec.into());
            }
        }
    }
    (StatusCode::OK, res_headers, tpl.1)
}

async fn get_db() -> impl IntoResponse {
    let _semaphore = SEMAPHORE.acquire().await;
    let result = crate::db_generator::list();
    json_response(result)
}

async fn get_db_config(AxumPath(db): AxumPath<String>) -> impl IntoResponse {
    let _semaphore = SEMAPHORE.acquire().await;
    let result = async move {
        let db_name = &db;
        check_ascii_name(db_name)?;
        let path = Path::new(SCHEMA_PATH).join(format!("{db_name}.yml"));
        let config: ConfigDef = parse_yml_file(&path)?;
        let config: ConfigJson = config.into();
        Ok(config)
    }
    .await;
    json_response(result)
}

async fn save_db_config(
    AxumPath(db): AxumPath<String>,
    Json(data): Json<ConfigJson>,
) -> impl IntoResponse {
    let _semaphore = SEMAPHORE.acquire().await;
    let result = async move {
        let db = &db;
        check_ascii_name(db)?;
        if !READ_ONLY.load(Ordering::SeqCst) {
            let path = Path::new(SCHEMA_PATH).join(format!("{db}.yml"));
            let content = fs::read_to_string(&path)?;
            if let Some(bk) = BACKUP.get() {
                let dir = bk.join(format!("db_config-{db}-{}.yml", Local::now()));
                fs::write(dir, &content)?;
            }
            let old_config: ConfigDef = parse_yml(&content)?;
            let set: HashSet<_> = data.groups.iter().filter_map(|v| v._name.clone()).collect();
            for (group_name, _) in &old_config.groups {
                if !set.contains(group_name) {
                    check_ascii_name(group_name)?;
                    let group_path = Path::new(SCHEMA_PATH)
                        .join(db)
                        .join(format!("{group_name}.yml"));
                    fs::remove_file(group_path)?;
                }
            }
            for group in &data.groups {
                if let Some(old_name) = &group._name {
                    if old_name != &group.name {
                        let old_path = Path::new(SCHEMA_PATH)
                            .join(db)
                            .join(format!("{old_name}.yml"));
                        let new_path = Path::new(SCHEMA_PATH)
                            .join(db)
                            .join(format!("{}.yml", group.name));
                        fs::rename(old_path, new_path)?;
                    }
                }
            }

            let config: ConfigDef = data.into();
            let mut buf =
                "# yaml-language-server: $schema=../senax-schema.json#definitions/ConfigDef\n\n"
                    .to_string();
            buf.push_str(&simplify_yml(serde_yaml::to_string(&config)?)?);
            fs::write(path, &buf)?;
        }
        Ok(true)
    }
    .await;
    json_response(result)
}

async fn get_config_schema() -> impl IntoResponse {
    let _semaphore = SEMAPHORE.acquire().await;
    let result = async move {
        let schema = schema::json_schema::json_config_schema()?;
        Ok(schema)
    }
    .await;
    json_response(result)
}

async fn get_model_schema() -> impl IntoResponse {
    let _semaphore = SEMAPHORE.acquire().await;
    let result = async move {
        let schema = schema::json_schema::json_model_schema()?;
        Ok(schema)
    }
    .await;
    json_response(result)
}

async fn get_vo_schema() -> impl IntoResponse {
    let _semaphore = SEMAPHORE.acquire().await;
    let result = async move {
        let schema = schema::json_schema::json_simple_vo_schema()?;
        Ok(schema)
    }
    .await;
    json_response(result)
}

async fn get_api_config_schema() -> impl IntoResponse {
    let _semaphore = SEMAPHORE.acquire().await;
    let result = async move {
        let schema = schema::json_schema::json_api_config_schema()?;
        Ok(schema)
    }
    .await;
    json_response(result)
}

async fn get_api_db_schema() -> impl IntoResponse {
    let _semaphore = SEMAPHORE.acquire().await;
    let result = async move {
        let schema = schema::json_schema::json_api_db_schema()?;
        Ok(schema)
    }
    .await;
    json_response(result)
}

async fn get_api_schema() -> impl IntoResponse {
    let _semaphore = SEMAPHORE.acquire().await;
    let result = async move {
        let schema = schema::json_schema::json_api_schema()?;
        Ok(schema)
    }
    .await;
    json_response(result)
}

async fn get_model_names(AxumPath(db): AxumPath<String>) -> impl IntoResponse {
    let _semaphore = SEMAPHORE.acquire().await;
    let result = async move {
        let db_name = &db;
        check_ascii_name(db_name)?;
        let path = Path::new(SCHEMA_PATH).join(format!("{db_name}.yml"));
        let config: ConfigDef = parse_yml_file(&path)?;
        let mut result = IndexMap::new();
        for (group_name, _) in config.groups {
            let models: Vec<_> = read_group_yml(db_name, &group_name)?
                .into_iter()
                .map(|(n, _)| n)
                .collect();
            result.insert(group_name, models);
        }
        Ok(result)
    }
    .await;
    json_response(result)
}

async fn get_models(AxumPath(path): AxumPath<(String, String)>) -> impl IntoResponse {
    let _semaphore = SEMAPHORE.acquire().await;
    let result = async move {
        let models: Vec<_> = read_group_yml(&path.0, &path.1)?
            .into_iter()
            .map(|(k, v)| {
                let mut model: ModelJson = v.into();
                model.name = k;
                model
            })
            .collect();
        Ok(models)
    }
    .await;
    json_response(result)
}

async fn get_merged_models(AxumPath(path): AxumPath<(String, String)>) -> impl IntoResponse {
    let _semaphore = SEMAPHORE.acquire().await;
    let result = async move {
        check_ascii_name(&path.0)?;
        check_ascii_name(&path.1)?;
        crate::schema::parse(&path.0, true, false)?;
        let models = schema::GROUPS
            .read()
            .unwrap()
            .as_ref()
            .unwrap()
            .get(&path.1)
            .cloned()
            .unwrap_or_default();
        let models: Vec<_> = models
            .into_iter()
            .map(|(k, v)| {
                let mut model: ModelJson = v.as_ref().clone().into();
                model.name = k;
                model
            })
            .collect();
        Ok(models)
    }
    .await;
    json_response(result)
}

async fn create_model(
    AxumPath(path): AxumPath<(String, String)>,
    Json(data): Json<ModelJson>,
) -> impl IntoResponse {
    let _semaphore = SEMAPHORE.acquire().await;
    let result = async move {
        let mut models = read_group_yml(&path.0, &path.1)?;
        anyhow::ensure!(!models.contains_key(&data.name), "Duplicate names.");
        let name = data.name.clone();
        let model: ModelDef = data.try_into()?;
        models.insert(name, model);
        write_group_yml(&path.0, &path.1, &models)?;
        Ok(true)
    }
    .await;
    json_response(result)
}

async fn save_model(
    AxumPath(path): AxumPath<(String, String, String)>,
    Json(data): Json<ModelJson>,
) -> impl IntoResponse {
    let _semaphore = SEMAPHORE.acquire().await;
    let result = async move {
        let mut models = read_group_yml(&path.0, &path.1)?;
        let name = data.name.clone();
        let model: ModelDef = data.try_into()?;
        if !path.2.eq(&name) {
            anyhow::ensure!(!models.contains_key(&name), "Duplicate names.");
            models.insert(name, model);
            models.swap_remove(&path.2);
        } else {
            models.insert(name, model);
        }
        write_group_yml(&path.0, &path.1, &models)?;
        Ok(true)
    }
    .await;
    json_response(result)
}

async fn delete_model(AxumPath(path): AxumPath<(String, String, String)>) -> impl IntoResponse {
    let _semaphore = SEMAPHORE.acquire().await;
    let result = async move {
        let mut models = read_group_yml(&path.0, &path.1)?;
        models.remove(&path.2);
        write_group_yml(&path.0, &path.1, &models)?;
        Ok(true)
    }
    .await;
    json_response(result)
}

async fn save_models(
    AxumPath(path): AxumPath<(String, String)>,
    Json(data): Json<Vec<ModelJson>>,
) -> impl IntoResponse {
    let _semaphore = SEMAPHORE.acquire().await;
    let result = async move {
        let mut models: IndexMap<String, ModelDef> = IndexMap::new();
        for v in data {
            let name = v.name.clone();
            let model: ModelDef = v.try_into()?;
            models.insert(name, model);
        }
        write_group_yml(&path.0, &path.1, &models)?;
        Ok(true)
    }
    .await;
    json_response(result)
}

async fn get_simple_vo() -> impl IntoResponse {
    let _semaphore = SEMAPHORE.acquire().await;
    let result = async move {
        let vo_list: Vec<_> = read_simple_vo_yml()?
            .into_iter()
            .map(|(k, v)| {
                let mut vo: ValueObjectJson = v.into();
                vo.name = k;
                vo
            })
            .collect();
        Ok(vo_list)
    }
    .await;
    json_response(result)
}

async fn save_simple_vo_list(Json(data): Json<Vec<ValueObjectJson>>) -> impl IntoResponse {
    let _semaphore = SEMAPHORE.acquire().await;
    let result = async move {
        let mut vo_map: IndexMap<String, FieldDef> = IndexMap::new();
        for v in data {
            let name = v.name.clone();
            let vo: FieldDef = v.into();
            vo_map.insert(name, vo);
        }
        write_simple_vo_yml(&vo_map)?;
        Ok(true)
    }
    .await;
    json_response(result)
}

async fn create_simple_vo(Json(data): Json<ValueObjectJson>) -> impl IntoResponse {
    let _semaphore = SEMAPHORE.acquire().await;
    let result = async move {
        let mut vo_list = read_simple_vo_yml()?;
        anyhow::ensure!(!vo_list.contains_key(&data.name), "Duplicate names.");
        let name = data.name.clone();
        let vo: FieldDef = data.into();
        vo_list.insert(name, vo);
        write_simple_vo_yml(&vo_list)?;
        Ok(true)
    }
    .await;
    json_response(result)
}

async fn save_simple_vo(
    AxumPath(path): AxumPath<String>,
    Json(data): Json<ValueObjectJson>,
) -> impl IntoResponse {
    let _semaphore = SEMAPHORE.acquire().await;
    let result = async move {
        let mut vo_list = read_simple_vo_yml()?;
        let name = data.name.clone();
        let vo: FieldDef = data.into();
        if !name.eq(&path) {
            anyhow::ensure!(!vo_list.contains_key(&name), "Duplicate names.");
            vo_list.insert(name, vo);
            vo_list.swap_remove(&path);
        } else {
            vo_list.insert(name, vo);
        }
        write_simple_vo_yml(&vo_list)?;
        Ok(true)
    }
    .await;
    json_response(result)
}

async fn delete_simple_vo(AxumPath(path): AxumPath<String>) -> impl IntoResponse {
    let _semaphore = SEMAPHORE.acquire().await;
    let result = async move {
        let mut vo_list = read_simple_vo_yml()?;
        vo_list.remove(&path);
        write_simple_vo_yml(&vo_list)?;
        Ok(true)
    }
    .await;
    json_response(result)
}

async fn get_api_server() -> impl IntoResponse {
    let _semaphore = SEMAPHORE.acquire().await;
    let result = async move {
        let mut list = Vec::new();
        for entry in fs::read_dir(Path::new("."))? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() && path.join(API_SCHEMA_PATH).exists() {
                list.push(path.file_name().unwrap().to_string_lossy().to_string());
            }
        }
        Ok(list)
    }
    .await;
    json_response(result)
}

async fn get_api_server_db(AxumPath(path): AxumPath<String>) -> impl IntoResponse {
    let _semaphore = SEMAPHORE.acquire().await;
    let result = async move {
        let server = check_ascii_name(&path)?;
        let mut list = Vec::new();
        for entry in fs::read_dir(Path::new(server).join(API_SCHEMA_PATH))? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                list.push(path.file_name().unwrap().to_string_lossy().to_string());
            }
        }
        Ok(list)
    }
    .await;
    json_response(result)
}

async fn get_api_server_config(AxumPath(path): AxumPath<String>) -> impl IntoResponse {
    let _semaphore = SEMAPHORE.acquire().await;
    let result = async move {
        let server = check_ascii_name(&path)?;
        let path = Path::new(server).join(API_SCHEMA_PATH).join("_config.yml");
        let config: ApiConfigDef = parse_yml_file(&path)?;
        let config: ApiConfigJson = config.into();
        Ok(config)
    }
    .await;
    json_response(result)
}

async fn save_api_server_config(
    AxumPath(path): AxumPath<String>,
    Json(data): Json<ApiConfigJson>,
) -> impl IntoResponse {
    let _semaphore = SEMAPHORE.acquire().await;
    let result = async move {
        let server = check_ascii_name(&path)?;
        if !READ_ONLY.load(Ordering::SeqCst) {
            let path = Path::new(server)
            .join(API_SCHEMA_PATH)
            .join("_config.yml");
            if let Some(bk) = BACKUP.get() {
                if path.exists() {
                    let content = fs::read_to_string(&path)?;
                    let dir = bk.join(format!("api_server-{server}-_config-{}.yml", Local::now()));
                    fs::write(dir, content)?;
                }
            }
            let config: ApiConfigDef = data.into();
            let mut buf =
                "# yaml-language-server: $schema=../../senax-schema.json#definitions/ApiConfigDef\n\n"
                    .to_string();
            buf.push_str(&simplify_yml(serde_yaml::to_string(&config)?)?);
            fs::write(path, &buf)?;
        }
        Ok(true)
    }
    .await;
    json_response(result)
}

async fn get_api_server_groups(AxumPath(path): AxumPath<(String, String)>) -> impl IntoResponse {
    let _semaphore = SEMAPHORE.acquire().await;
    let result = async move {
        let server = check_ascii_name(&path.0)?;
        let db_path = check_ascii_name(&path.1)?;
        let path = Path::new(server)
            .join(API_SCHEMA_PATH)
            .join(format!("{}.yml", db_path));
        let config: ApiDbDef = parse_yml_file(&path)?;
        let db = config.db.as_deref().unwrap_or(db_path);
        check_ascii_name(db)?;
        let path = Path::new(SCHEMA_PATH).join(format!("{db}.yml"));
        let config: ConfigDef = parse_yml_file(&path)?;
        let config: ConfigJson = config.into();
        Ok(config)
    }
    .await;
    json_response(result)
}

async fn get_api_server_db_config(AxumPath(path): AxumPath<(String, String)>) -> impl IntoResponse {
    let _semaphore = SEMAPHORE.acquire().await;
    let result = async move {
        let server = check_ascii_name(&path.0)?;
        let db_path = check_ascii_name(&path.1)?;
        set_api_config(server)?;
        let path = Path::new(server)
            .join(API_SCHEMA_PATH)
            .join(format!("{}.yml", db_path));
        let mut config: ApiDbDef = parse_yml_file(&path)?;
        config.fix();
        let config: ApiDbJson = config.into();
        Ok(config)
    }
    .await;
    json_response(result)
}

async fn save_api_server_db_config(
    AxumPath(path): AxumPath<(String, String)>,
    Json(data): Json<ApiDbJson>,
) -> impl IntoResponse {
    let _semaphore = SEMAPHORE.acquire().await;
    let result = async move {
        let server = check_ascii_name(&path.0)?;
        let db_path = check_ascii_name(&path.1)?;
        if !READ_ONLY.load(Ordering::SeqCst) {
            let path = Path::new(server)
                .join(API_SCHEMA_PATH)
                .join(format!("{}.yml", db_path));
            if path.exists() {
                let content = fs::read_to_string(&path)?;
                if let Some(bk) = BACKUP.get() {
                    let dir = bk.join(format!(
                        "api_server-{server}-{db_path}-{}.yml",
                        Local::now()
                    ));
                    fs::write(dir, &content)?;
                }

                let old_config: ApiDbDef = parse_yml(&content)?;
                let set: HashSet<_> = data.groups.iter().filter_map(|v| v._name.clone()).collect();
                let dir = Path::new(server).join(API_SCHEMA_PATH).join(db_path);
                for (group_name, _) in &old_config.groups {
                    if !set.contains(group_name) {
                        check_ascii_name(group_name)?;
                        let group_path = dir.join(format!("{group_name}.yml"));
                        fs::remove_file(group_path)?;
                    }
                }
                for group in &data.groups {
                    if let Some(old_name) = &group._name {
                        if old_name != &group.name {
                            let old_path = dir.join(format!("{old_name}.yml"));
                            let new_path = dir.join(format!("{}.yml", group.name));
                            fs::rename(old_path, new_path)?;
                        }
                    }
                }
            }
            let config: ApiDbDef = data.into();
            let mut buf =
                "# yaml-language-server: $schema=../../senax-schema.json#definitions/ApiDbDef\n\n"
                    .to_string();
            buf.push_str(&simplify_yml(serde_yaml::to_string(&config)?)?);
            fs::write(path, &buf)?;
        }
        Ok(true)
    }
    .await;
    json_response(result)
}

async fn get_api_server_models(
    AxumPath(path): AxumPath<(String, String, String)>,
) -> impl IntoResponse {
    let _semaphore = SEMAPHORE.acquire().await;
    let result = async move {
        let server = check_ascii_name(&path.0)?;
        let db_path = check_ascii_name(&path.1)?;
        let group_path = check_ascii_name(&path.2)?;
        let path = Path::new(server)
            .join(API_SCHEMA_PATH)
            .join(format!("{}.yml", db_path));
        let config: ApiDbDef = parse_yml_file(&path)?;
        let db = config.db.as_deref().unwrap_or(db_path);
        let group = if let Some(Some(group)) = config.groups.get(group_path) {
            group.group.as_deref().unwrap_or(group_path)
        } else {
            group_path
        };
        check_ascii_name(db)?;
        check_ascii_name(group)?;
        crate::schema::parse(db, true, false)?;
        let models = schema::GROUPS
            .read()
            .unwrap()
            .as_ref()
            .unwrap()
            .get(group)
            .cloned()
            .unwrap_or_default();
        let models: Vec<_> = models
            .into_iter()
            .map(|(k, v)| {
                let mut model: ModelJson = v.as_ref().clone().into();
                model.name = k;
                model
            })
            .collect();
        Ok(models)
    }
    .await;
    json_response(result)
}

async fn clean_api_server_models(
    AxumPath(path): AxumPath<(String, String, String)>,
) -> impl IntoResponse {
    let _semaphore = SEMAPHORE.acquire().await;
    let result = async move {
        let server = check_ascii_name(&path.0)?;
        let db_path = check_ascii_name(&path.1)?;
        let group_path = check_ascii_name(&path.2)?;
        let file_path = Path::new(server)
            .join(API_SCHEMA_PATH)
            .join(format!("{}.yml", db_path));
        let config: ApiDbDef = parse_yml_file(&file_path)?;
        let db = config.db.as_deref().unwrap_or(db_path);
        let group = if let Some(Some(group)) = config.groups.get(group_path) {
            group.group.as_deref().unwrap_or(group_path)
        } else {
            group_path
        };
        let models: HashMap<_, _> = read_group_yml(db, group)?
            .into_iter()
            .enumerate()
            .map(|(nth, (name, _))| (name, nth))
            .collect();
        let map: IndexMap<String, Option<ApiModelDef>> = read_api_yml(&path.0, &path.1, &path.2)?;
        let mut list: Vec<(String, Option<ApiModelDef>)> = map
            .into_iter()
            .filter(|(k, v)| {
                let name = if let Some(v) = v {
                    v.model.as_deref().unwrap_or(k)
                } else {
                    k
                };
                models.contains_key(name)
            })
            .collect();
        list.sort_by(|v1, v2| {
            let k1 = if let Some(v) = &v1.1 {
                v.model.as_deref().unwrap_or(&v1.0)
            } else {
                &v1.0
            };
            let k2 = if let Some(v) = &v2.1 {
                v.model.as_deref().unwrap_or(&v2.0)
            } else {
                &v2.0
            };
            models
                .get(k1)
                .unwrap_or(&0)
                .cmp(models.get(k2).unwrap_or(&0))
                .then(v1.0.cmp(&v2.0))
        });
        let map = list.into_iter().collect();
        write_api_yml(&path.0, &path.1, &path.2, &map)?;

        let list: Vec<_> = map
            .into_iter()
            .map(|(k, v)| {
                let mut api: ApiModelJson = v.unwrap_or_default().into();
                api.name = k;
                api
            })
            .collect();
        Ok(list)
    }
    .await;
    json_response(result)
}

async fn get_api_server_model_paths(
    AxumPath(path): AxumPath<(String, String, String)>,
) -> impl IntoResponse {
    let _semaphore = SEMAPHORE.acquire().await;
    let result = async move {
        let list: Vec<_> = read_api_yml(&path.0, &path.1, &path.2)?
            .into_iter()
            .map(|(k, v)| {
                let mut api: ApiModelJson = v.unwrap_or_default().into();
                api.name = k;
                api
            })
            .collect();
        Ok(list)
    }
    .await;
    json_response(result)
}

async fn save_api_server_models(
    AxumPath(path): AxumPath<(String, String, String)>,
    Json(data): Json<Vec<ApiModelJson>>,
) -> impl IntoResponse {
    let _semaphore = SEMAPHORE.acquire().await;
    let result = async move {
        let mut map: IndexMap<String, Option<ApiModelDef>> = IndexMap::new();
        for v in data {
            let name = v.name.clone();
            let api: ApiModelDef = v.try_into()?;
            if api == ApiModelDef::default() {
                map.insert(name, None);
            } else {
                map.insert(name, Some(api));
            }
        }
        write_api_yml(&path.0, &path.1, &path.2, &map)?;
        Ok(true)
    }
    .await;
    json_response(result)
}

async fn create_api_server_model(
    AxumPath(path): AxumPath<(String, String, String)>,
    Json(data): Json<ApiModelJson>,
) -> impl IntoResponse {
    let _semaphore = SEMAPHORE.acquire().await;
    let result = async move {
        data.validate()?;
        let mut map = read_api_yml(&path.0, &path.1, &path.2)?;
        anyhow::ensure!(!map.contains_key(&data.name), "Duplicate names.");
        let name = data.name.clone();
        let api: ApiModelDef = data.try_into()?;
        if api == ApiModelDef::default() {
            map.insert(name, None);
        } else {
            map.insert(name, Some(api));
        }
        write_api_yml(&path.0, &path.1, &path.2, &map)?;
        Ok(true)
    }
    .await;
    json_response(result)
}

async fn update_api_server_model(
    AxumPath(path): AxumPath<(String, String, String, String)>,
    Json(data): Json<ApiModelJson>,
) -> impl IntoResponse {
    let _semaphore = SEMAPHORE.acquire().await;
    let result = async move {
        data.validate()?;
        let mut map = read_api_yml(&path.0, &path.1, &path.2)?;
        let name = data.name.clone();
        let api: ApiModelDef = data.try_into()?;
        anyhow::ensure!(name.eq(&path.3), "Illegal name.");
        if api == ApiModelDef::default() {
            map.insert(name, None);
        } else {
            map.insert(name, Some(api));
        }
        write_api_yml(&path.0, &path.1, &path.2, &map)?;
        Ok(true)
    }
    .await;
    json_response(result)
}

async fn delete_api_server_model(
    AxumPath(path): AxumPath<(String, String, String, String)>,
) -> impl IntoResponse {
    let _semaphore = SEMAPHORE.acquire().await;
    let result = async move {
        let mut map = read_api_yml(&path.0, &path.1, &path.2)?;
        map.remove(&path.3);
        write_api_yml(&path.0, &path.1, &path.2, &map)?;
        Ok(true)
    }
    .await;
    json_response(result)
}

async fn build_exec() -> impl IntoResponse {
    let result = async move {
        use tokio::process::Command;
        let shell_command = "sh -e build.sh > build_result.txt 2>&1";
        let mut child = Command::new("sh").arg("-c").arg(shell_command).spawn()?;
        tokio::spawn(async move {
            let _ = child.wait().await;
        });
        Ok(true)
    }
    .await;
    json_response(result)
}

async fn build_result() -> impl IntoResponse {
    let result = async move {
        let path = Path::new("build_result.txt");
        let mut result = HashMap::new();
        if path.exists() {
            result.insert("result", fs::read_to_string(path)?);
        }
        Ok(result)
    }
    .await;
    json_response(result)
}

#[derive(serde::Deserialize)]
struct GitInfo {
    msg: Option<String>,
}

async fn git_exec(AxumPath(cmd): AxumPath<String>, Json(data): Json<GitInfo>) -> impl IntoResponse {
    let result = async move {
        use tokio::process::Command;
        let shell_command = format!(
            "sh -e git_proc.sh {} {}> git_result.txt 2>&1",
            shell_escape::escape(cmd.as_str().into()),
            shell_escape::escape(data.msg.unwrap_or_default().into())
        );
        let mut child = Command::new("sh").arg("-c").arg(shell_command).spawn()?;
        tokio::spawn(async move {
            let _ = child.wait().await;
        });
        Ok(true)
    }
    .await;
    json_response(result)
}

async fn git_result() -> impl IntoResponse {
    let result = async move {
        let path = Path::new("git_result.txt");
        let mut result = HashMap::new();
        if path.exists() {
            result.insert("result", fs::read_to_string(path)?);
        }
        Ok(result)
    }
    .await;
    json_response(result)
}

#[derive(Debug, Display, Serialize)]
pub struct BadRequest {
    msg: String,
}
impl std::error::Error for BadRequest {}
impl BadRequest {
    #[allow(dead_code)]
    pub fn new(msg: String) -> BadRequest {
        BadRequest { msg }
    }
}

#[derive(Debug)]
pub struct NotFound {
    pub path: String,
}
impl NotFound {
    #[allow(dead_code)]
    pub fn new(parts: http::request::Parts) -> NotFound {
        NotFound {
            path: parts.uri.path().to_string(),
        }
    }
}
impl std::fmt::Display for NotFound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Not Found: path={}", self.path)
    }
}
impl std::error::Error for NotFound {}

pub fn json_response<T: Serialize>(
    r: Result<T, anyhow::Error>,
) -> Result<Response, (StatusCode, Response)> {
    match r {
        Ok(data) => Ok(Json(data).into_response()),
        Err(err) => Err(error_response(err)),
    }
}

fn error_response(err: anyhow::Error) -> (StatusCode, Response) {
    println!("{}", err);
    if let Some(e) = err.downcast_ref::<validator::ValidationErrors>() {
        (StatusCode::BAD_REQUEST, Json(e).into_response())
    } else if let Some(e) = err.downcast_ref::<BadRequest>() {
        (StatusCode::BAD_REQUEST, Json(e).into_response())
    } else if err.downcast_ref::<NotFound>().is_some() {
        (StatusCode::NOT_FOUND, "not found".into_response())
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            err.to_string().into_response(),
        )
    }
}

fn check_ascii_name(name: &str) -> Result<&str> {
    use fancy_regex::Regex;
    static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[A-Za-z][_0-9A-Za-z]*(?<!_)$").unwrap());
    if !RE.is_match(name).unwrap() || schema::BAD_KEYWORDS.iter().any(|&x| x == name) {
        anyhow::bail!("{} is an incorrect name.", name)
    }
    Ok(name)
}