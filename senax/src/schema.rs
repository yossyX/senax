use anyhow::{bail, Context as _, Result};
use convert_case::{Case, Casing};
use format_serde_error::SerdeError;
use indexmap::IndexMap;
use inflector::string::singularize::to_singular;
use once_cell::sync::{Lazy, OnceCell};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::Arc;

pub mod column;
pub use column::*;
pub mod model;
pub use model::*;
pub mod relation;
pub use relation::*;
pub mod index;
pub use index::*;

const CREATED_AT: &str = "created_at";
const UPDATED_AT: &str = "updated_at";
const DELETED_AT: &str = "deleted_at";
const DELETED: &str = "deleted";
const DEFAULT_TYPE_FIELD: &str = "_type";
pub const VERSIONED: &str = "_version";

pub static mut CONFIG: OnceCell<ConfigDef> = OnceCell::new();
pub static mut GROUPS: OnceCell<IndexMap<String, IndexMap<String, Arc<ModelDef>>>> =
    OnceCell::new();
pub static mut ENUM_GROUPS: OnceCell<IndexMap<String, IndexMap<String, EnumDef>>> = OnceCell::new();
pub static mut MODELS: OnceCell<IndexMap<String, Arc<ModelDef>>> = OnceCell::new();
pub static mut MODEL: OnceCell<Arc<ModelDef>> = OnceCell::new();
pub static mut TYPE_IDS: Lazy<HashSet<u64>> = Lazy::new(HashSet::new);

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[schemars(title = "Schema Definition")]
pub struct SchemaDef {
    #[schemars(default)]
    conf: HashMap<String, ConfigDef>,
    #[schemars(default)]
    r#enum: HashMap<String, EnumDef>,
    #[schemars(default)]
    model: HashMap<String, ModelDef>,
}

/// データベース設定
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[schemars(deny_unknown_fields)]
#[schemars(title = "Config Definition")]
pub struct ConfigDef {
    /// リンカーで使用されるデータベースナンバー　自動生成では毎回現在時刻が使用されるので、強制上書き時に固定する場合に指定する
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub db_no: Option<u64>,
    /// 使用するDB。現在のところmysqlのみ対応
    pub db: DbType,
    /// 仕様書等のためのタイトル
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// 仕様書等のための著者
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// falseの場合は外部キー制約をDDLに出力しない
    #[serde(default)]
    pub ignore_foreign_key: bool,
    /// テーブル名を複数形にする
    #[serde(default)]
    pub plural_table_name: bool,
    /// デフォルトのタイムスタンプ設定
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestampable: Option<Timestampable>,
    /// 日時型のデフォルトのタイムゾーン設定
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_zone: Option<TimeZone>,
    /// created_at, updated_at, deleted_atに使用されるタイムゾーン
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp_time_zone: Option<TimeZone>,
    /// 論理削除のデフォルト設定
    #[serde(skip_serializing_if = "Option::is_none")]
    pub soft_delete: Option<SoftDelete>,
    /// キャッシュ使用のデフォルト設定
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_cache: Option<bool>,
    /// 高速キャッシュ使用設定（experimental）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_fast_cache: Option<bool>,
    /// 全キャッシュ使用のデフォルト設定
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_cache_all: Option<bool>,
    /// 遅延INSERTを使用する
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_insert_delayed: Option<bool>,
    /// 遅延SAVEを使用する
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_save_delayed: Option<bool>,
    /// 遅延UPDATEを使用する
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_update_delayed: Option<bool>,
    /// 遅延UPSERTを使用する
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_upsert_delayed: Option<bool>,
    /// 更新トランザクション分離レベル
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_isolation: Option<Isolation>,
    /// 参照トランザクション分離レベル
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_tx_isolation: Option<Isolation>,
    /// MySQLのストレージエンジン
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,
    /// 文字セット
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub character_set: Option<String>,
    /// 文字セット照合順序
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collate: Option<String>,
    /// DDL出力時のカラム順序維持設定
    #[serde(default, skip_serializing_if = "is_false")]
    pub preserve_column_order: bool,
    /// モデルグループ
    pub groups: IndexMap<String, GroupDef>,
}

impl ConfigDef {
    pub fn db_no(&self) -> u64 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64;
        self.db_no.unwrap_or(now)
    }

    pub fn use_fast_cache(&self) -> bool {
        self.use_fast_cache.unwrap_or(false)
    }
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize, Clone, JsonSchema)]
#[schemars(deny_unknown_fields)]
#[schemars(title = "Group Def")]
pub struct GroupDef {
    #[serde(rename = "type")]
    group_type: GroupType,
    title: Option<String>,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    models: IndexMap<String, ModelDef>,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    enums: IndexMap<String, EnumDef>,
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize, Copy, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(title = "Group Type")]
pub enum GroupType {
    /// モデル定義
    Model,
    /// 列挙型定義のみ
    Enum,
}

#[allow(dead_code)]
pub fn get_db_type() -> DbType {
    unsafe { CONFIG.get().unwrap() }.db
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize, Copy, Clone, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[schemars(title = "DB type")]
pub enum DbType {
    Mysql,
    // PgSql
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize, Copy, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(title = "Timestampable")]
pub enum Timestampable {
    None,
    /// クエリー実行日時
    RealTime,
    /// DbConnの生成日時
    FixedTime,
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize, Copy, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(title = "TimeZone")]
pub enum TimeZone {
    Local,
    Utc,
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize, Copy, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(title = "SoftDelete")]
pub enum SoftDelete {
    None,
    Time,
    Flag,
    /// ユニーク制約に使用するためのUNIXタイムスタンプ
    /// UNIX time for unique index support
    UnixTime,
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize, Copy, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(title = "Isolation")]
pub enum Isolation {
    RepeatableRead,
    ReadCommitted,
    ReadUncommitted,
    Serializable,
}

impl Isolation {
    pub fn as_str(&self) -> &'static str {
        match self {
            Isolation::RepeatableRead => "REPEATABLE READ",
            Isolation::ReadCommitted => "READ COMMITTED",
            Isolation::ReadUncommitted => "READ UNCOMMITTED",
            Isolation::Serializable => "SERIALIZABLE",
        }
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, JsonSchema)]
#[schemars(deny_unknown_fields)]
#[schemars(title = "Enum Def")]
pub struct EnumDef {
    #[serde(skip)]
    pub name: String,

    /// タイトル
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// コメント
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// 列挙値
    pub enum_values: Vec<EnumValue>,
    /// 列挙子の名前にマルチバイトを使用した場合のmod名
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = r"^[A-Za-z][0-9A-Z_a-z]*$"))]
    pub mod_name: Option<String>,
}

impl EnumDef {
    pub fn mod_name(&self) -> &str {
        self.mod_name.as_ref().unwrap_or(&self.name)
    }
}

pub fn is_false(val: &bool) -> bool {
    !(*val)
}

pub fn parse(db: &str) -> Result<(), anyhow::Error> {
    crate::common::check_ascii_name(db);
    let path = Path::new("./schema").join(&format!("{db}.yml"));
    let mut config = if path.exists() {
        let content =
            fs::read_to_string(&path).with_context(|| format!("file cannot read: {:?}", &path))?;
        let config: ConfigDef = serde_yaml::from_str(&content)
            .map_err(|err| SerdeError::new(content.to_string(), err))?;
        config
    } else {
        let path = Path::new("./schema/conf.yml");
        let content =
            fs::read_to_string(&path).with_context(|| format!("file cannot read: {:?}", &path))?;
        let map: HashMap<String, ConfigDef> = serde_yaml::from_str(&content)
            .map_err(|err| SerdeError::new(content.to_string(), err))?;
        map.get(db)
            .with_context(|| format!("db not found in conf.yml: {}", &db))?
            .clone()
    };

    for (name, def) in config.groups.iter_mut() {
        if def.title.is_none() {
            def.title = Some(name.to_case(Case::Title));
        }
    }
    unsafe {
        CONFIG.take();
        CONFIG.set(config.clone()).unwrap();
    }

    let path = Path::new("./schema").join(&db);
    let mut groups = IndexMap::new();
    let mut enum_groups: IndexMap<String, IndexMap<String, EnumDef>> = IndexMap::new();
    let mut name_check_set = HashSet::new();
    for (group_name, group_def) in &config.groups {
        let path = path.join(&format!("{group_name}.yml"));
        crate::common::check_ascii_name(group_name);
        let mut enum_map = IndexMap::new();
        let mut model_map = IndexMap::new();
        match group_def.group_type {
            GroupType::Enum => {
                let defs: IndexMap<String, EnumDef> = if group_def.enums.is_empty() {
                    let content = fs::read_to_string(&path)
                        .with_context(|| format!("file can not read: {:?}", &path))?;
                    serde_yaml::from_str(&content)
                        .map_err(|err| SerdeError::new(content.to_string(), err))?
                } else {
                    group_def.enums.clone()
                };
                for (model_name, mut def) in defs.into_iter() {
                    crate::common::check_name(&model_name);
                    if !crate::common::is_ascii_name(&model_name)
                        && def
                            .mod_name
                            .as_ref()
                            .filter(|v| crate::common::is_ascii_name(v))
                            .is_none()
                    {
                        bail!("Non-ASCII names {:?} require a ASCII mod.", &model_name);
                    }
                    let name_check = format!("{}_{}", group_name, model_name);
                    if !name_check_set.insert(name_check) {
                        bail!("{}_{} is duplicate", group_name, model_name);
                    }
                    def.name = model_name.to_string();
                    enum_map.insert(model_name, def);
                }
            }
            _ => {
                let defs: IndexMap<String, ModelDef> = if group_def.models.is_empty() {
                    let content = fs::read_to_string(&path)
                        .with_context(|| format!("file can not read: {:?}", &path))?;
                    serde_yaml::from_str(&content)
                        .map_err(|err| SerdeError::new(content.to_string(), err))?
                } else {
                    group_def.models.clone()
                };
                for (model_name, mut def) in defs.into_iter() {
                    crate::common::check_name(&model_name);
                    if !crate::common::is_ascii_name(&model_name)
                        && def
                            .mod_name
                            .as_ref()
                            .filter(|v| crate::common::is_ascii_name(v))
                            .is_none()
                    {
                        bail!("Non-ASCII names {:?} require a ASCII mod.", &model_name);
                    }
                    let name_check = format!("{}_{}", group_name, model_name);
                    if !name_check_set.insert(name_check) {
                        bail!("{}_{} is duplicate", group_name, model_name);
                    }
                    def.db = db.to_string();
                    def.group_name = group_name.clone();
                    def.name = model_name.to_string();
                    for (name, _col) in def.columns.iter() {
                        crate::common::check_name(name);
                    }
                    let non_primaries: Vec<String> = def
                        .columns
                        .iter()
                        .filter(|(_k, v)| !v.exact().primary)
                        .map(|(k, _v)| k.clone())
                        .collect();
                    if def.created_at_conf().is_some()
                        && !def.columns.contains_key(CREATED_AT)
                        && !def.disable_created_at
                    {
                        let mut col: ColumnDef = serde_json::from_str(
                            r#"{"type":"datetime","not_null":true,"exclude_from_cache":true,"skip_factory":true}"#,
                        )?;
                        col.time_zone = config.timestamp_time_zone;
                        col.auto_gen = true;
                        def.columns.insert(CREATED_AT.to_string(), col.into());
                    }
                    if def.updated_at_conf().is_some()
                        && !def.columns.contains_key(UPDATED_AT)
                        && !def.disable_updated_at
                    {
                        let mut col: ColumnDef = serde_json::from_str(
                            r#"{"type":"datetime","not_null":true,"exclude_from_cache":true,"skip_factory":true}"#,
                        )?;
                        col.time_zone = config.timestamp_time_zone;
                        col.auto_gen = true;
                        def.columns.insert(UPDATED_AT.to_string(), col.into());
                    }
                    if let Some(soft_delete) = def.soft_delete() {
                        match soft_delete {
                            SoftDelete::None => {}
                            SoftDelete::Time => {
                                if !def.columns.contains_key(DELETED_AT) {
                                    let mut col: ColumnDef = serde_json::from_str(
                                        r#"{"type":"datetime","not_serializable":true,"skip_factory":true}"#,
                                    )?;
                                    col.time_zone = config.timestamp_time_zone;
                                    col.auto_gen = true;
                                    def.columns.insert(DELETED_AT.to_string(), col.into());
                                }
                            }
                            SoftDelete::Flag => {
                                if !def.columns.contains_key(DELETED) {
                                    let mut col: ColumnDef = serde_json::from_str(
                                        r#"{"type":"boolean","not_null":true,"not_serializable":true,"skip_factory":true}"#,
                                    )?;
                                    col.auto_gen = true;
                                    def.columns.insert(DELETED.to_string(), col.into());
                                }
                            }
                            SoftDelete::UnixTime => {
                                if !def.columns.contains_key(DELETED) {
                                    let mut col: ColumnDef = serde_json::from_str(
                                        r#"{"type":"int","not_null":true,"not_serializable":true,"skip_factory":true}"#,
                                    )?;
                                    col.auto_gen = true;
                                    def.columns.insert(DELETED.to_string(), col.into());
                                }
                            }
                        }
                    }
                    if def.versioned {
                        if def.counting.is_some() {
                            bail!(
                                "Both versioned and counting cannot be set for {}.",
                                def.name
                            );
                        }
                        if !def.columns.contains_key(VERSIONED) {
                            let mut col: ColumnDef = serde_json::from_str(
                                r#"{"type":"int","not_null":true,"skip_factory":true,"default":"0"}"#,
                            )?;
                            col.auto_gen = true;
                            def.columns.insert(VERSIONED.to_string(), col.into());
                        }
                    }
                    if let Some(ref mut inheritance) = def.inheritance {
                        if inheritance.type_def == InheritanceType::ColumnAggregation {
                            if let Some(ref key_field) = inheritance.key_field {
                                crate::common::check_name(key_field);
                            } else {
                                inheritance.key_field = Some(DEFAULT_TYPE_FIELD.to_string());
                            }
                            if inheritance.key_value.is_none() {
                                inheritance.key_value = Some(Value::from(model_name.clone()));
                            }
                            let key_field = inheritance.key_field.as_ref().unwrap();
                            match def.columns.entry(key_field.to_string()) {
                                indexmap::map::Entry::Occupied(mut entry) => {
                                    let mut col = entry.get().exact();
                                    col.skip_factory = true;
                                    entry.insert(col.into());
                                }
                                indexmap::map::Entry::Vacant(entry) => {
                                    let mut col: ColumnDef = serde_json::from_str(
                                        r#"{"type":"varchar","not_null":true,"not_serializable":true, "skip_factory": true}"#,
                                    )?;
                                    col.auto_gen = true;
                                    entry.insert(col.into());
                                }
                            }
                        }
                    }
                    for name in non_primaries {
                        def.columns.move_index(
                            def.columns.get_index_of(&name).unwrap(),
                            def.columns.len() - 1,
                        );
                    }
                    for column in def.columns.clone().iter() {
                        let mut column_def = column.1.exact();
                        if column_def.primary {
                            column_def.not_null = true;
                        }
                        if column_def.type_def == ColumnType::Enum {
                            if column_def.enum_values.is_some() {
                                column_def.class = Some(format!(
                                    "crate::{}::{}::_{}",
                                    _to_var_name(group_name),
                                    _to_var_name(def.mod_name()),
                                    _to_var_name(&column.0.to_case(Case::Pascal))
                                ));
                            } else if let Some(ref name) = column_def.enum_model {
                                if name.contains(MODEL_NAME_SPLITTER) {
                                    let (group_name, stem_name) =
                                        name.split_once(MODEL_NAME_SPLITTER).unwrap();
                                    let a = enum_groups.get(group_name).with_context(|| {
                                        format!("{group_name} enum group declaration is required first.")
                                    })?;
                                    let b = a.get(stem_name).with_context(|| {
                                        format!("{stem_name} enum declaration is required first.")
                                    })?;
                                    column_def.class = Some(format!(
                                        "crate::{}::{}::_{}",
                                        _to_var_name(group_name),
                                        _to_var_name(b.mod_name()),
                                        _to_var_name(&stem_name.to_case(Case::Pascal))
                                    ));
                                } else {
                                    column_def.class = Some(format!(
                                        "crate::{}::{}::_{}",
                                        _to_var_name(group_name),
                                        _to_var_name(def.mod_name()),
                                        _to_var_name(&name.to_case(Case::Pascal))
                                    ));
                                };
                            } else {
                                bail!("enum_values or enum_model required")
                            }
                        }
                        def.columns.insert(column.0.clone(), column_def.into());
                    }
                    def.merged_columns =
                        def.columns.iter().fold(IndexMap::new(), |mut map, (k, v)| {
                            map.insert(k.clone(), v.exact());
                            map
                        });

                    def.relations =
                        def.relations
                            .iter()
                            .fold(IndexMap::new(), |mut map, (name, rel)| {
                                map.insert(name.clone(), fix_rel_def(rel, group_name, name));
                                map
                            });
                    def.merged_relations = def.relations.clone();
                    def.merged_indexes =
                        def.indexes
                            .iter()
                            .fold(IndexMap::new(), |mut map, (name, index)| {
                                map.insert(name.clone(), index.clone().unwrap_or_default());
                                map
                            });

                    model_map.insert(model_name, RefCell::new(def));
                }
            }
        }
        groups.insert(group_name.clone(), model_map);
        enum_groups.insert(group_name.clone(), enum_map);
    }
    for (cur_group_name, defs) in groups.iter() {
        for (cur_model_name, def) in defs.iter() {
            {
                // Columns, Relations, and Index Consolidation for Inheritance
                let mut sql_model_name = cur_model_name.clone();
                let mut cur_model = def;
                let mut cur_g = cur_group_name.clone();
                while let Some(ref inheritance) = cur_model.clone().borrow().inheritance {
                    let mut model = def.borrow_mut();
                    let base_model_refcell = get_model(&inheritance.extends, &cur_g, &groups);
                    let mut base_model = base_model_refcell.borrow_mut();
                    cur_g = base_model.group_name.clone();

                    let mut merged_columns = IndexMap::new();
                    for (name, col) in &base_model.columns {
                        merged_columns.insert(name.clone(), col.exact());
                    }
                    for (name, def1) in &model.merged_columns {
                        if let Some(def2) = merged_columns.get(name) {
                            if !def2.eq(def1) {
                                bail!("{} column is already defined.", name);
                            }
                        } else {
                            merged_columns.insert(name.clone(), def1.clone());
                        }
                    }
                    model.merged_columns = merged_columns;

                    if inheritance.type_def != InheritanceType::Concrete {
                        sql_model_name = inheritance.extends.clone();
                        for (name, def) in &model.columns {
                            if let Some(base) = base_model.merged_columns.get(name) {
                                if !base.eq(&def.exact()) {
                                    bail!("{} column is already defined.", name);
                                }
                            } else {
                                base_model.merged_columns.insert(name.clone(), def.exact());
                            }
                        }
                    }

                    let mut merged_relations = base_model.relations.clone();
                    for (name, def1) in &model.merged_relations {
                        if let Some(def2) = merged_relations.get(name) {
                            if !def2.eq(def1) {
                                bail!("{} relation is already defined.", name);
                            }
                        } else {
                            merged_relations.insert(name.clone(), def1.clone());
                        }
                    }
                    model.merged_relations = merged_relations;

                    if inheritance.type_def != InheritanceType::Concrete {
                        for (name, def) in &model.relations {
                            if let Some(base) = base_model.merged_relations.get(name) {
                                if !base.eq(def) {
                                    bail!("{} relation is already defined.", name);
                                }
                            } else {
                                base_model
                                    .merged_relations
                                    .insert(name.clone(), def.clone());
                            }
                        }
                    }

                    let mut merged_indexes = base_model.indexes.iter().fold(
                        IndexMap::new(),
                        |mut map, (name, index)| {
                            map.insert(name.clone(), index.clone().unwrap_or_default());
                            map
                        },
                    );
                    for (name, def1) in &model.merged_indexes {
                        if let Some(def2) = merged_indexes.get(name) {
                            if !def2.eq(def1) {
                                bail!("{} index is already defined.", name);
                            }
                        } else {
                            merged_indexes.insert(name.clone(), def1.clone());
                        }
                    }
                    model.merged_indexes = merged_indexes;

                    if inheritance.type_def != InheritanceType::Concrete {
                        for (name, def) in &model.indexes {
                            if let Some(base) = base_model.merged_indexes.get(name) {
                                if !base.eq(&def.clone().unwrap_or_default()) {
                                    bail!("{} index is already defined.", name);
                                }
                            } else {
                                base_model
                                    .merged_indexes
                                    .insert(name.clone(), def.clone().unwrap_or_default());
                            }
                        }
                    }
                    cur_model = base_model_refcell;
                }
                let model = get_model(&sql_model_name, cur_group_name, &groups);
                let table_name = model.borrow().table_name();
                let model = get_model(cur_model_name, cur_group_name, &groups);
                model.borrow_mut().table_name = Some(table_name);
            }
            {
                let mut model = get_model(cur_model_name, cur_group_name, &groups).borrow_mut();
                let mut main_primary = None;
                let primaries = model.primaries();
                if primaries.len() == 1 {
                    let primary = primaries.first().unwrap();
                    let has_relations = model.relations_one().iter().any(|(_model, name, rel)| {
                        if !RelDef::get_local_id(rel, name, &model.id_name()).eq(primary.0) {
                            return false;
                        }
                        if let Some(rel) = rel {
                            if rel.on_delete.is_none() {
                                return false;
                            }
                        }
                        true
                    });
                    if !has_relations {
                        main_primary = Some(primary.0.clone());
                    }
                }
                if let Some(main_primary) = main_primary {
                    if let Some(column_def) = model.merged_columns.get_mut(&main_primary) {
                        column_def.class = Some(to_id_name(cur_model_name));
                        column_def.main_primary = true;
                    }
                }
                let id_name = model.id_name();
                for (rel_name, rel_def) in model.merged_relations.clone().iter() {
                    let col_name = RelDef::get_local_id(rel_def, rel_name, &id_name);
                    if let Some(column_def) = model.merged_columns.get_mut(&col_name) {
                        column_def.rel = Some((rel_name.clone(), rel_def.clone()))
                    }
                }
            }
            {
                if !def.borrow().abstract_mode
                    && def.borrow().inheritance_type() != Some(InheritanceType::Simple)
                    && def.borrow().inheritance_type() != Some(InheritanceType::ColumnAggregation)
                {
                    for (rel_name, rel_def) in def.clone().borrow().merged_relations.iter() {
                        if let Some(rel_def) = rel_def {
                            if rel_def.type_def != Some(RelationsType::Many)
                                && rel_def.on_delete.is_some()
                            {
                                let model_name = rel_def.model.as_ref().unwrap_or(rel_name);
                                let mut model =
                                    get_model(model_name, cur_group_name, &groups).borrow_mut();
                                model.on_delete_list.insert(format!(
                                    "{}::{}::_{}::_{}",
                                    &_to_var_name(cur_group_name),
                                    &_to_var_name(cur_model_name),
                                    &cur_model_name,
                                    &cur_model_name.to_case(Case::Pascal)
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
    let groups: IndexMap<String, IndexMap<String, Arc<ModelDef>>> =
        groups.into_iter().fold(IndexMap::new(), |mut map, (k, v)| {
            let v = v.into_iter().fold(IndexMap::new(), |mut map, (k, v)| {
                let mut model = v.into_inner();
                let non_primaries: Vec<String> = model
                    .merged_columns
                    .iter()
                    .filter(|(_k, v)| !v.primary && !v.auto_gen)
                    .map(|(k, _v)| k.clone())
                    .collect();
                for name in non_primaries {
                    model.merged_columns.move_index(
                        model.merged_columns.get_index_of(&name).unwrap(),
                        model.merged_columns.len() - 1,
                    );
                }
                map.insert(k, Arc::new(model));
                map
            });
            map.insert(k, v);
            map
        });
    unsafe {
        GROUPS.take();
        GROUPS.set(groups).unwrap();
        ENUM_GROUPS.take();
        ENUM_GROUPS.set(enum_groups).unwrap();
    };
    Ok(())
}

fn fix_rel_def(rel: &Option<RelDef>, group: &str, name: &str) -> Option<RelDef> {
    if let Some(ref mut d) = rel.clone() {
        if let Some(ref n) = d.model {
            if !n.contains(MODEL_NAME_SPLITTER) {
                d.model = Some(format!("{}::{}", group, n));
            }
        } else {
            d.model = Some(format!("{}::{}", group, name));
        }
        Some(d.clone())
    } else {
        Some(RelDef {
            model: Some(format!("{}::{}", group, name)),
            ..Default::default()
        })
    }
}

pub fn get_model<'a>(
    model_name: &str,
    cur_group_name: &str,
    groups: &'a IndexMap<String, IndexMap<String, RefCell<ModelDef>>>,
) -> &'a RefCell<ModelDef> {
    let (group_name, stem_name) = if model_name.contains(MODEL_NAME_SPLITTER) {
        let (group_name, model_name) = model_name.split_once(MODEL_NAME_SPLITTER).unwrap();
        (group_name.to_string(), model_name.to_string())
    } else {
        (cur_group_name.to_owned(), model_name.to_owned())
    };
    let group = groups
        .get(&group_name)
        .unwrap_or_else(|| panic!("{} group is not defined", group_name));

    if let Some(model) = group.get(&stem_name) {
        return model;
    }
    let singular_name = to_singular(&stem_name);
    let model = group
        .get(&singular_name)
        .unwrap_or_else(|| panic!("{} {} model is not defined", group_name, stem_name));
    model
}

pub fn to_id_name(name: &str) -> String {
    format!("_{}Id", name.to_case(Case::Pascal))
}
pub fn _to_var_name(s: &str) -> String {
    let name = s;
    if BAD_KEYWORDS.iter().any(|&x| x == name) {
        panic!("{} is not supported", name);
    } else if KEYWORDS.iter().any(|&x| x == name) {
        format!("r#{}", name)
    } else {
        name.to_owned()
    }
}

static KEYWORDS: &[&str] = &[
    "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn", "for",
    "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
    "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where",
    "while", "async", "await", "dyn", "abstract", "become", "box", "do", "final", "macro",
    "override", "priv", "typeof", "unsized", "virtual", "yield", "try",
];
pub static BAD_KEYWORDS: &[&str] = &["super", "self", "Self", "extern", "crate"];
