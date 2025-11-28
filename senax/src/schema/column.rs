use schemars::{
    JsonSchema,
    schema::{InstanceType, Schema, SchemaObject, SingleOrVec},
};
use serde::{
    Deserialize, Deserializer, Serialize,
    de::{self, MapAccess, Visitor},
};
use std::{borrow::Cow, fmt};

use crate::{
    api_generator::schema::ApiFieldDef,
    common::{if_then_else, yaml_value_to_str},
    migration_generator::UTF8_BYTE_LEN,
    schema::is_mysql_mode,
};
use crate::{common::ToCase as _, schema::to_id_name_wo_changing_case};

use super::{_to_var_name, CONFIG, TimeZone, domain_mode};

pub const DEFAULT_VARCHAR_LENGTH: u32 = 255;
pub const DEFAULT_PRECISION: u16 = 36;
pub const DEFAULT_SCALE: u16 = 9;
pub const UUID_LENGTH: u32 = 36;
pub const BINARY_UUID_LENGTH: u16 = 16;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Copy, Default, JsonSchema)]
#[serde(rename_all = "lowercase")]
/// ### データ型
pub enum DataType {
    Char,
    #[serde(rename = "id_varchar")]
    IdVarchar,
    #[serde(rename = "text_varchar")]
    TextVarchar,
    Text,
    Uuid,
    #[serde(rename = "binary_uuid")]
    BinaryUuid,
    TinyInt,
    SmallInt,
    #[default]
    Int,
    BigInt,
    Float,
    Double,
    Decimal,
    Date,
    Time,
    #[serde(rename = "naive_datetime")]
    NaiveDateTime,
    #[serde(rename = "utc_datetime")]
    UtcDateTime,
    #[serde(rename = "timestamp_with_timezone")]
    TimestampWithTimeZone,
    Boolean,
    Binary,
    Varbinary,
    Blob,
    #[serde(rename = "array_int")]
    ArrayInt,
    #[serde(rename = "array_string")]
    ArrayString,
    Json,
    Jsonb,
    #[serde(rename = "db_enum")]
    DbEnum,
    #[serde(rename = "db_set")]
    DbSet,
    /// x,yポイント
    Point,
    /// lat,lngポイント
    #[serde(rename = "geo_point")]
    GeoPoint,
    Geometry,
    /// 外部キー連動型
    #[serde(rename = "auto_fk")]
    AutoFk,
    /// 値オブジェクト
    #[serde(rename = "value_object")]
    ValueObject,
    #[schemars(skip)]
    UnSupported,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Copy, JsonSchema)]
#[serde(rename_all = "snake_case")]
/// ### サブセット型
pub enum DataSubsetType {
    Tinyint,
    Smallint,
    Int,
    Bigint,
    Float,
    Double,
    Char,
    #[serde(rename = "id_varchar")]
    IdVarchar,
    #[serde(rename = "text_varchar")]
    TextVarchar,
    Uuid,
    BinaryUuid,
    Boolean,
    Text,
    Binary,
    Varbinary,
    Blob,
    #[serde(rename = "naive_datetime")]
    NaiveDateTime,
    #[serde(rename = "utc_datetime")]
    UtcDateTime,
    #[serde(rename = "timestamp_with_timezone")]
    TimestampWithTimeZone,
    Date,
    Time,
    Decimal,
    ArrayInt,
    ArrayString,
    Json,
    Jsonb,
    ValueObject,
    /// x,yポイント
    Point,
    /// lat,lngポイント
    GeoPoint,
    Geometry,
    TinyintNotNull,
    SmallintNotNull,
    IntNotNull,
    BigintNotNull,
    FloatNotNull,
    DoubleNotNull,
    CharNotNull,
    #[serde(rename = "id_varchar_not_null")]
    IdVarcharNotNull,
    #[serde(rename = "text_varchar_not_null")]
    TextVarcharNotNull,
    UuidNotNull,
    BinaryUuidNotNull,
    BooleanNotNull,
    TextNotNull,
    BinaryNotNull,
    VarbinaryNotNull,
    BlobNotNull,
    #[serde(rename = "naive_datetime_not_null")]
    NaiveDateTimeNotNull,
    #[serde(rename = "utc_datetime_not_null")]
    UtcDateTimeNotNull,
    #[serde(rename = "timestamp_with_timezone_not_null")]
    TimestampWithTimeZoneNotNull,
    DateNotNull,
    TimeNotNull,
    DecimalNotNull,
    ArrayIntNotNull,
    ArrayStringNotNull,
    JsonNotNull,
    JsonbNotNull,
    ValueObjectNotNull,
    PointNotNull,
    GeoPointNotNull,
    GeometryNotNull,
    /// 外部キー連動型
    AutoFk,
    AutoFkNotNull,
}
impl From<&DataSubsetType> for DataType {
    fn from(v: &DataSubsetType) -> Self {
        match v {
            DataSubsetType::Tinyint => DataType::TinyInt,
            DataSubsetType::Smallint => DataType::SmallInt,
            DataSubsetType::Int => DataType::Int,
            DataSubsetType::Bigint => DataType::BigInt,
            DataSubsetType::Float => DataType::Float,
            DataSubsetType::Double => DataType::Double,
            DataSubsetType::Char => DataType::Char,
            DataSubsetType::IdVarchar => DataType::IdVarchar,
            DataSubsetType::TextVarchar => DataType::TextVarchar,
            DataSubsetType::Uuid => DataType::Uuid,
            DataSubsetType::BinaryUuid => DataType::BinaryUuid,
            DataSubsetType::Boolean => DataType::Boolean,
            DataSubsetType::Text => DataType::Text,
            DataSubsetType::Binary => DataType::Binary,
            DataSubsetType::Varbinary => DataType::Varbinary,
            DataSubsetType::Blob => DataType::Blob,
            DataSubsetType::NaiveDateTime => DataType::NaiveDateTime,
            DataSubsetType::UtcDateTime => DataType::UtcDateTime,
            DataSubsetType::TimestampWithTimeZone => DataType::TimestampWithTimeZone,
            DataSubsetType::Date => DataType::Date,
            DataSubsetType::Time => DataType::Time,
            DataSubsetType::Decimal => DataType::Decimal,
            DataSubsetType::ArrayInt => DataType::ArrayInt,
            DataSubsetType::ArrayString => DataType::ArrayString,
            DataSubsetType::Json => DataType::Json,
            DataSubsetType::Jsonb => DataType::Jsonb,
            DataSubsetType::ValueObject => DataType::ValueObject,
            DataSubsetType::Point => DataType::Point,
            DataSubsetType::GeoPoint => DataType::GeoPoint,
            DataSubsetType::Geometry => DataType::Geometry,
            DataSubsetType::AutoFk => DataType::AutoFk,
            DataSubsetType::TinyintNotNull => DataType::TinyInt,
            DataSubsetType::SmallintNotNull => DataType::SmallInt,
            DataSubsetType::IntNotNull => DataType::Int,
            DataSubsetType::BigintNotNull => DataType::BigInt,
            DataSubsetType::FloatNotNull => DataType::Float,
            DataSubsetType::DoubleNotNull => DataType::Double,
            DataSubsetType::CharNotNull => DataType::Char,
            DataSubsetType::IdVarcharNotNull => DataType::IdVarchar,
            DataSubsetType::TextVarcharNotNull => DataType::TextVarchar,
            DataSubsetType::UuidNotNull => DataType::Uuid,
            DataSubsetType::BinaryUuidNotNull => DataType::BinaryUuid,
            DataSubsetType::BooleanNotNull => DataType::Boolean,
            DataSubsetType::TextNotNull => DataType::Text,
            DataSubsetType::BinaryNotNull => DataType::Binary,
            DataSubsetType::VarbinaryNotNull => DataType::Varbinary,
            DataSubsetType::BlobNotNull => DataType::Blob,
            DataSubsetType::NaiveDateTimeNotNull => DataType::NaiveDateTime,
            DataSubsetType::UtcDateTimeNotNull => DataType::UtcDateTime,
            DataSubsetType::TimestampWithTimeZoneNotNull => DataType::TimestampWithTimeZone,
            DataSubsetType::DateNotNull => DataType::Date,
            DataSubsetType::TimeNotNull => DataType::Time,
            DataSubsetType::DecimalNotNull => DataType::Decimal,
            DataSubsetType::ArrayIntNotNull => DataType::ArrayInt,
            DataSubsetType::ArrayStringNotNull => DataType::ArrayString,
            DataSubsetType::JsonNotNull => DataType::Json,
            DataSubsetType::JsonbNotNull => DataType::Jsonb,
            DataSubsetType::ValueObjectNotNull => DataType::ValueObject,
            DataSubsetType::PointNotNull => DataType::Point,
            DataSubsetType::GeoPointNotNull => DataType::GeoPoint,
            DataSubsetType::GeometryNotNull => DataType::Geometry,
            DataSubsetType::AutoFkNotNull => DataType::AutoFk,
        }
    }
}
impl DataSubsetType {
    pub fn not_null(&self) -> bool {
        match self {
            DataSubsetType::Tinyint => false,
            DataSubsetType::Smallint => false,
            DataSubsetType::Int => false,
            DataSubsetType::Bigint => false,
            DataSubsetType::Float => false,
            DataSubsetType::Double => false,
            DataSubsetType::Char => false,
            DataSubsetType::IdVarchar => false,
            DataSubsetType::TextVarchar => false,
            DataSubsetType::Uuid => false,
            DataSubsetType::BinaryUuid => false,
            DataSubsetType::Boolean => false,
            DataSubsetType::Text => false,
            DataSubsetType::Binary => false,
            DataSubsetType::Varbinary => false,
            DataSubsetType::Blob => false,
            DataSubsetType::NaiveDateTime => false,
            DataSubsetType::UtcDateTime => false,
            DataSubsetType::TimestampWithTimeZone => false,
            DataSubsetType::Date => false,
            DataSubsetType::Time => false,
            DataSubsetType::Decimal => false,
            DataSubsetType::ArrayInt => false,
            DataSubsetType::ArrayString => false,
            DataSubsetType::Json => false,
            DataSubsetType::Jsonb => false,
            DataSubsetType::ValueObject => false,
            DataSubsetType::Point => false,
            DataSubsetType::GeoPoint => false,
            DataSubsetType::Geometry => false,
            DataSubsetType::AutoFk => false,
            DataSubsetType::TinyintNotNull => true,
            DataSubsetType::SmallintNotNull => true,
            DataSubsetType::IntNotNull => true,
            DataSubsetType::BigintNotNull => true,
            DataSubsetType::FloatNotNull => true,
            DataSubsetType::DoubleNotNull => true,
            DataSubsetType::CharNotNull => true,
            DataSubsetType::IdVarcharNotNull => true,
            DataSubsetType::TextVarcharNotNull => true,
            DataSubsetType::UuidNotNull => true,
            DataSubsetType::BinaryUuidNotNull => true,
            DataSubsetType::BooleanNotNull => true,
            DataSubsetType::TextNotNull => true,
            DataSubsetType::BinaryNotNull => true,
            DataSubsetType::VarbinaryNotNull => true,
            DataSubsetType::BlobNotNull => true,
            DataSubsetType::NaiveDateTimeNotNull => true,
            DataSubsetType::UtcDateTimeNotNull => true,
            DataSubsetType::TimestampWithTimeZoneNotNull => true,
            DataSubsetType::DateNotNull => true,
            DataSubsetType::TimeNotNull => true,
            DataSubsetType::DecimalNotNull => true,
            DataSubsetType::ArrayIntNotNull => true,
            DataSubsetType::ArrayStringNotNull => true,
            DataSubsetType::JsonNotNull => true,
            DataSubsetType::JsonbNotNull => true,
            DataSubsetType::ValueObjectNotNull => true,
            DataSubsetType::PointNotNull => true,
            DataSubsetType::GeoPointNotNull => true,
            DataSubsetType::GeometryNotNull => true,
            DataSubsetType::AutoFkNotNull => true,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Copy, JsonSchema)]
#[serde(rename_all = "snake_case")]
/// ### 自動生成主キー
pub enum AutoGeneration {
    /// ### オートインクリメント
    AutoIncrement,
    /// ### シーケンス
    Sequence,
    /// ### UUID
    Uuid,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
/// ### Enum値
pub struct EnumValue {
    /// ### 名前
    pub name: String,
    /// ### 論理名
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// ### コメント
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// ### 値
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<i64>,
}

impl EnumValue {
    pub fn value_str(&self) -> String {
        if let Some(value) = self.value {
            format!(" = {value}")
        } else {
            "".to_string()
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Clone, JsonSchema)]
#[serde(untagged)]
/// ### フィールド定義またはサブセット型
#[allow(clippy::large_enum_variant)]
pub enum FieldDefOrSubsetType {
    Exact(FieldDef),
    Simple(DataSubsetType),
}

impl FieldDefOrSubsetType {
    pub fn exact(&self) -> FieldDef {
        match self {
            FieldDefOrSubsetType::Exact(def) => def.clone(),
            FieldDefOrSubsetType::Simple(_type) => FieldDef {
                data_type: _type.into(),
                not_null: _type.not_null(),
                ..Default::default()
            },
        }
    }
}
impl From<FieldDef> for FieldDefOrSubsetType {
    fn from(val: FieldDef) -> Self {
        FieldDefOrSubsetType::Exact(val)
    }
}

struct FieldDefOrSubsetTypeVisitor;
impl<'de> Visitor<'de> for FieldDefOrSubsetTypeVisitor {
    type Value = FieldDefOrSubsetType;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("string or map")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(FieldDefOrSubsetType::Simple(DataSubsetType::deserialize(
            de::value::StrDeserializer::new(value),
        )?))
    }

    fn visit_map<M>(self, map: M) -> Result<Self::Value, M::Error>
    where
        M: MapAccess<'de>,
    {
        Ok(FieldDefOrSubsetType::Exact(FieldDef::deserialize(
            de::value::MapAccessDeserializer::new(map),
        )?))
    }
}
impl<'de> Deserialize<'de> for FieldDefOrSubsetType {
    fn deserialize<D>(deserializer: D) -> Result<FieldDefOrSubsetType, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(FieldDefOrSubsetTypeVisitor)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct IdClass {
    pub outer_crate: bool,
    pub db: String,
    pub group: String,
    pub mod_name: String,
    pub name: String,
}

impl std::fmt::Display for IdClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if domain_mode() {
            write!(
                f,
                "domain::models::{}::{}::{}::{}Id",
                _to_var_name(&self.db),
                _to_var_name(&self.group),
                _to_var_name(&self.mod_name),
                &self.name
            )
        } else {
            write!(f, "{}", to_id_name_wo_changing_case(&self.name))
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct EnumClass {
    pub outer_crate: bool,
    pub db: String,
    pub group: String,
    pub mod_name: String,
    pub name: String,
}

impl std::fmt::Display for EnumClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if domain_mode() {
            write!(
                f,
                "domain::models::{}::{}::{}::{}",
                _to_var_name(&self.db),
                _to_var_name(&self.group),
                _to_var_name(&self.mod_name),
                _to_var_name(&self.name)
            )
        } else {
            write!(
                f,
                "{}::models::{}::{}::_{}",
                if_then_else!(
                    self.outer_crate,
                    format!("db_{}", self.db),
                    "db".to_string()
                ),
                _to_var_name(&self.group),
                _to_var_name(&self.mod_name),
                &self.name
            )
        }
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
/// ### フィールド定義
pub struct FieldDef {
    #[serde(skip)]
    pub id_class: Option<IdClass>,
    #[serde(skip)]
    pub enum_class: Option<EnumClass>,
    #[serde(skip)]
    pub rel: Option<(String, super::RelDef)>,
    #[serde(skip)]
    pub outer_db_rel: Option<(String, super::RelDef)>,
    #[serde(skip)]
    pub auto_gen: bool,
    #[serde(skip)]
    pub is_timestamp: bool,
    #[serde(skip)]
    pub in_abstract: bool,
    #[serde(skip)]
    pub is_version: bool,

    /// ### リネーム元カラム名
    /// よくわからない場合は手動で修正しないこと。また、コピー&ペーストを行う場合は削除した方が良い。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _before_rename_name: Option<String>,
    /// ### 論理名
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// ### コメント
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// ### データ型
    #[serde(rename = "type")]
    pub data_type: DataType,
    /// ### 値オブジェクト
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_object: Option<String>,
    /// ### 符号付き
    /// 指定がない場合はunsigned
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub signed: bool,
    /// ### NULL不可
    /// 指定がない場合はnullable
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub not_null: bool,
    /// ### 入力時必須設定
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub required: bool,
    /// ### 主キー
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub primary: bool,
    /// ### 主キー自動生成
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto: Option<AutoGeneration>,
    /// ### メイン主キー
    /// メインの主キーのフィールド名がid, {モデル名}_idではなく、また主キー自動生成も設定されていない場合に指定が必要
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub main_primary: bool,
    /// ### 長さ
    /// 文字列の場合はバイト数ではなく、文字数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length: Option<u32>,
    /// ### 最大値
    /// (decimalは非対応)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<u64>,
    /// ### 最小値
    /// (decimalは非対応)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<i64>,
    // #[serde(default, skip_serializing_if = "Option::is_none")]
    // pub character_set: Option<String>,
    /// ### 照合順序
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collation: Option<String>,
    /// ### 有効桁数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub precision: Option<u16>,
    /// ### 小数点以下桁数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<u16>,
    /// ### タイムゾーン
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_time_zone: Option<TimeZone>,
    /// ### 列挙型の値
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<EnumValue>>,
    /// ### Json型で使用する型名
    /// 省略時はserde_json::Value
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_defined_json_type: Option<String>,
    /// ### キャッシュからの除外設定
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude_from_cache: Option<bool>,
    /// ### ファクトリーからの除外設定
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_factory: Option<bool>,
    /// ### カラム別名設定
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column_name: Option<String>,
    /// ### 空間データのSRID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub srid: Option<u32>,
    /// ### デフォルト値
    #[schemars(default, schema_with = "default_value_schema")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_yaml::Value>,
    /// ### Generated Column のクエリ
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    /// ### Generated Column を保存する
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stored: Option<bool>,
    /// ### DBのテーブル定義に使用するコメント
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sql_comment: Option<String>,
    /// ### 非表示
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
    /// ### シークレット
    /// trueの場合はログに出力しない
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<bool>,
}
fn default_value_schema(_: &mut schemars::r#gen::SchemaGenerator) -> Schema {
    let schema = SchemaObject {
        instance_type: Some(SingleOrVec::Vec(vec![
            InstanceType::Boolean,
            InstanceType::String,
            InstanceType::Number,
            InstanceType::Integer,
        ])),
        ..Default::default()
    };
    Schema::Object(schema)
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
/// ### フィールド定義
pub struct FieldJson {
    /// ### フィールド名
    #[schemars(regex(pattern = r"^\p{XID_Start}\p{XID_Continue}*(?<!_)$"))]
    pub name: String,
    /// ### リネーム元カラム名
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _before_rename_name: Option<String>,
    /// ### 論理名
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// ### コメント
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// ### データ型
    #[serde(rename = "type")]
    pub data_type: DataType,
    /// ### 値オブジェクト
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_object: Option<String>,
    /// ### 符号付き
    /// 指定がない場合はfloatやdoubleも含め、unsignedとなる
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub signed: bool,
    /// ### NULL不可
    /// 指定がない場合はnullable
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub not_null: bool,
    /// ### 入力時必須設定
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub required: bool,
    /// ### 主キー
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub primary: bool,
    /// ### 主キー自動生成
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto: Option<AutoGeneration>,
    /// ### メイン主キー
    /// メインの主キーのフィールド名がid, {モデル名}_idではなく、また主キー自動生成も設定されていない場合に指定が必要
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub main_primary: bool,
    /// ### 長さ
    /// 文字列の場合はバイト数ではなく、文字数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length: Option<u32>,
    /// ### 最大値
    /// (decimalは非対応)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<u64>,
    /// ### 最小値
    /// (decimalは非対応)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<i64>,
    // #[serde(default, skip_serializing_if = "Option::is_none")]
    // pub character_set: Option<String>,
    /// ### 照合順序
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collation: Option<String>,
    /// ### 有効桁数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub precision: Option<u16>,
    /// ### 小数点以下桁数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<u16>,
    /// ### タイムゾーン
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_time_zone: Option<TimeZone>,
    /// ### 列挙型の値
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub enum_values: Vec<EnumValue>,
    /// ### Json型で使用する型名
    /// 省略時はserde_json::Value
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_defined_json_type: Option<String>,
    /// ### キャッシュからの除外設定
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude_from_cache: Option<bool>,
    /// ### ファクトリーからの除外設定
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_factory: Option<bool>,
    /// ### カラム別名設定
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column_name: Option<String>,
    /// ### 空間データのSRID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub srid: Option<u32>,
    /// ### デフォルト値
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    /// ### Generated Column のクエリ
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    /// ### Generated Column を保存する
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stored: Option<bool>,
    /// ### DBのテーブル定義に使用するコメント
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sql_comment: Option<String>,
    /// ### 非表示
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
    /// ### シークレット
    /// trueの場合はログに出力しない
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<bool>,
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
/// ### 値オブジェクト定義
pub struct ValueObjectJson {
    /// ### 値オブジェクト名
    #[schemars(regex(pattern = r"^\p{XID_Start}\p{XID_Continue}*(?<!_)$"))]
    pub name: String,
    /// ### 論理名
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// ### コメント
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// ### データ型
    #[serde(rename = "type")]
    pub data_type: DataType,
    /// ### 符号付き
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub signed: bool,
    /// ### 長さ
    /// 文字列の場合はバイト数ではなく、文字数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length: Option<u32>,
    /// ### 最大値
    /// (decimalは非対応)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<u64>,
    /// ### 最小値
    /// (decimalは非対応)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<i64>,
    // #[serde(default, skip_serializing_if = "Option::is_none")]
    // pub character_set: Option<String>,
    /// ### 照合順序
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collation: Option<String>,
    /// ### 有効桁数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub precision: Option<u16>,
    /// ### 小数点以下桁数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<u16>,
    /// ### タイムゾーン
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_time_zone: Option<TimeZone>,
    /// ### 列挙型の値
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub enum_values: Vec<EnumValue>,
    /// ### Json型で使用する型名
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_defined_json_type: Option<String>,
    /// ### キャッシュからの除外設定
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub exclude_from_cache: bool,
    /// ### factoryからの除外設定
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub skip_factory: bool,
    /// ### カラム別名設定
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column_name: Option<String>,
    /// ### 空間データのSRID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub srid: Option<u32>,
    /// ### デフォルト値
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    /// ### Generated Column のクエリ
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    /// ### Generated Column を保存する
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub stored: bool,
    /// ### DBのテーブル定義に使用するコメント
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sql_comment: Option<String>,
    /// ### 非表示
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub hidden: bool,
    /// ### シークレット
    /// trueの場合はログに出力しない
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub secret: bool,
}

impl From<FieldDef> for FieldJson {
    fn from(value: FieldDef) -> Self {
        Self {
            name: Default::default(),
            _before_rename_name: value._before_rename_name,
            label: value.label,
            comment: value.comment,
            data_type: value.data_type,
            value_object: value.value_object,
            signed: value.signed,
            not_null: value.not_null,
            required: value.required,
            primary: value.primary,
            auto: value.auto,
            main_primary: value.main_primary,
            length: value.length,
            max: value.max,
            min: value.min,
            collation: value.collation,
            precision: value.precision,
            scale: value.scale,
            output_time_zone: value.output_time_zone,
            enum_values: value.enum_values.unwrap_or_default(),
            user_defined_json_type: value.user_defined_json_type,
            exclude_from_cache: value.exclude_from_cache,
            skip_factory: value.skip_factory,
            column_name: value.column_name,
            srid: value.srid,
            default: value.default.map(|v| yaml_value_to_str(&v).unwrap()),
            query: value.query,
            stored: value.stored,
            sql_comment: value.sql_comment,
            hidden: value.hidden,
            secret: value.secret,
        }
    }
}

impl From<FieldJson> for FieldDef {
    #[allow(clippy::collapsible_if)]
    fn from(value: FieldJson) -> Self {
        Self {
            _before_rename_name: value._before_rename_name,
            id_class: Default::default(),
            enum_class: Default::default(),
            rel: Default::default(),
            outer_db_rel: Default::default(),
            auto_gen: Default::default(),
            is_timestamp: Default::default(),
            in_abstract: Default::default(),
            is_version: Default::default(),
            label: value.label,
            comment: value.comment,
            data_type: value.data_type,
            value_object: value.value_object,
            signed: value.signed,
            not_null: value.not_null,
            required: value.required,
            primary: value.primary,
            auto: value.auto,
            main_primary: value.main_primary,
            length: value.length,
            max: value.max,
            min: value.min,
            collation: value.collation,
            precision: value.precision,
            scale: value.scale,
            output_time_zone: value.output_time_zone,
            enum_values: if value.enum_values.is_empty() {
                None
            } else {
                Some(value.enum_values)
            },
            user_defined_json_type: value.user_defined_json_type,
            exclude_from_cache: value.exclude_from_cache,
            skip_factory: value.skip_factory,
            column_name: value.column_name,
            srid: value.srid,
            default: value.default.map(|v| {
                if value.data_type == DataType::Boolean {
                    if v.eq_ignore_ascii_case("true") {
                        return serde_yaml::Value::Bool(true);
                    } else if v.eq_ignore_ascii_case("false") {
                        return serde_yaml::Value::Bool(false);
                    }
                } else if value.data_type == DataType::TinyInt
                    || value.data_type == DataType::SmallInt
                    || value.data_type == DataType::Int
                    || value.data_type == DataType::BigInt
                {
                    if let Ok(i) = v.parse::<i64>() {
                        return serde_yaml::Value::Number(i.into());
                    }
                } else if value.data_type == DataType::Float
                    || value.data_type == DataType::Double
                    || value.data_type == DataType::Decimal
                {
                    if let Ok(i) = v.parse::<f64>() {
                        return serde_yaml::Value::Number(i.into());
                    }
                }
                serde_yaml::Value::String(v)
            }),
            query: value.query,
            stored: value.stored,
            sql_comment: value.sql_comment,
            hidden: value.hidden,
            secret: value.secret,
        }
    }
}

impl From<FieldDef> for ValueObjectJson {
    fn from(value: FieldDef) -> Self {
        Self {
            name: Default::default(),
            label: value.label,
            comment: value.comment,
            data_type: value.data_type,
            signed: value.signed,
            length: value.length,
            max: value.max,
            min: value.min,
            collation: value.collation,
            precision: value.precision,
            scale: value.scale,
            output_time_zone: value.output_time_zone,
            enum_values: value.enum_values.unwrap_or_default(),
            user_defined_json_type: value.user_defined_json_type,
            exclude_from_cache: value.exclude_from_cache.unwrap_or_default(),
            skip_factory: value.skip_factory.unwrap_or_default(),
            column_name: value.column_name,
            srid: value.srid,
            default: value.default.map(|v| yaml_value_to_str(&v).unwrap()),
            query: value.query,
            stored: value.stored.unwrap_or_default(),
            sql_comment: value.sql_comment,
            hidden: value.hidden.unwrap_or_default(),
            secret: value.secret.unwrap_or_default(),
        }
    }
}

impl From<ValueObjectJson> for FieldDef {
    fn from(value: ValueObjectJson) -> Self {
        Self {
            _before_rename_name: None,
            id_class: Default::default(),
            enum_class: Default::default(),
            rel: Default::default(),
            outer_db_rel: Default::default(),
            main_primary: Default::default(),
            auto_gen: Default::default(),
            is_timestamp: Default::default(),
            in_abstract: Default::default(),
            is_version: Default::default(),
            label: value.label,
            comment: value.comment,
            data_type: value.data_type,
            value_object: Default::default(),
            signed: value.signed,
            not_null: Default::default(),
            required: Default::default(),
            primary: Default::default(),
            auto: Default::default(),
            length: value.length,
            max: value.max,
            min: value.min,
            collation: value.collation,
            precision: value.precision,
            scale: value.scale,
            output_time_zone: value.output_time_zone,
            enum_values: if value.enum_values.is_empty() {
                None
            } else {
                Some(value.enum_values)
            },
            user_defined_json_type: value.user_defined_json_type,
            exclude_from_cache: Some(value.exclude_from_cache),
            skip_factory: Some(value.skip_factory),
            column_name: value.column_name,
            srid: value.srid,
            default: value.default.map(|v| serde_yaml::from_str(&v).unwrap()),
            query: value.query,
            stored: Some(value.stored),
            sql_comment: value.sql_comment,
            hidden: Some(value.hidden),
            secret: Some(value.secret),
        }
    }
}

impl FieldDef {
    pub fn overwrite(&mut self, org: Self, postfix: &str) {
        // data_type, signed, and other type-related items should not be overwritten.
        self._before_rename_name = org._before_rename_name;
        self.id_class = org.id_class;
        self.enum_class = org.enum_class;
        self.rel = org.rel;
        self.outer_db_rel = org.outer_db_rel;
        self.main_primary = org.main_primary;
        self.auto_gen = org.auto_gen;
        self.in_abstract = org.in_abstract;
        if let Some(label) = org.label {
            self.label = Some(label);
        } else if let Some(label) = &self.label {
            self.label = Some(format!("{} {}", label, postfix));
        }
        if let Some(comment) = org.comment {
            self.comment = Some(comment);
        }
        self.not_null = org.not_null;
        self.required = org.required;
        self.primary = org.primary;
        self.auto = org.auto;
        if let Some(exclude_from_cache) = org.exclude_from_cache {
            self.exclude_from_cache = Some(exclude_from_cache);
        }
        if let Some(skip_factory) = org.skip_factory {
            self.skip_factory = Some(skip_factory);
        }
        if let Some(column_name) = org.column_name {
            self.column_name = Some(column_name);
        }
        if let Some(default) = org.default {
            self.default = Some(default);
        }
        if let Some(query) = org.query {
            self.query = Some(query);
        }
        if let Some(stored) = org.stored {
            self.stored = Some(stored);
        }
        if let Some(sql_comment) = org.sql_comment {
            self.sql_comment = Some(sql_comment);
        }
        if let Some(exclude_from_cache) = org.exclude_from_cache {
            self.exclude_from_cache = Some(exclude_from_cache);
        }
        if let Some(hidden) = org.hidden {
            self.hidden = Some(hidden);
        }
        if let Some(secret) = org.secret {
            self.secret = Some(secret);
        }
    }
    pub fn is_utc_output(&self) -> bool {
        let tz =
            self.output_time_zone
                .or(CONFIG.read().unwrap().as_ref().unwrap().output_time_zone);
        tz == Some(TimeZone::Utc)
    }
    pub fn is_integer(&self) -> bool {
        self.data_type == DataType::TinyInt
            || self.data_type == DataType::SmallInt
            || self.data_type == DataType::Int
            || self.data_type == DataType::BigInt
    }
    pub fn exclude_from_cache(&self) -> bool {
        self.exclude_from_cache == Some(true) || self.query.is_some()
    }
    pub fn skip_factory(&self) -> bool {
        if self.query.is_some() {
            return true;
        }
        if let Some(v) = self.skip_factory {
            return v;
        }
        self.auto.is_some()
    }

    pub fn get_default(&self) -> String {
        if let Some(value) = &self.default {
            let result = match self.data_type {
                DataType::Char | DataType::IdVarchar | DataType::TextVarchar => {
                    format!("{:?}.to_string().into()", yaml_value_to_str(value).unwrap())
                }
                DataType::Text => {
                    format!("{:?}.to_string().into()", yaml_value_to_str(value).unwrap())
                }
                DataType::TinyInt | DataType::SmallInt | DataType::Int | DataType::BigInt
                    if self.enum_values.is_some() =>
                {
                    format!(
                        "{}::{}.inner()",
                        self.enum_class.as_ref().unwrap(),
                        yaml_value_to_str(value).unwrap()
                    )
                }
                DataType::TinyInt | DataType::SmallInt | DataType::Int | DataType::BigInt => {
                    yaml_value_to_str(value).unwrap()
                }
                DataType::Float => {
                    format!("{}f32", yaml_value_to_str(value).unwrap())
                }
                DataType::Double => {
                    format!("{}f64", yaml_value_to_str(value).unwrap())
                }
                DataType::Decimal => yaml_value_to_str(value).unwrap(),
                DataType::Boolean => {
                    let v = yaml_value_to_str(value).unwrap();
                    if v.eq_ignore_ascii_case("true") || v.eq("1") {
                        "true".to_string()
                    } else {
                        "false".to_string()
                    }
                }
                DataType::Binary | DataType::Varbinary | DataType::Blob => {
                    yaml_value_to_str(value).unwrap()
                }
                _ if self.enum_class.is_some() => {
                    format!(
                        "{}::{}",
                        self.enum_class.as_ref().unwrap(),
                        yaml_value_to_str(value).unwrap()
                    )
                }
                _ if self.enum_values.is_some() => {
                    format!("{:?}.to_string()", yaml_value_to_str(value).unwrap())
                }
                _ => return "Default::default()".to_string(),
            };
            if self.not_null {
                result
            } else {
                format!("Some({})", &result)
            }
        } else {
            "Default::default()".to_string()
        }
    }

    pub fn get_api_default(&self, name: &str) -> String {
        let conv = |value| -> String {
            let result = match self.data_type {
                DataType::Char | DataType::IdVarchar | DataType::TextVarchar => {
                    format!("{:?}.to_string()", yaml_value_to_str(value).unwrap())
                }
                DataType::Text => format!("{:?}.to_string()", yaml_value_to_str(value).unwrap()),
                DataType::TinyInt | DataType::SmallInt | DataType::Int | DataType::BigInt
                    if self.enum_values.is_some() =>
                {
                    format!(
                        "{}::{}",
                        self.enum_class.as_ref().unwrap(),
                        yaml_value_to_str(value).unwrap()
                    )
                }
                DataType::TinyInt | DataType::SmallInt | DataType::Int | DataType::BigInt => {
                    yaml_value_to_str(value).unwrap()
                }
                DataType::Float => {
                    format!("{}f32", yaml_value_to_str(value).unwrap())
                }
                DataType::Double => {
                    format!("{}f64", yaml_value_to_str(value).unwrap())
                }
                DataType::Decimal => yaml_value_to_str(value).unwrap(),
                DataType::Boolean => {
                    let v = yaml_value_to_str(value).unwrap();
                    format!("{}", v.eq_ignore_ascii_case("true") || v.eq("1"))
                }
                DataType::Binary | DataType::Varbinary | DataType::Blob => {
                    yaml_value_to_str(value).unwrap()
                }
                // _ if self.enum_values.is_some() => {
                //     format!("{:?}.to_string()", yaml_value_to_str(value).unwrap())
                // }
                _ => return "Default::default()".to_string(),
            };
            // if self.value_object.is_some() {
            //     result.push_str(".into()")
            // }
            if self.not_null {
                result
            } else {
                format!("Some(Some({})).into()", &result)
            }
        };
        if let Some(value) = ApiFieldDef::default(name) {
            conv(&value)
        } else {
            "Default::default()".to_string()
        }
    }

    pub fn get_column_query(&self, name: &str) -> String {
        if is_mysql_mode() {
            match self.data_type {
                DataType::Uuid => {
                    format!(
                        "    #[sql(query = {:?})]\n",
                        &format!("UUID_TO_BIN(`{}`)", self.get_col_name(name))
                    )
                }
                DataType::Point => {
                    format!(
                        "    #[sql(query = {:?})]\n",
                        &format!("ST_AsBinary(`{}`)", self.get_col_name(name))
                    )
                }
                DataType::GeoPoint => {
                    format!(
                        "    #[sql(query = {:?})]\n",
                        &format!(
                            "ST_AsBinary(`{}`, 'axis-order=long-lat')",
                            self.get_col_name(name)
                        )
                    )
                }
                DataType::Geometry => {
                    format!(
                        "    #[sql(query = {:?})]\n",
                        &format!("ST_AsGeoJSON(`{}`)", self.get_col_name(name))
                    )
                }
                _ => {
                    if self
                        .collation
                        .as_deref()
                        .is_some_and(|v| v.ends_with("_bin"))
                    {
                        format!(
                            "    #[sql(query = {:?})]\n",
                            &format!("CONVERT(`{}` USING utf8mb4)", self.get_col_name(name))
                        )
                    } else if self.column_name.is_some() || self.query.is_some() {
                        format!(
                            "    #[sql(query = {:?})]\n",
                            &format!("`{}`", self.get_col_name(name))
                        )
                    } else {
                        "".to_owned()
                    }
                }
            }
        } else {
            match self.data_type {
                DataType::Jsonb => {
                    format!(
                        "    #[sql(query = {:?})]\n",
                        &format!("\"{}\"::json", self.get_col_name(name))
                    )
                }
                DataType::Point => {
                    format!(
                        "    #[sql(query = {:?})]\n",
                        &format!("ST_AsBinary(\"{}\")", self.get_col_name(name))
                    )
                }
                DataType::GeoPoint => {
                    format!(
                        "    #[sql(query = {:?})]\n",
                        &format!("ST_AsBinary(\"{}\")", self.get_col_name(name))
                    )
                }
                DataType::Geometry => {
                    format!(
                        "    #[sql(query = {:?})]\n",
                        &format!("ST_AsGeoJSON(\"{}\")", self.get_col_name(name))
                    )
                }
                _ => {
                    if self.column_name.is_some() || self.query.is_some() {
                        format!(
                            "    #[sql(query = {:?})]\n",
                            &format!("\"{}\"", self.get_col_name(name))
                        )
                    } else {
                        "".to_owned()
                    }
                }
            }
        }
    }

    pub fn get_col_query(&self, name: &str) -> String {
        if is_mysql_mode() {
            match self.data_type {
                DataType::Uuid => {
                    format!("UUID_TO_BIN(`{}`)", name)
                }
                DataType::Point => {
                    format!("ST_AsBinary(`{}`)", name)
                }
                DataType::GeoPoint => {
                    format!("ST_AsBinary(`{}`, 'axis-order=long-lat')", name)
                }
                DataType::Geometry => {
                    format!("ST_AsGeoJSON(`{}`)", name)
                }
                _ => {
                    if self
                        .collation
                        .as_deref()
                        .is_some_and(|v| v.ends_with("_bin"))
                    {
                        format!("CONVERT(`{}` USING utf8mb4)", name)
                    } else {
                        format!("`{}`", name)
                    }
                }
            }
        } else {
            match self.data_type {
                DataType::Jsonb => {
                    format!("\"{}\"::json", name)
                }
                _ => {
                    format!("\"{}\"", name)
                }
            }
        }
    }

    pub fn get_validate(&self, name: &str) -> String {
        let var_name = &_to_var_name(name);
        match self.data_type {
            DataType::Char | DataType::IdVarchar | DataType::TextVarchar if !self.not_null => {
                let length = self.length.unwrap_or(DEFAULT_VARCHAR_LENGTH);
                format!(
                    r#"
        if let Some(v) = &self.{var_name} {{
            if v.as_ref().chars().count() > {length} {{
                errors.add({name:?}, validator::ValidationError::new("length"))
            }}
        }}"#
                )
            }
            DataType::Char | DataType::IdVarchar | DataType::TextVarchar => {
                let length = self.length.unwrap_or(DEFAULT_VARCHAR_LENGTH);
                format!(
                    r#"
        if self.{var_name}.as_ref().chars().count() > {length} {{
            errors.add({name:?}, validator::ValidationError::new("length"))
        }}"#
                )
            }
            DataType::Text if !self.not_null => {
                let limit = CONFIG.read().unwrap().as_ref().unwrap().max_db_str_len();
                let length = self
                    .length
                    .map(|l| limit.min(l as u64 * UTF8_BYTE_LEN as u64))
                    .unwrap_or(limit);
                format!(
                    r#"
        if let Some(v) = &self.{var_name} {{
            if v.as_ref().len() > {length} {{
                errors.add({name:?}, validator::ValidationError::new("length"))
            }}
        }}"#
                )
            }
            DataType::Text => {
                let limit = CONFIG.read().unwrap().as_ref().unwrap().max_db_str_len();
                let length = self
                    .length
                    .map(|l| limit.min(l as u64 * UTF8_BYTE_LEN as u64))
                    .unwrap_or(limit);
                format!(
                    r#"
        if self.{var_name}.as_ref().len() > {length} {{
            errors.add({name:?}, validator::ValidationError::new("length"))
        }}"#
                )
            }
            DataType::Binary | DataType::Varbinary | DataType::Blob if !self.not_null => {
                let limit = CONFIG.read().unwrap().as_ref().unwrap().max_db_str_len();
                let length = self.length.map(|l| limit.min(l as u64)).unwrap_or(limit);
                format!(
                    r#"
        if let Some(v) = &self.{var_name} {{
            if v.as_ref().len() > {length} {{
                errors.add({name:?}, validator::ValidationError::new("length"))
            }}
        }}"#
                )
            }
            DataType::Binary | DataType::Varbinary | DataType::Blob => {
                let limit = CONFIG.read().unwrap().as_ref().unwrap().max_db_str_len();
                let length = self.length.map(|l| limit.min(l as u64)).unwrap_or(limit);
                format!(
                    r#"
        if self.{var_name}.as_ref().len() > {length} {{
            errors.add({name:?}, validator::ValidationError::new("length"))
        }}"#
                )
            }
            DataType::Double | DataType::Float if !self.signed && !self.not_null => {
                format!(
                    r#"
        if let Some(v) = self.{var_name} {{
            if v < 0.0 {{
                errors.add({name:?}, validator::ValidationError::new("range"))
            }}
        }}"#
                )
            }
            DataType::Double | DataType::Float if !self.signed => {
                format!(
                    r#"
        if self.{var_name} < 0.0 {{
            errors.add({name:?}, validator::ValidationError::new("range"))
        }}"#
                )
            }
            DataType::Decimal if !self.signed && !self.not_null => {
                format!(
                    r#"
        if let Some(v) = self.{var_name} {{
            if v.is_sign_negative() {{
                errors.add({name:?}, validator::ValidationError::new("range"))
            }}
        }}"#
                )
            }
            DataType::Decimal if !self.signed => {
                format!(
                    r#"
        if self.{var_name}.is_sign_negative() {{
            errors.add({name:?}, validator::ValidationError::new("range"))
        }}"#
                )
            }
            _ => "".to_owned(),
        }
    }

    pub fn get_api_validate_const(&self, name: &str) -> String {
        let name = name.to_upper_snake();
        let typ = self.get_inner_type(true, false);
        match self.data_type {
            DataType::Char | DataType::IdVarchar | DataType::TextVarchar => {
                let length = self.length.unwrap_or(DEFAULT_VARCHAR_LENGTH);
                format!("\n    pub const {}_MAX_LEN: usize = {};", name, length)
            }
            DataType::Text => {
                if let Some(length) = self.length {
                    format!("\n    pub const {}_MAX_LEN: usize = {};", name, length)
                } else {
                    "".to_owned()
                }
            }
            _ if self.min.is_some() && self.max.is_some() => {
                format!(
                    "\n    pub const {}_MIN: {} = {};\n    pub const {}_MAX: {} = {};",
                    name,
                    typ,
                    self.min.unwrap(),
                    name,
                    typ,
                    self.max.unwrap()
                )
            }
            _ if self.min.is_some() => {
                format!(
                    "\n    pub const {}_MIN: {} = {};",
                    name,
                    typ,
                    self.min.unwrap()
                )
            }
            _ if self.max.is_some() => {
                format!(
                    "\n    pub const {}_MAX: {} = {};",
                    name,
                    typ,
                    self.max.unwrap()
                )
            }
            _ => "".to_owned(),
        }
    }

    pub fn api_required(&self, name: &str) -> bool {
        (ApiFieldDef::required(name) || self.required || self.not_null) && self.auto.is_none()
    }
    pub fn get_api_validate(&self, name: &str) -> String {
        let mut validators = Vec::new();
        if !self.primary && (ApiFieldDef::required(name) || self.required) && !self.not_null {
            validators.push("required".to_string());
        }
        let custom = ApiFieldDef::validator(name);
        let has_custom = custom.is_some();
        if let Some(validator) = custom {
            validators.push(format!("custom(function = {:?})", validator));
        }
        match self.data_type {
            DataType::Char | DataType::IdVarchar | DataType::TextVarchar => {
                if !has_custom {
                    if self.not_null {
                        validators.push(
                            "custom(function = \"_server::validator::validate_varchar\")"
                                .to_string(),
                        );
                    } else {
                        validators.push(
                            "custom(function = \"_server::validator::validate_varchar_opt\")"
                                .to_string(),
                        );
                    }
                }
                let length = self.length.unwrap_or(DEFAULT_VARCHAR_LENGTH);
                validators.push(format!("length(max = {})", length));
            }
            DataType::Text => {
                if !has_custom {
                    if self.not_null {
                        validators.push(
                            "custom(function = \"_server::validator::validate_text\")".to_string(),
                        );
                    } else {
                        validators.push(
                            "custom(function = \"_server::validator::validate_text_opt\")"
                                .to_string(),
                        );
                    }
                }
                if let Some(length) = self.length {
                    validators.push(format!("length(max = {})", length));
                }
            }
            DataType::ArrayString => {
                if !has_custom {
                    if self.not_null {
                        validators.push(
                            "custom(function = \"_server::validator::validate_array_of_varchar\")"
                                .to_string(),
                        );
                    } else {
                        validators.push(
                            "custom(function = \"_server::validator::validate_array_of_varchar_opt\")"
                                .to_string(),
                        );
                    }
                }
            }
            _ if self.min.is_some() && self.max.is_some() => {
                validators.push(format!(
                    "range(min = {}, max = {})",
                    self.min.unwrap(),
                    self.max.unwrap()
                ));
            }
            _ if self.min.is_some() => {
                validators.push(format!("range(min = {})", self.min.unwrap(),));
            }
            _ if self.max.is_some() => {
                validators.push(format!("range(max = {})", self.max.unwrap(),));
            }
            DataType::Double | DataType::Float if !self.signed => {
                validators.push("range(min = 0.0)".to_string());
            }
            DataType::Decimal if !self.signed => {
                if self.not_null {
                    validators.push(
                        "custom(function = \"_server::validator::validate_unsigned_decimal\")"
                            .to_string(),
                    );
                } else {
                    validators.push(
                        "custom(function = \"_server::validator::validate_unsigned_decimal_opt\")"
                            .to_string(),
                    );
                }
            }
            DataType::Json | DataType::Jsonb | DataType::Geometry
                if self.user_defined_json_type.is_none() =>
            {
                if self.not_null {
                    validators.push(
                        "custom(function = \"_server::validator::validate_json_object\")"
                            .to_string(),
                    );
                } else {
                    validators.push(
                        "custom(function = \"_server::validator::validate_json_object_opt\")"
                            .to_string(),
                    );
                }
            }
            _ => {}
        }
        if !validators.is_empty() {
            format!("    #[validate({})]\n", validators.join(", "))
        } else {
            "".to_owned()
        }
    }
    pub fn graphql_secret(&self) -> &str {
        if self.secret.unwrap_or_default() {
            "    #[graphql(secret)]\n"
        } else {
            ""
        }
    }

    pub fn get_api_default_attribute(&self, name: &str) -> String {
        if ApiFieldDef::default(name).is_some() {
            format!(
                "    #[serde(default = \"default_{}\")]\n    #[graphql(default_with = \"{}\")]\n",
                name,
                self.get_api_default(name)
            )
        } else {
            "".to_string()
        }
    }
    #[allow(clippy::match_like_matches_macro)]
    pub fn is_arc(&self) -> bool {
        match self.data_type {
            DataType::Char | DataType::IdVarchar | DataType::TextVarchar | DataType::Text => true,
            DataType::Binary | DataType::Varbinary | DataType::Blob => true,
            DataType::ArrayInt | DataType::ArrayString => true,
            _ => false,
        }
    }

    pub fn get_inner_type(&self, raw: bool, without_option: bool) -> String {
        if let Some(enum_class) = &self.enum_class
            && !raw
        {
            let typ = enum_class.to_string();
            if without_option || self.not_null {
                return typ;
            } else {
                return format!("Option<{}>", &typ);
            }
        }
        let signed_only: bool = CONFIG.read().unwrap().as_ref().unwrap().signed_only();
        let typ = match self.data_type {
            DataType::TinyInt if self.signed || signed_only => "i8",
            DataType::TinyInt => "u8",
            DataType::SmallInt if self.signed || signed_only => "i16",
            DataType::SmallInt => "u16",
            DataType::Int if self.signed || signed_only => "i32",
            DataType::Int => "u32",
            DataType::BigInt if self.signed || signed_only => "i64",
            DataType::BigInt => "u64",
            DataType::Float => "f32",
            DataType::Double => "f64",
            DataType::Char | DataType::IdVarchar | DataType::TextVarchar if raw => "String",
            DataType::Char | DataType::IdVarchar | DataType::TextVarchar => {
                "std::sync::Arc<String>"
            }
            DataType::Uuid => "uuid::Uuid",
            DataType::BinaryUuid => "uuid::Uuid",
            DataType::Boolean => "bool",
            DataType::Text if raw => "String",
            DataType::Text => "std::sync::Arc<String>",
            DataType::Binary | DataType::Varbinary | DataType::Blob if raw => "Vec<u8>",
            DataType::Binary | DataType::Varbinary | DataType::Blob => "std::sync::Arc<Vec<u8>>",
            DataType::NaiveDateTime => "chrono::NaiveDateTime",
            DataType::UtcDateTime if self.is_utc_output() => {
                "chrono::DateTime<chrono::offset::Utc>"
            }
            DataType::UtcDateTime => "chrono::DateTime<chrono::offset::Local>",
            DataType::TimestampWithTimeZone if self.is_utc_output() => {
                "chrono::DateTime<chrono::offset::Utc>"
            }
            DataType::TimestampWithTimeZone => "chrono::DateTime<chrono::offset::Local>",
            DataType::Date => "chrono::NaiveDate",
            DataType::Time => "chrono::NaiveTime",
            DataType::Decimal => "rust_decimal::Decimal",
            DataType::ArrayInt if raw => "Vec<u64>",
            DataType::ArrayInt => "std::sync::Arc<Vec<u64>>",
            DataType::ArrayString if raw => "Vec<String>",
            DataType::ArrayString => "std::sync::Arc<Vec<String>>",
            DataType::Json | DataType::Jsonb => "crate::misc::JsonRawValue",
            DataType::DbEnum => "",
            DataType::DbSet => "String",
            DataType::Point => "senax_common::types::point::Point",
            DataType::GeoPoint => "senax_common::types::geo_point::GeoPoint",
            DataType::Geometry => "crate::misc::JsonRawValue",
            DataType::ValueObject => unimplemented!(),
            DataType::AutoFk => unimplemented!(),
            DataType::UnSupported => unimplemented!(),
        };
        if without_option {
            return typ.to_owned();
        }
        if self.not_null {
            typ.to_owned()
        } else {
            format!("Option<{}>", typ)
        }
    }

    pub fn get_inner_to_raw(&self) -> &'static str {
        match self.data_type {
            DataType::Char | DataType::IdVarchar | DataType::TextVarchar | DataType::Text => {
                ".to_string()"
            }
            _ => "",
        }
    }

    pub fn get_raw_to_inner(&self) -> &'static str {
        match self.data_type {
            DataType::Char | DataType::IdVarchar | DataType::TextVarchar | DataType::Text => {
                ".into()"
            }
            _ => "",
        }
    }

    pub fn get_deref_type(&self, without_option: bool) -> String {
        let signed_only: bool = CONFIG.read().unwrap().as_ref().unwrap().signed_only();
        let typ = match self.data_type {
            DataType::TinyInt if self.signed || signed_only => "i8",
            DataType::TinyInt => "u8",
            DataType::SmallInt if self.signed || signed_only => "i16",
            DataType::SmallInt => "u16",
            DataType::Int if self.signed || signed_only => "i32",
            DataType::Int => "u32",
            DataType::BigInt if self.signed || signed_only => "i64",
            DataType::BigInt => "u64",
            DataType::Float => "f32",
            DataType::Double => "f64",
            DataType::Char | DataType::IdVarchar | DataType::TextVarchar => "str",
            DataType::Uuid => "uuid::Uuid",
            DataType::BinaryUuid => "uuid::Uuid",
            DataType::Boolean => "bool",
            DataType::Text => "str",
            DataType::Binary | DataType::Varbinary | DataType::Blob => "Vec<u8>",
            DataType::NaiveDateTime => "chrono::NaiveDateTime",
            DataType::UtcDateTime if self.is_utc_output() => {
                "chrono::DateTime<chrono::offset::Utc>"
            }
            DataType::UtcDateTime => "chrono::DateTime<chrono::offset::Local>",
            DataType::TimestampWithTimeZone if self.is_utc_output() => {
                "chrono::DateTime<chrono::offset::Utc>"
            }
            DataType::TimestampWithTimeZone => "chrono::DateTime<chrono::offset::Local>",
            DataType::Date => "chrono::NaiveDate",
            DataType::Time => "chrono::NaiveTime",
            DataType::Decimal => "rust_decimal::Decimal",
            DataType::ArrayInt => "Vec<u64>",
            DataType::ArrayString => "Vec<String>",
            DataType::Json | DataType::Jsonb if self.user_defined_json_type.is_some() => {
                self.user_defined_json_type.as_ref().unwrap()
            }
            DataType::Json | DataType::Jsonb => "serde_json::Value",
            DataType::DbEnum => "str",
            DataType::DbSet => "str",
            DataType::Point => "senax_common::types::point::Point",
            DataType::GeoPoint => "senax_common::types::geo_point::GeoPoint",
            DataType::Geometry if self.user_defined_json_type.is_some() => {
                self.user_defined_json_type.as_ref().unwrap()
            }
            DataType::Geometry => "serde_json::Value",
            DataType::ValueObject => unimplemented!(),
            DataType::AutoFk => unimplemented!(),
            DataType::UnSupported => unimplemented!(),
        };
        if without_option {
            return typ.to_owned();
        }
        if self.not_null {
            typ.to_owned()
        } else {
            format!("Option<{}>", typ)
        }
    }

    pub fn get_may_null(&self) -> &str {
        if_then_else!(self.not_null, "false", "true")
    }
    pub fn get_null_question(&self) -> &str {
        if !self.is_copyable() {
            if_then_else!(self.not_null, "", ".as_ref()?")
        } else {
            if_then_else!(self.not_null, "", "?")
        }
    }

    pub fn get_filter_type(&self, is_domain: bool) -> String {
        if is_domain && self.value_object.is_some() {
            let name = self.value_object.as_ref().unwrap().to_pascal();
            return format!("value_objects::{}", name);
        }
        if let Some(ref class) = self.id_class {
            return class.to_string();
        }
        if let Some(ref class) = self.enum_class {
            return class.to_string();
        }
        if let Some(ref rel) = self.rel {
            let (_rel_name, def) = rel;
            let name = def.get_id_name();
            if domain_mode() {
                let mod_name = def.get_group_mod_var();
                return format!("_model_::{}::{}", mod_name, name);
            } else {
                let mod_name = def.get_group_mod_name();
                return format!("rel_{}::{}", mod_name, name);
            }
        }
        if let Some(ref rel) = self.outer_db_rel {
            let (_rel_name, def) = rel;
            let name = def.get_id_name();
            if domain_mode() {
                let mod_name = def.get_group_mod_var();
                return format!("_{}_model_::{}::{}", def.db().to_snake(), mod_name, name);
            } else {
                let mod_name = def.get_group_mod_name();
                return format!("rel_{}::{}", mod_name, name);
            }
        }
        let signed_only: bool = CONFIG.read().unwrap().as_ref().unwrap().signed_only();
        let type_str = match self.data_type {
            DataType::TinyInt if self.signed || signed_only => "i8",
            DataType::TinyInt => "u8",
            DataType::SmallInt if self.signed || signed_only => "i16",
            DataType::SmallInt => "u16",
            DataType::Int if self.signed || signed_only => "i32",
            DataType::Int => "u32",
            DataType::BigInt if self.signed || signed_only => "i64",
            DataType::BigInt => "u64",
            DataType::Float => "f32",
            DataType::Double => "f64",
            DataType::Char | DataType::IdVarchar | DataType::TextVarchar => "String",
            DataType::Uuid => "uuid::Uuid",
            DataType::BinaryUuid => "uuid::Uuid",
            DataType::Boolean => "bool",
            DataType::Text => "String",
            DataType::Binary | DataType::Varbinary | DataType::Blob => "Vec<u8>",
            DataType::NaiveDateTime => "chrono::NaiveDateTime",
            DataType::UtcDateTime if self.is_utc_output() => {
                "chrono::DateTime<chrono::offset::Utc>"
            }
            DataType::UtcDateTime => "chrono::DateTime<chrono::offset::Local>",
            DataType::TimestampWithTimeZone if self.is_utc_output() => {
                "chrono::DateTime<chrono::offset::Utc>"
            }
            DataType::TimestampWithTimeZone => "chrono::DateTime<chrono::offset::Local>",
            DataType::Date => "chrono::NaiveDate",
            DataType::Time => "chrono::NaiveTime",
            DataType::Decimal => "rust_decimal::Decimal",
            DataType::ArrayInt => "u64",
            DataType::ArrayString => "String",
            DataType::Json | DataType::Jsonb if self.user_defined_json_type.is_some() => {
                self.user_defined_json_type.as_ref().unwrap()
            }
            DataType::Json | DataType::Jsonb => "serde_json::Value",
            DataType::DbEnum => "",
            DataType::DbSet => "String",
            DataType::Point if is_domain => "domain::models::Point",
            DataType::Point => "senax_common::types::point::Point",
            DataType::GeoPoint if is_domain => "domain::models::GeoPoint",
            DataType::GeoPoint => "senax_common::types::geo_point::GeoPoint",
            DataType::Geometry if self.user_defined_json_type.is_some() => {
                self.user_defined_json_type.as_ref().unwrap()
            }
            DataType::Geometry => "serde_json::Value",
            DataType::ValueObject => unimplemented!(),
            DataType::AutoFk => unimplemented!(),
            DataType::UnSupported => unimplemented!(),
        };
        type_str.to_owned()
    }
    pub fn get_filter_null(&self, name: &str) -> String {
        if self.not_null {
            "false".to_string()
        } else {
            format!("_obj.{name}().is_none()")
        }
    }
    pub fn get_filter_eq(&self, index: Option<usize>, _ref: bool) -> String {
        let as_ref = if self.id_class.is_none()
            && self.rel.is_none()
            && self.outer_db_rel.is_none()
            && self.value_object.is_none()
            && self.is_arc()
        {
            ".as_ref()"
        } else {
            ""
        };
        let index = if let Some(index) = index {
            index.to_string()
        } else {
            "".to_string()
        };
        let r = if _ref { "&" } else { "" };
        if self.not_null {
            format!("{as_ref}.eq({r}c{index})")
        } else {
            format!(".map(|v| v{as_ref}.eq({r}c{index})).unwrap_or(false)")
        }
    }
    pub fn get_filter_cmp(&self, index: Option<usize>) -> String {
        let as_ref = if self.id_class.is_none()
            && self.rel.is_none()
            && self.outer_db_rel.is_none()
            && self.value_object.is_none()
            && self.is_arc()
        {
            ".as_ref()"
        } else {
            ""
        };
        let index = if let Some(index) = index {
            index.to_string()
        } else {
            "".to_string()
        };
        let cmp = if self.data_type == DataType::Float || self.data_type == DataType::Double {
            format!("partial_cmp(c{index}).ok_or(false)?")
        } else {
            format!("cmp(c{index})")
        };
        if self.not_null {
            format!("{as_ref}.{cmp}")
        } else {
            format!(".ok_or(false)?{as_ref}.{cmp}")
        }
    }
    pub fn get_filter_like(&self) -> &str {
        if self.not_null {
            ".like(c)"
        } else {
            ".map(|v| v.like(c)).unwrap_or(false)"
        }
    }

    pub fn get_factory_type(&self) -> String {
        let signed_only: bool = CONFIG.read().unwrap().as_ref().unwrap().signed_only();
        let mut typ = match self.data_type {
            DataType::TinyInt if self.signed || signed_only => "i8",
            DataType::TinyInt => "u8",
            DataType::SmallInt if self.signed || signed_only => "i16",
            DataType::SmallInt => "u16",
            DataType::Int if self.signed || signed_only => "i32",
            DataType::Int => "u32",
            DataType::BigInt if self.signed || signed_only => "i64",
            DataType::BigInt => "u64",
            DataType::Float => "f32",
            DataType::Double => "f64",
            DataType::Char | DataType::IdVarchar | DataType::TextVarchar => "String",
            DataType::Uuid => "uuid::Uuid",
            DataType::BinaryUuid => "uuid::Uuid",
            DataType::Boolean => "bool",
            DataType::Text => "String",
            DataType::Binary | DataType::Varbinary | DataType::Blob => {
                "senax_common::types::blob::Blob"
            }
            DataType::NaiveDateTime => "chrono::NaiveDateTime",
            DataType::UtcDateTime if self.is_utc_output() => {
                "chrono::DateTime<chrono::offset::Utc>"
            }
            DataType::UtcDateTime => "chrono::DateTime<chrono::offset::Local>",
            DataType::TimestampWithTimeZone if self.is_utc_output() => {
                "chrono::DateTime<chrono::offset::Utc>"
            }
            DataType::TimestampWithTimeZone => "chrono::DateTime<chrono::offset::Local>",
            DataType::Date => "chrono::NaiveDate",
            DataType::Time => "chrono::NaiveTime",
            DataType::Decimal => "rust_decimal::Decimal",
            DataType::ArrayInt => "Vec<u64>",
            DataType::ArrayString => "Vec<String>",
            DataType::Json | DataType::Jsonb if self.user_defined_json_type.is_some() => {
                self.user_defined_json_type.as_ref().unwrap()
            }
            DataType::Json | DataType::Jsonb => "serde_json::Value",
            DataType::DbEnum => "String",
            DataType::DbSet => "String",
            DataType::Point => "senax_common::types::point::Point",
            DataType::GeoPoint => "senax_common::types::geo_point::GeoPoint",
            DataType::Geometry if self.user_defined_json_type.is_some() => {
                self.user_defined_json_type.as_ref().unwrap()
            }
            DataType::Geometry => "serde_json::Value",
            DataType::ValueObject => unimplemented!(),
            DataType::AutoFk => unimplemented!(),
            DataType::UnSupported => unimplemented!(),
        }
        .to_string();
        if self.auto.is_none() {
            typ = if let Some(ref class) = self.id_class {
                class.to_string()
            } else if let Some(ref class) = self.enum_class {
                class.to_string()
            } else if let Some(ref rel) = self.rel {
                let (_rel_name, def) = rel;
                let name = def.get_id_name();
                let mod_name = def.get_group_mod_name();
                format!("rel_{}::{}", mod_name, name)
            } else if let Some(ref rel) = self.outer_db_rel {
                let (_rel_name, def) = rel;
                let name = def.get_id_name();
                let mod_name = def.get_group_mod_name();
                format!("rel_{}::{}", mod_name, name)
            } else {
                typ
            };
        }
        if self.not_null {
            typ
        } else {
            format!("Option<{}>", typ)
        }
    }

    pub fn get_factory_default(&self) -> &str {
        if self.auto.is_some() {
            "    #[serde(default)]\n"
        } else {
            ""
        }
    }

    pub fn convert_domain_factory(&self) -> &str {
        if self.id_class.is_some()
            || self.enum_class.is_some()
            || self.rel.is_some()
            || self.outer_db_rel.is_some()
            || self.value_object.is_some()
        {
            return "";
        }
        match self.data_type {
            DataType::Char | DataType::IdVarchar | DataType::TextVarchar if !self.not_null => {
                ".map(|v| v.into())"
            }
            DataType::Char | DataType::IdVarchar | DataType::TextVarchar => ".into()",
            DataType::Text if !self.not_null => ".map(|v| v.into())",
            DataType::Text => ".into()",
            DataType::Binary | DataType::Varbinary | DataType::Blob if !self.not_null => {
                ".map(|v| v.into())"
            }
            DataType::Binary | DataType::Varbinary | DataType::Blob => ".into()",
            DataType::ArrayInt if !self.not_null => ".map(|v| v.into())",
            DataType::ArrayInt => ".into()",
            DataType::ArrayString if !self.not_null => ".map(|v| v.into())",
            DataType::ArrayString => ".into()",
            DataType::Json | DataType::Jsonb if !self.not_null => {
                ".map(|v| v.to_raw_value().unwrap())"
            }
            DataType::Json | DataType::Jsonb => ".to_raw_value().unwrap()",
            DataType::Geometry if !self.not_null => ".map(|v| v.to_raw_value().unwrap())",
            DataType::Geometry => ".to_raw_value().unwrap()",
            _ => "",
        }
    }

    pub fn convert_factory_type(&self) -> String {
        let mut id_str = "";
        if self.auto.is_none() {
            if let Some(ref _class) = self.enum_class {
                return "".to_string();
            }
            id_str = if self.id_class.is_some() || self.rel.is_some() || self.outer_db_rel.is_some()
            {
                ".inner()"
            } else {
                ""
            }
        }
        let conv_str = match self.data_type {
            DataType::TinyInt => "",
            DataType::SmallInt => "",
            DataType::Int => "",
            DataType::BigInt => "",
            DataType::Float => "",
            DataType::Double => "",
            DataType::Char | DataType::IdVarchar | DataType::TextVarchar => ".into()",
            DataType::Boolean => "",
            DataType::Text => ".into()",
            DataType::Uuid => "",
            DataType::BinaryUuid => "",
            DataType::Binary | DataType::Varbinary | DataType::Blob if !id_str.is_empty() => "",
            DataType::Binary | DataType::Varbinary | DataType::Blob => ".0.into()",
            DataType::NaiveDateTime if self.not_null => ".into()",
            DataType::NaiveDateTime => "",
            DataType::UtcDateTime if self.not_null => ".into()",
            DataType::UtcDateTime => "",
            DataType::TimestampWithTimeZone if self.not_null => ".into()",
            DataType::TimestampWithTimeZone => "",
            DataType::Date if self.not_null => ".into()",
            DataType::Date => "",
            DataType::Time if self.not_null => ".into()",
            DataType::Time => "",
            DataType::Decimal => "",
            DataType::ArrayInt => ".into()",
            DataType::ArrayString => ".into()",
            DataType::Json | DataType::Jsonb => "._to_json_raw_value().unwrap()",
            DataType::DbEnum => "",
            DataType::DbSet => "",
            DataType::Point => ".into()",
            DataType::GeoPoint => ".into()",
            DataType::Geometry => "._to_json_raw_value().unwrap()",
            DataType::ValueObject => unimplemented!(),
            DataType::AutoFk => unimplemented!(),
            DataType::UnSupported => unimplemented!(),
        };
        if !self.not_null {
            if id_str.is_empty() && conv_str.is_empty() {
                "".to_string()
            } else {
                format!(".map(|v| v{}{})", id_str, conv_str)
            }
        } else {
            format!("{}{}", id_str, conv_str)
        }
    }

    pub fn convert_from_entity(&self) -> String {
        let id_str = if self.enum_class.is_some() {
            ".into()"
        } else if self.id_class.is_some()
            || self.rel.is_some()
            || self.outer_db_rel.is_some()
            || self.value_object.is_some()
        {
            ".inner()"
        } else {
            ""
        };
        let conv_str = match self.data_type {
            DataType::TinyInt => "",
            DataType::SmallInt => "",
            DataType::Int => "",
            DataType::BigInt => "",
            DataType::Float => "",
            DataType::Double => "",
            DataType::Char | DataType::IdVarchar | DataType::TextVarchar => "",
            DataType::Boolean => "",
            DataType::Text => "",
            DataType::Uuid => "",
            DataType::BinaryUuid => "",
            DataType::Binary | DataType::Varbinary | DataType::Blob if !id_str.is_empty() => "",
            DataType::Binary | DataType::Varbinary | DataType::Blob => "",
            DataType::NaiveDateTime => "",
            DataType::UtcDateTime => "",
            DataType::TimestampWithTimeZone => "",
            DataType::Date => "",
            DataType::Time => "",
            DataType::Decimal => "",
            DataType::ArrayInt => "",
            DataType::ArrayString => "",
            DataType::Json | DataType::Jsonb => "._to_json_raw_value().unwrap()",
            DataType::DbEnum => "",
            DataType::DbSet => "",
            DataType::Point => ".to_tuple().to_point()",
            DataType::GeoPoint => ".to_tuple().to_geo_point()",
            DataType::Geometry => "._to_json_raw_value().unwrap()",
            DataType::ValueObject => unimplemented!(),
            DataType::AutoFk => unimplemented!(),
            DataType::UnSupported => unimplemented!(),
        };
        if !self.not_null {
            if id_str.is_empty() && conv_str.is_empty() {
                "".to_string()
            } else {
                format!(".map(|v| v{}{})", id_str, conv_str)
            }
        } else {
            format!("{}{}", id_str, conv_str)
        }
    }

    pub fn get_api_schema_type(&self) -> &'static str {
        match self.data_type {
            DataType::Json | DataType::Jsonb if self.not_null => {
                "    #[schema(value_type = Object)]\n"
            }
            DataType::Json | DataType::Jsonb => "    #[schema(value_type = Option<Object>)]\n",
            DataType::Geometry if self.not_null => "    #[schema(value_type = Object)]\n",
            DataType::Geometry => "    #[schema(value_type = Option<Object>)]\n",
            _ => "",
        }
    }

    pub fn get_api_type(&self, option: bool, req: bool) -> String {
        let typ = self._get_api_type(req);
        let option = option || req && self.is_version;
        if self.not_null && !option {
            typ
        } else if option || !req {
            format!("Option<{}>", typ)
        } else {
            format!("_server::MaybeUndefined<{}>", typ)
        }
    }

    pub fn get_api_schema(&self) -> String {
        let typ = self._get_api_type(true);
        if self.not_null {
            String::new()
        } else {
            format!(
                "    #[schema(value_type = Option<{typ}>)]
    #[schemars(with = \"Option<{typ}>\")]\n"
            )
        }
    }

    fn _get_api_type(&self, req: bool) -> String {
        let signed_only: bool = CONFIG.read().unwrap().as_ref().unwrap().signed_only();
        let typ = match self.data_type {
            DataType::TinyInt if self.signed || signed_only => "i8",
            DataType::TinyInt => "u8",
            DataType::SmallInt if self.signed || signed_only => "i16",
            DataType::SmallInt => "u16",
            DataType::Int if self.signed || signed_only => "i32",
            DataType::Int => "u32",
            DataType::BigInt if self.signed || signed_only => "i64",
            DataType::BigInt => "u64",
            DataType::Float if !req => "f64",
            DataType::Float => "f32",
            DataType::Double => "f64",
            DataType::Char | DataType::IdVarchar | DataType::TextVarchar if req => "String",
            DataType::Char | DataType::IdVarchar | DataType::TextVarchar => {
                "std::sync::Arc<String>"
            }
            DataType::Boolean => "bool",
            DataType::Text if req => "String",
            DataType::Text => "std::sync::Arc<String>",
            DataType::Uuid => "uuid::Uuid",
            DataType::BinaryUuid => "uuid::Uuid",
            DataType::Binary | DataType::Varbinary | DataType::Blob => "String",
            DataType::NaiveDateTime => "chrono::NaiveDateTime",
            DataType::UtcDateTime if self.is_utc_output() => {
                "chrono::DateTime<chrono::offset::Utc>"
            }
            DataType::UtcDateTime => "chrono::DateTime<chrono::offset::Local>",
            DataType::TimestampWithTimeZone if self.is_utc_output() => {
                "chrono::DateTime<chrono::offset::Utc>"
            }
            DataType::TimestampWithTimeZone => "chrono::DateTime<chrono::offset::Local>",
            DataType::Date => "chrono::NaiveDate",
            DataType::Time => "chrono::NaiveTime",
            DataType::Decimal => "rust_decimal::Decimal",
            DataType::ArrayInt if req => "Vec<u64>",
            DataType::ArrayInt => "std::sync::Arc<Vec<u64>>",
            DataType::ArrayString if req => "Vec<String>",
            DataType::ArrayString => "std::sync::Arc<Vec<String>>",
            DataType::Json | DataType::Jsonb if !req => {
                "async_graphql::Json<std::sync::Arc<Box<serde_json::value::RawValue>>>"
            }
            DataType::Json | DataType::Jsonb if self.user_defined_json_type.is_some() => {
                self.user_defined_json_type.as_ref().unwrap()
            }
            DataType::Json | DataType::Jsonb => "serde_json::Value",
            DataType::DbEnum => "",
            DataType::DbSet => "String",
            DataType::Point => "domain::models::Point",
            DataType::GeoPoint => "domain::models::GeoPoint",
            DataType::Geometry if !req => {
                "async_graphql::Json<std::sync::Arc<Box<serde_json::value::RawValue>>>"
            }
            DataType::Geometry if self.user_defined_json_type.is_some() => {
                self.user_defined_json_type.as_ref().unwrap()
            }
            DataType::Geometry => "serde_json::Value",
            DataType::ValueObject => unimplemented!(),
            DataType::AutoFk => unimplemented!(),
            DataType::UnSupported => unimplemented!(),
        };
        let mut typ = typ.to_owned();
        if let Some(ref name) = self.value_object {
            if self.enum_values.is_some() {
                let name = name.to_pascal();
                typ = format!("value_objects::{}", name);
            }
        } else if let Some(ref class) = self.enum_class {
            typ = class.to_string();
        }
        typ
    }

    pub fn get_gql_type(&self) -> String {
        let mut typ = match self.data_type {
            DataType::TinyInt => "Int",
            DataType::SmallInt => "Int",
            DataType::Int => "Int",
            DataType::BigInt => "Int",
            DataType::Float => "Float",
            DataType::Double => "Float",
            DataType::Char | DataType::IdVarchar | DataType::TextVarchar => "String",
            DataType::Boolean => "Boolean",
            DataType::Text => "String",
            DataType::Uuid => "UUID",
            DataType::BinaryUuid => "UUID",
            DataType::Binary | DataType::Varbinary | DataType::Blob => "String",
            DataType::NaiveDateTime => "NaiveDateTime",
            DataType::UtcDateTime => "DateTime",
            DataType::TimestampWithTimeZone => "DateTime",
            DataType::Date => "NaiveDate",
            DataType::Time => "NaiveTime",
            DataType::Decimal => "Decimal",
            DataType::ArrayInt => "[Int!]",
            DataType::ArrayString => "[String!]",
            DataType::Json | DataType::Jsonb => "JSON",
            DataType::DbEnum => "String",
            DataType::DbSet => "String",
            DataType::Point => "domain::models::Point",
            DataType::GeoPoint => "domain::models::GeoPoint",
            DataType::Geometry => "JSON",
            DataType::ValueObject => unimplemented!(),
            DataType::AutoFk => unimplemented!(),
            DataType::UnSupported => unimplemented!(),
        }
        .to_string();
        if let Some(ref name) = self.value_object
            && self.data_type != DataType::Char
            && self.data_type != DataType::IdVarchar
            && self.data_type != DataType::TextVarchar
            && self.data_type != DataType::Text
        {
            let name = name.to_pascal();
            typ = format!("Vo{}", name);
        }
        if self.not_null {
            format!("{}!", typ)
        } else {
            typ
        }
    }

    pub fn get_ts_type(&self) -> &str {
        match self.data_type {
            _ if self.enum_values.is_some() => "string",
            DataType::TinyInt => "number",
            DataType::SmallInt => "number",
            DataType::Int => "number",
            DataType::BigInt => "number",
            DataType::Float => "number",
            DataType::Double => "number",
            DataType::Char | DataType::IdVarchar | DataType::TextVarchar => "string",
            DataType::Boolean => "boolean",
            DataType::Text => "string",
            DataType::Uuid => "string",
            DataType::BinaryUuid => "string",
            DataType::Binary | DataType::Varbinary | DataType::Blob => "",
            DataType::NaiveDateTime => "string",
            DataType::UtcDateTime => "string",
            DataType::TimestampWithTimeZone => "string",
            DataType::Date => "string",
            DataType::Time => "string",
            DataType::Decimal => "string",
            DataType::ArrayInt => "number[]",
            DataType::ArrayString => "string[]",
            DataType::Json | DataType::Jsonb => "any",
            DataType::DbEnum => "string",
            DataType::DbSet => "string",
            DataType::Point => "any",
            DataType::GeoPoint => "any",
            DataType::Geometry => "any",
            DataType::ValueObject => unimplemented!(),
            DataType::AutoFk => unimplemented!(),
            DataType::UnSupported => unimplemented!(),
        }
    }

    pub fn get_to_api_type(&self, req: bool) -> &str {
        if self.enum_class.is_some() {
            return "";
        }
        if self.id_class.is_some() || self.rel.is_some() || self.outer_db_rel.is_some() {
            if !self.not_null {
                return match self.data_type {
                    DataType::Char | DataType::IdVarchar | DataType::TextVarchar => {
                        ".map(|v| v.inner().as_ref().to_owned().into())"
                    }
                    _ => ".map(|v| v.into())",
                };
            } else {
                return match self.data_type {
                    DataType::Char | DataType::IdVarchar | DataType::TextVarchar => {
                        ".inner().as_ref().to_owned().into()"
                    }
                    _ => ".into()",
                };
            }
        }
        if self.value_object.is_some() {
            return if !self.not_null {
                match self.data_type {
                    DataType::Char | DataType::IdVarchar | DataType::TextVarchar => {
                        ".map(|v| v.inner().as_ref().to_owned().into())"
                    }
                    DataType::ArrayString => {
                        ".map(|v| v.inner().iter().map(|v| v.into()).collect())"
                    }
                    _ if !self.is_copyable() => ".map(|v| v.inner().to_owned())",
                    _ => ".map(|v| v.inner())",
                }
            } else {
                match self.data_type {
                    DataType::Char | DataType::IdVarchar | DataType::TextVarchar => {
                        ".inner().as_ref().to_owned().into()"
                    }
                    DataType::ArrayString => ".inner().iter().map(|v| v.into()).collect()",
                    _ if !self.is_copyable() => ".inner().to_owned()",
                    _ => ".inner()",
                }
            };
        }
        match self.data_type {
            DataType::TinyInt => "",
            DataType::SmallInt => "",
            DataType::Int => "",
            DataType::BigInt => "",
            DataType::Float if !self.not_null && !req => {
                ".map(|v| v.to_string().parse()).transpose()?"
            }
            DataType::Float if !req => ".to_string().parse()?",
            DataType::Float => "",
            DataType::Double => "",
            DataType::Char | DataType::IdVarchar | DataType::TextVarchar
                if !self.not_null && req =>
            {
                ".map(|v| v.as_ref().to_owned())"
            }
            DataType::Char | DataType::IdVarchar | DataType::TextVarchar if req => {
                ".as_ref().to_owned()"
            }
            DataType::Char | DataType::IdVarchar | DataType::TextVarchar if !self.not_null => {
                ".cloned()"
            }
            DataType::Char | DataType::IdVarchar | DataType::TextVarchar => ".clone()",
            DataType::Uuid => "",
            DataType::BinaryUuid => "",
            DataType::Boolean => "",
            DataType::Text if !self.not_null && req => ".map(|v| v.as_ref().to_owned())",
            DataType::Text if req => ".as_ref().to_owned()",
            DataType::Text if !self.not_null => ".cloned()",
            DataType::Text => ".clone()",
            DataType::Binary | DataType::Varbinary | DataType::Blob if !self.not_null => {
                ".map(|v| v.to_str())"
            }
            DataType::Binary | DataType::Varbinary | DataType::Blob => ".to_str()",
            DataType::NaiveDateTime => "",
            DataType::UtcDateTime => "",
            DataType::TimestampWithTimeZone => "",
            DataType::Date => "",
            DataType::Time => "",
            DataType::Decimal => "",
            DataType::ArrayInt if !self.not_null && req => ".map(|v| v.as_ref().to_owned())",
            DataType::ArrayInt if req => ".as_ref().to_owned()",
            DataType::ArrayInt if !self.not_null => ".cloned()",
            DataType::ArrayInt => ".clone()",
            DataType::ArrayString if !self.not_null && req => ".map(|v| v.as_ref().to_owned())",
            DataType::ArrayString if req => ".as_ref().to_owned()",
            DataType::ArrayString if !self.not_null => ".cloned()",
            DataType::ArrayString => ".clone()",
            DataType::Json | DataType::Jsonb if req && !self.not_null => {
                ".map(|v| v.from_raw_value().unwrap())"
            }
            DataType::Json | DataType::Jsonb if req => ".from_raw_value().unwrap()",
            DataType::Json | DataType::Jsonb if !self.not_null => ".map(|v| v.into())",
            DataType::Json | DataType::Jsonb => ".into()",
            DataType::DbEnum if !self.not_null => ".map(|v| v.to_string())",
            DataType::DbEnum => ".to_string()",
            DataType::DbSet if !self.not_null => ".map(|v| v.to_string())",
            DataType::DbSet => ".to_string()",
            DataType::Point => "",
            DataType::GeoPoint => "",
            DataType::Geometry if req && !self.not_null => ".map(|v| v.from_raw_value().unwrap())",
            DataType::Geometry if req => ".from_raw_value().unwrap()",
            DataType::Geometry if !self.not_null => ".map(|v| v.into())",
            DataType::Geometry => ".into()",
            DataType::ValueObject => unimplemented!(),
            DataType::AutoFk => unimplemented!(),
            DataType::UnSupported => unimplemented!(),
        }
    }

    pub fn ignore_request(&self, name: &str) -> bool {
        !ApiFieldDef::check(name, true) || self.is_cascade_on_delete()
    }

    pub fn get_from_api_type(
        &self,
        name: &str,
        rel: bool,
        foreign: &[String],
        for_update: bool,
    ) -> String {
        if for_update {
            if let Some(value) = ApiFieldDef::on_update_formula(name) {
                return value;
            }
        } else if let Some(value) = ApiFieldDef::on_insert_formula(name) {
            return value;
        }
        let var = super::_to_var_name(name);
        if self.auto.is_some() {
            if rel {
                return format!("input.{var}.unwrap_or_default()");
            } else {
                return "0".to_owned();
            }
        }
        if (rel && foreign.iter().any(|e| e == name)) || self.ignore_request(name) {
            if !self.not_null {
                return "None".to_owned();
            }
            match self.data_type {
                DataType::TinyInt | DataType::SmallInt | DataType::Int | DataType::BigInt => {
                    return "0.into()".to_owned();
                }
                DataType::Char | DataType::IdVarchar | DataType::TextVarchar | DataType::Text => {
                    return "\"\".to_string().into()".to_owned();
                }
                _ => {
                    return "Default::default()".to_owned();
                }
            }
        }
        if self.enum_class.is_some() {
            if !self.not_null {
                return format!("input.{var}.take()");
            } else {
                return format!("input.{var}");
            }
        }
        if self.id_class.is_some() || self.rel.is_some() || self.outer_db_rel.is_some() {
            if !self.not_null {
                return format!("input.{var}.take().map(|v| v.into())");
            } else {
                return format!("input.{var}.into()");
            }
        }
        if self.value_object.is_some() {
            if !self.not_null {
                return format!("input.{var}.take().map(|v| v.into())");
            } else {
                return format!("input.{var}.into()");
            }
        }
        match self.data_type {
            DataType::Binary | DataType::Varbinary | DataType::Blob if !self.not_null => {
                format!("input.{var}.take().map(|v| v.to_vec())")
            }
            DataType::Binary | DataType::Varbinary | DataType::Blob => {
                format!("input.{var}.to_vec()")
            }
            _ if !self.not_null => format!("input.{var}.take()"),
            _ => format!("input.{var}"),
        }
    }

    pub fn get_outer_type(&self, is_domain: bool) -> String {
        let typ = if is_domain && self.value_object.is_some() {
            let name = self.value_object.as_ref().unwrap().to_pascal();
            format!("value_objects::{}", name)
        } else if let Some(ref class) = self.id_class {
            class.to_string()
        } else if let Some(ref class) = self.enum_class {
            class.to_string()
        } else if let Some(ref rel) = self.rel {
            let (_rel_name, def) = rel;
            let name = def.get_id_name();
            if domain_mode() {
                let mod_name = def.get_group_mod_var();
                format!("_model_::{}::{}", mod_name, name)
            } else {
                let mod_name = def.get_group_mod_name();
                format!("rel_{}::{}", mod_name, name)
            }
        } else if let Some(ref rel) = self.outer_db_rel {
            let (_rel_name, def) = rel;
            let name = def.get_id_name();
            if domain_mode() {
                let mod_name = def.get_group_mod_var();
                format!("_{}_model_::{}::{}", def.db().to_snake(), mod_name, name)
            } else {
                let mod_name = def.get_group_mod_name();
                format!("rel_{}::{}", mod_name, name)
            }
        } else {
            let signed_only: bool = CONFIG.read().unwrap().as_ref().unwrap().signed_only();
            let typ = match self.data_type {
                DataType::TinyInt if self.signed || signed_only => "i8",
                DataType::TinyInt => "u8",
                DataType::SmallInt if self.signed || signed_only => "i16",
                DataType::SmallInt => "u16",
                DataType::Int if self.signed || signed_only => "i32",
                DataType::Int => "u32",
                DataType::BigInt if self.signed || signed_only => "i64",
                DataType::BigInt => "u64",
                DataType::Float => "f32",
                DataType::Double => "f64",
                DataType::Char | DataType::IdVarchar | DataType::TextVarchar => {
                    "&std::sync::Arc<String>"
                }
                DataType::Boolean => "bool",
                DataType::Text => "&std::sync::Arc<String>",
                DataType::Uuid => "uuid::Uuid",
                DataType::BinaryUuid => "uuid::Uuid",
                DataType::Binary | DataType::Varbinary | DataType::Blob => {
                    "&std::sync::Arc<Vec<u8>>"
                }
                DataType::NaiveDateTime => "chrono::NaiveDateTime",
                DataType::UtcDateTime if self.is_utc_output() => {
                    "chrono::DateTime<chrono::offset::Utc>"
                }
                DataType::UtcDateTime => "chrono::DateTime<chrono::offset::Local>",
                DataType::TimestampWithTimeZone if self.is_utc_output() => {
                    "chrono::DateTime<chrono::offset::Utc>"
                }
                DataType::TimestampWithTimeZone => "chrono::DateTime<chrono::offset::Local>",
                DataType::Date => "chrono::NaiveDate",
                DataType::Time => "chrono::NaiveTime",
                DataType::Decimal => "rust_decimal::Decimal",
                DataType::ArrayInt => "&std::sync::Arc<Vec<u64>>",
                DataType::ArrayString => "&std::sync::Arc<Vec<String>>",
                // DataType::Json | DataType::Jsonb if self.user_defined_json_type.is_some() => self.user_defined_json_type.as_ref().unwrap(),
                DataType::Json | DataType::Jsonb => {
                    "std::sync::Arc<Box<serde_json::value::RawValue>>"
                }
                DataType::DbEnum => "",
                DataType::DbSet => "&str",
                DataType::Point if is_domain => "domain::models::Point",
                DataType::Point => "senax_common::types::point::Point",
                DataType::GeoPoint if is_domain => "domain::models::GeoPoint",
                DataType::GeoPoint => "senax_common::types::geo_point::GeoPoint",
                // DataType::Geometry if self.user_defined_json_type.is_some() => {
                //     self.user_defined_json_type.as_ref().unwrap()
                // }
                DataType::Geometry => "std::sync::Arc<Box<serde_json::value::RawValue>>",
                DataType::ValueObject => unimplemented!(),
                DataType::AutoFk => unimplemented!(),
                DataType::UnSupported => unimplemented!(),
            };
            typ.to_owned()
        };
        if self.not_null {
            typ
        } else {
            format!("Option<{}>", typ)
        }
    }
    pub fn get_outer_ref_type(&self) -> String {
        if let Some(ref class) = self.id_class {
            return format!("&{}", class);
        }
        if let Some(ref class) = self.enum_class {
            return format!("&{}", class);
        }
        if let Some(ref rel) = self.rel {
            let (_rel_name, def) = rel;
            let name = def.get_id_name();
            if domain_mode() {
                let mod_name = def.get_group_mod_var();
                return format!("&_model_::{}::{}", mod_name, name);
            } else {
                let mod_name = def.get_group_mod_name();
                return format!("&rel_{}::{}", mod_name, name);
            }
        }
        if let Some(ref rel) = self.outer_db_rel {
            let (_rel_name, def) = rel;
            let name = def.get_id_name();
            if domain_mode() {
                let mod_name = def.get_group_mod_var();
                return format!("&_{}_model_::{}::{}", def.db().to_snake(), mod_name, name);
            } else {
                let mod_name = def.get_group_mod_name();
                return format!("&rel_{}::{}", mod_name, name);
            }
        }
        let user_defined_json_type = self
            .user_defined_json_type
            .as_ref()
            .map(|v| format!("&{}", v));
        let signed_only: bool = CONFIG.read().unwrap().as_ref().unwrap().signed_only();
        let typ = match self.data_type {
            DataType::TinyInt if self.signed || signed_only => "i8",
            DataType::TinyInt => "u8",
            DataType::SmallInt if self.signed || signed_only => "i16",
            DataType::SmallInt => "u16",
            DataType::Int if self.signed || signed_only => "i32",
            DataType::Int => "u32",
            DataType::BigInt if self.signed || signed_only => "i64",
            DataType::BigInt => "u64",
            DataType::Float => "f32",
            DataType::Double => "f64",
            DataType::Char | DataType::IdVarchar | DataType::TextVarchar => "&str",
            DataType::Boolean => "bool",
            DataType::Text => "&str",
            DataType::Uuid => "uuid::Uuid",
            DataType::BinaryUuid => "uuid::Uuid",
            DataType::Binary | DataType::Varbinary | DataType::Blob => "&[u8]",
            DataType::NaiveDateTime => "chrono::NaiveDateTime",
            DataType::UtcDateTime if self.is_utc_output() => {
                "chrono::DateTime<chrono::offset::Utc>"
            }
            DataType::UtcDateTime => "chrono::DateTime<chrono::offset::Local>",
            DataType::TimestampWithTimeZone if self.is_utc_output() => {
                "chrono::DateTime<chrono::offset::Utc>"
            }
            DataType::TimestampWithTimeZone => "chrono::DateTime<chrono::offset::Local>",
            DataType::Date => "chrono::NaiveDate",
            DataType::Time => "chrono::NaiveTime",
            DataType::Decimal => "rust_decimal::Decimal",
            DataType::ArrayInt => "&Vec<u64>",
            DataType::ArrayString => "&Vec<String>",
            DataType::Json | DataType::Jsonb if user_defined_json_type.is_some() => {
                user_defined_json_type.as_ref().unwrap()
            }
            DataType::Json | DataType::Jsonb => "&serde_json::Value",
            DataType::DbEnum => "&str",
            DataType::DbSet => "&str",
            DataType::Point => "senax_common::types::point::Point",
            DataType::GeoPoint => "senax_common::types::geo_point::GeoPoint",
            DataType::Geometry if user_defined_json_type.is_some() => {
                user_defined_json_type.as_ref().unwrap()
            }
            DataType::Geometry => "&serde_json::Value",
            DataType::ValueObject => unimplemented!(),
            DataType::AutoFk => unimplemented!(),
            DataType::UnSupported => unimplemented!(),
        };
        if self.not_null {
            typ.to_owned()
        } else {
            format!("Option<{}>", typ)
        }
    }
    pub fn get_outer_owned_type(&self, is_domain: bool, factory: bool) -> String {
        let typ = if is_domain && self.value_object.is_some() {
            let name = self.value_object.as_ref().unwrap().to_pascal();
            format!("value_objects::{}", name)
        } else if let Some(ref class) = self.id_class {
            class.to_string()
        } else if let Some(ref class) = self.enum_class {
            class.to_string()
        } else if let Some(ref rel) = self.rel {
            let (_rel_name, def) = rel;
            let name = def.get_id_name();
            if domain_mode() {
                let mod_name = def.get_group_mod_var();
                format!("_model_::{}::{}", mod_name, name)
            } else {
                let mod_name = def.get_group_mod_name();
                format!("rel_{}::{}", mod_name, name)
            }
        } else if let Some(ref rel) = self.outer_db_rel {
            let (_rel_name, def) = rel;
            let name = def.get_id_name();
            if domain_mode() {
                let mod_name = def.get_group_mod_var();
                format!("_{}_model_::{}::{}", def.db().to_snake(), mod_name, name)
            } else {
                let mod_name = def.get_group_mod_name();
                format!("rel_{}::{}", mod_name, name)
            }
        } else {
            let signed_only: bool = CONFIG.read().unwrap().as_ref().unwrap().signed_only();
            let typ = match self.data_type {
                DataType::TinyInt if self.signed || signed_only => "i8",
                DataType::TinyInt => "u8",
                DataType::SmallInt if self.signed || signed_only => "i16",
                DataType::SmallInt => "u16",
                DataType::Int if self.signed || signed_only => "i32",
                DataType::Int => "u32",
                DataType::BigInt if self.signed || signed_only => "i64",
                DataType::BigInt => "u64",
                DataType::Float => "f32",
                DataType::Double => "f64",
                DataType::Char | DataType::IdVarchar | DataType::TextVarchar if factory => "String",
                DataType::Char | DataType::IdVarchar | DataType::TextVarchar => {
                    "std::sync::Arc<String>"
                }
                DataType::Boolean => "bool",
                DataType::Text if factory => "String",
                DataType::Text => "std::sync::Arc<String>",
                DataType::Uuid => "uuid::Uuid",
                DataType::BinaryUuid => "uuid::Uuid",
                DataType::Binary | DataType::Varbinary | DataType::Blob if factory => "Vec<u8>",
                DataType::Binary | DataType::Varbinary | DataType::Blob => {
                    "std::sync::Arc<Vec<u8>>"
                }
                DataType::NaiveDateTime => "chrono::NaiveDateTime",
                DataType::UtcDateTime if self.is_utc_output() => {
                    "chrono::DateTime<chrono::offset::Utc>"
                }
                DataType::UtcDateTime => "chrono::DateTime<chrono::offset::Local>",
                DataType::TimestampWithTimeZone if self.is_utc_output() => {
                    "chrono::DateTime<chrono::offset::Utc>"
                }
                DataType::TimestampWithTimeZone => "chrono::DateTime<chrono::offset::Local>",
                DataType::Date => "chrono::NaiveDate",
                DataType::Time => "chrono::NaiveTime",
                DataType::Decimal => "rust_decimal::Decimal",
                DataType::ArrayInt if factory => "Vec<u64>",
                DataType::ArrayInt => "std::sync::Arc<Vec<u64>>",
                DataType::ArrayString if factory => "Vec<String>",
                DataType::ArrayString => "std::sync::Arc<Vec<String>>",
                DataType::Json | DataType::Jsonb if !factory => {
                    "std::sync::Arc<Box<serde_json::value::RawValue>>"
                }
                DataType::Json | DataType::Jsonb if self.user_defined_json_type.is_some() => {
                    self.user_defined_json_type.as_ref().unwrap()
                }
                DataType::Json | DataType::Jsonb => "serde_json::Value",
                DataType::DbEnum => "String",
                DataType::DbSet => "String",
                DataType::Point if is_domain => "domain::models::Point",
                DataType::Point => "senax_common::types::point::Point",
                DataType::GeoPoint if is_domain => "domain::models::GeoPoint",
                DataType::GeoPoint => "senax_common::types::geo_point::GeoPoint",
                DataType::Geometry if !factory => {
                    "std::sync::Arc<Box<serde_json::value::RawValue>>"
                }
                DataType::Geometry if self.user_defined_json_type.is_some() => {
                    self.user_defined_json_type.as_ref().unwrap()
                }
                DataType::Geometry => "serde_json::Value",
                DataType::ValueObject => unimplemented!(),
                DataType::AutoFk => unimplemented!(),
                DataType::UnSupported => unimplemented!(),
            };
            typ.to_owned()
        };
        if self.not_null {
            typ
        } else {
            format!("Option<{}>", typ)
        }
    }
    pub fn convert_outer_prefix(&self) -> &'static str {
        match self.data_type {
            _ if self.id_class.is_some()
                || self.enum_class.is_some()
                || self.rel.is_some()
                || self.outer_db_rel.is_some() =>
            {
                ""
            }
            _ if !self.not_null => "",
            DataType::Char | DataType::IdVarchar | DataType::TextVarchar | DataType::Text => "&",
            DataType::Binary | DataType::Varbinary | DataType::Blob => "&",
            DataType::ArrayInt => "&",
            DataType::ArrayString => "&",
            _ => "",
        }
    }
    pub fn convert_outer_type(&self) -> &'static str {
        if self.id_class.is_some()
            || self.enum_class.is_some()
            || self.rel.is_some()
            || self.outer_db_rel.is_some()
        {
            return if self.not_null {
                ".into()"
            } else {
                ".map(|v| v.into())"
            };
        }
        match self.data_type {
            DataType::TinyInt => "",
            DataType::SmallInt => "",
            DataType::Int => "",
            DataType::BigInt => "",
            DataType::Float => "",
            DataType::Double => "",
            DataType::Char | DataType::IdVarchar | DataType::TextVarchar if !self.not_null => {
                ".as_ref()"
            }
            DataType::Char | DataType::IdVarchar | DataType::TextVarchar => "",
            DataType::Boolean => "",
            DataType::Text if !self.not_null => ".as_ref()",
            DataType::Text => "",
            DataType::Uuid => "",
            DataType::BinaryUuid => "",
            DataType::Binary | DataType::Varbinary | DataType::Blob if !self.not_null => {
                ".as_ref()"
            }
            DataType::Binary | DataType::Varbinary | DataType::Blob => "",
            DataType::NaiveDateTime => "",
            DataType::UtcDateTime => "",
            DataType::TimestampWithTimeZone => "",
            DataType::Date => "",
            DataType::Time => "",
            DataType::Decimal => "",
            DataType::ArrayInt if !self.not_null => ".as_ref()",
            DataType::ArrayInt => "",
            DataType::ArrayString if !self.not_null => ".as_ref()",
            DataType::ArrayString => "",
            DataType::Json | DataType::Jsonb if !self.not_null => {
                ".as_ref().and_then(|v| Some(v.to_raw_value()))"
            }
            DataType::Json | DataType::Jsonb => ".to_raw_value()",
            DataType::DbEnum => "",
            DataType::DbSet if !self.not_null => ".as_deref()",
            DataType::DbSet => ".as_ref()",
            DataType::Point => "",
            DataType::GeoPoint => "",
            DataType::Geometry if !self.not_null => {
                ".as_ref().and_then(|v| Some(v.to_raw_value()))"
            }
            DataType::Geometry => ".to_raw_value()",
            DataType::ValueObject => unimplemented!(),
            DataType::AutoFk => unimplemented!(),
            DataType::UnSupported => unimplemented!(),
        }
    }
    pub fn convert_domain_outer_prefix(&self) -> &'static str {
        match self.data_type {
            _ if self.enum_class.is_some() => "",
            _ if self.id_class.is_some() || self.rel.is_some() || self.outer_db_rel.is_some() => "",
            _ if self.value_object.is_some() => "",
            _ if !self.not_null => "",
            DataType::Char | DataType::IdVarchar | DataType::TextVarchar | DataType::Text => "&",
            DataType::Binary | DataType::Varbinary | DataType::Blob => "&",
            DataType::ArrayInt => "&",
            DataType::ArrayString => "&",
            _ => "",
        }
    }
    pub fn convert_domain_outer_type(&self, is_impl: bool, inner: bool) -> &'static str {
        if self.enum_class.is_some() {
            if !is_impl {
                return "";
            } else {
                return if self.not_null {
                    ".into()"
                } else {
                    ".map(|v| v.into())"
                };
            }
        }
        if inner && (self.id_class.is_some() || self.rel.is_some() || self.outer_db_rel.is_some()) {
            return if self.not_null {
                ".inner()"
            } else {
                ".map(|v| v.inner())"
            };
        }
        if self.id_class.is_some() || self.rel.is_some() || self.outer_db_rel.is_some() {
            return if self.not_null {
                ".inner().into()"
            } else {
                ".map(|v| v.inner().into())"
            };
        }
        if self.value_object.is_some() {
            if !is_impl {
                if self.is_copyable() {
                    return "";
                } else {
                    return ".clone()";
                }
            } else {
                return match self.data_type {
                    DataType::Char
                    | DataType::IdVarchar
                    | DataType::TextVarchar
                    | DataType::Text
                        if !self.not_null =>
                    {
                        ".map(|v| v.clone().into())"
                    }
                    DataType::Char
                    | DataType::IdVarchar
                    | DataType::TextVarchar
                    | DataType::Text => ".clone().into()",
                    _ if !self.not_null => ".map(|v| v.into())",
                    _ => ".into()",
                };
            }
        }
        match self.data_type {
            DataType::TinyInt => "",
            DataType::SmallInt => "",
            DataType::Int => "",
            DataType::BigInt => "",
            DataType::Float => "",
            DataType::Double => "",
            DataType::Char | DataType::IdVarchar | DataType::TextVarchar if is_impl => "",
            DataType::Char | DataType::IdVarchar | DataType::TextVarchar if !self.not_null => {
                ".as_ref()"
            }
            DataType::Char | DataType::IdVarchar | DataType::TextVarchar => "",
            DataType::Boolean => "",
            DataType::Text if is_impl => "",
            DataType::Text if !self.not_null => ".as_ref()",
            DataType::Text => "",
            DataType::Uuid => "",
            DataType::BinaryUuid => "",
            DataType::Binary | DataType::Varbinary | DataType::Blob if is_impl => "",
            DataType::Binary | DataType::Varbinary | DataType::Blob if !self.not_null => {
                ".as_ref()"
            }
            DataType::Binary | DataType::Varbinary | DataType::Blob => "",
            DataType::NaiveDateTime => "",
            DataType::UtcDateTime => "",
            DataType::TimestampWithTimeZone => "",
            DataType::Date => "",
            DataType::Time => "",
            DataType::Decimal => "",
            DataType::ArrayInt if is_impl => "",
            DataType::ArrayInt if !self.not_null => ".as_ref()",
            DataType::ArrayInt => "",
            DataType::ArrayString if is_impl => "",
            DataType::ArrayString if !self.not_null => ".as_ref()",
            DataType::ArrayString => "",
            DataType::Json | DataType::Jsonb if !is_impl && !self.not_null => ".as_ref().cloned()",
            DataType::Json | DataType::Jsonb if !is_impl => ".clone()",
            DataType::Json | DataType::Jsonb => "",
            DataType::DbEnum if is_impl => "",
            DataType::DbEnum => "",
            DataType::DbSet if is_impl => "",
            DataType::DbSet if !self.not_null => ".as_deref()",
            DataType::DbSet => ".as_ref()",
            DataType::Point if self.not_null => ".to_tuple().point()",
            DataType::Point => ".as_ref().map(|v| v.to_tuple().point())",
            DataType::GeoPoint if self.not_null => ".to_tuple().geo_point()",
            DataType::GeoPoint => ".as_ref().map(|v| v.to_tuple().geo_point())",
            DataType::Geometry if !is_impl && !self.not_null => ".as_ref().cloned()",
            DataType::Geometry if !is_impl => ".clone()",
            DataType::Geometry => "",
            DataType::ValueObject => unimplemented!(),
            DataType::AutoFk => unimplemented!(),
            DataType::UnSupported => unimplemented!(),
        }
    }
    pub fn convert_domain_inner_type(&self) -> &'static str {
        if self.enum_class.is_some() {
            return if self.not_null {
                ".into()"
            } else {
                ".map(|v| v.into())"
            };
        }
        if self.id_class.is_some() || self.rel.is_some() || self.outer_db_rel.is_some() {
            return if self.not_null {
                ".inner().into()"
            } else {
                ".map(|v| v.inner().into())"
            };
        }
        if self.value_object.is_some() {
            return if self.not_null {
                ".inner()"
            } else {
                ".map(|v| v.inner())"
            };
        }
        match self.data_type {
            DataType::TinyInt => "",
            DataType::SmallInt => "",
            DataType::Int => "",
            DataType::BigInt => "",
            DataType::Float => "",
            DataType::Double => "",
            DataType::Char | DataType::IdVarchar | DataType::TextVarchar if !self.not_null => "",
            DataType::Char | DataType::IdVarchar | DataType::TextVarchar => "",
            DataType::Boolean => "",
            DataType::Text if !self.not_null => "",
            DataType::Text => "",
            DataType::Uuid => "",
            DataType::BinaryUuid => "",
            DataType::Binary | DataType::Varbinary | DataType::Blob if !self.not_null => "",
            DataType::Binary | DataType::Varbinary | DataType::Blob => "",
            DataType::NaiveDateTime => "",
            DataType::UtcDateTime => "",
            DataType::TimestampWithTimeZone => "",
            DataType::Date => "",
            DataType::Time => "",
            DataType::Decimal => "",
            DataType::ArrayInt if !self.not_null => "",
            DataType::ArrayInt => "",
            DataType::ArrayString if !self.not_null => "",
            DataType::ArrayString => "",
            DataType::Json | DataType::Jsonb if !self.not_null => "",
            DataType::Json | DataType::Jsonb => "",
            DataType::DbEnum if !self.not_null => "",
            DataType::DbEnum => "",
            DataType::DbSet if !self.not_null => "",
            DataType::DbSet => "",
            DataType::Point if self.not_null => ".to_tuple().to_point()",
            DataType::Point => ".map(|v| v.to_tuple().to_point())",
            DataType::GeoPoint if self.not_null => ".to_tuple().to_geo_point()",
            DataType::GeoPoint => ".map(|v| v.to_tuple().to_geo_point())",
            DataType::Geometry => "",
            DataType::ValueObject => unimplemented!(),
            DataType::AutoFk => unimplemented!(),
            DataType::UnSupported => unimplemented!(),
        }
    }
    pub fn convert_impl_domain_outer_for_updater(&self, name: &str) -> String {
        let var = format!("self._data.{}", _to_var_name(name));
        let clone = self.clone_for_outer_str();
        let var_clone = format!("{var}{clone}");
        if self.enum_class.is_some() {
            return if self.not_null {
                format!("{var}{clone}.into()")
            } else {
                format!("{var}{clone}.map(|v| v.into())")
            };
        }
        if self.id_class.is_some() || self.rel.is_some() || self.outer_db_rel.is_some() {
            return match self.data_type {
                DataType::Char | DataType::IdVarchar | DataType::TextVarchar | DataType::Text
                    if !self.not_null =>
                {
                    format!("{var}.as_ref().map(|v| v.clone().into())")
                }
                DataType::Char | DataType::IdVarchar | DataType::TextVarchar | DataType::Text => {
                    format!("{var}.clone().into()")
                }
                _ if !self.not_null => format!("{var}{clone}.map(|v| v.into())"),
                _ => format!("{var}{clone}.into()"),
            };
        }
        if self.value_object.is_some() {
            return match self.data_type {
                DataType::Char | DataType::IdVarchar | DataType::TextVarchar | DataType::Text
                    if !self.not_null =>
                {
                    format!("{var}.as_ref().map(|v| v.clone().into())")
                }
                DataType::Char | DataType::IdVarchar | DataType::TextVarchar | DataType::Text => {
                    format!("{var}.clone().into()")
                }
                _ if !self.not_null => format!("{var}{clone}.map(|v| v.into())"),
                _ => format!("{var}{clone}.into()"),
            };
        }
        match self.data_type {
            DataType::TinyInt => var_clone,
            DataType::SmallInt => var_clone,
            DataType::Int => var_clone,
            DataType::BigInt => var_clone,
            DataType::Float => var_clone,
            DataType::Double => var_clone,
            DataType::Char | DataType::IdVarchar | DataType::TextVarchar if !self.not_null => {
                format!("{var}{clone}.as_ref()")
            }
            DataType::Char | DataType::IdVarchar | DataType::TextVarchar => {
                format!("&{var}{clone}")
            }
            DataType::Boolean => var_clone,
            DataType::Text if !self.not_null => format!("{var}{clone}.as_ref()"),
            DataType::Text => format!("&{var}{clone}"),
            DataType::Uuid => var_clone,
            DataType::BinaryUuid => var_clone,
            DataType::Binary | DataType::Varbinary | DataType::Blob if !self.not_null => {
                format!("{var}{clone}.as_ref()")
            }
            DataType::Binary | DataType::Varbinary | DataType::Blob => format!("&{var}{clone}"),
            DataType::NaiveDateTime => var_clone,
            DataType::UtcDateTime => var_clone,
            DataType::TimestampWithTimeZone => var_clone,
            DataType::Date => var_clone,
            DataType::Time => var_clone,
            DataType::Decimal => var_clone,
            DataType::ArrayInt if !self.not_null => {
                format!("{var}{clone}.as_ref()")
            }
            DataType::ArrayInt => format!("&{var}{clone}"),
            DataType::ArrayString if !self.not_null => {
                format!("{var}{clone}.as_ref()")
            }
            DataType::ArrayString => format!("&{var}{clone}"),
            DataType::Json | DataType::Jsonb if !self.not_null => {
                format!("{var}.as_ref().and_then(|v| Some(v.to_raw_value()))")
            }
            DataType::Json | DataType::Jsonb => format!("{var}.to_raw_value()"),
            DataType::DbEnum => unimplemented!(),
            DataType::DbSet if !self.not_null => format!("{var}{clone}.as_deref()"),
            DataType::DbSet => format!("{var}{clone}.as_ref()"),
            DataType::Point if self.not_null => format!("{var}{clone}.to_tuple().point()"),
            DataType::Point => format!("{var}{clone}.as_ref().map(|v| v.to_tuple().point())"),
            DataType::GeoPoint if self.not_null => {
                format!("{var}{clone}.to_tuple().geo_point()")
            }
            DataType::GeoPoint => {
                format!("{var}{clone}.as_ref().map(|v| v.to_tuple().geo_point())")
            }
            DataType::Geometry if !self.not_null => {
                format!("{var}.as_ref().and_then(|v| Some(v.to_raw_value()))")
            }
            DataType::Geometry => format!("{var}.to_raw_value()"),
            DataType::ValueObject => unimplemented!(),
            DataType::AutoFk => unimplemented!(),
            DataType::UnSupported => unimplemented!(),
        }
    }
    pub fn convert_serialize(&self) -> &'static str {
        match self.data_type {
            DataType::TinyInt => "",
            DataType::SmallInt => "",
            DataType::Int => "",
            DataType::BigInt => "",
            DataType::Float => "",
            DataType::Double => "",
            DataType::Char | DataType::IdVarchar | DataType::TextVarchar => "",
            DataType::Boolean => "",
            DataType::Text => "",
            DataType::Uuid => "",
            DataType::BinaryUuid => "",
            DataType::Binary | DataType::Varbinary | DataType::Blob => "",
            DataType::NaiveDateTime => "",
            DataType::UtcDateTime => "",
            DataType::TimestampWithTimeZone => "",
            DataType::Date => "",
            DataType::Time => "",
            DataType::Decimal => "",
            DataType::ArrayInt => "",
            DataType::ArrayString => "",
            DataType::Json | DataType::Jsonb => "",
            DataType::DbEnum => "",
            DataType::DbSet => "",
            DataType::Point => "",
            DataType::GeoPoint => "",
            DataType::Geometry => "",
            DataType::ValueObject => unimplemented!(),
            DataType::AutoFk => unimplemented!(),
            DataType::UnSupported => unimplemented!(),
        }
    }
    pub fn get_outer_for_updater_type(&self, arc: bool) -> String {
        if let Some(ref class) = self.id_class {
            class.to_string()
        } else if let Some(ref class) = self.enum_class {
            class.to_string()
        } else if let Some(ref rel) = self.rel {
            let (_rel_name, def) = rel;
            let name = def.get_id_name();
            let mod_name = def.get_group_mod_name();
            format!("rel_{}::{}", mod_name, name)
        } else if let Some(ref rel) = self.outer_db_rel {
            let (_rel_name, def) = rel;
            let name = def.get_id_name();
            let mod_name = def.get_group_mod_name();
            format!("rel_{}::{}", mod_name, name)
        } else {
            let signed_only: bool = CONFIG.read().unwrap().as_ref().unwrap().signed_only();
            match self.data_type {
                DataType::TinyInt if self.signed || signed_only => "i8",
                DataType::TinyInt => "u8",
                DataType::SmallInt if self.signed || signed_only => "i16",
                DataType::SmallInt => "u16",
                DataType::Int if self.signed || signed_only => "i32",
                DataType::Int => "u32",
                DataType::BigInt if self.signed || signed_only => "i64",
                DataType::BigInt => "u64",
                DataType::Float => "f32",
                DataType::Double => "f64",
                DataType::Char | DataType::IdVarchar | DataType::TextVarchar if arc => {
                    "std::sync::Arc<String>"
                }
                DataType::Char | DataType::IdVarchar | DataType::TextVarchar => "String",
                DataType::Boolean => "bool",
                DataType::Text if arc => "std::sync::Arc<String>",
                DataType::Text => "String",
                DataType::Uuid => "uuid::Uuid",
                DataType::BinaryUuid => "uuid::Uuid",
                DataType::Binary | DataType::Varbinary | DataType::Blob if arc => {
                    "std::sync::Arc<Vec<u8>>"
                }
                DataType::Binary | DataType::Varbinary | DataType::Blob => "Vec<u8>",
                DataType::NaiveDateTime => "chrono::NaiveDateTime",
                DataType::UtcDateTime if self.is_utc_output() => {
                    "chrono::DateTime<chrono::offset::Utc>"
                }
                DataType::UtcDateTime => "chrono::DateTime<chrono::offset::Local>",
                DataType::TimestampWithTimeZone if self.is_utc_output() => {
                    "chrono::DateTime<chrono::offset::Utc>"
                }
                DataType::TimestampWithTimeZone => "chrono::DateTime<chrono::offset::Local>",
                DataType::Date => "chrono::NaiveDate",
                DataType::Time => "chrono::NaiveTime",
                DataType::Decimal => "rust_decimal::Decimal",
                DataType::ArrayInt => "Vec<u64>",
                DataType::ArrayString => "Vec<String>",
                DataType::Json | DataType::Jsonb if self.user_defined_json_type.is_some() => {
                    self.user_defined_json_type.as_ref().unwrap()
                }
                DataType::Json | DataType::Jsonb => "serde_json::Value",
                DataType::DbEnum => "",
                DataType::DbSet => "",
                DataType::Point => "senax_common::types::point::Point",
                DataType::GeoPoint => "senax_common::types::geo_point::GeoPoint",
                DataType::Geometry if self.user_defined_json_type.is_some() => {
                    self.user_defined_json_type.as_ref().unwrap()
                }
                DataType::Geometry => "serde_json::Value",
                DataType::ValueObject => unimplemented!(),
                DataType::AutoFk => unimplemented!(),
                DataType::UnSupported => unimplemented!(),
            }
            .to_string()
        }
    }
    pub fn is_addable(&self) -> bool {
        if self.query.is_some() {
            return false;
        }
        let mut is_num = self.data_type == DataType::TinyInt
            || self.data_type == DataType::SmallInt
            || self.data_type == DataType::Int
            || self.data_type == DataType::BigInt;
        let is_float = self.data_type == DataType::Float || self.data_type == DataType::Double;
        if self.id_class.is_some()
            || self.enum_class.is_some()
            || self.rel.is_some()
            || self.outer_db_rel.is_some()
        {
            is_num = false;
        }
        is_num || is_float
    }
    pub fn accessor(&self, with_type: bool, sep: &str) -> String {
        let inner = self.get_inner_type(false, true);
        let outer = self.get_outer_for_updater_type(self.primary);
        let null = !self.not_null;
        let is_num = self.data_type == DataType::TinyInt
            || self.data_type == DataType::SmallInt
            || self.data_type == DataType::Int
            || self.data_type == DataType::BigInt;
        let is_float = self.data_type == DataType::Float || self.data_type == DataType::Double;
        let is_ord = self.data_type != DataType::Boolean
            && self.data_type != DataType::Binary
            && self.data_type != DataType::Varbinary
            && self.data_type != DataType::Blob
            && self.data_type != DataType::ArrayInt
            && self.data_type != DataType::ArrayString
            && self.data_type != DataType::Json
            && self.data_type != DataType::Jsonb
            && self.data_type != DataType::Point
            && self.data_type != DataType::GeoPoint
            && self.data_type != DataType::Geometry;
        if self.primary {
            if with_type {
                format!(
                    "{}Primary{sep}<'_, {inner}, {outer}>",
                    if_then_else!(null, "Null", ""),
                )
            } else {
                format!("{}Primary", if_then_else!(null, "Null", ""))
            }
        } else if self.id_class.is_some()
            || self.enum_class.is_some()
            || self.rel.is_some()
            || self.outer_db_rel.is_some()
        {
            if with_type {
                format!(
                    "{}Null{sep}<'_, {inner}, {outer}>",
                    if_then_else!(null, "", "Not"),
                )
            } else {
                format!("{}Null", if_then_else!(null, "", "Not"))
            }
        } else if is_num {
            if with_type {
                format!(
                    "{}NullNum{sep}<'_, {inner}>",
                    if_then_else!(null, "", "Not"),
                )
            } else {
                format!("{}NullNum", if_then_else!(null, "", "Not"))
            }
        } else if is_float {
            if with_type {
                format!(
                    "{}NullFloat{sep}<'_, {inner}>",
                    if_then_else!(null, "", "Not"),
                )
            } else {
                format!("{}NullFloat", if_then_else!(null, "", "Not"))
            }
        } else if self.data_type == DataType::Boolean {
            if with_type {
                format!("{}NullBool{sep}<'_>", if_then_else!(null, "", "Not"))
            } else {
                format!("{}NullBool", if_then_else!(null, "", "Not"))
            }
        } else if self.data_type == DataType::Char
            || self.data_type == DataType::IdVarchar
            || self.data_type == DataType::TextVarchar
            || self.data_type == DataType::Text
            || self.data_type == DataType::DbSet
        {
            if with_type {
                format!("{}NullArc{sep}<'_, String>", if_then_else!(null, "", "Not"),)
            } else {
                format!("{}NullArc", if_then_else!(null, "", "Not"))
            }
        } else if self.data_type == DataType::Binary
            || self.data_type == DataType::Varbinary
            || self.data_type == DataType::Blob
        {
            if with_type {
                format!("{}NullBlob{sep}<'_>", if_then_else!(null, "", "Not"))
            } else {
                format!("{}NullBlob", if_then_else!(null, "", "Not"))
            }
        } else if self.data_type == DataType::Json
            || self.data_type == DataType::Jsonb
            || self.data_type == DataType::Geometry
        {
            if with_type {
                format!(
                    "{}NullJson{sep}<'_, {outer}>",
                    if_then_else!(null, "", "Not"),
                )
            } else {
                format!("{}NullJson", if_then_else!(null, "", "Not"))
            }
        } else if self.is_arc() {
            if with_type {
                format!(
                    "{}NullArc{sep}<'_, {outer}>",
                    if_then_else!(null, "", "Not"),
                )
            } else {
                format!("{}NullArc", if_then_else!(null, "", "Not"))
            }
        } else if is_ord {
            if with_type {
                format!(
                    "{}NullOrd{sep}<'_, {inner}>",
                    if_then_else!(null, "", "Not"),
                )
            } else {
                format!("{}NullOrd", if_then_else!(null, "", "Not"))
            }
        } else if with_type {
            format!(
                "{}Null{sep}<'_, {inner}, {outer}>",
                if_then_else!(null, "", "Not"),
            )
        } else {
            format!("{}Null", if_then_else!(null, "", "Not"))
        }
    }
    pub fn convert_inner_type(&self) -> String {
        let id_str = if self.id_class.is_some()
            || self.enum_class.is_some()
            || self.rel.is_some()
            || self.outer_db_rel.is_some()
        {
            ".get()"
        } else {
            ""
        };
        let conv_str = match self.data_type {
            DataType::TinyInt => "",
            DataType::SmallInt => "",
            DataType::Int => "",
            DataType::BigInt => "",
            DataType::Float => "",
            DataType::Double => "",
            DataType::Char | DataType::IdVarchar | DataType::TextVarchar => ".to_owned()",
            DataType::Boolean => "",
            DataType::Text => ".to_owned()",
            DataType::Uuid => "",
            DataType::BinaryUuid => "",
            DataType::Binary | DataType::Varbinary | DataType::Blob => "",
            DataType::NaiveDateTime if self.not_null => ".into()",
            DataType::NaiveDateTime => "",
            DataType::UtcDateTime if self.not_null => ".into()",
            DataType::UtcDateTime => "",
            DataType::TimestampWithTimeZone if self.not_null => ".into()",
            DataType::TimestampWithTimeZone => "",
            DataType::Date if self.not_null => ".into()",
            DataType::Date => "",
            DataType::Time if self.not_null => ".into()",
            DataType::Time => "",
            DataType::Decimal => "",
            DataType::ArrayInt => "",
            DataType::ArrayString => "",
            DataType::Json | DataType::Jsonb => "",
            DataType::DbEnum => ".to_owned()",
            DataType::DbSet => ".to_owned()",
            DataType::Point => ".into()",
            DataType::GeoPoint => ".into()",
            DataType::Geometry => "",
            DataType::ValueObject => unimplemented!(),
            DataType::AutoFk => unimplemented!(),
            DataType::UnSupported => unimplemented!(),
        };
        if !self.not_null {
            if id_str.is_empty() && conv_str.is_empty() {
                "".to_string()
            } else {
                format!(".map(|v| v{}{})", id_str, conv_str)
            }
        } else {
            format!("{}{}", id_str, conv_str)
        }
    }

    pub fn get_bind_as_for_filter(&self) -> &'static str {
        let exclude_from_domain = CONFIG.read().unwrap().as_ref().unwrap().exclude_from_domain;
        if let Some(ref _class) = self.enum_class {
            if self.is_integer() {
                if exclude_from_domain {
                    return "";
                } else {
                    return ".inner()";
                }
            } else {
                return ".as_static_str()";
            }
        }
        if self.id_class.is_some()
            || self.rel.is_some()
            || self.outer_db_rel.is_some()
            || (self.value_object.is_some() && !exclude_from_domain)
        {
            let s = if exclude_from_domain {
                ".0.as_ref().to_owned()"
            } else {
                ".inner().as_ref().to_owned()"
            };
            return match self.data_type {
                DataType::Char | DataType::IdVarchar | DataType::TextVarchar | DataType::Text => s,
                DataType::Binary | DataType::Varbinary | DataType::Blob => s,
                _ => ".inner()",
            };
        }
        match self.data_type {
            DataType::Boolean => "",
            _ => "",
        }
    }

    pub fn get_bind_as(&self) -> &'static str {
        if let Some(ref _class) = self.enum_class {
            if self.is_integer() {
                return "";
            } else if self.not_null {
                return ".as_static_str()";
            } else {
                return ".as_ref().map(|v| v.as_static_str())";
            }
        }
        if is_mysql_mode() {
            match self.data_type {
                DataType::Char | DataType::IdVarchar | DataType::TextVarchar | DataType::Text
                    if self.not_null =>
                {
                    ".as_str()"
                }
                DataType::Char | DataType::IdVarchar | DataType::TextVarchar | DataType::Text => {
                    ".as_ref().map(|v| v.as_str())"
                }
                DataType::Binary | DataType::Varbinary | DataType::Blob if self.not_null => {
                    ".as_slice()"
                }
                DataType::Binary | DataType::Varbinary | DataType::Blob => {
                    ".as_ref().map(|v| v.as_slice())"
                }
                DataType::GeoPoint | DataType::Point if self.not_null => ".to_lng_lat_wkb()",
                DataType::GeoPoint | DataType::Point => ".as_ref().map(|v| v.to_lng_lat_wkb())",
                DataType::ArrayInt
                | DataType::ArrayString
                | DataType::Json
                | DataType::Jsonb
                | DataType::Geometry
                    if self.not_null =>
                {
                    "._to_bindable_json()"
                }
                DataType::ArrayInt
                | DataType::ArrayString
                | DataType::Json
                | DataType::Jsonb
                | DataType::Geometry => ".as_ref().map(|v| v._to_bindable_json())",
                _ => "",
            }
        } else {
            match self.data_type {
                DataType::Char | DataType::IdVarchar | DataType::TextVarchar | DataType::Text
                    if self.not_null =>
                {
                    ".as_str()"
                }
                DataType::Char | DataType::IdVarchar | DataType::TextVarchar | DataType::Text => {
                    ".as_ref().map(|v| v.as_str())"
                }
                DataType::Binary | DataType::Varbinary | DataType::Blob if self.not_null => {
                    ".as_slice()"
                }
                DataType::Binary | DataType::Varbinary | DataType::Blob => {
                    ".as_ref().map(|v| v.as_slice())"
                }
                DataType::GeoPoint | DataType::Point if self.not_null => ".to_lng_lat_wkb()",
                DataType::GeoPoint | DataType::Point => ".as_ref().map(|v| v.to_lng_lat_wkb())",
                DataType::ArrayInt
                | DataType::ArrayString
                | DataType::Json
                | DataType::Jsonb
                | DataType::Geometry
                    if self.not_null =>
                {
                    "._to_bindable_json()"
                }
                DataType::ArrayInt
                | DataType::ArrayString
                | DataType::Json
                | DataType::Jsonb
                | DataType::Geometry => ".as_ref().map(|v| v._to_bindable_json())",
                _ => "",
            }
        }
    }

    pub fn get_from_row(&self, name: &&String, index: i32) -> String {
        let signed_only: bool = CONFIG.read().unwrap().as_ref().unwrap().signed_only();
        if let Some(ref class) = self.enum_class {
            if self.is_integer() {
                let typ = match self.data_type {
                    DataType::TinyInt if self.signed || signed_only => "i8",
                    DataType::TinyInt => "u8",
                    DataType::SmallInt if self.signed || signed_only => "i16",
                    DataType::SmallInt => "u16",
                    DataType::Int if self.signed || signed_only => "i32",
                    DataType::Int => "u32",
                    DataType::BigInt if self.signed || signed_only => "i64",
                    DataType::BigInt => "u64",
                    _ => panic!("unsupported type"),
                };
                if self.not_null {
                    return format!("row.try_get::<{}, _>({index})?.into()", typ,);
                } else {
                    return format!(
                        "row.try_get::<Option<{}>, _>({index})?.map(|v| v.into())",
                        typ,
                    );
                }
            } else if self.not_null {
                return format!(
                    "{}::try_from(row.try_get::<&str, _>({index})?).map_err(|e| sqlx::Error::ColumnDecode {{
                index: {name:?}.to_string(),
                source: Box::new(e),
            }})?",
                    &class.to_string()
                );
            } else {
                return format!("row.try_get::<Option<&str>, _>({index})?.map({}::try_from).transpose().map_err(|e| sqlx::Error::ColumnDecode {{
                index: {name:?}.to_string(),
                source: Box::new(e),
            }})?", &class.to_string());
            }
        }
        if !is_mysql_mode() && self.data_type == DataType::UtcDateTime {
            return format!("row.try_get_unchecked({index})?",);
        }
        if self.data_type == DataType::Char
            || self.data_type == DataType::IdVarchar
            || self.data_type == DataType::TextVarchar
            || self.data_type == DataType::Text
        {
            if self.not_null {
                return format!("row.try_get::<String, _>({index})?.into()",);
            } else {
                return format!("row.try_get::<Option<String>, _>({index})?.map(|v| v.into())",);
            }
        }
        if self.data_type == DataType::Json
            || self.data_type == DataType::Jsonb
            || self.data_type == DataType::Geometry
        {
            if self.not_null {
                return format!("row.try_get_unchecked::<String, _>({index})?.try_into().map_err(|e| sqlx::Error::ColumnDecode {{
                index: {name:?}.to_string(),
                source: e,
            }})?",);
            } else {
                return format!("row.try_get_unchecked::<Option<String>, _>({index})?.map(|v| v.try_into()).transpose().map_err(|e| sqlx::Error::ColumnDecode {{
                index: {name:?}.to_string(),
                source: e,
            }})?",);
            }
        }
        if self.data_type == DataType::Binary
            || self.data_type == DataType::Varbinary
            || self.data_type == DataType::Blob
        {
            if self.not_null {
                return format!("row.try_get::<Vec<u8>, _>({index})?.into()",);
            } else {
                return format!("row.try_get::<Option<Vec<u8>>, _>({index})?.map(|v| v.into())",);
            }
        }
        if self.data_type == DataType::ArrayInt || self.data_type == DataType::ArrayString {
            let ty = self.get_inner_type(true, true);
            if self.not_null {
                return format!("row.try_get::<::sqlx::types::Json<{ty}>, _>({index})?.0.into()",);
            } else {
                return format!(
                    "row.try_get::<Option<::sqlx::types::Json<{ty}>>, _>({index})?.map(|x| x.0.into())",
                );
            }
        }
        if self.data_type == DataType::Point || self.data_type == DataType::GeoPoint {
            if self.not_null {
                return format!("row.try_get::<&[u8], _>({index})?.into()",);
            } else {
                return format!("row.try_get::<Option<&[u8]>, _>({index})?.map(|v| v.into())",);
            }
        }
        format!("row.try_get({index})?",)
    }

    pub fn get_col_name<'a>(&'a self, name: &'a str) -> Cow<'a, str> {
        if let Some(column_name) = &self.column_name {
            column_name.into()
        } else if let Some(query) = &self.query {
            format!("{}_{}", name, crate::common::hex_digest(query)).into()
        } else {
            name.into()
        }
    }

    pub fn is_equivalence(&self) -> bool {
        match self.data_type {
            DataType::TinyInt => true,
            DataType::SmallInt => true,
            DataType::Int => true,
            DataType::BigInt => true,
            DataType::Float => false,
            DataType::Double => false,
            DataType::Char | DataType::IdVarchar | DataType::TextVarchar => true,
            DataType::Boolean => true,
            DataType::Text => true,
            DataType::Uuid => true,
            DataType::BinaryUuid => true,
            DataType::Binary | DataType::Varbinary | DataType::Blob => true,
            DataType::NaiveDateTime => true,
            DataType::UtcDateTime => true,
            DataType::TimestampWithTimeZone => true,
            DataType::Date => true,
            DataType::Time => true,
            DataType::Decimal => true,
            DataType::ArrayInt => false,
            DataType::ArrayString => false,
            DataType::Json | DataType::Jsonb => false,
            DataType::DbEnum => true,
            DataType::DbSet => false, // TODO enumset
            DataType::Point => false,
            DataType::GeoPoint => false,
            DataType::Geometry => false,
            DataType::ValueObject => unimplemented!(),
            DataType::AutoFk => unimplemented!(),
            DataType::UnSupported => unimplemented!(),
        }
    }

    pub fn is_comparable(&self) -> bool {
        if self.enum_class.is_some() {
            return false;
        }
        match self.data_type {
            DataType::TinyInt => true,
            DataType::SmallInt => true,
            DataType::Int => true,
            DataType::BigInt => true,
            DataType::Float => true,
            DataType::Double => true,
            DataType::Char | DataType::IdVarchar | DataType::TextVarchar => true,
            DataType::Boolean => true,
            DataType::Text => false,
            DataType::Uuid => true,
            DataType::BinaryUuid => true,
            DataType::Binary | DataType::Varbinary => true,
            DataType::Blob => false,
            DataType::NaiveDateTime => true,
            DataType::UtcDateTime => true,
            DataType::TimestampWithTimeZone => true,
            DataType::Date => true,
            DataType::Time => true,
            DataType::Decimal => true,
            DataType::ArrayInt => false,
            DataType::ArrayString => false,
            DataType::Json | DataType::Jsonb => false,
            DataType::DbEnum => true,
            DataType::DbSet => false,
            DataType::Point => false,
            DataType::GeoPoint => false,
            DataType::Geometry => false,
            DataType::ValueObject => unimplemented!(),
            DataType::AutoFk => unimplemented!(),
            DataType::UnSupported => unimplemented!(),
        }
    }

    pub fn is_hashable(&self) -> bool {
        match self.data_type {
            DataType::TinyInt => true,
            DataType::SmallInt => true,
            DataType::Int => true,
            DataType::BigInt => true,
            DataType::Float => false,
            DataType::Double => false,
            DataType::Char | DataType::IdVarchar | DataType::TextVarchar => true,
            DataType::Boolean => true,
            DataType::Text => true,
            DataType::Uuid => true,
            DataType::BinaryUuid => true,
            DataType::Binary | DataType::Varbinary | DataType::Blob => true,
            DataType::NaiveDateTime => true,
            DataType::UtcDateTime => true,
            DataType::TimestampWithTimeZone => true,
            DataType::Date => true,
            DataType::Time => true,
            DataType::Decimal => true,
            DataType::ArrayInt => false,
            DataType::ArrayString => false,
            DataType::Json | DataType::Jsonb => false,
            DataType::DbEnum => true,
            DataType::DbSet => false,
            DataType::Point => false,
            DataType::GeoPoint => false,
            DataType::Geometry => false,
            DataType::ValueObject => unimplemented!(),
            DataType::AutoFk => unimplemented!(),
            DataType::UnSupported => unimplemented!(),
        }
    }

    pub fn is_copyable(&self) -> bool {
        match self.data_type {
            DataType::TinyInt => true,
            DataType::SmallInt => true,
            DataType::Int => true,
            DataType::BigInt => true,
            DataType::Float => true,
            DataType::Double => true,
            DataType::Char | DataType::IdVarchar | DataType::TextVarchar => false,
            DataType::Boolean => true,
            DataType::Text => false,
            DataType::Uuid => true,
            DataType::BinaryUuid => true,
            DataType::Binary | DataType::Varbinary | DataType::Blob => false,
            DataType::NaiveDateTime => true,
            DataType::UtcDateTime => true,
            DataType::TimestampWithTimeZone => true,
            DataType::Date => true,
            DataType::Time => true,
            DataType::Decimal => true,
            DataType::ArrayInt => false,
            DataType::ArrayString => false,
            DataType::Json | DataType::Jsonb => false,
            DataType::DbEnum => false,
            DataType::DbSet => false,
            DataType::Point => true,
            DataType::GeoPoint => true,
            DataType::Geometry => false,
            DataType::ValueObject => unimplemented!(),
            DataType::AutoFk => unimplemented!(),
            DataType::UnSupported => unimplemented!(),
        }
    }

    pub fn is_displayable(&self) -> bool {
        match self.data_type {
            DataType::TinyInt => true,
            DataType::SmallInt => true,
            DataType::Int => true,
            DataType::BigInt => true,
            DataType::Float => true,
            DataType::Double => true,
            DataType::Char | DataType::IdVarchar | DataType::TextVarchar => true,
            DataType::Boolean => true,
            DataType::Text => true,
            DataType::Uuid => true,
            DataType::BinaryUuid => true,
            DataType::Binary | DataType::Varbinary | DataType::Blob => false,
            DataType::NaiveDateTime => true,
            DataType::UtcDateTime => true,
            DataType::TimestampWithTimeZone => true,
            DataType::Date => true,
            DataType::Time => true,
            DataType::Decimal => true,
            DataType::ArrayInt => false,
            DataType::ArrayString => false,
            DataType::Json | DataType::Jsonb => false,
            DataType::DbEnum => true,
            DataType::DbSet => false,
            DataType::Point => true,
            DataType::GeoPoint => true,
            DataType::Geometry => false,
            DataType::ValueObject => unimplemented!(),
            DataType::AutoFk => unimplemented!(),
            DataType::UnSupported => unimplemented!(),
        }
    }

    pub fn srid(&self) -> u32 {
        let default = match self.data_type {
            DataType::Point => 0,
            DataType::GeoPoint => crate::common::DEFAULT_SRID,
            DataType::Geometry => crate::common::DEFAULT_SRID,
            _ => 0,
        };
        self.srid.unwrap_or(default)
    }

    pub fn gql_type(&self) -> &str {
        match self.data_type {
            DataType::Point => "{x,y}",
            DataType::GeoPoint => "{lat,lng}",
            _ => "",
        }
    }

    pub fn placeholder(&self) -> String {
        if is_mysql_mode() {
            match self.data_type {
                DataType::Uuid => "BIN_TO_UUID(?)".to_string(),
                DataType::Point if self.srid.is_some() => {
                    format!("ST_GeomFromWKB(?,{})", self.srid.unwrap())
                }
                DataType::Point => "ST_GeomFromWKB(?)".to_string(),
                DataType::GeoPoint if self.srid.is_some() => {
                    format!(
                        "ST_GeomFromWKB(?,{},'axis-order=long-lat')",
                        self.srid.unwrap()
                    )
                }
                DataType::GeoPoint => "ST_GeomFromWKB(?)".to_string(),
                DataType::Geometry if self.srid.is_some() => {
                    format!("ST_GeomFromGeoJSON(?,1,{})", self.srid.unwrap())
                }
                DataType::Geometry => "ST_GeomFromGeoJSON(?)".to_string(),
                _ => "?".to_owned(),
            }
        } else {
            match self.data_type {
                DataType::Point if self.srid.is_some() => {
                    format!("ST_GeomFromWKB(?,{})", self.srid.unwrap())
                }
                DataType::Point => "ST_GeomFromWKB(?)".to_string(),
                DataType::GeoPoint if self.srid.is_some() => {
                    format!("ST_GeomFromWKB(?,{})", self.srid.unwrap())
                }
                DataType::GeoPoint => "ST_GeomFromWKB(?)".to_string(),
                DataType::Geometry if self.srid.is_some() => {
                    format!("ST_SetSRID(ST_GeomFromGeoJSON(?), {})", self.srid.unwrap())
                }
                DataType::Geometry => "ST_GeomFromGeoJSON(?)".to_string(),
                _ => "?".to_owned(),
            }
        }
    }

    pub fn clone_str(&self) -> &'static str {
        if self.is_copyable() { "" } else { ".clone()" }
    }

    pub fn clone_for_outer_str(&self) -> &'static str {
        if self.enum_values.is_some() {
            return "";
        }
        let copyable =
            if self.id_class.is_some() || self.rel.is_some() || self.outer_db_rel.is_some() {
                self.is_copyable()
            } else {
                true
            };
        if copyable { "" } else { ".clone()" }
    }

    pub fn is_cascade_on_delete(&self) -> bool {
        if let Some((_, rel)) = &self.rel {
            rel.on_delete == Some(super::ReferenceOption::Cascade)
        } else {
            false
        }
    }
}
