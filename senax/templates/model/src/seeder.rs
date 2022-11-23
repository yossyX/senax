// This code is auto-generated and will always be overwritten.

use anyhow::{bail, Result};
use regex::Regex;
use schemars::gen::SchemaSettings;
use schemars::schema::{InstanceType, Schema, SchemaObject, SingleOrVec};
use schemars::JsonSchema;
use senax_common::types::blob::FILES;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::PathBuf;
use std::sync::RwLock;

use crate::{connection, exec_ddl, DbConn};

// SEEDS
include!(concat!(env!("OUT_DIR"), "/seeds.rs"));

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct SeedSchema {
@%- for (name, defs) in groups %@@% if !defs.is_empty() %@
    @{ name|to_var_name }@: Option<crate::@{ name|to_var_name }@::@{ name|pascal }@>,
@%- endif %@@% endfor %@
}
impl SeedSchema {
    #[allow(clippy::single_match)]
    async fn seed(data: &str) -> Result<()> {
        let seeds: serde_yaml::Value = serde_yaml::from_str(data)?;
        let mut conns: Vec<_> = DbConn::shard_num_range().map(DbConn::_new).collect();
        for conn in conns.iter_mut() {
            conn.begin().await?;
        }
        if let Some(mapping) = seeds.as_mapping() {
            for (name, value) in mapping {
                match name.as_str() {
                @%- for (name, defs) in groups %@@% if !defs.is_empty() %@
                    Some("@{ name }@") => crate::@{ name|to_var_name }@::seed(value, &mut conns).await?,
                @%- endif %@@% endfor %@
                    _ => {}
                }
            }
        }
        for mut conn in conns {
            conn.commit().await?;
        }
        Ok(())
    }
}

pub fn gen_seed_schema() -> Result<String> {
    let settings = SchemaSettings::draft07().with(|s| {
        s.option_nullable = false;
        s.option_add_null_type = true;
    });
    let gen = settings.into_generator();
    let schema = gen.into_root_schema_for::<SeedSchema>();
    let schema = serde_json::to_string_pretty(&schema)?;
    Ok(schema)
}

pub async fn seed(use_test: bool, file_path: Option<PathBuf>) -> Result<()> {
    let mut yml_files = BTreeMap::new();
    if FILES.get().is_none() {
        let _ = FILES.set(RwLock::new(HashMap::new()));
    }
    let re1 = Regex::new(r"(\d+)(_([^/]+)\.yml)?$")?;
    let file_name = file_path
        .as_ref()
        .and_then(|x| x.file_name().map(|y| y.to_string_lossy()));
    let file_num = file_name
        .as_ref()
        .and_then(|x| re1.captures(x.as_ref()).map(|y| y.get(1).unwrap().as_str()));
    let re2 = Regex::new(r"^seeds/(\d+)_([^/]+)\.yml$")?;
    {
        let mut files = FILES.get().unwrap().write().unwrap();
        for name in SEEDS.file_names() {
            if let Some(caps) = re2.captures(name) {
                let content = SEEDS.get(name)?;
                let data = std::str::from_utf8(content.as_ref())?.to_string();
                let name = name.trim_start_matches("seeds/").to_string();
                if let Some(file_num) = file_num {
                    if file_num == caps.get(1).unwrap().as_str() {
                        yml_files.insert(name, data);
                    }
                } else {
                    yml_files.insert(name, data);
                }
            } else {
                files.insert(
                    name.trim_start_matches("seeds/").to_string(),
                    SEEDS.get(name)?,
                );
            }
        }
    }
    if let Some(ref path) = file_path {
        if path.is_file() {
            yml_files.clear();
            let data = fs::read_to_string(path).unwrap();
            yml_files.insert(
                path.file_name().unwrap().to_str().unwrap().to_string(),
                data,
            );
        }
    }
    if file_num.is_some() && yml_files.is_empty() {
        FILES.get().unwrap().write().unwrap().clear();
        bail!("seed file not found!");
    }
    if use_test {
        connection::init_test().await?;
    } else {
        connection::init().await?;
    }
    for (name, data) in yml_files {
        let caps = re1.captures(&name).unwrap();
        let version: i64 = caps.get(1).unwrap().as_str().parse()?;
        let description = caps.get(3).unwrap().as_str().to_string();
        let mut source = DbConn::_new(0).acquire_source().await?;
        exec_ddl(
            r#"
                CREATE TABLE IF NOT EXISTS _seeds (
                    version BIGINT PRIMARY KEY,
                    description TEXT NOT NULL,
                    installed_on DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
                );
            "#,
            &mut source,
        )
        .await?;
        let result = sqlx::query("SELECT version FROM _seeds WHERE version=?")
            .bind(version)
            .fetch_optional(&mut source)
            .await?;
        if result.is_some() {
            continue;
        }
        SeedSchema::seed(&data).await?;
        sqlx::query("INSERT INTO _seeds (version, description) VALUES (?,?)")
            .bind(version)
            .bind(description)
            .execute(&mut source)
            .await?;
    }
    FILES.get().unwrap().write().unwrap().clear();
    Ok(())
}

pub(crate) fn id_schema(_: &mut schemars::gen::SchemaGenerator) -> Schema {
    let schema = SchemaObject {
        instance_type: Some(SingleOrVec::Vec(vec![
            InstanceType::String,
            InstanceType::Integer,
        ])),
        ..Default::default()
    };
    Schema::Object(schema)
}
@{-"\n"}@