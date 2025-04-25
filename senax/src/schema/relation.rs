use convert_case::{Case, Casing};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::common::to_singular;
use crate::schema::_to_var_name;

use super::{FieldDef, GROUPS, ModelDef, domain_mode, to_id_name};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Copy, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
/// ### リレーションタイプ
pub enum RelationsType {
    HasMany,
    BelongsTo,
    HasOne,
    BelongsToOuterDb,
}

#[derive(
    Debug, PartialEq, Eq, Serialize, Deserialize, Copy, Clone, JsonSchema, derive_more::Display,
)]
#[serde(rename_all = "snake_case")]
/// ### 参照オプション
pub enum ReferenceOption {
    #[display("restrict")]
    Restrict,
    #[display("cascade")]
    Cascade,
    #[display("set_null")]
    SetNull,
    // NoAction,
    #[display("set_zero")]
    SetZero,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(untagged)]
/// ### IDまたはIDの配列
pub enum StringOrArray {
    One(String),
    Many(Vec<String>),
}
impl StringOrArray {
    pub fn to_vec(&self) -> Vec<String> {
        match self {
            StringOrArray::One(v) => vec![v.to_string()],
            StringOrArray::Many(v) => v.clone(),
        }
    }
    pub fn from_vec(value: Option<Vec<String>>) -> Option<StringOrArray> {
        let value: Option<Vec<String>> = value.map(|v| {
            v.iter()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
                .collect()
        });
        if let Some(mut value) = value {
            if value.is_empty() {
                None
            } else if value.len() == 1 {
                Some(StringOrArray::One(value.pop().unwrap()))
            } else {
                Some(StringOrArray::Many(value))
            }
        } else {
            None
        }
    }
    #[allow(dead_code)]
    pub fn last(&self) -> &str {
        match self {
            StringOrArray::One(v) => v,
            StringOrArray::Many(v) => v.last().unwrap(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
/// ### has_oneリレーション定義
pub struct HasOneDef {
    /// ### 論理名
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// ### コメント
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// ### 結合先のグループ
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// ### 結合先のモデル
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// ### 結合先のフィールド名
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foreign: Option<StringOrArray>,
    /// ### 親モデルのキャッシュに含まれない
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub disable_cache: bool,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
/// ### has_oneリレーション定義
pub struct HasOneJson {
    /// ### リレーション名
    /// 単数形
    #[schemars(regex(pattern = r"^\p{XID_Start}\p{XID_Continue}*(?<!_)$"))]
    pub name: String,
    /// ### 論理名
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// ### コメント
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// ### 結合先のグループ
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// ### 結合先のモデル
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// ### 結合先のフィールド名
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(inner(regex(pattern = r"^\p{XID_Start}\p{XID_Continue}*(?<!_)$")))]
    pub foreign: Option<Vec<String>>,
    /// ### 親モデルのキャッシュに含まれない
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub disable_cache: bool,
}

impl From<HasOneDef> for HasOneJson {
    fn from(value: HasOneDef) -> Self {
        Self {
            name: Default::default(),
            label: value.label,
            comment: value.comment,
            group: value.group,
            model: value.model,
            foreign: value.foreign.map(|v| v.to_vec()),
            disable_cache: value.disable_cache,
        }
    }
}

impl From<HasOneJson> for HasOneDef {
    fn from(value: HasOneJson) -> Self {
        Self {
            label: value.label,
            comment: value.comment,
            group: value.group,
            model: value.model,
            foreign: StringOrArray::from_vec(value.foreign),
            disable_cache: value.disable_cache,
        }
    }
}

impl From<&HasOneDef> for RelDef {
    fn from(value: &HasOneDef) -> Self {
        Self {
            label: value.label.clone(),
            comment: value.comment.clone(),
            model: if let Some(group) = &value.group {
                format!("{}::{}", group, value.model.as_deref().unwrap_or_default())
            } else {
                value.model.clone().unwrap_or_default()
            },
            rel_type: Some(RelationsType::HasOne),
            foreign: value.foreign.as_ref().map(|v| v.to_vec()),
            in_cache: !value.disable_cache,
            ..Default::default()
        }
    }
}
impl HasOneDef {
    pub fn convert(rel: &Option<HasOneDef>, group: &str, name: &str) -> RelDef {
        crate::common::check_name(name);
        if let Some(mut d) = rel.as_ref().map(RelDef::from) {
            if d.model.is_empty() {
                d.model = format!("{}::{}", group, name);
            } else if d.model.contains(MODEL_NAME_SPLITTER) {
                let (group_name, stem_name) = d.model.split_once(MODEL_NAME_SPLITTER).unwrap();
                crate::common::check_name(group_name);
                if stem_name.is_empty() {
                    d.model = format!("{}::{}", group_name, name);
                } else {
                    crate::common::check_name(stem_name);
                }
            } else {
                crate::common::check_name(&d.model);
                d.model = format!("{}::{}", group, d.model);
            }
            d
        } else {
            RelDef {
                model: format!("{}::{}", group, name),
                rel_type: Some(RelationsType::HasOne),
                in_cache: true,
                ..Default::default()
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
/// ### has_manyリレーション定義
pub struct HasManyDef {
    /// ### 論理名
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// ### コメント
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// ### 結合先のグループ
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// ### 結合先のモデル
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// ### 結合先のフィールド名
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foreign: Option<StringOrArray>,
    /// ### 親モデルのキャッシュに含まれない
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub disable_cache: bool,
    /// ### 追加条件クエリー
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_filter: Option<String>,
    /// ### ソートフィールド
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_by: Option<String>,
    /// ### 降順ソート
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub desc: bool,
    /// ### 取得数
    /// 参照時のみ適用される
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
/// ### has_manyリレーション定義
pub struct HasManyJson {
    /// ### リレーション名
    /// 複数形
    #[schemars(regex(pattern = r"^\p{XID_Start}\p{XID_Continue}*(?<!_)$"))]
    pub name: String,
    /// ### 論理名
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// ### コメント
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// ### 結合先のグループ
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// ### 結合先のモデル
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// ### 結合先のフィールド名
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(inner(regex(pattern = r"^\p{XID_Start}\p{XID_Continue}*(?<!_)$")))]
    pub foreign: Option<Vec<String>>,
    /// ### 親モデルのキャッシュに含まれない
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub disable_cache: bool,
    /// ### 追加条件クエリー
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_filter: Option<String>,
    /// ### ソートフィールド
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_by: Option<String>,
    /// ### 降順ソート
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub desc: bool,
    /// ### 取得数
    /// 参照時のみ適用される
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

impl From<HasManyDef> for HasManyJson {
    fn from(value: HasManyDef) -> Self {
        Self {
            name: Default::default(),
            label: value.label,
            comment: value.comment,
            group: value.group,
            model: value.model,
            foreign: value.foreign.map(|v| v.to_vec()),
            disable_cache: value.disable_cache,
            additional_filter: value.additional_filter,
            order_by: value.order_by,
            desc: value.desc,
            limit: value.limit,
        }
    }
}

impl From<HasManyJson> for HasManyDef {
    fn from(value: HasManyJson) -> Self {
        Self {
            label: value.label,
            comment: value.comment,
            group: value.group,
            model: value.model,
            foreign: StringOrArray::from_vec(value.foreign),
            disable_cache: value.disable_cache,
            additional_filter: value.additional_filter,
            order_by: value.order_by,
            desc: value.desc,
            limit: value.limit,
        }
    }
}

impl From<&HasManyDef> for RelDef {
    fn from(value: &HasManyDef) -> Self {
        Self {
            label: value.label.clone(),
            comment: value.comment.clone(),
            model: if let Some(group) = &value.group {
                format!("{}::{}", group, value.model.as_deref().unwrap_or_default())
            } else {
                value.model.clone().unwrap_or_default()
            },
            rel_type: Some(RelationsType::HasMany),
            foreign: value.foreign.as_ref().map(|v| v.to_vec()),
            in_cache: !value.disable_cache,
            additional_filter: value.additional_filter.clone(),
            order_by: value.order_by.clone(),
            desc: value.desc,
            limit: value.limit,
            ..Default::default()
        }
    }
}
impl HasManyDef {
    pub fn convert(rel: &Option<HasManyDef>, group: &str, name: &str) -> RelDef {
        crate::common::check_name(name);
        if let Some(mut d) = rel.as_ref().map(RelDef::from) {
            if d.model.is_empty() {
                d.model = format!("{}::{}", group, name);
            } else if d.model.contains(MODEL_NAME_SPLITTER) {
                let (group_name, stem_name) = d.model.split_once(MODEL_NAME_SPLITTER).unwrap();
                crate::common::check_name(group_name);
                if stem_name.is_empty() {
                    d.model = format!("{}::{}", group_name, name);
                } else {
                    crate::common::check_name(stem_name);
                }
            } else {
                crate::common::check_name(&d.model);
                d.model = format!("{}::{}", group, d.model);
            }
            d
        } else {
            RelDef {
                model: format!("{}::{}", group, name),
                rel_type: Some(RelationsType::HasMany),
                in_cache: true,
                ..Default::default()
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
/// ### belongs_toリレーション定義
pub struct BelongsToDef {
    /// ### 論理名
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// ### コメント
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// ### 結合先のグループ
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// ### 結合先のモデル
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// ### 結合するローカルのフィールド名
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local: Option<StringOrArray>,
    /// ### リレーション先が論理削除されていても取得する
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub with_trashed: bool,
    /// ### リレーションのインデックスを設定しない
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub disable_index: bool,
    /// ### カスケード削除
    /// DBの外部キー制約による削除およびソフトウェア側での削除制御
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_delete: Option<ReferenceOption>,
    /// ### カスケード更新
    /// DBの外部キー制約による更新
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_update: Option<ReferenceOption>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
/// ### belongs_toリレーション定義
pub struct BelongsToJson {
    /// ### リレーション名
    /// 単数形
    #[schemars(regex(pattern = r"^\p{XID_Start}\p{XID_Continue}*(?<!_)$"))]
    pub name: String,
    /// ### 論理名
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// ### コメント
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// ### 結合先のグループ
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// ### 結合先のモデル
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// ### 結合するローカルのフィールド名
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(inner(regex(pattern = r"^\p{XID_Start}\p{XID_Continue}*(?<!_)$")))]
    pub local: Option<Vec<String>>,
    /// ### リレーション先が論理削除されていても取得する
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub with_trashed: bool,
    /// ### リレーションのインデックスを設定しない
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub disable_index: bool,
    /// ### カスケード削除
    /// DBの外部キー制約による削除およびソフトウェア側での削除制御
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_delete: Option<ReferenceOption>,
    /// ### カスケード更新
    /// DBの外部キー制約による更新
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_update: Option<ReferenceOption>,
}

impl From<BelongsToDef> for BelongsToJson {
    fn from(value: BelongsToDef) -> Self {
        Self {
            name: Default::default(),
            label: value.label,
            comment: value.comment,
            group: value.group,
            model: value.model,
            local: value.local.map(|v| v.to_vec()),
            with_trashed: value.with_trashed,
            disable_index: value.disable_index,
            on_delete: value.on_delete,
            on_update: value.on_update,
        }
    }
}

impl From<BelongsToJson> for BelongsToDef {
    fn from(value: BelongsToJson) -> Self {
        Self {
            label: value.label,
            comment: value.comment,
            group: value.group,
            model: value.model,
            local: StringOrArray::from_vec(value.local),
            with_trashed: value.with_trashed,
            disable_index: value.disable_index,
            on_delete: value.on_delete,
            on_update: value.on_update,
        }
    }
}

impl From<&BelongsToDef> for RelDef {
    fn from(value: &BelongsToDef) -> Self {
        Self {
            label: value.label.clone(),
            comment: value.comment.clone(),
            model: if let Some(group) = &value.group {
                format!("{}::{}", group, value.model.as_deref().unwrap_or_default())
            } else {
                value.model.clone().unwrap_or_default()
            },
            rel_type: Some(RelationsType::BelongsTo),
            local: value.local.as_ref().map(|v| v.to_vec()),
            with_trashed: value.with_trashed,
            disable_index: value.disable_index,
            on_delete: value.on_delete,
            on_update: value.on_update,
            ..Default::default()
        }
    }
}
impl BelongsToDef {
    pub fn convert(rel: &Option<BelongsToDef>, group: &str, name: &str) -> RelDef {
        crate::common::check_name(name);
        if let Some(mut d) = rel.as_ref().map(RelDef::from) {
            if d.model.is_empty() {
                d.model = format!("{}::{}", group, name);
            } else if d.model.contains(MODEL_NAME_SPLITTER) {
                let (group_name, stem_name) = d.model.split_once(MODEL_NAME_SPLITTER).unwrap();
                crate::common::check_name(group_name);
                if stem_name.is_empty() {
                    d.model = format!("{}::{}", group_name, name);
                } else {
                    crate::common::check_name(stem_name);
                }
            } else {
                crate::common::check_name(&d.model);
                d.model = format!("{}::{}", group, d.model);
            }
            d
        } else {
            RelDef {
                model: format!("{}::{}", group, name),
                rel_type: Some(RelationsType::BelongsTo),
                ..Default::default()
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
/// ### belongs_toリレーション定義
pub struct BelongsToOuterDbDef {
    /// ### 論理名
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// ### コメント
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// ### 結合先のデータベース
    pub db: String,
    /// ### 結合先のグループ
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// ### 結合先のモデル
    pub model: String,
    /// ### 結合するローカルのフィールド名
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local: Option<StringOrArray>,
    /// ### リレーション先が論理削除されていても取得する
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub with_trashed: bool,
    /// ### リレーションのインデックスを設定しない
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub disable_index: bool,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
/// ### belongs_toリレーション定義
pub struct BelongsToOuterDbJson {
    /// ### リレーション名
    /// 単数形
    #[schemars(regex(pattern = r"^\p{XID_Start}\p{XID_Continue}*(?<!_)$"))]
    pub name: String,
    /// ### 論理名
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// ### コメント
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// ### 結合先のデータベース
    pub db: String,
    /// ### 結合先のグループ
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// ### 結合先のモデル
    pub model: String,
    /// ### 結合するローカルのフィールド名
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(inner(regex(pattern = r"^\p{XID_Start}\p{XID_Continue}*(?<!_)$")))]
    pub local: Option<Vec<String>>,
    /// ### リレーション先が論理削除されていても取得する
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub with_trashed: bool,
    /// ### リレーションのインデックスを設定しない
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub disable_index: bool,
}

impl From<BelongsToOuterDbDef> for BelongsToOuterDbJson {
    fn from(value: BelongsToOuterDbDef) -> Self {
        Self {
            name: Default::default(),
            label: value.label,
            comment: value.comment,
            db: value.db,
            group: value.group,
            model: value.model,
            local: value.local.map(|v| v.to_vec()),
            with_trashed: value.with_trashed,
            disable_index: value.disable_index,
        }
    }
}

impl From<BelongsToOuterDbJson> for BelongsToOuterDbDef {
    fn from(value: BelongsToOuterDbJson) -> Self {
        Self {
            label: value.label,
            comment: value.comment,
            db: value.db,
            group: value.group,
            model: value.model,
            local: StringOrArray::from_vec(value.local),
            with_trashed: value.with_trashed,
            disable_index: value.disable_index,
        }
    }
}
impl From<&BelongsToOuterDbDef> for RelDef {
    fn from(value: &BelongsToOuterDbDef) -> Self {
        Self {
            label: value.label.clone(),
            comment: value.comment.clone(),
            db: Some(value.db.clone()),
            model: if let Some(group) = &value.group {
                format!("{}::{}", group, value.model)
            } else {
                value.model.clone()
            },
            rel_type: Some(RelationsType::BelongsToOuterDb),
            local: value.local.as_ref().map(|v| v.to_vec()),
            with_trashed: value.with_trashed,
            disable_index: value.disable_index,
            ..Default::default()
        }
    }
}
impl BelongsToOuterDbDef {
    pub fn convert(rel: &BelongsToOuterDbDef, group: &str) -> RelDef {
        let mut d: RelDef = rel.into();
        if d.model.contains(MODEL_NAME_SPLITTER) {
            let (group_name, stem_name) = d.model.split_once(MODEL_NAME_SPLITTER).unwrap();
            crate::common::check_name(group_name);
            crate::common::check_name(stem_name);
        } else {
            crate::common::check_name(&d.model);
            d.model = format!("{}::{}", group, d.model);
        }
        d
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Clone, Default)]
pub struct RelDef {
    pub in_abstract: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    pub db: Option<String>,
    pub model: String,
    #[serde(rename = "type")]
    pub rel_type: Option<RelationsType>,
    pub local: Option<Vec<String>>,
    pub foreign: Option<Vec<String>>,
    pub in_cache: bool,
    pub additional_filter: Option<String>,
    pub order_by: Option<String>,
    pub desc: bool,
    pub limit: Option<u32>,
    pub with_trashed: bool,
    pub on_delete: Option<ReferenceOption>,
    pub on_update: Option<ReferenceOption>,
    pub disable_index: bool,
}
pub const MODEL_NAME_SPLITTER: &str = "::";
impl RelDef {
    pub fn is_type_of_has(&self) -> bool {
        self.rel_type.unwrap() == RelationsType::HasMany
            || self.rel_type.unwrap() == RelationsType::HasOne
    }
    pub fn is_type_of_has_many(&self) -> bool {
        self.rel_type.unwrap() == RelationsType::HasMany
    }
    pub fn is_type_of_has_one(&self) -> bool {
        self.rel_type.unwrap() == RelationsType::HasOne
    }
    pub fn is_type_of_belongs_to(&self) -> bool {
        self.rel_type.unwrap() == RelationsType::BelongsTo
    }
    pub fn is_type_of_belongs_to_outer_db(&self) -> bool {
        self.rel_type.unwrap() == RelationsType::BelongsToOuterDb
    }
    pub fn db(&self) -> &str {
        self.db.as_deref().unwrap_or("--RELATION HAS NO DB--")
    }
    pub fn get_foreign_class_name(&self) -> String {
        if domain_mode() {
            self.get_foreign_model_name().1.to_case(Case::Pascal)
        } else {
            format!("_{}", self.get_foreign_model_name().1.to_case(Case::Pascal))
        }
    }
    pub fn get_id_name(&self) -> String {
        if self.is_type_of_belongs_to_outer_db() {
            let (_group_name, stem_name) = self.model.split_once(MODEL_NAME_SPLITTER).unwrap();
            return to_id_name(stem_name);
        }
        let target_model = self.get_foreign_model();
        if target_model.id().is_empty() {
            error_exit!(
                "The {} model needs a main_primary field.",
                target_model.name
            );
        }
        to_id_name(&self.get_foreign_model_name().1)
    }
    pub fn get_group_var(&self) -> String {
        _to_var_name(&self.get_group_name().to_case(Case::Snake))
    }
    pub fn get_mod_name(&self) -> String {
        self.get_foreign_model_name().0.to_case(Case::Snake)
    }
    pub fn get_group_mod_name(&self) -> String {
        if let Some(db) = &self.db {
            format!("{}_{}_{}", db, self.get_group_name().to_case(Case::Snake), self.get_mod_name())
        } else {
            format!("{}_{}", self.get_group_name().to_case(Case::Snake), self.get_mod_name())
        }
    }
    pub fn get_group_mod_var(&self) -> String {
        format!(
            "{}::{}",
            _to_var_name(&self.get_group_name().to_case(Case::Snake)),
            _to_var_name(&self.get_mod_name())
        )
    }
    pub fn get_base_group_mod_var(&self) -> String {
        format!(
            "{}::_base::_{}",
            _to_var_name(&self.get_group_name().to_case(Case::Snake)),
            &self.get_mod_name()
        )
    }
    pub fn get_local_id(&self, name: &str) -> Vec<String> {
        match self.local {
            None => vec![format!("{}_id", name)],
            Some(ref local) => local.to_owned(),
        }
    }
    pub fn get_local_cols<'a>(
        &self,
        name: &'a str,
        model: &'a ModelDef,
    ) -> Vec<(&'a String, &'a FieldDef)> {
        let ids = self.get_local_id(name);
        let mut result = Vec::new();
        for id in &ids {
            if let Some(v) = model.merged_fields.get_key_value(id) {
                result.push(v);
            }
        }
        result
    }
    pub fn get_foreign_id(&self, model: &ModelDef) -> Vec<String> {
        match self.foreign {
            None => vec![format!("{}_id", model.name)],
            Some(ref foreign) => foreign.to_owned(),
        }
    }
    pub fn get_foreign_cols(&self, model: &ModelDef) -> Vec<(String, FieldDef)> {
        let ids = self.get_foreign_id(model);
        let target_model = self.get_foreign_model();
        let mut result = Vec::new();
        for id in &ids {
            result.push(
                target_model
                    .merged_fields
                    .get_key_value(id)
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .unwrap_or_else(|| {
                        error_exit!("{} is not defined in {} model", id, target_model.name)
                    }),
            )
        }
        result
    }
    pub fn get_foreign_table_name(&self) -> String {
        self.get_foreign_model().table_name()
    }

    pub fn get_foreign_model(&self) -> Arc<ModelDef> {
        let (group_name, stem_name) = self.model.split_once(MODEL_NAME_SPLITTER).unwrap();
        get_model(group_name, stem_name)
    }

    pub fn get_foreign_model_name(&self) -> (String, String) {
        if self.is_type_of_belongs_to_outer_db() {
            let (_group_name, stem_name) = self.model.split_once(MODEL_NAME_SPLITTER).unwrap();
            (stem_name.to_case(Case::Snake), stem_name.to_string())
        } else {
            let (group_name, stem_name) = self.model.split_once(MODEL_NAME_SPLITTER).unwrap();
            get_model_name(group_name, stem_name)
        }
    }

    pub fn get_model_by_name(name: &str, cur_group_name: &str) -> Arc<ModelDef> {
        let (group_name, stem_name) = if name.contains(MODEL_NAME_SPLITTER) {
            let (group_name, stem_name) = name.split_once(MODEL_NAME_SPLITTER).unwrap();
            crate::common::check_name(group_name);
            crate::common::check_name(stem_name);
            (group_name, stem_name)
        } else {
            (cur_group_name, name)
        };
        get_model(group_name, stem_name)
    }

    pub fn get_group_name(&self) -> String {
        let (group_name, _) = self.model.split_once(MODEL_NAME_SPLITTER).unwrap();
        group_name.to_string()
    }

    pub fn in_cache(&self) -> bool {
        let target_model = self.get_foreign_model();
        target_model.use_cache() && self.in_cache
    }
}

fn get_model(group_name: &str, stem_name: &str) -> Arc<ModelDef> {
    if let Some(model) = GROUPS
        .read()
        .unwrap()
        .as_ref()
        .unwrap()
        .get(group_name)
        .unwrap_or_else(|| error_exit!("{} group is not defined", group_name))
        .get(stem_name)
    {
        return model.clone();
    }
    let singular_name = to_singular(stem_name);
    GROUPS
        .read()
        .unwrap()
        .as_ref()
        .unwrap()
        .get(group_name)
        .unwrap_or_else(|| error_exit!("{} group is not defined", group_name))
        .get(&singular_name)
        .unwrap_or_else(|| error_exit!("{} model is not defined", stem_name))
        .clone()
}

fn get_model_name(group_name: &str, stem_name: &str) -> (String, String) {
    if let Some(model) = GROUPS
        .read()
        .unwrap()
        .as_ref()
        .unwrap()
        .get(group_name)
        .unwrap_or_else(|| error_exit!("{} group is not defined", group_name))
        .get(stem_name)
    {
        return (model.mod_name().to_string(), model.name.clone());
    }
    let singular_name = to_singular(stem_name);
    let group_lock = GROUPS.read().unwrap();
    let model = group_lock
        .as_ref()
        .unwrap()
        .get(group_name)
        .unwrap_or_else(|| error_exit!("{} group is not defined", group_name))
        .get(&singular_name)
        .unwrap_or_else(|| error_exit!("{} model is not defined", stem_name));
    (model.mod_name().to_string(), model.name.clone())
}
