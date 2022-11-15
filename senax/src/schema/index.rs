use convert_case::{Case, Casing};
use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{ColumnDef, ModelDef, DELETED};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Copy, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(title = "Index Type")]
pub enum IndexType {
    Index,
    Unique,
    Fulltext,
    Spatial,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Copy, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(title = "Sort Type")]
pub enum SortType {
    Asc,
    Desc,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, JsonSchema)]
#[schemars(deny_unknown_fields)]
#[schemars(title = "Index Field Def")]
pub struct IndexFieldDef {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sorting: Option<SortType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length: Option<u32>,
}

#[derive(
    Debug, PartialEq, Eq, Serialize, Deserialize, Copy, Clone, derive_more::Display, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
#[schemars(title = "Parser")]
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
            _ => panic!("unsupported parser: {}", p),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[schemars(deny_unknown_fields)]
#[schemars(title = "Index Def")]
pub struct IndexDef {
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub fields: IndexMap<String, Option<IndexFieldDef>>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_def: Option<IndexType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parser: Option<Parser>,
}

impl IndexDef {
    pub fn fields<'a>(
        &'a self,
        name: &'a String,
        model: &'a ModelDef,
    ) -> Vec<(&'a String, &'a ColumnDef)> {
        let mut ret = Vec::new();
        if !self.fields.is_empty() {
            for row in &self.fields {
                if row.0 == DELETED {
                    continue;
                }
                let col = model
                    .merged_columns
                    .get(row.0)
                    .unwrap_or_else(|| panic!("{} index is not in columns", row.0));
                ret.push((row.0, col));
            }
        } else {
            let col = model
                .merged_columns
                .get(name)
                .unwrap_or_else(|| panic!("{} index is not in columns", name));
            ret.push((name, col));
        }
        ret
    }
}
