use crate::{
    common::yaml_value_to_str,
    schema::{FieldDef, ModelDef},
};
use indexmap::IndexMap;
use schemars::{
    JsonSchema,
    schema::{InstanceType, Schema, SchemaObject, SingleOrVec},
};
use serde::{Deserialize, Serialize};
use std::sync::{Mutex, RwLock};
use validator::{Validate, ValidationError};

pub static API_CONFIG: RwLock<Option<ApiConfigDef>> = RwLock::new(None);

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
/// ### API設定
pub struct ApiConfigDef {
    /// ### キャメルケースを使用する
    #[serde(default)]
    pub camel_case: bool,
    /// ### APIのスキーマに論理名を設定する
    #[serde(default)]
    pub with_label: bool,
    /// ### APIのスキーマにコメントを設定する
    #[serde(default)]
    pub with_comment: bool,
    /// ### タイムスタンプを非表示にする
    #[serde(default, skip_serializing_if = "is_false")]
    pub hide_timestamp: bool,
    /// ### セレクタ取得数デフォルト上限
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selector_limit: Option<u64>,
    /// ### GraphQLを無効化する
    /// (未実装)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disable_graphql: Option<bool>,
    /// ### JSON APIを使用する
    /// (未実装)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enable_json_api: Option<bool>,
    /// ### ストリーミング取得APIを使用する
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enable_streaming_api: Option<bool>,
    /// ### 権限
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub roles: IndexMap<String, Option<ApiRoleDef>>,
    /// ### デフォルト権限
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_role: Option<String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
/// ### API設定
pub struct ApiConfigJson {
    /// ### キャメルケースを使用する
    #[serde(default)]
    pub camel_case: bool,
    /// ### APIのスキーマに論理名を設定する
    #[serde(default)]
    pub with_label: bool,
    /// ### APIのスキーマにコメントを設定する
    #[serde(default)]
    pub with_comment: bool,
    /// ### タイムスタンプを非表示にする
    #[serde(default, skip_serializing_if = "is_false")]
    pub hide_timestamp: bool,
    /// ### セレクタ取得数デフォルト上限
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selector_limit: Option<u64>,
    /// ### GraphQLを無効化する
    /// (未実装)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disable_graphql: Option<bool>,
    /// ### JSON APIを使用する
    /// (未実装)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enable_json_api: Option<bool>,
    /// ### ストリーミング取得APIを使用する
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enable_streaming_api: Option<bool>,
    /// ### 権限
    #[serde(default)]
    pub roles: Vec<ApiRoleJson>,
    /// ### デフォルト権限
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_role: Option<String>,
}

impl From<ApiConfigDef> for ApiConfigJson {
    fn from(value: ApiConfigDef) -> Self {
        Self {
            camel_case: value.camel_case,
            with_label: value.with_label,
            with_comment: value.with_comment,
            hide_timestamp: value.hide_timestamp,
            selector_limit: value.selector_limit,
            disable_graphql: value.disable_graphql,
            enable_json_api: value.enable_json_api,
            enable_streaming_api: value.enable_streaming_api,
            roles: value
                .roles
                .into_iter()
                .map(|(k, v)| {
                    let mut v: ApiRoleJson = v.unwrap_or_default().into();
                    v.name = k;
                    v
                })
                .collect(),
            default_role: value.default_role,
        }
    }
}

impl From<ApiConfigJson> for ApiConfigDef {
    fn from(value: ApiConfigJson) -> Self {
        Self {
            camel_case: value.camel_case,
            with_label: value.with_label,
            with_comment: value.with_comment,
            hide_timestamp: value.hide_timestamp,
            selector_limit: value.selector_limit,
            disable_graphql: value.disable_graphql,
            enable_json_api: value.enable_json_api,
            enable_streaming_api: value.enable_streaming_api,
            roles: value
                .roles
                .into_iter()
                .map(|v| {
                    let name = v.name.clone();
                    let v: ApiRoleDef = v.into();
                    if v == ApiRoleDef::default() {
                        (name, None)
                    } else {
                        (name, Some(v))
                    }
                })
                .collect(),
            default_role: value.default_role,
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
/// ### API権限設定
pub struct ApiRoleDef {
    /// ### シリアライズ名
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
/// ### API権限設定
pub struct ApiRoleJson {
    /// ### 権限名
    #[schemars(regex(pattern = r"^[A-Za-z][_0-9A-Za-z]*(?<!_)$"))]
    pub name: String,
    /// ### シリアライズ名
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
}
impl From<ApiRoleDef> for ApiRoleJson {
    fn from(value: ApiRoleDef) -> Self {
        Self {
            name: String::new(),
            alias: value.alias,
        }
    }
}
impl From<ApiRoleJson> for ApiRoleDef {
    fn from(value: ApiRoleJson) -> Self {
        Self { alias: value.alias }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
/// ### API DB設定
pub struct ApiDbDef {
    /// ### データベース名
    /// データベースパスと対象のデータベースが異なるときに指定する
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub db: Option<String>,
    /// ### キャメルケースを使用する
    #[serde(default)]
    pub camel_case: Option<bool>,
    /// ### APIのスキーマに論理名を設定する
    #[serde(default)]
    pub with_label: Option<bool>,
    /// ### APIのスキーマにコメントを設定する
    #[serde(default)]
    pub with_comment: Option<bool>,
    /// ### タイムスタンプを非表示にする
    #[serde(default)]
    pub hide_timestamp: Option<bool>,
    /// ### 下位のグループパスを昇格する
    #[serde(default, skip_serializing_if = "is_false")]
    pub promote_group_paths: bool,
    /// ### グループ
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub groups: IndexMap<String, Option<ApiGroupDef>>,
}
impl ApiDbDef {
    pub fn fix(&mut self) {
        let roles = API_CONFIG.read().unwrap().as_ref().unwrap().roles.clone();
        for (_, group_def) in self.groups.iter_mut() {
            if let Some(group_def) = group_def {
                group_def.readable_roles.retain(|v| roles.contains_key(v));
                group_def.creatable_roles.retain(|v| roles.contains_key(v));
                group_def.importable_roles.retain(|v| roles.contains_key(v));
                group_def.updatable_roles.retain(|v| roles.contains_key(v));
                group_def.deletable_roles.retain(|v| roles.contains_key(v));
            }
        }
    }
    pub fn camel_case(&self) -> bool {
        self.camel_case
            .unwrap_or_else(|| API_CONFIG.read().unwrap().as_ref().unwrap().camel_case)
    }
    pub fn with_label(&self) -> bool {
        self.with_label
            .unwrap_or_else(|| API_CONFIG.read().unwrap().as_ref().unwrap().with_label)
    }
    pub fn with_comment(&self) -> bool {
        self.with_comment
            .unwrap_or_else(|| API_CONFIG.read().unwrap().as_ref().unwrap().with_comment)
    }
    pub fn hide_timestamp(&self) -> bool {
        self.hide_timestamp
            .unwrap_or_else(|| API_CONFIG.read().unwrap().as_ref().unwrap().hide_timestamp)
    }

    pub fn promote_group_children(&self, group_route: &str) -> bool {
        let group = self
            .groups
            .get(group_route)
            .cloned()
            .unwrap_or_default()
            .unwrap_or_default();
        group.promote_model_paths
    }

    pub fn graphql_name(&self, db_route: &str, group_route: &str, model_route: &str) -> String {
        use crate::common::ToCase;
        format!(
            "{}{}{}",
            if self.promote_group_paths {
                String::new()
            } else {
                db_route.to_pascal()
            },
            if self.promote_group_children(group_route) {
                String::new()
            } else {
                group_route.to_pascal()
            },
            model_route.to_pascal()
        )
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
/// ### API DB設定
pub struct ApiDbJson {
    /// ### データベース名
    /// データベースパスと対象のデータベースが異なるときに指定する
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub db: Option<String>,
    /// ### キャメルケースを使用する
    #[serde(default)]
    pub camel_case: Option<bool>,
    /// ### APIのスキーマに論理名を設定する
    #[serde(default)]
    pub with_label: Option<bool>,
    /// ### APIのスキーマにコメントを設定する
    #[serde(default)]
    pub with_comment: Option<bool>,
    /// ### タイムスタンプを非表示にする
    #[serde(default)]
    pub hide_timestamp: Option<bool>,
    /// ### 下位のグループパスを昇格する
    #[serde(default, skip_serializing_if = "is_false")]
    pub promote_group_paths: bool,
    /// ### グループ
    #[serde(default)]
    pub groups: Vec<ApiGroupJson>,
}

impl From<ApiDbDef> for ApiDbJson {
    fn from(value: ApiDbDef) -> Self {
        Self {
            db: value.db,
            camel_case: value.camel_case,
            with_label: value.with_label,
            with_comment: value.with_comment,
            hide_timestamp: value.hide_timestamp,
            promote_group_paths: value.promote_group_paths,
            groups: value
                .groups
                .into_iter()
                .map(|(k, v)| {
                    let mut v: ApiGroupJson = v.unwrap_or_default().into();
                    v.name.clone_from(&k);
                    v._name = Some(k);
                    v
                })
                .collect(),
        }
    }
}

impl From<ApiDbJson> for ApiDbDef {
    fn from(value: ApiDbJson) -> Self {
        Self {
            db: value.db,
            camel_case: value.camel_case,
            with_label: value.with_label,
            with_comment: value.with_comment,
            hide_timestamp: value.hide_timestamp,
            promote_group_paths: value.promote_group_paths,
            groups: value
                .groups
                .into_iter()
                .map(|v| {
                    let name = v.name.clone();
                    let v: ApiGroupDef = v.into();
                    if v == ApiGroupDef::default() {
                        (name, None)
                    } else {
                        (name, Some(v))
                    }
                })
                .collect(),
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
/// ### APIグループ設定
pub struct ApiGroupDef {
    /// ### グループ名
    /// グループパスと対象のグループが異なるときに指定する
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// ### 下位のモデルパスを昇格する
    #[serde(default, skip_serializing_if = "is_false")]
    pub promote_model_paths: bool,
    /// ### デフォルト参照権限
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub readable_roles: Vec<String>,
    /// ### デフォルト登録権限
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub creatable_roles: Vec<String>,
    /// ### デフォルトインポート権限
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub importable_roles: Vec<String>,
    /// ### デフォルト更新権限
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub updatable_roles: Vec<String>,
    /// ### デフォルト削除権限
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deletable_roles: Vec<String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
/// ### APIグループ設定
pub struct ApiGroupJson {
    /// ### グループパス
    #[schemars(regex(pattern = r"^[A-Za-z][_0-9A-Za-z]*(?<!_)$"))]
    pub name: String,
    pub _name: Option<String>,
    /// ### グループ名
    /// グループパスと対象のグループが異なるときに指定する
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// ### 下位のモデルパスを昇格する
    #[serde(default, skip_serializing_if = "is_false")]
    pub promote_model_paths: bool,
    /// ### デフォルト参照権限
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub readable_roles: Vec<String>,
    /// ### デフォルト登録権限
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub creatable_roles: Vec<String>,
    /// ### デフォルトインポート権限
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub importable_roles: Vec<String>,
    /// ### デフォルト更新権限
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub updatable_roles: Vec<String>,
    /// ### デフォルト削除権限
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deletable_roles: Vec<String>,
}
impl From<ApiGroupDef> for ApiGroupJson {
    fn from(value: ApiGroupDef) -> Self {
        Self {
            name: String::new(),
            _name: None,
            group: value.group,
            promote_model_paths: value.promote_model_paths,
            readable_roles: value.readable_roles,
            creatable_roles: value.creatable_roles,
            importable_roles: value.importable_roles,
            updatable_roles: value.updatable_roles,
            deletable_roles: value.deletable_roles,
        }
    }
}
impl From<ApiGroupJson> for ApiGroupDef {
    fn from(value: ApiGroupJson) -> Self {
        Self {
            group: value.group,
            promote_model_paths: value.promote_model_paths,
            readable_roles: value.readable_roles,
            creatable_roles: value.creatable_roles,
            importable_roles: value.importable_roles,
            updatable_roles: value.updatable_roles,
            deletable_roles: value.deletable_roles,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Copy, JsonSchema)]
#[serde(rename_all = "snake_case")]
/// ### フィールド可視性
pub enum FieldVisibility {
    Hidden,
    ReadOnly,
    WriteOnly,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Copy, JsonSchema)]
#[serde(rename_all = "snake_case")]
/// ### リレーション可視性
pub enum RelationVisibility {
    ReadOnly,
    WriteOnly,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
/// ### Apiフィールド定義
pub struct ApiFieldDef {
    /// ### 可視性
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visibility: Option<FieldVisibility>,
    /// ### 必須設定
    #[serde(default, skip_serializing_if = "is_false")]
    pub required: bool,
    /// ### アップデート不可
    /// 登録時のみ入力可能で、更新時は入力不可となる
    #[serde(default, skip_serializing_if = "is_false")]
    pub disable_update: bool,
    /// ### バリデータ
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validator: Option<String>,
    /// ### デフォルト値
    #[schemars(default, schema_with = "default_value_schema")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_yaml::Value>,
    /// ### 登録時Rust式
    /// 更新を防止するためには read_only 設定が必要
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_insert_formula: Option<String>,
    /// ### 更新時Rust式
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_update_formula: Option<String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
/// ### Apiフィールド定義
pub struct ApiFieldJson {
    /// ### フィールド名
    #[schemars(regex(pattern = r"^\p{XID_Start}\p{XID_Continue}*(?<!_)$"))]
    pub name: String,
    /// ### 可視性
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visibility: Option<FieldVisibility>,
    /// ### 必須設定
    #[serde(default, skip_serializing_if = "is_false")]
    pub required: bool,
    /// ### アップデート不可
    /// 登録時のみ入力可能で、更新時は入力不可となる
    #[serde(default, skip_serializing_if = "is_false")]
    pub disable_update: bool,
    /// ### バリデータ
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validator: Option<String>,
    /// ### デフォルト値
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    /// ### 登録時Rust式
    /// 更新を防止するためには read_only 設定が必要
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_insert_formula: Option<String>,
    /// ### 更新時Rust式
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_update_formula: Option<String>,
}
impl From<ApiFieldDef> for ApiFieldJson {
    fn from(value: ApiFieldDef) -> Self {
        Self {
            name: String::new(),
            visibility: value.visibility,
            required: value.required,
            disable_update: value.disable_update,
            validator: value.validator,
            default: value.default.map(|v| yaml_value_to_str(&v).unwrap()),
            on_insert_formula: value.on_insert_formula,
            on_update_formula: value.on_update_formula,
        }
    }
}
impl From<ApiFieldJson> for ApiFieldDef {
    fn from(value: ApiFieldJson) -> Self {
        Self {
            visibility: value.visibility,
            required: value.required,
            disable_update: value.disable_update,
            validator: value.validator,
            default: value.default.map(|v| serde_yaml::from_str(&v).unwrap()),
            on_insert_formula: value.on_insert_formula,
            on_update_formula: value.on_update_formula,
        }
    }
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

pub type Fields = IndexMap<String, Option<ApiFieldDef>>;
pub type Relations = IndexMap<String, Option<ApiRelationDef>>;

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
/// ### APIモデル定義
pub struct ApiModelDef {
    /// ### モデル名
    /// モデルパスと対象のモデルが異なるときに指定する
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// ### フィールド自動追加の無効化
    #[serde(default, skip_serializing_if = "is_false")]
    pub disable_auto_fields: bool,
    /// ### 主キーでのfindを有効化
    #[serde(default, skip_serializing_if = "is_false")]
    pub enable_find_by_pk: bool,
    /// ### 主キーでのdeleteを有効化
    #[serde(default, skip_serializing_if = "is_false")]
    pub enable_delete_by_pk: bool,
    /// ### 登録、更新、削除を無効化
    #[serde(default, skip_serializing_if = "is_false")]
    pub disable_mutation: bool,
    /// ### インポートを使用
    #[serde(default, skip_serializing_if = "is_false")]
    pub enable_import: bool,
    /// ### 閲覧権限
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub readable_roles: Vec<String>,
    /// ### 登録権限
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub creatable_roles: Vec<String>,
    /// ### インポート権限
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub importable_roles: Vec<String>,
    /// ### 更新権限
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub updatable_roles: Vec<String>,
    /// ### 削除権限
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deletable_roles: Vec<String>,
    /// ### 閲覧権限フィルタ式
    #[serde(skip_serializing_if = "Option::is_none")]
    pub readable_filter: Option<String>,
    /// ### 登録権限フィルタ式
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creatable_filter: Option<String>,
    /// ### 更新権限フィルタ式
    /// 省略時は閲覧権限フィルタ式が適用される
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updatable_filter: Option<String>,
    /// ### 削除権限フィルタ式
    /// 省略時は更新権限フィルタ式が適用される
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deletable_filter: Option<String>,
    /// ### フィールド
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub fields: Fields,
    /// ### リレーション
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub relations: Relations,
    /// ### セレクタ
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub selector: IndexMap<String, Option<ApiSelectorDef>>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default, JsonSchema, Validate)]
#[serde(deny_unknown_fields)]
/// ### APIモデル定義
pub struct ApiModelJson {
    /// ### モデルパス
    #[schemars(regex(pattern = r"^[A-Za-z][_0-9A-Za-z]*(?<!_)$"))]
    pub name: String,
    /// ### モデル名
    /// モデルパスと対象のモデルが異なるときに指定する
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// ### フィールド自動追加の無効化
    #[serde(default, skip_serializing_if = "is_false")]
    pub disable_auto_fields: bool,
    /// ### 主キーでのfindを有効化
    #[serde(default, skip_serializing_if = "is_false")]
    pub enable_find_by_pk: bool,
    /// ### 主キーでのdeleteを有効化
    #[serde(default, skip_serializing_if = "is_false")]
    pub enable_delete_by_pk: bool,
    /// ### 登録、更新、削除を無効化
    #[serde(default, skip_serializing_if = "is_false")]
    pub disable_mutation: bool,
    /// ### インポートを使用
    #[serde(default, skip_serializing_if = "is_false")]
    pub enable_import: bool,
    /// ### 閲覧権限
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub readable_roles: Vec<String>,
    /// ### 登録権限
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub creatable_roles: Vec<String>,
    /// ### インポート権限
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub importable_roles: Vec<String>,
    /// ### 更新権限
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub updatable_roles: Vec<String>,
    /// ### 削除権限
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deletable_roles: Vec<String>,
    /// ### 閲覧権限フィルタ式
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(custom(function = "validate_filter"))]
    pub readable_filter: Option<String>,
    /// ### 登録権限フィルタ式
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creatable_filter: Option<String>,
    /// ### 更新権限フィルタ式
    /// 省略時は閲覧権限フィルタ式が適用される
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(custom(function = "validate_filter"))]
    pub updatable_filter: Option<String>,
    /// ### 削除権限フィルタ式
    /// 省略時は更新権限フィルタ式が適用される
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(custom(function = "validate_filter"))]
    pub deletable_filter: Option<String>,
    /// ### フィールド
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<ApiFieldJson>,
    /// ### リレーション
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relations: Vec<ApiRelationJson>,
    /// ### セレクタ
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[validate(nested)]
    pub selector: Vec<ApiSelectorJson>,
}
fn validate_filter(filter: &str) -> Result<(), ValidationError> {
    if syn::parse_str::<proc_macro2::TokenTree>(filter).is_err() {
        return Err(ValidationError::new("illegal filter"));
    }
    Ok(())
}
impl From<ApiModelDef> for ApiModelJson {
    fn from(value: ApiModelDef) -> Self {
        Self {
            name: String::new(),
            model: value.model,
            disable_auto_fields: value.disable_auto_fields,
            enable_find_by_pk: value.enable_find_by_pk,
            enable_delete_by_pk: value.enable_delete_by_pk,
            disable_mutation: value.disable_mutation,
            enable_import: value.enable_import,
            readable_roles: value.readable_roles,
            creatable_roles: value.creatable_roles,
            importable_roles: value.importable_roles,
            updatable_roles: value.updatable_roles,
            deletable_roles: value.deletable_roles,
            readable_filter: value.readable_filter,
            creatable_filter: value.creatable_filter,
            updatable_filter: value.updatable_filter,
            deletable_filter: value.deletable_filter,
            fields: value
                .fields
                .into_iter()
                .map(|(k, v)| {
                    let mut v: ApiFieldJson = v.unwrap_or_default().into();
                    v.name = k;
                    v
                })
                .collect(),
            relations: value
                .relations
                .into_iter()
                .map(|(k, v)| {
                    let mut v: ApiRelationJson = v.unwrap_or_default().into();
                    v.name = k;
                    v
                })
                .collect(),
            selector: value
                .selector
                .into_iter()
                .map(|(k, v)| {
                    let mut v: ApiSelectorJson = v.unwrap_or_default().into();
                    v.name = k;
                    v
                })
                .collect(),
        }
    }
}
impl TryFrom<ApiModelJson> for ApiModelDef {
    type Error = anyhow::Error;
    fn try_from(value: ApiModelJson) -> Result<Self, Self::Error> {
        Ok(Self {
            model: value.model,
            disable_auto_fields: value.disable_auto_fields,
            enable_find_by_pk: value.enable_find_by_pk,
            enable_delete_by_pk: value.enable_delete_by_pk,
            disable_mutation: value.disable_mutation,
            enable_import: value.enable_import,
            readable_roles: value.readable_roles,
            creatable_roles: value.creatable_roles,
            importable_roles: value.importable_roles,
            updatable_roles: value.updatable_roles,
            deletable_roles: value.deletable_roles,
            readable_filter: value.readable_filter,
            creatable_filter: value.creatable_filter,
            updatable_filter: value.updatable_filter,
            deletable_filter: value.deletable_filter,
            fields: value
                .fields
                .into_iter()
                .map(|v| {
                    let name = v.name.clone();
                    let v: ApiFieldDef = v.into();
                    if v == <ApiFieldDef as Default>::default() {
                        (name, None)
                    } else {
                        (name, Some(v))
                    }
                })
                .collect(),
            relations: value
                .relations
                .into_iter()
                .map(|v| {
                    let name = v.name.clone();
                    let v: ApiRelationDef = v.into();
                    if v == ApiRelationDef::default() {
                        (name, None)
                    } else {
                        (name, Some(v))
                    }
                })
                .collect(),
            selector: value
                .selector
                .into_iter()
                .map(|v| {
                    let name = v.name.clone();
                    let v: ApiSelectorDef = v.into();
                    if v == ApiSelectorDef::default() {
                        (name, None)
                    } else {
                        (name, Some(v))
                    }
                })
                .collect(),
        })
    }
}
#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
/// ### APIリレーション定義
pub struct ApiRelationDef {
    /// ### 可視性
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visibility: Option<RelationVisibility>,
    /// ### 更新時置換
    /// 更新時に削除した後に、新規登録（has_one リレーションのみ対応）
    #[serde(default, skip_serializing_if = "is_false")]
    pub use_replace: bool,
    /// ### フィールド自動追加の無効化
    #[serde(default, skip_serializing_if = "is_false")]
    pub disable_auto_fields: bool,
    /// ### フィールド
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub fields: Fields,
    /// ### リレーション
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub relations: Relations,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
/// ### APIリレーション定義
pub struct ApiRelationJson {
    /// ### リレーション名
    #[schemars(regex(pattern = r"^\p{XID_Start}\p{XID_Continue}*(?<!_)$"))]
    pub name: String,
    /// ### 可視性
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visibility: Option<RelationVisibility>,
    /// ### 更新時置換
    /// 更新時に削除した後に、新規登録（has_one リレーションのみ対応）
    #[serde(default, skip_serializing_if = "is_false")]
    pub use_replace: bool,
    /// ### フィールド自動追加の無効化
    #[serde(default, skip_serializing_if = "is_false")]
    pub disable_auto_fields: bool,
    /// ### フィールド
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<ApiFieldJson>,
    /// ### リレーション
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relations: Vec<ApiRelationJson>,
}
impl From<ApiRelationDef> for ApiRelationJson {
    fn from(value: ApiRelationDef) -> Self {
        Self {
            name: String::new(),
            visibility: value.visibility,
            use_replace: value.use_replace,
            disable_auto_fields: value.disable_auto_fields,
            fields: value
                .fields
                .into_iter()
                .map(|(k, v)| {
                    let mut v: ApiFieldJson = v.unwrap_or_default().into();
                    v.name = k;
                    v
                })
                .collect(),
            relations: value
                .relations
                .into_iter()
                .map(|(k, v)| {
                    let mut v: ApiRelationJson = v.unwrap_or_default().into();
                    v.name = k;
                    v
                })
                .collect(),
        }
    }
}
impl From<ApiRelationJson> for ApiRelationDef {
    fn from(value: ApiRelationJson) -> Self {
        Self {
            visibility: value.visibility,
            use_replace: value.use_replace,
            disable_auto_fields: value.disable_auto_fields,
            fields: value
                .fields
                .into_iter()
                .map(|v| {
                    let name = v.name.clone();
                    let v: ApiFieldDef = v.into();
                    if v == <ApiFieldDef as Default>::default() {
                        (name, None)
                    } else {
                        (name, Some(v))
                    }
                })
                .collect(),
            relations: value
                .relations
                .into_iter()
                .map(|v| {
                    let name = v.name.clone();
                    let v: ApiRelationDef = v.into();
                    if v == ApiRelationDef::default() {
                        (name, None)
                    } else {
                        (name, Some(v))
                    }
                })
                .collect(),
        }
    }
}

pub fn is_false(val: &bool) -> bool {
    !(*val)
}

impl ApiModelDef {
    pub fn fix(&mut self) {
        let roles = API_CONFIG.read().unwrap().as_ref().unwrap().roles.clone();
        self.readable_roles.retain(|v| roles.contains_key(v));
        self.creatable_roles.retain(|v| roles.contains_key(v));
        self.importable_roles.retain(|v| roles.contains_key(v));
        self.updatable_roles.retain(|v| roles.contains_key(v));
        self.deletable_roles.retain(|v| roles.contains_key(v));
    }
    #[allow(dead_code)]
    pub fn disable_graphql(&self) -> bool {
        API_CONFIG
            .read()
            .unwrap()
            .as_ref()
            .unwrap()
            .disable_graphql
            .unwrap_or_default()
    }
    pub fn enable_json_api(&self) -> bool {
        API_CONFIG
            .read()
            .unwrap()
            .as_ref()
            .unwrap()
            .enable_json_api
            .unwrap_or_default()
    }
    pub fn readable_roles(&self, config: &ApiDbDef, group: &str) -> Vec<String> {
        if self.readable_roles.is_empty() {
            config
                .groups
                .get(group)
                .and_then(|v| v.as_ref().map(|v| v.readable_roles.clone()))
                .unwrap_or_default()
        } else {
            self.readable_roles.clone()
        }
    }
    pub fn creatable_roles(&self, config: &ApiDbDef, group: &str) -> Vec<String> {
        if self.creatable_roles.is_empty() {
            config
                .groups
                .get(group)
                .and_then(|v| v.as_ref().map(|v| v.creatable_roles.clone()))
                .unwrap_or_default()
        } else {
            self.creatable_roles.clone()
        }
    }
    pub fn importable_roles(&self, config: &ApiDbDef, group: &str) -> Vec<String> {
        if self.importable_roles.is_empty() {
            config
                .groups
                .get(group)
                .and_then(|v| v.as_ref().map(|v| v.importable_roles.clone()))
                .unwrap_or_default()
        } else {
            self.importable_roles.clone()
        }
    }
    pub fn updatable_roles(&self, config: &ApiDbDef, group: &str) -> Vec<String> {
        if self.updatable_roles.is_empty() {
            config
                .groups
                .get(group)
                .and_then(|v| v.as_ref().map(|v| v.updatable_roles.clone()))
                .unwrap_or_default()
        } else {
            self.updatable_roles.clone()
        }
    }
    pub fn deletable_roles(&self, config: &ApiDbDef, group: &str) -> Vec<String> {
        if self.deletable_roles.is_empty() {
            config
                .groups
                .get(group)
                .and_then(|v| v.as_ref().map(|v| v.deletable_roles.clone()))
                .unwrap_or_default()
        } else {
            self.deletable_roles.clone()
        }
    }
    pub fn readable_filter(&self) -> &str {
        self.readable_filter
            .as_ref()
            .map(|v| v.trim())
            .unwrap_or("")
    }
    pub fn creatable_filter(&self) -> &str {
        self.creatable_filter
            .as_ref()
            .map(|v| v.trim())
            .unwrap_or("")
    }
    pub fn updatable_filter(&self) -> &str {
        if self.updatable_filter.is_some() {
            self.updatable_filter
                .as_ref()
                .map(|v| v.trim())
                .unwrap_or("")
        } else {
            self.readable_filter()
        }
    }
    pub fn deletable_filter(&self) -> &str {
        if self.deletable_filter.is_some() {
            self.deletable_filter
                .as_ref()
                .map(|v| v.trim())
                .unwrap_or("")
        } else {
            self.updatable_filter()
        }
    }

    pub fn fields(&self, model: &ModelDef, config: &ApiDbDef) -> anyhow::Result<Fields> {
        for (k, _) in &self.fields {
            anyhow::ensure!(
                model.merged_fields.contains_key(k),
                "There is no {} column in the {} model.",
                k,
                model.name
            );
        }
        if self.disable_auto_fields {
            Ok(self.fields.clone())
        } else {
            let mut fields: IndexMap<_, _> = model
                .merged_fields
                .iter()
                .filter(|(_k, v)| {
                    !v.hidden.unwrap_or_default() && (!v.is_timestamp || !config.hide_timestamp())
                })
                .map(|(k, _)| ((*k).clone(), None))
                .collect();
            for (name, column) in &self.fields {
                fields.insert(name.clone(), column.clone());
            }
            Ok(fields)
        }
    }
    pub fn relations(&self, model: &ModelDef) -> anyhow::Result<Relations> {
        for (k, _) in &self.relations {
            anyhow::ensure!(
                model.merged_relations.contains_key(k),
                "There is no {} relation in the {} model.",
                k,
                model.name
            );
        }
        Ok(self.relations.clone())
    }
    pub fn selector(&self, name: &str) -> Option<ApiSelectorDef> {
        if let Some(def) = self.selector.get(name) {
            if let Some(def) = def {
                Some(def.clone())
            } else {
                Some(ApiSelectorDef::default())
            }
        } else {
            None
        }
    }
}

static API_RELATIONS: Mutex<Vec<Relations>> = Mutex::new(Vec::new());

impl ApiRelationDef {
    pub fn push(relation: Relations) {
        API_RELATIONS.lock().unwrap().push(relation);
    }
    pub fn pop() {
        API_RELATIONS.lock().unwrap().pop();
    }
    pub fn get(name: &str) -> Option<ApiRelationDef> {
        if let Some(r) = API_RELATIONS.lock().unwrap().last() {
            r.get(name).cloned().map(|v| v.unwrap_or_default())
        } else {
            None
        }
    }
    pub fn has(name: &str) -> bool {
        if let Some(r) = API_RELATIONS.lock().unwrap().last() {
            r.contains_key(name)
        } else {
            true
        }
    }
    pub fn check(name: &str, write: bool) -> bool {
        if let Some(r) = API_RELATIONS.lock().unwrap().last() {
            if let Some(r) = r.get(name) {
                match r.as_ref().map(|r| r.visibility).unwrap_or_default() {
                    Some(RelationVisibility::ReadOnly) => !write,
                    Some(RelationVisibility::WriteOnly) => write,
                    None => true,
                }
            } else {
                false
            }
        } else {
            true
        }
    }
    pub fn fields(&self, model: &ModelDef, rel_id: &[String]) -> anyhow::Result<Fields> {
        for (k, _) in &self.fields {
            anyhow::ensure!(
                model.merged_fields.contains_key(k),
                "There is no {} column in the {} model.",
                k,
                model.name
            );
        }
        if self.disable_auto_fields {
            Ok(self.fields.clone())
        } else {
            let mut fields: IndexMap<_, _> = model
                .merged_fields
                .iter()
                .filter(|(k, v)| {
                    !v.hidden.unwrap_or_default() && !v.is_timestamp && !rel_id.contains(*k)
                })
                .map(|(k, _)| ((*k).clone(), None))
                .collect();
            for (name, column) in &self.fields {
                fields.insert(name.clone(), column.clone());
            }
            Ok(fields)
        }
    }
    pub fn relations(&self, model: &ModelDef) -> anyhow::Result<Relations> {
        for (k, _) in &self.relations {
            anyhow::ensure!(
                model.merged_relations.contains_key(k),
                "There is no {} relation in the {} model.",
                k,
                model.name
            );
        }
        Ok(self.relations.clone())
    }
}

static API_COLUMNS: Mutex<Vec<Fields>> = Mutex::new(Vec::new());

impl ApiFieldDef {
    pub fn push(relation: Fields) {
        API_COLUMNS.lock().unwrap().push(relation);
    }
    pub fn pop() {
        API_COLUMNS.lock().unwrap().pop();
    }
    pub fn has(name: &str) -> bool {
        if let Some(c) = API_COLUMNS.lock().unwrap().last() {
            c.contains_key(name)
        } else {
            true
        }
    }
    pub fn check(name: &str, write: bool) -> bool {
        if let Some(c) = API_COLUMNS.lock().unwrap().last() {
            if let Some(c) = c.get(name) {
                match c.clone().unwrap_or_default().visibility {
                    Some(FieldVisibility::Hidden) => false,
                    Some(FieldVisibility::ReadOnly) => !write,
                    Some(FieldVisibility::WriteOnly) => write,
                    None => true,
                }
            } else {
                false
            }
        } else {
            true
        }
    }
    pub fn required(name: &str) -> bool {
        if let Some(c) = API_COLUMNS.lock().unwrap().last() {
            if let Some(c) = c.get(name) {
                c.clone().unwrap_or_default().required
            } else {
                false
            }
        } else {
            false
        }
    }
    pub fn disable_update(name: &str) -> bool {
        if let Some(c) = API_COLUMNS.lock().unwrap().last() {
            if let Some(c) = c.get(name) {
                c.clone().unwrap_or_default().disable_update
            } else {
                false
            }
        } else {
            false
        }
    }
    pub fn validator(name: &str) -> Option<String> {
        if let Some(c) = API_COLUMNS.lock().unwrap().last() {
            if let Some(c) = c.get(name) {
                c.clone().unwrap_or_default().validator
            } else {
                None
            }
        } else {
            None
        }
    }
    pub fn default(name: &str, field: &FieldDef) -> Option<serde_yaml::Value> {
        if let Some(c) = API_COLUMNS.lock().unwrap().last()
            && let Some(Some(c)) = c.get(name)
        {
            if c.required {
                return None;
            }
            return c.default.clone();
        }
        if !field.auto_gen
            && let Some(default) = &field.default
        {
            return Some(default.clone());
        }
        None
    }
    pub fn on_insert_formula(name: &str) -> Option<String> {
        if let Some(c) = API_COLUMNS.lock().unwrap().last()
            && let Some(Some(c)) = c.get(name)
        {
            return c.on_insert_formula.clone();
        }
        None
    }
    pub fn on_update_formula(name: &str) -> Option<String> {
        if let Some(c) = API_COLUMNS.lock().unwrap().last()
            && let Some(Some(c)) = c.get(name)
        {
            return c.on_update_formula.clone();
        }
        None
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
/// ### APIセレクタ定義
pub struct ApiSelectorDef {
    /// ### JavaScriptによる更新
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub js_updater: IndexMap<String, JsUpdaterDef>,
    /// ### オペレータによる更新を有効化する
    /// MongoDBの$currentDate, $inc, $min, $max, $mul, $rename, $set, $unset, $addToSet, $pop, $push, $pullAll, $bit相当に対応
    /// $currentDateはDateのみで、Timestampには対応していない。$pushの$sortには非対応
    /// ただし、オペレーターの "$" はすべて "_" に置き換える必要がある
    #[serde(default, skip_serializing_if = "crate::schema::is_false")]
    pub enable_update_by_operator: bool,
    /// ### 削除を有効化する
    #[serde(default, skip_serializing_if = "crate::schema::is_false")]
    pub enable_delete_by_selector: bool,
    /// ### 取得数上限
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u64>,
    /// ### ストリーミング取得APIを使用する
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enable_streaming_api: Option<bool>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default, JsonSchema, Validate)]
#[serde(deny_unknown_fields)]
/// ### APIセレクタ定義
pub struct ApiSelectorJson {
    /// ### セレクタ名
    #[schemars(regex(pattern = r"^\p{XID_Start}\p{XID_Continue}*(?<!_)$"))]
    pub name: String,
    /// ### JavaScriptによる更新
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[validate(nested)]
    pub js_updater: Vec<JsUpdaterJson>,
    /// ### オペレータによる更新を有効化する
    /// MongoDBの$currentDate, $inc, $min, $max, $mul, $rename, $set, $unset, $addToSet, $pop, $push, $pullAll, $bit相当に対応
    /// $currentDateはDateのみで、Timestampには対応していない。$pushの$sortには非対応
    /// ただし、オペレーターの "$" はすべて "_" に置き換える必要がある
    #[serde(default, skip_serializing_if = "crate::schema::is_false")]
    pub enable_update_by_operator: bool,
    /// ### 削除を有効化する
    #[serde(default, skip_serializing_if = "crate::schema::is_false")]
    pub enable_delete_by_selector: bool,
    /// ### 取得数上限
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u64>,
    /// ### ストリーミング取得APIを使用する
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enable_streaming_api: Option<bool>,
}
impl From<ApiSelectorDef> for ApiSelectorJson {
    fn from(value: ApiSelectorDef) -> Self {
        Self {
            name: String::new(),
            js_updater: value
                .js_updater
                .into_iter()
                .map(|(k, v)| {
                    let mut v: JsUpdaterJson = v.into();
                    v.name = k;
                    v
                })
                .collect(),
            enable_update_by_operator: value.enable_update_by_operator,
            enable_delete_by_selector: value.enable_delete_by_selector,
            limit: value.limit,
            enable_streaming_api: value.enable_streaming_api,
        }
    }
}
impl From<ApiSelectorJson> for ApiSelectorDef {
    fn from(value: ApiSelectorJson) -> Self {
        Self {
            js_updater: value
                .js_updater
                .into_iter()
                .map(|v| {
                    let name = v.name.clone();
                    let v: JsUpdaterDef = v.into();
                    (name, v)
                })
                .collect(),
            enable_update_by_operator: value.enable_update_by_operator,
            enable_delete_by_selector: value.enable_delete_by_selector,
            limit: value.limit,
            enable_streaming_api: value.enable_streaming_api,
        }
    }
}
impl ApiSelectorDef {
    pub fn limit(&self) -> Option<u64> {
        let mut limit = self.limit;
        let selector_limit = API_CONFIG.read().unwrap().as_ref().unwrap().selector_limit;
        if limit.is_none() {
            limit = selector_limit;
        }
        limit
    }
    pub fn limit_def(&self) -> String {
        self.limit()
            .map(|l| format!("\n            const LIMIT: usize = {l};"))
            .unwrap_or_default()
    }
    pub fn limit_validator(&self) -> String {
        if let Some(l) = self.limit() {
            format!("#[graphql(validator(maximum = {l}))] ")
        } else {
            "".to_string()
        }
    }
    pub fn limit_str(&self) -> &'static str {
        if self.limit().is_some() {
            "Some(LIMIT)"
        } else {
            "None"
        }
    }
    pub fn check_limit(&self) -> &'static str {
        if self.limit().is_some() {
            ".map(|l| l.min(LIMIT))"
        } else {
            ""
        }
    }
    pub fn enable_streaming_api(&self) -> bool {
        if let Some(enable_streaming_api) = self.enable_streaming_api {
            return enable_streaming_api;
        }
        let conf = API_CONFIG.read().unwrap();
        if let Some(enable_streaming_api) = conf.as_ref().unwrap().enable_streaming_api {
            return enable_streaming_api;
        }
        false
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
/// ### JavaScript Updater定義
pub struct JsUpdaterDef {
    /// ### JavaScript
    /// 更新対象のオブジェクトをAPIで受け取ったvalueで更新して更新後のオブジェクトを返す関数を定義する。
    /// NULLを返した場合は更新されない。
    /// APIの呼び出し時にcreateIfEmptyにtrueを指定した場合は、対象オブジェクトが存在しない場合にobjがNULLで渡される。
    #[schemars(example = "default_script")]
    pub script: String,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, JsonSchema, Validate)]
#[serde(deny_unknown_fields)]
/// ### JavaScript Updater定義
pub struct JsUpdaterJson {
    /// ### 名前
    /// キャメルケースに変換されてAPIのメソッド名に使用される。
    #[schemars(regex(pattern = r"^\p{XID_Start}\p{XID_Continue}*(?<!_)$"))]
    pub name: String,
    /// ### JavaScript
    /// 更新対象のオブジェクトをAPIで受け取ったvalueで更新して更新後のオブジェクトを返す関数を定義する。
    /// NULLを返した場合は更新されない。
    /// APIの呼び出し時にcreateIfEmptyにtrueを指定した場合は、対象オブジェクトが存在しない場合にobjがNULLで渡される。
    #[schemars(example = "default_script")]
    #[validate(custom(function = "validate_script"))]
    pub script: String,
}

fn default_script() -> &'static str {
    "function update(obj, value, auth) {
    if (obj) {
        obj = Object.assign(obj, value);
    } else {
        obj = value;
    }
    return obj;
}"
}
fn validate_script(filter: &str) -> Result<(), ValidationError> {
    if crate::common::check_js(filter).is_err() {
        return Err(ValidationError::new("illegal script"));
    }
    Ok(())
}

impl From<JsUpdaterDef> for JsUpdaterJson {
    fn from(value: JsUpdaterDef) -> Self {
        Self {
            name: String::new(),
            script: value.script,
        }
    }
}

impl From<JsUpdaterJson> for JsUpdaterDef {
    fn from(value: JsUpdaterJson) -> Self {
        Self {
            script: value.script,
        }
    }
}

impl JsUpdaterDef {
    pub fn esc_script(&self) -> String {
        format!("{:?}", self.script)
    }
}
