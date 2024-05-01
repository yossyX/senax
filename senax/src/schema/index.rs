use convert_case::{Case, Casing};
use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::model_generator::template::filters::_to_db_col;

use super::{ConfigDef, FieldDef, ModelDef, _to_var_name};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Copy, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
/// ### インデックスタイプ
pub enum IndexType {
    /// ### インデックス
    Index,
    /// ### ユニーク
    Unique,
    /// ### フルテキスト
    Fulltext,
    /// ### 空間インデックス
    Spatial,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Copy, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
/// ### ソート順序
pub enum SortDirection {
    /// ### 昇順
    Asc,
    /// ### 降順
    Desc,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Default, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
/// ### インデックスフィールド定義
pub struct IndexFieldDef {
    /// ### 方向
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<SortDirection>,
    /// ### 長さ
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length: Option<u32>,
    /// ### クエリー
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
/// ### インデックスフィールド定義
pub struct IndexFieldJson {
    /// ### フィールド名
    #[schemars(regex(pattern = r"^\p{XID_Start}\p{XID_Continue}*(?<!_)$"))]
    pub name: String,
    /// ### 方向
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<SortDirection>,
    /// ### 長さ
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length: Option<u32>,
    /// ### クエリー
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
}

impl From<IndexFieldDef> for IndexFieldJson {
    fn from(value: IndexFieldDef) -> Self {
        Self {
            name: String::new(),
            direction: value.direction,
            length: value.length,
            query: value.query,
        }
    }
}

impl From<IndexFieldJson> for IndexFieldDef {
    fn from(value: IndexFieldJson) -> Self {
        Self {
            direction: value.direction,
            length: value.length,
            query: value.query,
        }
    }
}

#[derive(
    Debug, PartialEq, Eq, Serialize, Deserialize, Copy, Clone, derive_more::Display, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
/// ### パーサー
pub enum Parser {
    #[display(fmt = "ngram")]
    Ngram,
    #[display(fmt = "mecab")]
    Mecab,
}

impl From<&String> for Parser {
    fn from(p: &String) -> Self {
        match p.to_case(Case::Lower).as_str() {
            "ngram" => Parser::Ngram,
            "mecab" => Parser::Mecab,
            _ => error_exit!("unsupported parser: {}", p),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
/// ### インデックス定義
pub struct IndexDef {
    /// ### フィールド
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub fields: IndexMap<String, Option<IndexFieldDef>>,
    /// ### タイプ
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub index_type: Option<IndexType>,
    /// ### パーサー
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parser: Option<Parser>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
/// ### インデックス定義
pub struct IndexJson {
    /// ### インデックス名
    #[schemars(regex(pattern = r"^\p{XID_Start}\p{XID_Continue}*(?<!_)$"))]
    pub name: String,
    /// ### フィールド
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<IndexFieldJson>,
    /// ### タイプ
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub index_type: Option<IndexType>,
    /// ### パーサー
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parser: Option<Parser>,
}

impl From<IndexDef> for IndexJson {
    fn from(value: IndexDef) -> Self {
        Self {
            name: Default::default(),
            fields: value
                .fields
                .into_iter()
                .map(|(k, v)| {
                    let mut v: IndexFieldJson = v.unwrap_or_default().into();
                    v.name = k;
                    v
                })
                .collect(),
            index_type: value.index_type,
            parser: value.parser,
        }
    }
}

impl From<IndexJson> for IndexDef {
    fn from(value: IndexJson) -> Self {
        Self {
            fields: value
                .fields
                .into_iter()
                .map(|v| {
                    let name = v.name.clone();
                    let v: IndexFieldDef = v.into();
                    if v == IndexFieldDef::default() {
                        (name, None)
                    } else {
                        (name, Some(v))
                    }
                })
                .collect(),
            index_type: value.index_type,
            parser: value.parser,
        }
    }
}

impl IndexDef {
    pub fn fields<'a>(
        &'a self,
        name: &'a String,
        model: &'a ModelDef,
    ) -> Vec<(&'a String, &'a FieldDef)> {
        let mut ret = Vec::new();
        if !self.fields.is_empty() {
            for row in &self.fields {
                if row.0 == ConfigDef::deleted().as_str() {
                    continue;
                }
                let col = model.merged_fields.get(row.0).unwrap_or_else(|| {
                    error_exit!("There is no {} column on the {} model.", row.0, model.name)
                });
                ret.push((row.0, col));
            }
        } else {
            let col = model.merged_fields.get(name).unwrap_or_else(|| {
                error_exit!("There is no {} column on the {} model.", name, model.name)
            });
            ret.push((name, col));
        }
        ret
    }
    pub fn join_fields(&self, model: &ModelDef, tpl: &str, sep: &str) -> String {
        let mut v = Vec::new();
        for (index, (name, _)) in (&self.fields).into_iter().enumerate() {
            let col = model.merged_fields.get(name).unwrap_or_else(|| {
                error_exit!("There is no {} column on the {} model.", name, model.name)
            });
            v.push(
                tpl.replace("{name}", name)
                    .replace("{var}", &_to_var_name(name))
                    .replace("{type}", &col.get_filter_type(super::domain_mode()))
                    .replace("{col_esc}", &_to_db_col(name, true))
                    .replace("{index}", &index.to_string())
                    .replace("{bind_as_for_filter}", col.get_bind_as_for_filter())
                    .replace("{placeholder}", &col.placeholder())
                    .replace("{filter_check_eq}", &col.get_filter_eq(Some(index)))
                    .replace("{filter_check_cmp}", &col.get_filter_cmp(Some(index))),
            );
        }
        v.join(sep)
    }
}
