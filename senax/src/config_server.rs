use actix_web::http::header::{self, ContentEncoding};
use actix_web::{delete, get, post, put, web, HttpResponse, Responder, Result};
use actix_web::{error, HttpRequest};
use chrono::Local;
use derive_more::Display;
use includedir::Compression;
use indexmap::IndexMap;
use mime_guess::Mime;
use once_cell::sync::OnceCell;
use serde::Serialize;
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use validator::Validate;

use crate::api_generator::schema::{
    ApiConfigDef, ApiConfigJson, ApiDbDef, ApiDbJson, ApiModelDef, ApiModelJson, API_CONFIG,
};
use crate::common::{parse_yml, parse_yml_file, simplify_yml};
use crate::schema::{self, ConfigDef, ConfigJson, FieldDef, ModelDef, ModelJson, ValueObjectJson};
use crate::{API_SCHEMA_PATH, SCHEMA_PATH, SIMPLE_VALUE_OBJECTS_FILE};

pub static BACKUP: OnceCell<PathBuf> = OnceCell::new();
pub static READ_ONLY: AtomicBool = AtomicBool::new(false);

#[cfg(feature = "config")]
pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(files);
}

#[allow(dead_code)]
pub fn api(cfg: &mut web::ServiceConfig) {
    cfg.service(get_db)
        .service(get_db_config)
        .service(save_db_config)
        .service(get_config_schema)
        .service(get_model_schema)
        .service(get_vo_schema)
        .service(get_api_config_schema)
        .service(get_api_db_schema)
        .service(get_api_schema)
        .service(get_model_names)
        .service(get_models)
        .service(create_model)
        .service(save_model)
        .service(delete_model)
        .service(save_models)
        .service(get_simple_vo)
        .service(save_simple_vo_list)
        .service(create_simple_vo)
        .service(save_simple_vo)
        .service(delete_simple_vo)
        .service(get_api_server)
        .service(get_api_server_db)
        .service(get_api_server_config)
        .service(save_api_server_config)
        .service(get_api_server_db_config)
        .service(save_api_server_db_config)
        .service(clean_api_server_models)
        .service(get_api_server_models)
        .service(save_api_server_models)
        .service(create_api_server_model)
        .service(update_api_server_model)
        .service(delete_api_server_model)
        .service(build_exec)
        .service(build_result)
        .service(git_exec)
        .service(git_result);
}

#[cfg(feature = "config")]
#[get("/{filename:.*}")]
async fn files(req: HttpRequest) -> impl Responder {
    let mut path = req.match_info().query("filename").to_string();
    if path.is_empty() || path.ends_with('/') {
        path.push_str("index.html");
    }
    if let Ok(tpl) = crate::CONFIG_APP.get_raw(&format!("config-app/dist/{}", path)) {
        file_response(tpl, mime_guess::from_path(path).first())
    } else if let Ok(tpl) = crate::CONFIG_APP.get_raw("config-app/dist/index.html") {
        file_response(tpl, mime_guess::from_path("index.html").first())
    } else {
        HttpResponse::NotFound().body("not found")
    }
}
#[allow(dead_code)]
fn file_response(tpl: (Compression, Cow<'static, [u8]>), mime: Option<Mime>) -> HttpResponse {
    if let Some(mime) = mime {
        match tpl.0 {
            Compression::None => HttpResponse::Ok()
                .insert_header(header::ContentType(mime))
                .body(tpl.1.into_owned()),
            Compression::Gzip => HttpResponse::Ok()
                .insert_header(header::ContentType(mime))
                .insert_header(ContentEncoding::Gzip)
                .body(tpl.1.into_owned()),
        }
    } else {
        match tpl.0 {
            Compression::None => HttpResponse::Ok().body(tpl.1.into_owned()),
            Compression::Gzip => HttpResponse::Ok()
                .insert_header(ContentEncoding::Gzip)
                .body(tpl.1.into_owned()),
        }
    }
}

#[get("/db")]
async fn get_db() -> impl Responder {
    let result = crate::db_generator::list();
    json_response(result)
}

#[get("/db/{db}")]
async fn get_db_config(db: web::Path<String>) -> impl Responder {
    let result = async move {
        let db_name = &*db;
        crate::common::check_ascii_name(db_name);
        let path = Path::new(SCHEMA_PATH).join(format!("{db_name}.yml"));
        let config: ConfigDef = parse_yml_file(&path)?;
        let config: ConfigJson = config.into();
        Ok(config)
    }
    .await;
    json_response(result)
}

#[post("/db/{db}")]
async fn save_db_config(db: web::Path<String>, data: web::Json<ConfigJson>) -> impl Responder {
    let result = async move {
        let db = &*db;
        crate::common::check_ascii_name(db);
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
                    crate::common::check_ascii_name(group_name);
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

            let config: ConfigDef = data.0.into();
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

#[get("/config_schema")]
async fn get_config_schema() -> impl Responder {
    let result = async move {
        let schema = schema::json_schema::json_config_schema()?;
        Ok(schema)
    }
    .await;
    json_response(result)
}

#[get("/model_schema")]
async fn get_model_schema() -> impl Responder {
    let result = async move {
        let schema = schema::json_schema::json_model_schema()?;
        Ok(schema)
    }
    .await;
    json_response(result)
}

#[get("/vo_schema/simple")]
async fn get_vo_schema() -> impl Responder {
    let result = async move {
        let schema = schema::json_schema::json_simple_vo_schema()?;
        Ok(schema)
    }
    .await;
    json_response(result)
}

#[get("/api_config_schema")]
async fn get_api_config_schema() -> impl Responder {
    let result = async move {
        let schema = schema::json_schema::json_api_config_schema()?;
        Ok(schema)
    }
    .await;
    json_response(result)
}

#[get("/api_db_schema")]
async fn get_api_db_schema() -> impl Responder {
    let result = async move {
        let schema = schema::json_schema::json_api_db_schema()?;
        Ok(schema)
    }
    .await;
    json_response(result)
}

#[get("/api_schema")]
async fn get_api_schema() -> impl Responder {
    let result = async move {
        let schema = schema::json_schema::json_api_schema()?;
        Ok(schema)
    }
    .await;
    json_response(result)
}

#[get("/model_names/{db}")]
async fn get_model_names(db: web::Path<String>) -> impl Responder {
    let result = async move {
        let db_name = &*db;
        crate::common::check_ascii_name(db_name);
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

#[get("/models/{db}/{group}")]
async fn get_models(path: web::Path<(String, String)>) -> impl Responder {
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

fn read_group_yml(db: &str, group_name: &str) -> anyhow::Result<IndexMap<String, ModelDef>> {
    crate::common::check_ascii_name(db);
    crate::common::check_ascii_name(group_name);
    let path = Path::new(SCHEMA_PATH)
        .join(db)
        .join(format!("{group_name}.yml"));
    if path.exists() {
        parse_yml_file(&path)
    } else {
        Ok(IndexMap::default())
    }
}

fn write_group_yml(
    db: &str,
    group_name: &str,
    data: &IndexMap<String, ModelDef>,
) -> anyhow::Result<()> {
    if READ_ONLY.load(Ordering::SeqCst) {
        return Ok(());
    }
    crate::common::check_ascii_name(db);
    crate::common::check_ascii_name(group_name);
    let path = Path::new(SCHEMA_PATH)
        .join(db)
        .join(format!("{group_name}.yml"));
    if let Some(bk) = BACKUP.get() {
        if path.exists() {
            let content = fs::read_to_string(&path)?;
            let dir = bk.join(format!("group-{db}-{group_name}-{}.yml", Local::now()));
            fs::write(dir, content)?;
        }
    }
    let mut buf =
        "# yaml-language-server: $schema=../../senax-schema.json#properties/model\n\n".to_string();
    buf.push_str(&simplify_yml(serde_yaml::to_string(&data)?)?);
    fs::write(path, &buf)?;
    Ok(())
}

#[post("/models/{db}/{group}")]
async fn create_model(
    path: web::Path<(String, String)>,
    data: web::Json<ModelJson>,
) -> impl Responder {
    let result = async move {
        let mut models = read_group_yml(&path.0, &path.1)?;
        anyhow::ensure!(!models.contains_key(&data.name), "Duplicate names.");
        let name = data.name.clone();
        let model: ModelDef = data.0.try_into()?;
        models.insert(name, model);
        write_group_yml(&path.0, &path.1, &models)?;
        Ok(true)
    }
    .await;
    json_response(result)
}

#[put("/models/{db}/{group}/{model}")]
async fn save_model(
    path: web::Path<(String, String, String)>,
    data: web::Json<ModelJson>,
) -> impl Responder {
    let result = async move {
        let mut models = read_group_yml(&path.0, &path.1)?;
        let name = data.name.clone();
        let model: ModelDef = data.0.try_into()?;
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

#[delete("/models/{db}/{group}/{model}")]
async fn delete_model(path: web::Path<(String, String, String)>) -> impl Responder {
    let result = async move {
        let mut models = read_group_yml(&path.0, &path.1)?;
        models.remove(&path.2);
        write_group_yml(&path.0, &path.1, &models)?;
        Ok(true)
    }
    .await;
    json_response(result)
}

#[put("/models/{db}/{group}")]
async fn save_models(
    path: web::Path<(String, String)>,
    data: web::Json<Vec<ModelJson>>,
) -> impl Responder {
    let result = async move {
        let mut models: IndexMap<String, ModelDef> = IndexMap::new();
        for v in data.0 {
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

fn read_simple_vo_yml() -> anyhow::Result<IndexMap<String, FieldDef>> {
    let path = Path::new(SCHEMA_PATH).join(SIMPLE_VALUE_OBJECTS_FILE);
    if path.exists() {
        parse_yml_file(&path)
    } else {
        Ok(IndexMap::default())
    }
}

fn write_simple_vo_yml(data: &IndexMap<String, FieldDef>) -> anyhow::Result<()> {
    if READ_ONLY.load(Ordering::SeqCst) {
        return Ok(());
    }
    let path = Path::new(SCHEMA_PATH).join(SIMPLE_VALUE_OBJECTS_FILE);
    if let Some(bk) = BACKUP.get() {
        if path.exists() {
            let content = fs::read_to_string(&path)?;
            let dir = bk.join(format!("vo-{}.yml", Local::now()));
            fs::write(dir, content)?;
        }
    }
    let mut buf =
        "# yaml-language-server: $schema=../senax-schema.json#properties/simple_value_object\n\n"
            .to_string();
    buf.push_str(&simplify_yml(serde_yaml::to_string(&data)?)?);
    fs::write(path, &buf)?;
    Ok(())
}

#[get("/vo/simple")]
async fn get_simple_vo() -> impl Responder {
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

#[put("/vo/simple")]
async fn save_simple_vo_list(data: web::Json<Vec<ValueObjectJson>>) -> impl Responder {
    let result = async move {
        let mut vo_map: IndexMap<String, FieldDef> = IndexMap::new();
        for v in data.0 {
            let name = v.name.clone();
            let vo: FieldDef = v.try_into()?;
            vo_map.insert(name, vo);
        }
        write_simple_vo_yml(&vo_map)?;
        Ok(true)
    }
    .await;
    json_response(result)
}

#[post("/vo/simple")]
async fn create_simple_vo(data: web::Json<ValueObjectJson>) -> impl Responder {
    let result = async move {
        let mut vo_list = read_simple_vo_yml()?;
        anyhow::ensure!(!vo_list.contains_key(&data.name), "Duplicate names.");
        let name = data.name.clone();
        let vo: FieldDef = data.0.try_into()?;
        vo_list.insert(name, vo);
        write_simple_vo_yml(&vo_list)?;
        Ok(true)
    }
    .await;
    json_response(result)
}

#[put("/vo/simple/{vo}")]
async fn save_simple_vo(
    path: web::Path<String>,
    data: web::Json<ValueObjectJson>,
) -> impl Responder {
    let result = async move {
        let mut vo_list = read_simple_vo_yml()?;
        let name = data.name.clone();
        let vo: FieldDef = data.0.try_into()?;
        if !name.eq(&*path) {
            anyhow::ensure!(!vo_list.contains_key(&name), "Duplicate names.");
            vo_list.insert(name, vo);
            vo_list.swap_remove(&*path);
        } else {
            vo_list.insert(name, vo);
        }
        write_simple_vo_yml(&vo_list)?;
        Ok(true)
    }
    .await;
    json_response(result)
}

#[delete("/vo/simple/{vo}")]
async fn delete_simple_vo(path: web::Path<String>) -> impl Responder {
    let result = async move {
        let mut vo_list = read_simple_vo_yml()?;
        vo_list.remove(&*path);
        write_simple_vo_yml(&vo_list)?;
        Ok(true)
    }
    .await;
    json_response(result)
}

#[get("/api_server")]
async fn get_api_server() -> impl Responder {
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
#[get("/api_server/{server}/_db")]
async fn get_api_server_db(path: web::Path<String>) -> impl Responder {
    let result = async move {
        let server = sanitize_filename::sanitize(&*path);
        let mut list = Vec::new();
        for entry in fs::read_dir(Path::new(&server).join(API_SCHEMA_PATH))? {
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

#[get("/api_server/{server}/_config")]
async fn get_api_server_config(path: web::Path<String>) -> impl Responder {
    let result = async move {
        let server = sanitize_filename::sanitize(&*path);
        let path = Path::new(&server).join(API_SCHEMA_PATH).join("_config.yml");
        let config: ApiConfigDef = parse_yml_file(&path)?;
        let config: ApiConfigJson = config.into();
        Ok(config)
    }
    .await;
    json_response(result)
}

#[post("/api_server/{server}/_config")]
async fn save_api_server_config(
    path: web::Path<String>,
    data: web::Json<ApiConfigJson>,
) -> impl Responder {
    let result = async move {
        let server = sanitize_filename::sanitize(&*path);
        if !READ_ONLY.load(Ordering::SeqCst) {
            let path = Path::new(&server)
            .join(API_SCHEMA_PATH)
            .join("_config.yml");
            if let Some(bk) = BACKUP.get() {
                if path.exists() {
                    let content = fs::read_to_string(&path)?;
                    let dir = bk.join(format!("api_server-{server}-_config-{}.yml", Local::now()));
                    fs::write(dir, content)?;
                }
            }
            let config: ApiConfigDef = data.0.into();
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

#[get("/api_server/{server}/{db}/_config")]
async fn get_api_server_db_config(path: web::Path<(String, String)>) -> impl Responder {
    let result = async move {
        let server = sanitize_filename::sanitize(&path.0);
        let db = sanitize_filename::sanitize(&path.1);
        set_api_config(&server)?;
        let path = Path::new(&server)
            .join(API_SCHEMA_PATH)
            .join(format!("{}.yml", &db));
        let mut config: ApiDbDef = parse_yml_file(&path)?;
        config.fix();
        let config: ApiDbJson = config.into();
        Ok(config)
    }
    .await;
    json_response(result)
}

#[post("/api_server/{server}/{db}/_config")]
async fn save_api_server_db_config(
    path: web::Path<(String, String)>,
    data: web::Json<ApiDbJson>,
) -> impl Responder {
    let result = async move {
        let server = sanitize_filename::sanitize(&path.0);
        let db = sanitize_filename::sanitize(&path.1);
        if !READ_ONLY.load(Ordering::SeqCst) {
            let path = Path::new(&server)
                .join(API_SCHEMA_PATH)
                .join(format!("{}.yml", db));
            if path.exists() {
                let content = fs::read_to_string(&path)?;
                if let Some(bk) = BACKUP.get() {
                    let dir = bk.join(format!("api_server-{server}-{db}-{}.yml", Local::now()));
                    fs::write(dir, &content)?;
                }

                let old_config: ApiDbDef = parse_yml(&content)?;
                let set: HashSet<_> = data.groups.iter().filter_map(|v| v._name.clone()).collect();
                let dir = Path::new(&server).join(API_SCHEMA_PATH).join(&db);
                for (group_name, _) in &old_config.groups {
                    if !set.contains(group_name) {
                        crate::common::check_ascii_name(group_name);
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
            let config: ApiDbDef = data.0.into();
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

#[get("/clean_api_server/{server}/{db}/{group}")]
async fn clean_api_server_models(path: web::Path<(String, String, String)>) -> impl Responder {
    let result = async move {
        let models: HashMap<_, _> = read_group_yml(&path.1, &path.2)?
            .into_iter()
            .enumerate()
            .map(|(nth, (name, _))| (name, nth))
            .collect();
        let map: IndexMap<String, Option<ApiModelDef>> = read_api_yml(&path.0, &path.1, &path.2)?;
        let mut list: Vec<(String, Option<ApiModelDef>)> = map
            .into_iter()
            .filter(|(k, _)| models.contains_key(k))
            .collect();
        list.sort_by_key(|(k, _)| models.get(k).unwrap_or(&0));
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

#[get("/api_server/{server}/{db}/{group}")]
async fn get_api_server_models(path: web::Path<(String, String, String)>) -> impl Responder {
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

#[put("/api_server/{server}/{db}/{group}")]
async fn save_api_server_models(
    path: web::Path<(String, String, String)>,
    data: web::Json<Vec<ApiModelJson>>,
) -> impl Responder {
    let result = async move {
        let mut map: IndexMap<String, Option<ApiModelDef>> = IndexMap::new();
        for v in data.0 {
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

#[post("/api_server/{server}/{db}/{group}")]
async fn create_api_server_model(
    path: web::Path<(String, String, String)>,
    data: web::Json<ApiModelJson>,
) -> impl Responder {
    let result = async move {
        data.validate()?;
        let mut map = read_api_yml(&path.0, &path.1, &path.2)?;
        anyhow::ensure!(!map.contains_key(&data.name), "Duplicate names.");
        let name = data.name.clone();
        let api: ApiModelDef = data.0.try_into()?;
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

#[put("/api_server/{server}/{db}/{group}/{model}")]
async fn update_api_server_model(
    path: web::Path<(String, String, String, String)>,
    data: web::Json<ApiModelJson>,
) -> impl Responder {
    let result = async move {
        data.validate()?;
        let mut map = read_api_yml(&path.0, &path.1, &path.2)?;
        let name = data.name.clone();
        let api: ApiModelDef = data.0.try_into()?;
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

#[delete("/api_server/{server}/{db}/{group}/{model}")]
async fn delete_api_server_model(
    path: web::Path<(String, String, String, String)>,
) -> impl Responder {
    let result = async move {
        let mut map = read_api_yml(&path.0, &path.1, &path.2)?;
        map.remove(&path.3);
        write_api_yml(&path.0, &path.1, &path.2, &map)?;
        Ok(true)
    }
    .await;
    json_response(result)
}

fn read_api_yml(
    server: &str,
    db: &str,
    group: &str,
) -> anyhow::Result<IndexMap<String, Option<ApiModelDef>>> {
    let server = sanitize_filename::sanitize(server);
    let db = sanitize_filename::sanitize(db);
    crate::common::check_ascii_name(group);

    let config_path = Path::new(&server).join(API_SCHEMA_PATH).join("_config.yml");
    let config: ApiConfigDef = parse_yml_file(&config_path)?;
    API_CONFIG.write().unwrap().replace(config);

    let path = Path::new(&server)
        .join(API_SCHEMA_PATH)
        .join(db)
        .join(format!("{}.yml", group));
    if path.exists() {
        let mut map: IndexMap<String, Option<ApiModelDef>> = parse_yml_file(&path)?;
        for (_, def) in map.iter_mut() {
            if let Some(v) = def.as_mut() {
                v.fix()
            }
        }
        Ok(map)
    } else {
        Ok(IndexMap::default())
    }
}

fn write_api_yml(
    server: &str,
    db: &str,
    group: &str,
    data: &IndexMap<String, Option<ApiModelDef>>,
) -> anyhow::Result<()> {
    if READ_ONLY.load(Ordering::SeqCst) {
        return Ok(());
    }
    let server = sanitize_filename::sanitize(server);
    let db = sanitize_filename::sanitize(db);
    crate::common::check_ascii_name(group);

    let path = Path::new(&server)
        .join(API_SCHEMA_PATH)
        .join(&db)
        .join(format!("{}.yml", group));
    if let Some(bk) = BACKUP.get() {
        if path.exists() {
            let content = fs::read_to_string(&path)?;
            let dir = bk.join(format!("api-{server}-{db}-{group}-{}.yml", Local::now()));
            fs::write(dir, content)?;
        }
    }
    let mut buf =
        "# yaml-language-server: $schema=../../../senax-schema.json#properties/api_model\n\n"
            .to_string();
    buf.push_str(&simplify_yml(serde_yaml::to_string(&data)?)?);
    fs::write(path, &buf)?;
    Ok(())
}

#[post("/build/exec")]
async fn build_exec() -> impl Responder {
    let result = async move {
        use tokio::process::Command;
        let shell_command = "bash -e build.sh > build_result.txt 2>&1";
        let mut child = Command::new("bash").arg("-c").arg(shell_command).spawn()?;
        tokio::spawn(async move {
            let _ = child.wait().await;
        });
        Ok(true)
    }
    .await;
    json_response(result)
}

#[get("/build/result")]
async fn build_result() -> impl Responder {
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

#[post("/git/exec/{cmd}")]
async fn git_exec(cmd: web::Path<String>, data: web::Json<GitInfo>) -> impl Responder {
    let result = async move {
        use tokio::process::Command;
        let shell_command = format!(
            "bash -e git_proc.sh {} {}> git_result.txt 2>&1",
            shell_escape::escape(cmd.as_str().into()),
            shell_escape::escape(data.into_inner().msg.unwrap_or_default().into())
        );
        let mut child = Command::new("bash").arg("-c").arg(shell_command).spawn()?;
        tokio::spawn(async move {
            let _ = child.wait().await;
        });
        Ok(true)
    }
    .await;
    json_response(result)
}

#[get("/git/result")]
async fn git_result() -> impl Responder {
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
    pub fn new(http_req: &HttpRequest) -> NotFound {
        NotFound {
            path: http_req.path().to_string(),
        }
    }
}
impl std::fmt::Display for NotFound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Not Found: path={}", self.path)
    }
}
impl std::error::Error for NotFound {}

#[allow(dead_code)]
pub fn json_response<T: Serialize>(r: Result<T, anyhow::Error>) -> impl Responder {
    match r {
        Ok(data) => {
            let mut writer = Vec::with_capacity(65536);
            match serde_json::to_writer(&mut writer, &data) {
                Ok(_) => {
                    let response = unsafe { String::from_utf8_unchecked(writer) };
                    HttpResponse::Ok()
                        .content_type("application/json")
                        .body(response)
                }
                Err(err) => error_response(err.into()),
            }
        }
        Err(err) => error_response(err),
    }
}

#[allow(dead_code)]
pub fn json_error_handler(err: error::JsonPayloadError, _req: &HttpRequest) -> error::Error {
    use actix_web::error::JsonPayloadError;

    let detail = err.to_string();
    let resp = match &err {
        JsonPayloadError::ContentType => HttpResponse::UnsupportedMediaType().body(detail),
        JsonPayloadError::Deserialize(json_err) if json_err.is_data() => {
            HttpResponse::UnprocessableEntity().json(BadRequest::new(detail))
        }
        _ => HttpResponse::BadRequest().body(detail),
    };
    error::InternalError::from_response(err, resp).into()
}

fn error_response(err: anyhow::Error) -> HttpResponse {
    if let Some(e) = err.downcast_ref::<validator::ValidationErrors>() {
        HttpResponse::BadRequest().json(e)
    } else if let Some(e) = err.downcast_ref::<BadRequest>() {
        HttpResponse::BadRequest().json(e)
    } else if err.downcast_ref::<NotFound>().is_some() {
        HttpResponse::NotFound().body("not found")
    } else {
        HttpResponse::InternalServerError().body(err.to_string())
    }
}

pub fn fix_schema(db: &str) -> anyhow::Result<()> {
    let path = Path::new(SCHEMA_PATH).join(format!("{db}.yml"));
    let config: ConfigDef = parse_yml_file(&path)?;
    config.fix_static_vars();
    schema::CONFIG.write().unwrap().replace(config.clone());

    for (group_name, group_def) in config.groups {
        let group_def = group_def.unwrap_or_default();
        let mut models = read_group_yml(db, &group_name)?;
        for (name, model) in models.iter_mut() {
            model.group_name = group_name.clone();
            model.name = name.clone();
            if model.exclude_group_from_table_name.is_none() {
                model.exclude_group_from_table_name = Some(group_def.exclude_group_from_table_name);
            }
            model._name = Some(model.table_name());
            model._soft_delete = model
                .soft_delete()
                .map(|s| format!("{},{}", model.soft_delete_col().unwrap(), s.as_ref()));
            for (name, field) in model.fields.clone().into_iter() {
                let mut field = field.exact();
                field._name = Some(field.get_col_name(&name).to_string());
                model
                    .fields
                    .insert(name, schema::FieldDefOrSubsetType::Exact(field));
            }
        }
        write_group_yml(db, &group_name, &models)?;
    }
    Ok(())
}

fn set_api_config(server: &str) -> anyhow::Result<()> {
    let path = Path::new(server).join(API_SCHEMA_PATH).join("_config.yml");
    if !path.exists() {
        API_CONFIG.write().unwrap().replace(ApiConfigDef::default());
    } else {
        let config: ApiConfigDef = parse_yml_file(&path)?;
        API_CONFIG.write().unwrap().replace(config);
    }
    Ok(())
}
