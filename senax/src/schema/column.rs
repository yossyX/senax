use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

use crate::common::if_then_else;

use super::{RelDef, TimeZone, CONFIG};

pub const DEFAULT_VARCHAR_LENGTH: u32 = 255;
pub const DEFAULT_PRECISION: u16 = 36;
pub const DEFAULT_SCALE: u16 = 9;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Copy, Default, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[schemars(title = "Column Type")]
pub enum ColumnType {
    TinyInt,
    SmallInt,
    #[default]
    Int,
    BigInt,
    Float,
    Double,
    Varchar,
    Boolean,
    Text,
    Blob,
    Timestamp, // 非推奨
    DateTime,
    Date,
    Time,
    Decimal,
    #[serde(rename = "array_int")]
    ArrayInt,
    #[serde(rename = "array_string")]
    ArrayString,
    Json,
    Enum,
    #[serde(rename = "db_enum")]
    DbEnum,
    #[serde(rename = "db_set")]
    DbSet,
    Point,
    #[schemars(skip)]
    UnSupported,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Copy, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(title = "Column Subset Type")]
pub enum ColumnSubsetType {
    Tinyint,
    Smallint,
    Int,
    Bigint,
    Float,
    Double,
    Varchar,
    Boolean,
    Text,
    Blob,
    Datetime,
    Date,
    Time,
    Decimal,
    ArrayInt,
    ArrayString,
    Json,
    TinyintNotNull,
    SmallintNotNull,
    IntNotNull,
    BigintNotNull,
    FloatNotNull,
    DoubleNotNull,
    VarcharNotNull,
    BooleanNotNull,
    TextNotNull,
    BlobNotNull,
    DatetimeNotNull,
    DateNotNull,
    TimeNotNull,
    DecimalNotNull,
    ArrayIntNotNull,
    ArrayStringNotNull,
    JsonNotNull,
}
impl From<&ColumnSubsetType> for ColumnType {
    fn from(v: &ColumnSubsetType) -> Self {
        match v {
            ColumnSubsetType::Tinyint => ColumnType::TinyInt,
            ColumnSubsetType::Smallint => ColumnType::SmallInt,
            ColumnSubsetType::Int => ColumnType::Int,
            ColumnSubsetType::Bigint => ColumnType::BigInt,
            ColumnSubsetType::Float => ColumnType::Float,
            ColumnSubsetType::Double => ColumnType::Double,
            ColumnSubsetType::Varchar => ColumnType::Varchar,
            ColumnSubsetType::Boolean => ColumnType::Boolean,
            ColumnSubsetType::Text => ColumnType::Text,
            ColumnSubsetType::Blob => ColumnType::Blob,
            ColumnSubsetType::Datetime => ColumnType::DateTime,
            ColumnSubsetType::Date => ColumnType::Date,
            ColumnSubsetType::Time => ColumnType::Time,
            ColumnSubsetType::Decimal => ColumnType::Decimal,
            ColumnSubsetType::ArrayInt => ColumnType::ArrayInt,
            ColumnSubsetType::ArrayString => ColumnType::ArrayString,
            ColumnSubsetType::Json => ColumnType::Json,
            ColumnSubsetType::TinyintNotNull => ColumnType::TinyInt,
            ColumnSubsetType::SmallintNotNull => ColumnType::SmallInt,
            ColumnSubsetType::IntNotNull => ColumnType::Int,
            ColumnSubsetType::BigintNotNull => ColumnType::BigInt,
            ColumnSubsetType::FloatNotNull => ColumnType::Float,
            ColumnSubsetType::DoubleNotNull => ColumnType::Double,
            ColumnSubsetType::VarcharNotNull => ColumnType::Varchar,
            ColumnSubsetType::BooleanNotNull => ColumnType::Boolean,
            ColumnSubsetType::TextNotNull => ColumnType::Text,
            ColumnSubsetType::BlobNotNull => ColumnType::Blob,
            ColumnSubsetType::DatetimeNotNull => ColumnType::DateTime,
            ColumnSubsetType::DateNotNull => ColumnType::Date,
            ColumnSubsetType::TimeNotNull => ColumnType::Time,
            ColumnSubsetType::DecimalNotNull => ColumnType::Decimal,
            ColumnSubsetType::ArrayIntNotNull => ColumnType::ArrayInt,
            ColumnSubsetType::ArrayStringNotNull => ColumnType::ArrayString,
            ColumnSubsetType::JsonNotNull => ColumnType::Json,
        }
    }
}
impl ColumnSubsetType {
    pub fn not_null(&self) -> bool {
        match self {
            ColumnSubsetType::Tinyint => false,
            ColumnSubsetType::Smallint => false,
            ColumnSubsetType::Int => false,
            ColumnSubsetType::Bigint => false,
            ColumnSubsetType::Float => false,
            ColumnSubsetType::Double => false,
            ColumnSubsetType::Varchar => false,
            ColumnSubsetType::Boolean => false,
            ColumnSubsetType::Text => false,
            ColumnSubsetType::Blob => false,
            ColumnSubsetType::Datetime => false,
            ColumnSubsetType::Date => false,
            ColumnSubsetType::Time => false,
            ColumnSubsetType::Decimal => false,
            ColumnSubsetType::ArrayInt => false,
            ColumnSubsetType::ArrayString => false,
            ColumnSubsetType::Json => false,
            ColumnSubsetType::TinyintNotNull => true,
            ColumnSubsetType::SmallintNotNull => true,
            ColumnSubsetType::IntNotNull => true,
            ColumnSubsetType::BigintNotNull => true,
            ColumnSubsetType::FloatNotNull => true,
            ColumnSubsetType::DoubleNotNull => true,
            ColumnSubsetType::VarcharNotNull => true,
            ColumnSubsetType::BooleanNotNull => true,
            ColumnSubsetType::TextNotNull => true,
            ColumnSubsetType::BlobNotNull => true,
            ColumnSubsetType::DatetimeNotNull => true,
            ColumnSubsetType::DateNotNull => true,
            ColumnSubsetType::TimeNotNull => true,
            ColumnSubsetType::DecimalNotNull => true,
            ColumnSubsetType::ArrayIntNotNull => true,
            ColumnSubsetType::ArrayStringNotNull => true,
            ColumnSubsetType::JsonNotNull => true,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Copy, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(title = "Auto Increment")]
pub enum AutoIncrement {
    Auto,
    // Sequence,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, JsonSchema)]
#[schemars(deny_unknown_fields)]
#[schemars(title = "Enum Value")]
pub struct EnumValue {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[schemars(range(min = 0, max = 255))]
    pub value: u8,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, JsonSchema)]
#[schemars(deny_unknown_fields)]
#[schemars(title = "DB Enum Value")]
pub struct DbEnumValue {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(untagged)]
#[schemars(title = "Column Type Or Def")]
#[allow(clippy::large_enum_variant)]
pub enum ColumnTypeOrDef {
    Exact(ColumnDef),
    Simple(ColumnSubsetType),
}

impl ColumnTypeOrDef {
    pub fn exact(&self) -> ColumnDef {
        match self {
            ColumnTypeOrDef::Exact(def) => def.clone(),
            ColumnTypeOrDef::Simple(type_def) => ColumnDef {
                type_def: type_def.into(),
                not_null: type_def.not_null(),
                ..Default::default()
            },
        }
    }
}
impl From<ColumnDef> for ColumnTypeOrDef {
    fn from(val: ColumnDef) -> Self {
        ColumnTypeOrDef::Exact(val)
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[schemars(deny_unknown_fields)]
#[schemars(title = "Column Def")]
pub struct ColumnDef {
    #[serde(skip)]
    pub class: Option<String>,
    #[serde(skip)]
    pub rel: Option<(String, Option<super::RelDef>)>,
    #[serde(skip)]
    pub main_primary: bool,
    #[serde(skip)]
    pub auto_gen: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(rename = "type")]
    pub type_def: ColumnType,
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub signed: bool,
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub not_null: bool,
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub primary: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_increment: Option<AutoIncrement>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length: Option<u32>,
    // #[serde(default, skip_serializing_if = "Option::is_none")]
    // pub character_set: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collate: Option<String>,
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub not_serializable: bool, // パスワード等保護用
    #[serde(skip_serializing_if = "Option::is_none")]
    pub precision: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_zone: Option<TimeZone>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<EnumValue>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub db_enum_values: Option<Vec<DbEnumValue>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enum_model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub json_class: Option<String>,
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub exclude_from_cache: bool,
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub skip_factory: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rename: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub srid: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sql_comment: Option<String>,
}

impl ColumnDef {
    pub fn is_utc(&self) -> bool {
        let tz = self
            .time_zone
            .or(unsafe { CONFIG.get().unwrap() }.time_zone);
        tz == Some(TimeZone::Utc)
    }

    pub fn get_serde_default(&self) -> String {
        let result = match self.type_def {
            ColumnType::TinyInt if self.signed => {
                "    #[serde(default, skip_serializing_if = \"Empty::is_zero_i8\")]\n"
            }
            ColumnType::TinyInt => {
                "    #[serde(default, skip_serializing_if = \"Empty::is_zero_u8\")]\n"
            }
            ColumnType::SmallInt if self.signed => {
                "    #[serde(default, skip_serializing_if = \"Empty::is_zero_i16\")]\n"
            }
            ColumnType::SmallInt => {
                "    #[serde(default, skip_serializing_if = \"Empty::is_zero_u16\")]\n"
            }
            ColumnType::Int if self.signed => {
                "    #[serde(default, skip_serializing_if = \"Empty::is_zero_i32\")]\n"
            }
            ColumnType::Int => {
                "    #[serde(default, skip_serializing_if = \"Empty::is_zero_u32\")]\n"
            }
            ColumnType::BigInt if self.signed => {
                "    #[serde(default, skip_serializing_if = \"Empty::is_zero_i64\")]\n"
            }
            ColumnType::BigInt => {
                "    #[serde(default, skip_serializing_if = \"Empty::is_zero_u64\")]\n"
            }
            ColumnType::Float => {
                "    #[serde(default, skip_serializing_if = \"Empty::is_zero_f32\")]\n"
            }
            ColumnType::Double => {
                "    #[serde(default, skip_serializing_if = \"Empty::is_zero_f64\")]\n"
            }
            ColumnType::Varchar => {
                "    #[serde(default, skip_serializing_if = \"String::is_empty\")]\n"
            }
            ColumnType::Boolean => {
                "    #[serde(default, skip_serializing_if = \"Empty::is_zero_i8\")]\n"
            }
            ColumnType::Text => {
                "    #[serde(default, skip_serializing_if = \"String::is_empty\")]\n"
            }
            ColumnType::Blob => {
                "    #[serde(default, skip_serializing_if = \"Empty::is_zero_len\")]\n"
            }
            ColumnType::Timestamp if self.is_utc() => {
                "    #[serde(default, skip_serializing_if = \"Empty::is_default_utc_date_time\")]\n"
            }
            ColumnType::Timestamp => {
                "    #[serde(default, skip_serializing_if = \"Empty::is_default_local_date_time\")]\n"
            }
            ColumnType::DateTime if self.is_utc() => {
                "    #[serde(default, skip_serializing_if = \"Empty::is_default_utc_date_time\")]\n"
            }
            ColumnType::DateTime => {
                "    #[serde(default, skip_serializing_if = \"Empty::is_default_local_date_time\")]\n"
            }
            ColumnType::Date => {
                "    #[serde(default, skip_serializing_if = \"Empty::is_default_date\")]\n"
            }
            ColumnType::Time => {
                "    #[serde(default, skip_serializing_if = \"Empty::is_default_time\")]\n"
            }
            ColumnType::Decimal => {
                "    #[serde(default, skip_serializing_if = \"Empty::is_zero_decimal\")]\n"
            }
            ColumnType::ArrayInt => {
                "    #[serde(default, skip_serializing_if = \"Empty::is_zero_json_len\")]\n"
            }
            ColumnType::ArrayString => {
                "    #[serde(default, skip_serializing_if = \"Empty::is_zero_json_len\")]\n"
            }
            ColumnType::Json => {
                "    #[serde(default, skip_serializing_if = \"Empty::is_default_json\")]\n"
            }
            ColumnType::Enum => {
                "    #[serde(default, skip_serializing_if = \"Empty::is_zero_u8\")]\n"
            }
            ColumnType::DbEnum => {
                "    #[serde(default, skip_serializing_if = \"String::is_empty\")]\n"
            }
            ColumnType::DbSet => {
                "    #[serde(default, skip_serializing_if = \"String::is_empty\")]\n"
            }
            ColumnType::Point => {
                "    #[serde(default, skip_serializing_if = \"Empty::is_zero_len\")]\n"
            }
            ColumnType::UnSupported => unimplemented!(),
        };
        if self.primary {
            "".to_owned()
        } else if self.not_null {
            result.to_owned()
        } else {
            "    #[serde(skip_serializing_if = \"Option::is_none\")]\n".to_owned()
        }
    }

    pub fn get_rename(&self, name: &str) -> String {
        match self.type_def {
            ColumnType::Point => {
                format!(
                    "    #[sql(query = {:?})]\n",
                    &format!("ST_AsBinary(`{}`)", self.get_col_name(name))
                )
            }
            _ => {
                if let Some(ref rename) = self.rename {
                    format!("    #[sql(query = {:?})]\n", &format!("`{}`", rename))
                } else {
                    "".to_owned()
                }
            }
        }
    }

    pub fn get_validate(&self) -> String {
        match self.type_def {
            ColumnType::Varchar => {
                let length = self.length.unwrap_or(DEFAULT_VARCHAR_LENGTH);
                format!("    #[validate(length(max = {}))]\n", length)
            }
            ColumnType::Text => {
                let length = self.length.unwrap_or(65536);
                if length < 256 {
                    "    #[validate(custom = \"crate::misc::validate_tinytext_length\")]\n"
                        .to_owned()
                } else if length < 65536 {
                    "    #[validate(custom = \"crate::misc::validate_text_length\")]\n".to_owned()
                } else {
                    "".to_owned()
                }
            }
            ColumnType::Blob => {
                if let Some(length) = self.length {
                    format!("    #[validate(length(max = {}))]\n", length)
                } else {
                    "".to_owned()
                }
            }
            ColumnType::Double if !self.signed => "    #[validate(range(min = 0))]\n".to_owned(),
            ColumnType::Float if !self.signed => "    #[validate(range(min = 0))]\n".to_owned(),
            ColumnType::Decimal if !self.signed => {
                "    #[validate(custom = \"crate::misc::validate_unsigned_decimal\")]\n".to_owned()
            }
            _ => "".to_owned(),
        }
    }

    pub fn get_inner_type(&self, without_option: &bool) -> String {
        let json_class = self
            .json_class
            .as_ref()
            .map(|v| format!("sqlx::types::Json<{}>", v));
        let typ = match self.type_def {
            ColumnType::TinyInt if self.signed => "i8",
            ColumnType::TinyInt => "u8",
            ColumnType::SmallInt if self.signed => "i16",
            ColumnType::SmallInt => "u16",
            ColumnType::Int if self.signed => "i32",
            ColumnType::Int => "u32",
            ColumnType::BigInt if self.signed => "i64",
            ColumnType::BigInt => "u64",
            ColumnType::Float => "f32",
            ColumnType::Double => "f64",
            ColumnType::Varchar => "String",
            ColumnType::Boolean => "i8",
            ColumnType::Text => "String",
            ColumnType::Blob => "Vec<u8>",
            ColumnType::Timestamp if self.is_utc() => "chrono::DateTime<chrono::offset::Utc>",
            ColumnType::Timestamp => "chrono::DateTime<chrono::offset::Local>",
            ColumnType::DateTime if self.is_utc() => "chrono::DateTime<chrono::offset::Utc>",
            ColumnType::DateTime => "chrono::DateTime<chrono::offset::Local>",
            ColumnType::Date => "chrono::NaiveDate",
            ColumnType::Time => "chrono::NaiveTime",
            ColumnType::Decimal => "rust_decimal::Decimal",
            ColumnType::ArrayInt => "sqlx::types::Json<Vec<u64>>",
            ColumnType::ArrayString => "sqlx::types::Json<Vec<String>>",
            ColumnType::Json if json_class.is_some() => json_class.as_ref().unwrap(),
            ColumnType::Json => "sqlx::types::Json<Value>",
            ColumnType::Enum => "u8",
            ColumnType::DbEnum => "String",
            ColumnType::DbSet => "String",
            ColumnType::Point => "Vec<u8>",
            ColumnType::UnSupported => unimplemented!(),
        };
        if *without_option {
            return typ.to_owned();
        }
        if self.not_null {
            typ.to_owned()
        } else {
            format!("Option<{}>", typ)
        }
    }

    pub fn get_may_null(&self) -> &str {
        let may_null = self.get_inner_type(&false).starts_with("Option<");
        if_then_else!(may_null, "true", "false")
    }

    pub fn get_cond_type(&self) -> String {
        if let Some(ref name) = self.class {
            return name.to_string();
        }
        if let Some(ref rel) = self.rel {
            let (rel_name, def) = rel;
            let name = RelDef::get_id_name(def, rel_name);
            let mod_name = RelDef::get_group_mod_name(def, rel_name);
            return format!("rel_{}::{}", mod_name, name);
        }
        let type_str = match self.type_def {
            ColumnType::TinyInt if self.signed => "i8",
            ColumnType::TinyInt => "u8",
            ColumnType::SmallInt if self.signed => "i16",
            ColumnType::SmallInt => "u16",
            ColumnType::Int if self.signed => "i32",
            ColumnType::Int => "u32",
            ColumnType::BigInt if self.signed => "i64",
            ColumnType::BigInt => "u64",
            ColumnType::Float => "f32",
            ColumnType::Double => "f64",
            ColumnType::Varchar => "String",
            ColumnType::Boolean => "bool",
            ColumnType::Text => "String",
            ColumnType::Blob => "Vec<u8>",
            ColumnType::Timestamp if self.is_utc() => "chrono::DateTime<chrono::offset::Utc>",
            ColumnType::Timestamp => "chrono::DateTime<chrono::offset::Local>",
            ColumnType::DateTime if self.is_utc() => "chrono::DateTime<chrono::offset::Utc>",
            ColumnType::DateTime => "chrono::DateTime<chrono::offset::Local>",
            ColumnType::Date => "chrono::NaiveDate",
            ColumnType::Time => "chrono::NaiveTime",
            ColumnType::Decimal => "rust_decimal::Decimal",
            ColumnType::ArrayInt => "u64",
            ColumnType::ArrayString => "String",
            ColumnType::Json if self.json_class.is_some() => self.json_class.as_ref().unwrap(),
            ColumnType::Json => "Value",
            ColumnType::Enum => "",
            ColumnType::DbEnum => "String",
            ColumnType::DbSet => "String",
            ColumnType::Point => "Point",
            ColumnType::UnSupported => unimplemented!(),
        };
        type_str.to_owned()
    }

    pub fn get_factory_type(&self) -> String {
        let mut typ = match self.type_def {
            ColumnType::TinyInt if self.signed => "i8",
            ColumnType::TinyInt => "u8",
            ColumnType::SmallInt if self.signed => "i16",
            ColumnType::SmallInt => "u16",
            ColumnType::Int if self.signed => "i32",
            ColumnType::Int => "u32",
            ColumnType::BigInt if self.signed => "i64",
            ColumnType::BigInt => "u64",
            ColumnType::Float => "f32",
            ColumnType::Double => "f64",
            ColumnType::Varchar => "String",
            ColumnType::Boolean => "bool",
            ColumnType::Text => "String",
            ColumnType::Blob => "Blob",
            ColumnType::Timestamp if self.is_utc() => "chrono::DateTime<chrono::offset::Utc>",
            ColumnType::Timestamp => "chrono::DateTime<chrono::offset::Local>",
            ColumnType::DateTime if self.is_utc() => "chrono::DateTime<chrono::offset::Utc>",
            ColumnType::DateTime => "chrono::DateTime<chrono::offset::Local>",
            ColumnType::Date => "chrono::NaiveDate",
            ColumnType::Time => "chrono::NaiveTime",
            ColumnType::Decimal => "rust_decimal::Decimal",
            ColumnType::ArrayInt => "Vec<u64>",
            ColumnType::ArrayString => "Vec<String>",
            ColumnType::Json if self.json_class.is_some() => self.json_class.as_ref().unwrap(),
            ColumnType::Json => "Value",
            ColumnType::Enum => "",
            ColumnType::DbEnum => "String",
            ColumnType::DbSet => "String",
            ColumnType::Point => "Point",
            ColumnType::UnSupported => unimplemented!(),
        }
        .to_string();
        if self.auto_increment.is_none() {
            typ = if let Some(ref name) = self.class {
                name.to_string()
            } else if let Some(ref rel) = self.rel {
                let (rel_name, def) = rel;
                let name = RelDef::get_id_name(def, rel_name);
                let mod_name = RelDef::get_group_mod_name(def, rel_name);
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
        if self.auto_increment.is_some() {
            "    #[serde(default)]\n"
        } else {
            ""
        }
    }

    pub fn convert_factory_type(&self) -> String {
        let mut id_str = "";
        if self.auto_increment.is_none() {
            id_str = if let Some(ref _name) = self.class {
                ".get()"
            } else if let Some(ref _rel) = self.rel {
                ".get()"
            } else {
                ""
            }
        }
        let conv_str = match self.type_def {
            ColumnType::TinyInt => "",
            ColumnType::SmallInt => "",
            ColumnType::Int => "",
            ColumnType::BigInt => "",
            ColumnType::Float => "",
            ColumnType::Double => "",
            ColumnType::Varchar => "",
            ColumnType::Boolean => " as i8",
            ColumnType::Text => "",
            ColumnType::Blob => ".into()",
            ColumnType::Timestamp if self.not_null => ".into()",
            ColumnType::Timestamp => "",
            ColumnType::DateTime if self.not_null => ".into()",
            ColumnType::DateTime => "",
            ColumnType::Date if self.not_null => ".into()",
            ColumnType::Date => "",
            ColumnType::Time if self.not_null => ".into()",
            ColumnType::Time => "",
            ColumnType::Decimal => "",
            ColumnType::ArrayInt => "._into_json()",
            ColumnType::ArrayString => "._into_json()",
            ColumnType::Json => "._into_json()",
            ColumnType::Enum => "",
            ColumnType::DbEnum => "",
            ColumnType::DbSet => "",
            ColumnType::Point => ".into()",
            ColumnType::UnSupported => unimplemented!(),
        };
        if !self.not_null {
            format!(".map(|v| v{}{})", id_str, conv_str)
        } else {
            format!("{}{}", id_str, conv_str)
        }
    }
    pub fn get_outer_type(&self) -> String {
        let typ = if let Some(ref name) = self.class {
            name.to_string()
        } else if let Some(ref rel) = self.rel {
            let (rel_name, def) = rel;
            let name = RelDef::get_id_name(def, rel_name);
            let mod_name = RelDef::get_group_mod_name(def, rel_name);
            format!("rel_{}::{}", mod_name, name)
        } else {
            let json_class = self.json_class.as_ref().map(|v| format!("&{}", v));
            match self.type_def {
                ColumnType::TinyInt if self.signed => "i8",
                ColumnType::TinyInt => "u8",
                ColumnType::SmallInt if self.signed => "i16",
                ColumnType::SmallInt => "u16",
                ColumnType::Int if self.signed => "i32",
                ColumnType::Int => "u32",
                ColumnType::BigInt if self.signed => "i64",
                ColumnType::BigInt => "u64",
                ColumnType::Float => "f32",
                ColumnType::Double => "f64",
                ColumnType::Varchar => "&str",
                ColumnType::Boolean => "bool",
                ColumnType::Text => "&str",
                ColumnType::Blob => "&[u8]",
                ColumnType::Timestamp if self.is_utc() => "chrono::DateTime<chrono::offset::Utc>",
                ColumnType::Timestamp => "chrono::DateTime<chrono::offset::Local>",
                ColumnType::DateTime if self.is_utc() => "chrono::DateTime<chrono::offset::Utc>",
                ColumnType::DateTime => "chrono::DateTime<chrono::offset::Local>",
                ColumnType::Date => "chrono::NaiveDate",
                ColumnType::Time => "chrono::NaiveTime",
                ColumnType::Decimal => "rust_decimal::Decimal",
                ColumnType::ArrayInt => "&Vec<u64>",
                ColumnType::ArrayString => "&Vec<String>",
                ColumnType::Json if json_class.is_some() => json_class.as_ref().unwrap(),
                ColumnType::Json => "&Value",
                ColumnType::Enum => "",
                ColumnType::DbEnum => "&str",
                ColumnType::DbSet => "&str",
                ColumnType::Point => "Point",
                ColumnType::UnSupported => unimplemented!(),
            }
            .to_string()
        };
        if self.not_null {
            typ
        } else {
            format!("Option<{}>", typ)
        }
    }
    pub fn get_outer_ref_type(&self) -> String {
        if let Some(ref name) = self.class {
            return format!("&{}", name);
        }
        if let Some(ref rel) = self.rel {
            let (rel_name, def) = rel;
            let name = RelDef::get_id_name(def, rel_name);
            let mod_name = RelDef::get_group_mod_name(def, rel_name);
            return format!("&rel_{}::{}", mod_name, name);
        }
        let json_class = self.json_class.as_ref().map(|v| format!("&{}", v));
        let typ = match self.type_def {
            ColumnType::TinyInt if self.signed => "i8",
            ColumnType::TinyInt => "u8",
            ColumnType::SmallInt if self.signed => "i16",
            ColumnType::SmallInt => "u16",
            ColumnType::Int if self.signed => "i32",
            ColumnType::Int => "u32",
            ColumnType::BigInt if self.signed => "i64",
            ColumnType::BigInt => "u64",
            ColumnType::Float => "f32",
            ColumnType::Double => "f64",
            ColumnType::Varchar => "&str",
            ColumnType::Boolean => "bool",
            ColumnType::Text => "&str",
            ColumnType::Blob => "&[u8]",
            ColumnType::Timestamp if self.is_utc() => "chrono::DateTime<chrono::offset::Utc>",
            ColumnType::Timestamp => "chrono::DateTime<chrono::offset::Local>",
            ColumnType::DateTime if self.is_utc() => "chrono::DateTime<chrono::offset::Utc>",
            ColumnType::DateTime => "chrono::DateTime<chrono::offset::Local>",
            ColumnType::Date => "chrono::NaiveDate",
            ColumnType::Time => "chrono::NaiveTime",
            ColumnType::Decimal => "rust_decimal::Decimal",
            ColumnType::ArrayInt => "&Vec<u64>",
            ColumnType::ArrayString => "&Vec<String>",
            ColumnType::Json if json_class.is_some() => json_class.as_ref().unwrap(),
            ColumnType::Json => "&Value",
            ColumnType::Enum => "",
            ColumnType::DbEnum => "&str",
            ColumnType::DbSet => "&str",
            ColumnType::Point => "Point",
            ColumnType::UnSupported => unimplemented!(),
        };
        if self.not_null {
            typ.to_owned()
        } else {
            format!("Option<{}>", typ)
        }
    }
    pub fn get_outer_owned_type(&self) -> String {
        if let Some(ref name) = self.class {
            return name.to_string();
        }
        if let Some(ref rel) = self.rel {
            let (rel_name, def) = rel;
            let name = RelDef::get_id_name(def, rel_name);
            let mod_name = RelDef::get_group_mod_name(def, rel_name);
            return format!("rel_{}::{}", mod_name, name);
        }
        let typ = match self.type_def {
            ColumnType::TinyInt if self.signed => "i8",
            ColumnType::TinyInt => "u8",
            ColumnType::SmallInt if self.signed => "i16",
            ColumnType::SmallInt => "u16",
            ColumnType::Int if self.signed => "i32",
            ColumnType::Int => "u32",
            ColumnType::BigInt if self.signed => "i64",
            ColumnType::BigInt => "u64",
            ColumnType::Float => "f32",
            ColumnType::Double => "f64",
            ColumnType::Varchar => "String",
            ColumnType::Boolean => "bool",
            ColumnType::Text => "String",
            ColumnType::Blob => "Vec<u8>",
            ColumnType::Timestamp if self.is_utc() => "chrono::DateTime<chrono::offset::Utc>",
            ColumnType::Timestamp => "chrono::DateTime<chrono::offset::Local>",
            ColumnType::DateTime if self.is_utc() => "chrono::DateTime<chrono::offset::Utc>",
            ColumnType::DateTime => "chrono::DateTime<chrono::offset::Local>",
            ColumnType::Date => "chrono::NaiveDate",
            ColumnType::Time => "chrono::NaiveTime",
            ColumnType::Decimal => "rust_decimal::Decimal",
            ColumnType::ArrayInt => "Vec<u64>",
            ColumnType::ArrayString => "Vec<String>",
            ColumnType::Json if self.json_class.is_some() => self.json_class.as_ref().unwrap(),
            ColumnType::Json => "Value",
            ColumnType::Enum => "",
            ColumnType::DbEnum => "String",
            ColumnType::DbSet => "String",
            ColumnType::Point => "Point",
            ColumnType::UnSupported => unimplemented!(),
        };
        if self.not_null {
            typ.to_owned()
        } else {
            format!("Option<{}>", typ)
        }
    }
    pub fn convert_outer_type(&self) -> &'static str {
        if let Some(ref _name) = self.class {
            return if self.not_null {
                ".into()"
            } else {
                ".map(|v| v.into())"
            };
        }
        if let Some(ref _rel) = self.rel {
            return if self.not_null {
                ".into()"
            } else {
                ".map(|v| v.into())"
            };
        }
        match self.type_def {
            ColumnType::TinyInt => "",
            ColumnType::SmallInt => "",
            ColumnType::Int => "",
            ColumnType::BigInt => "",
            ColumnType::Float => "",
            ColumnType::Double => "",
            ColumnType::Varchar if !self.not_null => ".as_deref()",
            ColumnType::Varchar => ".as_ref()",
            ColumnType::Boolean if self.not_null => " == 1",
            ColumnType::Boolean => ".map(|v| v == 1)",
            ColumnType::Text if !self.not_null => ".as_deref()",
            ColumnType::Text => ".as_ref()",
            ColumnType::Blob if !self.not_null => ".as_deref()",
            ColumnType::Blob => ".as_ref()",
            // ColumnType::Timestamp if self.not_null => ".unwrap()",
            ColumnType::Timestamp => "",
            // ColumnType::DateTime if self.not_null => ".unwrap()",
            ColumnType::DateTime => "",
            // ColumnType::Date if self.not_null => ".unwrap()",
            ColumnType::Date => "",
            // ColumnType::Time if self.not_null => ".unwrap()",
            ColumnType::Time => "",
            ColumnType::Decimal => "",
            ColumnType::ArrayInt if !self.not_null => ".as_deref()",
            ColumnType::ArrayInt => ".as_ref()",
            ColumnType::ArrayString if !self.not_null => ".as_deref()",
            ColumnType::ArrayString => ".as_ref()",
            ColumnType::Json if !self.not_null => ".as_deref()",
            ColumnType::Json => ".as_ref()",
            ColumnType::Enum => "",
            ColumnType::DbEnum if !self.not_null => ".as_deref()",
            ColumnType::DbEnum => ".as_ref()",
            ColumnType::DbSet if !self.not_null => ".as_deref()",
            ColumnType::DbSet => ".as_ref()",
            ColumnType::Point if self.not_null => ".clone().into()",
            ColumnType::Point => ".as_ref().map(|v| v.clone().into())",
            ColumnType::UnSupported => unimplemented!(),
        }
    }
    pub fn convert_serialize(&self) -> &'static str {
        match self.type_def {
            ColumnType::TinyInt => "",
            ColumnType::SmallInt => "",
            ColumnType::Int => "",
            ColumnType::BigInt => "",
            ColumnType::Float => "",
            ColumnType::Double => "",
            ColumnType::Varchar => "",
            ColumnType::Boolean if self.not_null => " == 1",
            ColumnType::Boolean => ".map(|v| v == 1)",
            ColumnType::Text => "",
            ColumnType::Blob => "",
            ColumnType::Timestamp => "",
            ColumnType::DateTime => "",
            ColumnType::Date => "",
            ColumnType::Time => "",
            ColumnType::Decimal => "",
            ColumnType::ArrayInt => "",
            ColumnType::ArrayString => "",
            ColumnType::Json => "",
            ColumnType::Enum => "",
            ColumnType::DbEnum => "",
            ColumnType::DbSet => "",
            ColumnType::Point if self.not_null => ".to_point()",
            ColumnType::Point => ".as_ref().map(|v| v.to_point())",
            ColumnType::UnSupported => unimplemented!(),
        }
    }
    pub fn get_outer_for_update_type(&self) -> String {
        let typ = if let Some(ref name) = self.class {
            name.to_string()
        } else if let Some(ref rel) = self.rel {
            let (rel_name, def) = rel;
            let name = RelDef::get_id_name(def, rel_name);
            let mod_name = RelDef::get_group_mod_name(def, rel_name);
            format!("rel_{}::{}", mod_name, name)
        } else {
            match self.type_def {
                ColumnType::TinyInt if self.signed => "i8",
                ColumnType::TinyInt => "u8",
                ColumnType::SmallInt if self.signed => "i16",
                ColumnType::SmallInt => "u16",
                ColumnType::Int if self.signed => "i32",
                ColumnType::Int => "u32",
                ColumnType::BigInt if self.signed => "i64",
                ColumnType::BigInt => "u64",
                ColumnType::Float => "f32",
                ColumnType::Double => "f64",
                ColumnType::Varchar => "&str",
                ColumnType::Boolean => "bool",
                ColumnType::Text => "&str",
                ColumnType::Blob => "Vec<u8>",
                ColumnType::Timestamp if self.is_utc() => "chrono::DateTime<chrono::offset::Utc>",
                ColumnType::Timestamp => "chrono::DateTime<chrono::offset::Local>",
                ColumnType::DateTime if self.is_utc() => "chrono::DateTime<chrono::offset::Utc>",
                ColumnType::DateTime => "chrono::DateTime<chrono::offset::Local>",
                ColumnType::Date => "chrono::NaiveDate",
                ColumnType::Time => "chrono::NaiveTime",
                ColumnType::Decimal => "rust_decimal::Decimal",
                ColumnType::ArrayInt => "Vec<u64>",
                ColumnType::ArrayString => "Vec<String>",
                ColumnType::Json if self.json_class.is_some() => self.json_class.as_ref().unwrap(),
                ColumnType::Json => "Value",
                ColumnType::Enum => "",
                ColumnType::DbEnum => "&str",
                ColumnType::DbSet => "&str",
                ColumnType::Point => "Point",
                ColumnType::UnSupported => unimplemented!(),
            }
            .to_string()
        };
        typ
    }
    pub fn accessor(&self, with_type: bool, sep: &str) -> String {
        let inner = self.get_inner_type(&true);
        let outer = self.get_outer_for_update_type();
        let null = !self.not_null;
        let mut is_num = self.type_def == ColumnType::TinyInt
            || self.type_def == ColumnType::SmallInt
            || self.type_def == ColumnType::Int
            || self.type_def == ColumnType::BigInt;
        let is_float = self.type_def == ColumnType::Float || self.type_def == ColumnType::Double;
        let mut is_ord = self.type_def != ColumnType::Boolean
            && self.type_def != ColumnType::Blob
            && self.type_def != ColumnType::ArrayInt
            && self.type_def != ColumnType::ArrayString
            && self.type_def != ColumnType::Json
            && self.type_def != ColumnType::Point;
        if self.class.is_some() || self.rel.is_some() {
            is_num = false;
            is_ord = false;
        }
        if self.primary {
            if with_type {
                format!(
                    "{}Primary{}<{}, {}>",
                    if_then_else!(null, "Null", ""),
                    sep,
                    inner,
                    outer
                )
            } else {
                format!("{}Primary", if_then_else!(null, "Null", ""))
            }
        } else if is_num {
            if with_type {
                format!(
                    "{}NullNum{}<{}>",
                    if_then_else!(null, "", "Not"),
                    sep,
                    inner
                )
            } else {
                format!("{}NullNum", if_then_else!(null, "", "Not"))
            }
        } else if is_float {
            if with_type {
                format!(
                    "{}NullFloat{}<{}>",
                    if_then_else!(null, "", "Not"),
                    sep,
                    inner
                )
            } else {
                format!("{}NullFloat", if_then_else!(null, "", "Not"))
            }
        } else if self.type_def == ColumnType::Boolean {
            format!("{}NullBool", if_then_else!(null, "", "Not"))
        } else if self.type_def == ColumnType::Text
            || self.type_def == ColumnType::Varchar
            || self.type_def == ColumnType::DbEnum
            || self.type_def == ColumnType::DbSet
        {
            format!("{}NullString", if_then_else!(null, "", "Not"))
        } else if self.type_def == ColumnType::Blob {
            if with_type {
                format!(
                    "{}NullRef{}<{}>",
                    if_then_else!(null, "", "Not"),
                    sep,
                    outer
                )
            } else {
                format!("{}NullRef", if_then_else!(null, "", "Not"))
            }
        } else if self.type_def == ColumnType::ArrayInt
            || self.type_def == ColumnType::ArrayString
            || self.type_def == ColumnType::Json
        {
            if with_type {
                format!(
                    "{}NullJson{}<{}>",
                    if_then_else!(null, "", "Not"),
                    sep,
                    outer
                )
            } else {
                format!("{}NullJson", if_then_else!(null, "", "Not"))
            }
        } else if is_ord {
            if with_type {
                format!(
                    "{}NullOrd{}<{}>",
                    if_then_else!(null, "", "Not"),
                    sep,
                    inner
                )
            } else {
                format!("{}NullOrd", if_then_else!(null, "", "Not"))
            }
        } else if with_type {
            format!(
                "{}Null{}<{}, {}>",
                if_then_else!(null, "", "Not"),
                sep,
                inner,
                outer
            )
        } else {
            format!("{}Null", if_then_else!(null, "", "Not"))
        }
    }
    pub fn convert_inner_type(&self) -> String {
        let id_str = if let Some(ref _name) = self.class {
            ".get()"
        } else if let Some(ref _rel) = self.rel {
            ".get()"
        } else {
            ""
        };
        let conv_str = match self.type_def {
            ColumnType::TinyInt => "",
            ColumnType::SmallInt => "",
            ColumnType::Int => "",
            ColumnType::BigInt => "",
            ColumnType::Float => "",
            ColumnType::Double => "",
            ColumnType::Varchar => ".to_owned()",
            ColumnType::Boolean => " as i8",
            ColumnType::Text => ".to_owned()",
            ColumnType::Blob => "",
            ColumnType::Timestamp if self.not_null => ".into()",
            ColumnType::Timestamp => "",
            ColumnType::DateTime if self.not_null => ".into()",
            ColumnType::DateTime => "",
            ColumnType::Date if self.not_null => ".into()",
            ColumnType::Date => "",
            ColumnType::Time if self.not_null => ".into()",
            ColumnType::Time => "",
            ColumnType::Decimal => "",
            ColumnType::ArrayInt => "._into_json()",
            ColumnType::ArrayString => "._into_json()",
            ColumnType::Json => "._into_json()",
            ColumnType::Enum => "",
            ColumnType::DbEnum => ".to_owned()",
            ColumnType::DbSet => ".to_owned()",
            ColumnType::Point => ".into()",
            ColumnType::UnSupported => unimplemented!(),
        };
        if !self.not_null {
            format!(".map(|v| v{}{})", id_str, conv_str)
        } else {
            format!("{}{}", id_str, conv_str)
        }
    }

    pub fn get_bind_as(&self) -> &'static str {
        if let Some(ref _name) = self.class {
            return ".get()";
        }
        if let Some(ref _rel) = self.rel {
            return ".get()";
        }
        match self.type_def {
            ColumnType::Boolean => " as i8",
            _ => "",
        }
    }

    pub fn get_col_name<'a>(&'a self, name: &'a str) -> Cow<'a, str> {
        if let Some(ref rename) = self.rename {
            rename.into()
        } else {
            name.into()
        }
    }

    pub fn is_copyable(&self) -> bool {
        let copyable = match self.type_def {
            ColumnType::TinyInt => true,
            ColumnType::SmallInt => true,
            ColumnType::Int => true,
            ColumnType::BigInt => true,
            ColumnType::Float => true,
            ColumnType::Double => true,
            ColumnType::Varchar => false,
            ColumnType::Boolean => true,
            ColumnType::Text => false,
            ColumnType::Blob => false,
            ColumnType::Timestamp => true,
            ColumnType::DateTime => true,
            ColumnType::Date => true,
            ColumnType::Time => true,
            ColumnType::Decimal => true,
            ColumnType::ArrayInt => false,
            ColumnType::ArrayString => false,
            ColumnType::Json => false,
            ColumnType::Enum => true,
            ColumnType::DbEnum => false,
            ColumnType::DbSet => false,
            ColumnType::Point => false,
            ColumnType::UnSupported => unimplemented!(),
        };
        if let Some(ref _name) = self.class {
            return copyable;
        }
        if let Some(ref _rel) = self.rel {
            return copyable;
        }
        true
    }

    pub fn placeholder(&self) -> String {
        match self.type_def {
            ColumnType::Point if self.srid.is_some() => {
                format!("ST_GeomFromWKB(?,{})", self.srid.unwrap())
            }
            ColumnType::Point => "ST_GeomFromWKB(?)".to_string(),
            _ => "?".to_owned(),
        }
    }

    pub fn clone_str(&self) -> &'static str {
        if self.is_copyable() {
            ""
        } else {
            ".clone()"
        }
    }
}
