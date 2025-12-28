use compact_str::CompactString;
use fancy_regex::Regex;
use indexmap::IndexMap;
use once_cell::sync::Lazy;
use schemars::JsonSchema;
use schemars::schema::{InstanceType, Schema, SchemaObject, SingleOrVec};
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::collections::HashSet;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use crate::api_generator::schema::{ApiFieldDef, ApiRelationDef};
use crate::common::{AtomicLoad as _, hash, if_then_else, to_plural, yaml_value_to_str};
use crate::schema::_to_ident_name;

use super::*;

pub static IS_MAIN_GROUP: AtomicBool = AtomicBool::new(true);

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
/// ### 継承
pub struct Inheritance {
    /// ### 継承元
    pub extends: String,
    /// ### 継承タイプ
    #[serde(rename = "type")]
    pub _type: InheritanceType,
    /// ### カラム集約テーブル継承の場合のキーカラム
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_field: Option<String>,
    /// ### カラム集約テーブル継承の場合のキーの値
    #[schemars(default, schema_with = "key_value_schema")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_value: Option<Value>,
}
fn key_value_schema(_: &mut schemars::r#gen::SchemaGenerator) -> Schema {
    let schema = SchemaObject {
        instance_type: Some(SingleOrVec::Vec(vec![
            InstanceType::String,
            InstanceType::Integer,
        ])),
        ..Default::default()
    };
    Schema::Object(schema)
}

#[derive(Debug, PartialEq, Eq, Default, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
/// ### 継承
pub struct InheritanceJson {
    /// ### 継承元
    pub extends: Option<String>,
    /// ### 継承タイプ
    #[serde(rename = "type")]
    pub _type: Option<InheritanceType>,
    /// ### カラム集約テーブル継承の場合のキーカラム
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_field: Option<String>,
    /// ### カラム集約テーブル継承の場合のキーの値
    #[schemars(default)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_value: Option<String>,
}

impl From<Inheritance> for InheritanceJson {
    fn from(value: Inheritance) -> Self {
        Self {
            extends: if value.extends.is_empty() {
                None
            } else {
                Some(value.extends)
            },
            _type: Some(value._type),
            key_field: value.key_field,
            key_value: value.key_value.map(|v| yaml_value_to_str(&v).unwrap()),
        }
    }
}

impl TryFrom<InheritanceJson> for Option<Inheritance> {
    type Error = anyhow::Error;
    fn try_from(value: InheritanceJson) -> Result<Self, Self::Error> {
        if value.extends.is_none() && value._type.is_none() {
            Ok(None)
        } else if value.extends.is_some() && value._type.is_some() {
            Ok(Some(Inheritance {
                extends: value.extends.unwrap(),
                _type: value._type.unwrap(),
                key_field: value.key_field,
                key_value: value.key_value.map(|v| serde_yaml::from_str(&v).unwrap()),
            }))
        } else {
            Err(anyhow::anyhow!(
                "Both \"extends\" and \"type\" settings are required. "
            ))
        }
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Copy, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
/// ### 継承タイプ
pub enum InheritanceType {
    /// ### 単一テーブル継承
    /// 子テーブルのカラムも含めたすべてのカラムを親となるテーブルに格納する
    Simple,
    /// ### 具象テーブル継承
    /// 子クラスごとに共通のカラムとそれぞれのモデルのカラムをすべて含んだ状態で独立したテーブルを作成する
    Concrete,
    /// ### カラム集約テーブル継承
    /// 単一テーブル継承と似ているが、型を特定するための _type カラムがある
    ColumnAggregation,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
/// ### 機能定義
pub struct ActAs {
    /// ### セッションDBとして使用
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub session: bool,
    /// ### ジョブキューとして使用
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub job_queue: bool,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
/// ### 機能定義
pub struct ActAsJson {
    /// ### ジョブキューとして使用
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub job_queue: bool,
}

impl From<ActAs> for ActAsJson {
    fn from(value: ActAs) -> Self {
        Self {
            job_queue: value.job_queue,
        }
    }
}

impl TryFrom<ActAsJson> for Option<ActAs> {
    type Error = anyhow::Error;
    fn try_from(value: ActAsJson) -> Result<Self, Self::Error> {
        let v = ActAs {
            session: false,
            job_queue: value.job_queue,
        };
        if v == Default::default() {
            Ok(None)
        } else {
            Ok(Some(v))
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
/// ### モデル定義
pub struct ModelDef {
    #[serde(skip)]
    pub db: String,
    #[serde(skip)]
    pub group_name: String,
    #[serde(skip)]
    pub name: String,
    #[serde(default, skip)]
    pub exclude_group_from_table_name: Option<bool>,
    #[serde(default, skip)]
    pub cache_owners: Vec<(String, String, String, u64)>,
    #[serde(default, skip)]
    pub merged_fields: IndexMap<String, FieldDef>,
    #[serde(default, skip)]
    pub relations: IndexMap<String, RelDef>,
    #[serde(default, skip)]
    pub merged_relations: IndexMap<String, RelDef>,
    #[serde(default, skip)]
    pub merged_indexes: IndexMap<String, IndexDef>,

    /// ### リネーム元テーブル名
    /// よくわからない場合は手動で修正しないこと。また、コピー&ペーストを行う場合は削除した方が良い。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _before_rename_name: Option<String>,
    /// ### 変更前論理削除設定
    /// 変更を検出してDDLにDELETE文を出力する。
    /// よくわからない場合は手動で修正しないこと。また、コピー&ペーストを行う場合は削除した方が良い。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _soft_delete: Option<String>,
    /// ### 論理名
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// ### コメント
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// ### テーブル名
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table_name: Option<String>,
    /// ### DDL定義を出力しない
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub skip_ddl: bool,
    /// ### 主キー以外で結合する多対多リレーション用ダミーテーブル
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub dummy_always_joinable: bool,
    /// ### Upsertコンフリクト判定ユニークキー(PostgreSQLのみ)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upsert_conflict_target: Option<String>,
    /// ### 外部キー制約をDDLに出力しない
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore_foreign_key: Option<bool>,
    /// ### タイムスタンプ設定
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestampable: Option<Timestampable>,
    /// ### created_atの無効化
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub disable_created_at: bool,
    /// ### updated_atの無効化
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub disable_updated_at: bool,
    /// ### 論理削除設定
    #[serde(skip_serializing_if = "Option::is_none")]
    pub soft_delete: Option<SoftDelete>,
    /// ### バージョンを使用する
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub versioned: bool,
    /// ### save_delayed のカウンターに使用するフィールド
    /// versioned との同時使用は不可
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub counting: Option<String>,
    /// ### キャッシュを使用する
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_cache: Option<bool>,
    /// ### 全行キャッシュを使用する
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_all_rows_cache: Option<bool>,
    /// ### 条件付き全行キャッシュを使用する
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_filtered_row_cache: Option<bool>,
    /// ### 更新時に常にすべてのキャッシュをクリアする
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_clear_whole_cache: Option<bool>,
    /// ### リレーションとして登録される場合に上書きする
    // 多対多テーブルで論理削除の場合、必要になる。リレーション以外のユニークキーがあると意図しない結果になるので注意が必要。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overwrite_on_relation_save: Option<bool>,
    /// ### 更新通知を使用する
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_update_notice: Option<bool>,
    /// ### 遅延INSERTを使用する
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_insert_delayed: Option<bool>,
    /// ### 遅延SAVEを使用する
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_save_delayed: Option<bool>,
    /// ### 遅延UPDATEを使用する
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_update_delayed: Option<bool>,
    /// ### 遅延UPSERTを使用する
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_upsert_delayed: Option<bool>,
    /// ### 更新を無効化する
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disable_update: Option<bool>,
    /// ### 削除を無効化する
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disable_delete: Option<bool>,
    /// ### insertされたデータのキャッシュを他のサーバに通知しない
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub disable_insert_cache_propagation: bool,
    /// ### 抽象化モード
    #[serde(default, skip_serializing_if = "super::is_false")]
    #[serde(rename = "abstract")]
    pub abstract_mode: bool,
    /// ### 継承モード
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inheritance: Option<Inheritance>,
    /// ### ストレージエンジン(MySQLのみ)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,
    /// ### 機能追加
    #[serde(skip_serializing_if = "Option::is_none")]
    pub act_as: Option<ActAs>,
    /// ### ER図のリレーションを非表示
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub hide_er_relations: bool,
    /// ### ユニークモデルID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<u64>,

    /// ### フィールド
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub fields: IndexMap<String, FieldDefOrSubsetType>,
    /// ### belongs_to リレーション
    /// 他のモデルのIDを参照している場合は設定必須
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub belongs_to: IndexMap<String, Option<BelongsToDef>>,
    /// ### belongs_to_outer_db リレーション
    /// 他のDBのモデルを参照するbelongs_to
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub belongs_to_outer_db: IndexMap<String, BelongsToOuterDbDef>,
    /// ### has_one リレーション
    /// 同時に取得する、または検索条件に含まれる場合に設定が必要
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub has_one: IndexMap<String, Option<HasOneDef>>,
    /// ### has_many リレーション
    /// 同時に取得する、または検索条件に含まれる場合に設定が必要
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub has_many: IndexMap<String, Option<HasManyDef>>,
    /// ### インデックス
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub indexes: IndexMap<String, Option<IndexDef>>,
    /// ### セレクタ
    /// API等での取得条件を設定
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub selectors: IndexMap<String, SelectorDef>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
/// ### Model Definition
pub struct ModelJson {
    #[serde(default, skip_deserializing, skip_serializing_if = "Vec::is_empty")]
    #[schemars(skip)]
    pub merged_fields: Vec<(String, FieldDef)>,
    #[serde(default, skip_deserializing, skip_serializing_if = "Vec::is_empty")]
    #[schemars(skip)]
    pub merged_relations: Vec<(String, RelDef)>,

    /// ### モデル名
    /// 単数形、スネークケース推奨
    #[schemars(regex(pattern = r"^[A-Za-z][_0-9A-Za-z]*(?<!_)$"))]
    pub name: String,
    /// ### リネーム元テーブル名
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _before_rename_name: Option<String>,
    /// ### 変更前論理削除設定
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _soft_delete: Option<String>,
    /// ### 論理名
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// ### コメント
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// ### テーブル名
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table_name: Option<String>,
    /// ### DDL定義を出力しない
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub skip_ddl: bool,
    /// ### 主キー以外で結合する多対多リレーション用ダミーテーブル
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub dummy_always_joinable: bool,
    /// ### Upsertコンフリクト判定ユニークキー(PostgreSQLのみ)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upsert_conflict_target: Option<String>,
    /// ### 外部キー制約をDDLに出力しない
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore_foreign_key: Option<bool>,
    /// ### タイムスタンプ設定
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestampable: Option<Timestampable>,
    /// ### created_atの無効化
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub disable_created_at: bool,
    /// ### updated_atの無効化
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub disable_updated_at: bool,
    /// ### 論理削除設定
    #[serde(skip_serializing_if = "Option::is_none")]
    pub soft_delete: Option<SoftDelete>,
    /// ### バージョンを使用する
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub versioned: bool,
    /// ### save_delayed のカウンターに使用するフィールド
    /// versionedとの同時使用は不可
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub counting: Option<String>,
    /// ### キャッシュを使用する
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_cache: Option<bool>,
    /// ### 全行キャッシュを使用する
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_all_rows_cache: Option<bool>,
    /// ### 条件付き全行キャッシュを使用する
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_filtered_row_cache: Option<bool>,
    /// ### 更新時に常にすべてのキャッシュをクリアする
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_clear_whole_cache: Option<bool>,
    /// ### リレーションとして登録される場合に上書きする
    // 多対多テーブルで論理削除の場合、必要になる。リレーション以外のユニークキーがあると意図しない結果になるので注意が必要。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overwrite_on_relation_save: Option<bool>,
    /// ### 更新通知を使用する
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_update_notice: Option<bool>,
    /// ### 遅延INSERTを使用する
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_insert_delayed: Option<bool>,
    /// ### 遅延SAVEを使用する
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_save_delayed: Option<bool>,
    /// ### 遅延UPDATEを使用する
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_update_delayed: Option<bool>,
    /// ### 遅延UPSERTを使用する
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_upsert_delayed: Option<bool>,
    /// ### 更新を無効化する
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disable_update: Option<bool>,
    /// ### 削除を無効化する
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disable_delete: Option<bool>,
    /// ### insertされたデータのキャッシュを他のサーバに通知しない
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub disable_insert_cache_propagation: bool,
    /// ### 抽象化モード
    #[serde(default, skip_serializing_if = "super::is_false")]
    #[serde(rename = "abstract")]
    pub abstract_mode: bool,
    /// ### 継承モード
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inheritance: Option<InheritanceJson>,
    /// ### ストレージエンジン(MySQLのみ)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,
    /// ### 機能追加
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub act_as: Option<ActAsJson>,
    /// ### ER図のリレーションを非表示
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub hide_er_relations: bool,
    /// ### ユニークモデルID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<u64>,

    /// ### フィールド
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<FieldJson>,
    /// ### belongs_to リレーション
    /// 他のモデルのIDを参照している場合は設定必須
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub belongs_to: Vec<BelongsToJson>,
    /// ### belongs_to_outer_db リレーション
    /// 他のDBのモデルを参照するbelongs_to
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub belongs_to_outer_db: Vec<BelongsToOuterDbJson>,
    /// ### has_one リレーション
    /// 同時に取得する、または検索条件に含まれる場合に設定が必要
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub has_one: Vec<HasOneJson>,
    /// ### has_many リレーション
    /// 同時に取得する、または検索条件に含まれる場合に設定が必要
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub has_many: Vec<HasManyJson>,
    /// ### インデックス
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub indexes: Vec<IndexJson>,
    /// ### セレクタ
    /// API等での取得条件を設定
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub selectors: Vec<SelectorJson>,
}

impl From<ModelDef> for ModelJson {
    fn from(value: ModelDef) -> Self {
        let created = value._before_rename_name.is_some();
        Self {
            merged_fields: value.merged_fields.into_iter().collect(),
            merged_relations: value.merged_relations.into_iter().collect(),
            name: value.name,
            _before_rename_name: value._before_rename_name,
            _soft_delete: value._soft_delete,
            label: value.label,
            comment: value.comment,
            table_name: value.table_name,
            skip_ddl: value.skip_ddl,
            dummy_always_joinable: value.dummy_always_joinable,
            upsert_conflict_target: value.upsert_conflict_target,
            ignore_foreign_key: value.ignore_foreign_key,
            timestampable: value.timestampable,
            disable_created_at: value.disable_created_at,
            disable_updated_at: value.disable_updated_at,
            soft_delete: value.soft_delete,
            versioned: value.versioned,
            counting: value.counting,
            use_cache: value.use_cache,
            use_all_rows_cache: value.use_all_rows_cache,
            use_filtered_row_cache: value.use_filtered_row_cache,
            use_clear_whole_cache: value.use_clear_whole_cache,
            overwrite_on_relation_save: value.overwrite_on_relation_save,
            use_update_notice: value.use_update_notice,
            use_insert_delayed: value.use_insert_delayed,
            use_save_delayed: value.use_save_delayed,
            use_update_delayed: value.use_update_delayed,
            use_upsert_delayed: value.use_upsert_delayed,
            disable_update: value.disable_update,
            disable_delete: value.disable_delete,
            disable_insert_cache_propagation: value.disable_insert_cache_propagation,
            abstract_mode: value.abstract_mode,
            inheritance: value.inheritance.map(|v| v.into()),
            engine: value.engine,
            act_as: value.act_as.map(|v| v.into()),
            hide_er_relations: value.hide_er_relations,
            model_id: value.model_id,
            fields: value
                .fields
                .into_iter()
                .map(|(k, v)| {
                    let mut v: FieldJson = v.exact().into();
                    v.name = k.clone();
                    if created && v._before_rename_name.is_none() {
                        v._before_rename_name = Some(k)
                    }
                    v
                })
                .collect(),
            has_one: value
                .has_one
                .into_iter()
                .map(|(k, v)| {
                    let mut v: HasOneJson = v.unwrap_or_default().into();
                    v.name = k;
                    v
                })
                .collect(),
            has_many: value
                .has_many
                .into_iter()
                .map(|(k, v)| {
                    let mut v: HasManyJson = v.unwrap_or_default().into();
                    v.name = k;
                    v
                })
                .collect(),
            belongs_to: value
                .belongs_to
                .into_iter()
                .map(|(k, v)| {
                    let mut v: BelongsToJson = v.unwrap_or_default().into();
                    v.name = k;
                    v
                })
                .collect(),
            belongs_to_outer_db: value
                .belongs_to_outer_db
                .into_iter()
                .map(|(k, v)| {
                    let mut v: BelongsToOuterDbJson = v.into();
                    v.name = k;
                    v
                })
                .collect(),
            indexes: value
                .indexes
                .into_iter()
                .map(|(k, v)| {
                    let mut v: IndexJson = v.unwrap_or_default().into();
                    v.name = k;
                    v
                })
                .collect(),
            selectors: value
                .selectors
                .into_iter()
                .map(|(k, v)| {
                    let mut v: SelectorJson = v.into();
                    v.name = k;
                    v
                })
                .collect(),
        }
    }
}

impl TryFrom<ModelJson> for ModelDef {
    type Error = anyhow::Error;
    fn try_from(value: ModelJson) -> Result<Self, Self::Error> {
        Ok(Self {
            _before_rename_name: value._before_rename_name,
            _soft_delete: value._soft_delete,
            db: Default::default(),
            group_name: Default::default(),
            name: Default::default(),
            exclude_group_from_table_name: None,
            cache_owners: Default::default(),
            merged_fields: Default::default(),
            relations: Default::default(),
            merged_relations: Default::default(),
            merged_indexes: Default::default(),
            label: value.label,
            comment: value.comment,
            table_name: value.table_name,
            skip_ddl: value.skip_ddl,
            dummy_always_joinable: value.dummy_always_joinable,
            upsert_conflict_target: value.upsert_conflict_target,
            ignore_foreign_key: value.ignore_foreign_key,
            timestampable: value.timestampable,
            disable_created_at: value.disable_created_at,
            disable_updated_at: value.disable_updated_at,
            soft_delete: value.soft_delete,
            versioned: value.versioned,
            counting: value.counting,
            use_cache: value.use_cache,
            use_all_rows_cache: value.use_all_rows_cache,
            use_filtered_row_cache: value.use_filtered_row_cache,
            use_clear_whole_cache: value.use_clear_whole_cache,
            overwrite_on_relation_save: value.overwrite_on_relation_save,
            use_update_notice: value.use_update_notice,
            use_insert_delayed: value.use_insert_delayed,
            use_save_delayed: value.use_save_delayed,
            use_update_delayed: value.use_update_delayed,
            use_upsert_delayed: value.use_upsert_delayed,
            disable_update: value.disable_update,
            disable_delete: value.disable_delete,
            disable_insert_cache_propagation: value.disable_insert_cache_propagation,
            abstract_mode: value.abstract_mode,
            inheritance: value
                .inheritance
                .map(|v| v.try_into())
                .transpose()?
                .flatten(),
            engine: value.engine,
            act_as: value.act_as.map(|v| v.try_into()).transpose()?.flatten(),
            hide_er_relations: value.hide_er_relations,
            model_id: value.model_id,
            fields: value
                .fields
                .into_iter()
                .map(|v| {
                    let name = v.name.clone();
                    let mut v: FieldDef = v.into();
                    if v._before_rename_name == Some(name.clone()) {
                        v._before_rename_name = None;
                    }
                    (name, FieldDefOrSubsetType::Exact(v))
                })
                .collect(),
            belongs_to: value
                .belongs_to
                .into_iter()
                .map(|v| {
                    let name = v.name.clone();
                    let v: BelongsToDef = v.into();
                    if v == BelongsToDef::default() {
                        (name, None)
                    } else {
                        (name, Some(v))
                    }
                })
                .collect(),
            belongs_to_outer_db: value
                .belongs_to_outer_db
                .into_iter()
                .map(|v| {
                    let name = v.name.clone();
                    let v: BelongsToOuterDbDef = v.into();
                    (name, v)
                })
                .collect(),
            has_one: value
                .has_one
                .into_iter()
                .map(|v| {
                    let name = v.name.clone();
                    let v: HasOneDef = v.into();
                    if v == HasOneDef::default() {
                        (name, None)
                    } else {
                        (name, Some(v))
                    }
                })
                .collect(),
            has_many: value
                .has_many
                .into_iter()
                .map(|v| {
                    let name = v.name.clone();
                    let v: HasManyDef = v.into();
                    if v == HasManyDef::default() {
                        (name, None)
                    } else {
                        (name, Some(v))
                    }
                })
                .collect(),
            indexes: value
                .indexes
                .into_iter()
                .map(|v| {
                    let name = v.name.clone();
                    let v: IndexDef = v.into();
                    if v == IndexDef::default() {
                        (name, None)
                    } else {
                        (name, Some(v))
                    }
                })
                .collect(),
            selectors: value
                .selectors
                .into_iter()
                .map(|v| {
                    let name = v.name.clone();
                    let v: SelectorDef = v.into();
                    (name, v)
                })
                .collect(),
        })
    }
}

impl ModelDef {
    pub fn table_name(&self) -> String {
        match self.table_name {
            Some(ref n) => n.clone(),
            None => self.derive_table_name(self.exclude_group_from_table_name),
        }
    }
    pub fn derive_table_name(&self, exclude_group_from_table_name: Option<bool>) -> String {
        let name = if exclude_group_from_table_name == Some(true) {
            self.name.clone()
        } else {
            format!("{}_{}", &self.group_name, &self.name)
        };
        if CONFIG.read().unwrap().as_ref().unwrap().plural_table_name {
            to_plural(&name)
        } else {
            name
        }
    }

    pub fn has_table(&self) -> bool {
        !self.abstract_mode
            && (self.inheritance_type().is_none()
                || self.inheritance_type() == Some(InheritanceType::Concrete))
    }

    pub fn mod_name(&self) -> String {
        self.name.to_snake()
    }

    pub fn full_name(&self) -> String {
        format!("{}::{}", &self.group_name, &self.name)
    }

    pub fn dummy_always_joinable(&self) -> bool {
        self.dummy_always_joinable
    }

    pub fn act_as_session(&self) -> bool {
        self.act_as.as_ref().map(|v| v.session).unwrap_or_default()
    }

    pub fn act_as_job_queue(&self) -> bool {
        self.act_as
            .as_ref()
            .map(|v| v.job_queue)
            .unwrap_or_default()
    }

    pub fn inheritance_type(&self) -> Option<InheritanceType> {
        self.inheritance
            .as_ref()
            .map(|inheritance| inheritance._type)
    }

    pub fn inheritance_cond(&self, param: &str) -> String {
        if let Some(ref inheritance) = self.inheritance {
            if inheritance._type == InheritanceType::ColumnAggregation {
                let key_value = match inheritance.key_value.as_ref().unwrap() {
                    Value::Null => "null".to_owned(),
                    Value::Bool(b) => if_then_else!(*b, "true", "false").to_owned(),
                    Value::Number(n) => format!("{}", n),
                    Value::String(s) => format!("{:?}", s),
                    Value::Sequence(_) => panic!("invalid key_value"),
                    Value::Mapping(_) => panic!("invalid key_value"),
                    Value::Tagged(_) => panic!("invalid key_value"),
                };
                if is_mysql_mode() {
                    format!(
                        "`{}`={}{}",
                        inheritance.key_field.as_ref().unwrap(),
                        key_value,
                        param
                    )
                } else {
                    format!(
                        "\"{}\"={}{}",
                        inheritance.key_field.as_ref().unwrap(),
                        key_value,
                        param
                    )
                }
            } else {
                "".to_owned()
            }
        } else {
            "".to_owned()
        }
    }

    pub fn inheritance_set(&self) -> String {
        if let Some(ref inheritance) = self.inheritance {
            if inheritance._type == InheritanceType::ColumnAggregation {
                let key_value = match inheritance.key_value.as_ref().unwrap() {
                    Value::Null => "null".to_owned(),
                    Value::Bool(b) => if_then_else!(*b, "true", "false").to_owned(),
                    Value::Number(n) => format!("{}", n),
                    Value::String(s) => format!("{:?}.to_string().into()", s),
                    Value::Sequence(_) => panic!("invalid key_value"),
                    Value::Mapping(_) => panic!("invalid key_value"),
                    Value::Tagged(_) => panic!("invalid key_value"),
                };
                format!(
                    "self._data.r#{} = {};",
                    inheritance.key_field.as_ref().unwrap(),
                    key_value,
                )
            } else {
                "".to_owned()
            }
        } else {
            "".to_owned()
        }
    }

    pub fn inheritance_check(&self) -> String {
        if let Some(ref inheritance) = self.inheritance {
            if inheritance._type == InheritanceType::ColumnAggregation {
                let k = inheritance.key_field.as_ref().unwrap();
                match inheritance.key_value.as_ref().unwrap() {
                    Value::Null => "null".to_owned(),
                    Value::Bool(b) => format!("r#{k} == {}", if_then_else!(*b, "true", "false")),
                    Value::Number(n) => format!("r#{k} == {}", n),
                    Value::String(s) => format!("r#{k}.as_str() == {:?}", s),
                    Value::Sequence(_) => panic!("invalid key_value"),
                    Value::Mapping(_) => panic!("invalid key_value"),
                    Value::Tagged(_) => panic!("invalid key_value"),
                }
            } else {
                "".to_owned()
            }
        } else {
            "".to_owned()
        }
    }

    pub fn use_cache(&self) -> bool {
        self.use_cache
            .unwrap_or(CONFIG.read().unwrap().as_ref().unwrap().use_cache)
    }

    pub fn use_all_rows_cache(&self) -> bool {
        self.use_cache()
            && self
                .use_all_rows_cache
                .unwrap_or(CONFIG.read().unwrap().as_ref().unwrap().use_all_rows_cache)
    }

    pub fn use_filtered_row_cache(&self) -> bool {
        self.use_filtered_row_cache.unwrap_or(false)
    }

    pub fn use_clear_whole_cache(&self) -> bool {
        self.use_clear_whole_cache.unwrap_or(
            CONFIG
                .read()
                .unwrap()
                .as_ref()
                .unwrap()
                .use_clear_whole_cache,
        )
    }

    pub fn overwrite_on_relation_save(&self) -> bool {
        self.overwrite_on_relation_save.unwrap_or(
            !self.has_auto_primary() && self.is_soft_delete() && self.unique_index().is_empty(),
        )
    }

    pub fn use_update_notice(&self) -> bool {
        self.use_update_notice
            .unwrap_or(CONFIG.read().unwrap().as_ref().unwrap().use_update_notice)
    }

    pub fn use_insert_delayed(&self) -> bool {
        if !IS_MAIN_GROUP.relaxed_load() {
            return false;
        }
        self.use_insert_delayed
            .unwrap_or(CONFIG.read().unwrap().as_ref().unwrap().use_insert_delayed)
    }

    pub fn has_delayed_update(&self) -> bool {
        self.use_save_delayed() || self.use_update_delayed() || self.use_upsert_delayed()
    }

    pub fn use_save_delayed(&self) -> bool {
        if !IS_MAIN_GROUP.relaxed_load() {
            return false;
        }
        !self.disable_update()
            && self
                .use_save_delayed
                .unwrap_or(CONFIG.read().unwrap().as_ref().unwrap().use_save_delayed)
    }

    pub fn use_update_delayed(&self) -> bool {
        if !IS_MAIN_GROUP.relaxed_load() {
            return false;
        }
        !self.disable_update()
            && self
                .use_update_delayed
                .unwrap_or(CONFIG.read().unwrap().as_ref().unwrap().use_update_delayed)
    }

    pub fn use_upsert_delayed(&self) -> bool {
        if !IS_MAIN_GROUP.relaxed_load() {
            return false;
        }
        !self.disable_update()
            && self
                .use_upsert_delayed
                .unwrap_or(CONFIG.read().unwrap().as_ref().unwrap().use_upsert_delayed)
    }

    pub fn disable_update(&self) -> bool {
        self.disable_update
            .unwrap_or(CONFIG.read().unwrap().as_ref().unwrap().disable_update)
    }

    pub fn disable_delete(&self) -> bool {
        self.disable_delete
            .unwrap_or(CONFIG.read().unwrap().as_ref().unwrap().disable_delete)
    }

    pub fn ignore_foreign_key(&self) -> bool {
        self.ignore_foreign_key
            .unwrap_or(CONFIG.read().unwrap().as_ref().unwrap().ignore_foreign_key)
    }

    pub fn timestampable(&self) -> Option<Timestampable> {
        let timestampable =
            self.timestampable
                .or(CONFIG.read().unwrap().as_ref().unwrap().timestampable);
        if timestampable == Some(Timestampable::None) {
            return None;
        }
        timestampable
    }

    pub fn created_at_conf(&self) -> Option<Timestampable> {
        if self.disable_created_at {
            return None;
        }
        self.timestampable()
    }

    pub fn updated_at_conf(&self) -> Option<Timestampable> {
        if self.disable_updated_at {
            return None;
        }
        self.timestampable()
    }

    pub fn get_updated_at(&self) -> &FieldDef {
        self.merged_fields
            .get(ConfigDef::updated_at().as_str())
            .unwrap()
    }

    pub fn is_soft_delete(&self) -> bool {
        self.soft_delete().is_some()
    }
    pub fn soft_delete(&self) -> Option<SoftDelete> {
        let soft_delete = self
            .soft_delete
            .or(CONFIG.read().unwrap().as_ref().unwrap().soft_delete);
        if soft_delete == Some(SoftDelete::None) {
            return None;
        }
        soft_delete
    }
    pub fn soft_delete_col(&self) -> Option<CompactString> {
        match self.soft_delete() {
            Some(SoftDelete::None) => None,
            Some(SoftDelete::Time) => Some(ConfigDef::deleted_at()),
            Some(SoftDelete::Flag) => Some(ConfigDef::deleted()),
            Some(SoftDelete::UnixTime) => Some(ConfigDef::deleted()),
            None => None,
        }
    }

    fn replace_deleted_at(s: &str) -> String {
        static DELETED_AT: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"(?<!_)deleted_at(?!_)").unwrap());
        static MUT_DELETED_AT: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"(?<!_)mut_deleted_at(?!_)").unwrap());
        let s = DELETED_AT.replace_all(s, _to_ident_name(ConfigDef::deleted_at().as_str()));
        let s = MUT_DELETED_AT.replace_all(s.as_ref(), format!("mut_{}", ConfigDef::deleted_at()));
        s.to_string()
    }

    fn replace_deleted(s: &str) -> String {
        static DELETED: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?<!_)deleted(?!_)").unwrap());
        static MUT_DELETED: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"(?<!_)mut_deleted(?!_)").unwrap());
        let s = DELETED.replace_all(s, _to_ident_name(ConfigDef::deleted().as_str()));
        let s = MUT_DELETED.replace_all(s.as_ref(), format!("mut_{}", ConfigDef::deleted()));
        s.to_string()
    }

    pub fn soft_delete_tpl(&self, none: &str, time: &str, flag: &str) -> String {
        let op = self.soft_delete();
        let time = Self::replace_deleted_at(time);
        let flag = Self::replace_deleted(flag);
        match op {
            None => none.to_owned(),
            Some(SoftDelete::None) => none.replace("{pascal_name}", &self.name.to_pascal()),
            Some(SoftDelete::Time) => {
                let col = self
                    .merged_fields
                    .get(ConfigDef::deleted_at().as_str())
                    .unwrap();
                time.replace("{filter_type}", &col.get_filter_type(false))
                    .replace("{pascal_name}", &self.name.to_pascal())
                    .replace(
                        "{val}",
                        if_then_else!(
                            self.timestampable() == Some(Timestampable::RealTime),
                            "SystemTime::now()",
                            "conn.time()"
                        ),
                    )
            }
            Some(SoftDelete::Flag) => flag.replace("{pascal_name}", &self.name.to_pascal()),
            Some(SoftDelete::UnixTime) => flag.replace("{pascal_name}", &self.name.to_pascal()),
        }
    }

    pub fn soft_delete_tpl2(&self, none: &str, time: &str, flag: &str, time_num: &str) -> String {
        let op = self.soft_delete();
        let time_num = Self::replace_deleted(time_num);
        let unix_time_type = if is_mysql_mode() { "u32" } else { "i32" };
        match op {
            None => none.to_owned(),
            Some(SoftDelete::None) => self.soft_delete_tpl(none, time, flag),
            Some(SoftDelete::Time) => self.soft_delete_tpl(none, time, flag),
            Some(SoftDelete::Flag) => self.soft_delete_tpl(none, time, flag),
            Some(SoftDelete::UnixTime) => time_num
                .replace("{pascal_name}", &self.name.to_pascal())
                .replace("{u32}", unix_time_type),
        }
    }

    pub fn get_counting(&self) -> &String {
        self.counting.as_ref().unwrap()
    }

    pub fn get_counting_col(&self) -> String {
        if self.versioned {
            error_exit!(
                "In the {} model, both versioned and counting cannot be set.",
                &self.name
            )
        }
        let name = self.counting.as_ref().unwrap();
        self.merged_fields
            .get(name)
            .unwrap_or_else(|| {
                error_exit!("The {} model does not have a {} column.", &self.name, name)
            })
            .get_col_name(name)
            .to_string()
    }

    pub fn get_counting_type(&self) -> String {
        let name = self.counting.as_ref().unwrap();
        self.merged_fields
            .get(name)
            .unwrap_or_else(|| {
                error_exit!("The {} model does not have a {} column.", &self.name, name)
            })
            .get_inner_type(true, true)
    }

    pub fn all_fields(&self) -> Vec<(&String, &FieldDef)> {
        self.merged_fields.iter().collect()
    }
    pub fn all_fields_except_read_only_and_auto_inc(&self) -> Vec<(&String, &FieldDef)> {
        self.merged_fields
            .iter()
            .filter(|v| {
                v.1.query.is_none()
                    && v.1.auto != Some(AutoGeneration::AutoIncrement)
                    && !v.1.is_version
            })
            .collect()
    }
    pub fn all_except_secret(&self) -> Vec<(&String, &FieldDef)> {
        self.merged_fields
            .iter()
            .filter(|(_k, v)| !v.secret.unwrap_or_default())
            .collect()
    }
    pub fn all_except_secret_and_primary(&self) -> Vec<(&String, &FieldDef)> {
        self.merged_fields
            .iter()
            .filter(|(_k, v)| !v.secret.unwrap_or_default() && !v.primary)
            .collect()
    }
    pub fn nullable(&self) -> Vec<(&String, &FieldDef)> {
        self.merged_fields
            .iter()
            .filter(|(_k, v)| !v.not_null)
            .collect()
    }
    pub fn text(&self) -> Vec<(&String, &FieldDef)> {
        self.merged_fields
            .iter()
            .filter(|(_k, v)| {
                v.data_type == DataType::Char
                    || v.data_type == DataType::IdVarchar
                    || v.data_type == DataType::TextVarchar
                    || v.data_type == DataType::Text
            })
            .collect()
    }
    pub fn serializable(&self) -> Vec<(&String, &FieldDef)> {
        self.merged_fields.iter().collect()
    }
    pub fn serializable_cache(&self) -> Vec<(&String, &FieldDef)> {
        self.merged_fields
            .iter()
            .filter(|(_k, v)| !v.exclude_from_cache())
            .collect()
    }
    pub fn id(&self) -> Vec<(&String, &FieldDef)> {
        self.merged_fields
            .iter()
            .filter(|(_k, v)| v.main_primary)
            .collect()
    }
    pub fn id_auto_inc_or_seq(&self) -> Vec<(&String, &FieldDef)> {
        self.merged_fields
            .iter()
            .filter(|(_k, v)| {
                v.main_primary
                    && (v.auto == Some(AutoGeneration::AutoIncrement)
                        || v.auto == Some(AutoGeneration::Sequence))
            })
            .collect()
    }
    pub fn id_except_auto_increment(&self) -> Vec<(&String, &FieldDef)> {
        self.merged_fields
            .iter()
            .filter(|(_k, v)| {
                v.main_primary && (v.auto.is_none() || v.auto == Some(AutoGeneration::Uuid))
            })
            .collect()
    }
    pub fn primary_except(&self, except: &[String]) -> &str {
        if self
            .primaries()
            .iter()
            .filter(|(k, _v)| !except.contains(*k))
            .count()
            != 1
        {
            error_exit!(
                "{} model must have only one primary key other than the key for the relation.",
                self.name
            )
        }
        self.primaries()
            .iter()
            .filter(|(k, _v)| !except.contains(*k))
            .map(|(name, _)| name.as_str())
            .next_back()
            .unwrap_or_else(|| {
                error_exit!(
                    "{} model must have a primary key other than the key for the relation.",
                    self.name
                )
            })
    }
    pub fn primary_except_has_inner(&self, except: &[String]) -> bool {
        self.primaries()
            .iter()
            .filter(|(k, _v)| !except.contains(*k))
            .map(|(_, def)| {
                def.id_class.is_some() || def.rel.is_some() || def.outer_db_rel.is_some()
            })
            .next_back()
            .unwrap_or_else(|| {
                error_exit!(
                    "{} model must have a primary key other than the key for the relation.",
                    self.name
                )
            })
    }
    pub fn is_auto_primary_except(&self, except: &[String]) -> bool {
        self.primaries()
            .iter()
            .filter(|(k, _v)| !except.contains(*k))
            .next_back()
            .map(|(_, def)| def.auto.is_some())
            .unwrap_or_else(|| {
                error_exit!(
                    "{} model must have a primary key other than the key for the relation.",
                    self.name
                )
            })
    }
    pub fn primaries(&self) -> Vec<(&String, &FieldDef)> {
        self.merged_fields
            .iter()
            .filter(|(_k, v)| v.primary)
            .collect()
    }
    pub fn non_main_primaries(&self) -> Vec<(&String, &FieldDef)> {
        self.merged_fields
            .iter()
            .filter(|(_k, v)| v.primary && !v.main_primary)
            .collect()
    }
    pub fn relation_primaries(&self, rel_id: String) -> Vec<String> {
        self.merged_fields
            .iter()
            .filter(|(_k, v)| v.primary)
            .map(|(k, v)| {
                if !v.main_primary {
                    k.to_owned()
                } else {
                    rel_id.clone()
                }
            })
            .collect()
    }
    pub fn main_primary_nth(&self) -> usize {
        self.merged_fields
            .iter()
            .filter(|(_k, v)| v.primary)
            .position(|(_k, v)| v.main_primary)
            .unwrap_or_default()
    }
    pub fn non_primaries(&self) -> Vec<(&String, &FieldDef)> {
        self.merged_fields
            .iter()
            .filter(|(_k, v)| !v.primary)
            .collect()
    }
    pub fn non_primaries_except_read_only(&self) -> Vec<(&String, &FieldDef)> {
        self.merged_fields
            .iter()
            .filter(|(_k, v)| !v.primary && v.query.is_none() && !v.is_version)
            .collect()
    }
    pub fn non_primaries_except_created_at(&self) -> Vec<(&String, &FieldDef)> {
        self.merged_fields
            .iter()
            .filter(|(k, v)| {
                !v.primary
                    && v.query.is_none()
                    && !v.is_version
                    && !ConfigDef::created_at().as_str().eq(k.as_str())
            })
            .collect()
    }
    pub fn non_primaries_addable(&self) -> Vec<(&String, &FieldDef)> {
        self.merged_fields
            .iter()
            .filter(|(_k, v)| !v.primary && v.is_addable())
            .collect()
    }
    pub fn non_primaries_except_invisibles(&self, self_only: bool) -> Vec<(&String, &FieldDef)> {
        self.merged_fields
            .iter()
            .filter(|(_k, v)| !self_only || !v.in_abstract)
            .filter(|(k, v)| {
                !v.primary
                    && !ConfigDef::version().eq(&**k)
                    && !ConfigDef::aggregation_type().eq(&**k)
            })
            .collect()
    }
    pub fn non_primaries_except_invisible_and_read_only(
        &self,
        self_only: bool,
    ) -> Vec<(&String, &FieldDef)> {
        self.merged_fields
            .iter()
            .filter(|(_k, v)| !self_only || !v.in_abstract)
            .filter(|(k, v)| {
                !v.primary
                    && !ConfigDef::version().eq(&**k)
                    && !ConfigDef::aggregation_type().eq(&**k)
                    && v.query.is_none()
                    && !v.is_version
            })
            .collect()
    }
    pub fn only_version(&self) -> Vec<(&String, &FieldDef)> {
        self.merged_fields
            .iter()
            .filter(|(k, _v)| ConfigDef::version().eq(&**k))
            .collect()
    }
    pub fn cache_cols_except_primary(&self) -> Vec<(&String, &FieldDef)> {
        self.merged_fields
            .iter()
            .filter(|(_k, v)| !v.primary && !v.exclude_from_cache())
            .collect()
    }
    pub fn cache_cols(&self) -> Vec<(&String, &FieldDef)> {
        self.merged_fields
            .iter()
            .filter(|(_k, v)| !v.exclude_from_cache())
            .collect()
    }
    pub fn cols_except_primaries_and_invisibles(&self) -> Vec<(&String, &FieldDef)> {
        self.merged_fields
            .iter()
            .filter(|(k, v)| {
                !v.in_abstract
                    && !v.primary
                    && !ConfigDef::version().eq(&**k)
                    && !ConfigDef::aggregation_type().eq(&**k)
            })
            .collect()
    }
    pub fn cache_cols_except_primaries_and_invisibles(&self) -> Vec<(&String, &FieldDef)> {
        self.merged_fields
            .iter()
            .filter(|(k, v)| {
                !v.in_abstract
                    && !v.primary
                    && !v.exclude_from_cache()
                    && !ConfigDef::version().eq(&**k)
                    && !ConfigDef::aggregation_type().eq(&**k)
            })
            .collect()
    }
    pub fn non_cache_cols_except_primaries_and_invisibles(&self) -> Vec<(&String, &FieldDef)> {
        self.merged_fields
            .iter()
            .filter(|(k, v)| {
                !v.in_abstract
                    && !v.primary
                    && v.exclude_from_cache()
                    && !ConfigDef::version().eq(&**k)
                    && !ConfigDef::aggregation_type().eq(&**k)
            })
            .collect()
    }
    pub fn cache_cols_not_null_sized(&self) -> Vec<(&String, &FieldDef)> {
        self.merged_fields
            .iter()
            .filter(|(_k, v)| {
                !v.exclude_from_cache()
                    && v.not_null
                    && (v.data_type == DataType::Text
                        || v.data_type == DataType::Binary
                        || v.data_type == DataType::Varbinary
                        || v.data_type == DataType::Blob
                        || v.data_type == DataType::ArrayInt
                        || v.data_type == DataType::ArrayString
                        || v.data_type == DataType::Json
                        || v.data_type == DataType::Jsonb
                        || v.data_type == DataType::Geometry)
            })
            .collect()
    }
    pub fn cache_cols_null_sized(&self) -> Vec<(&String, &FieldDef)> {
        self.merged_fields
            .iter()
            .filter(|(_k, v)| {
                !v.exclude_from_cache()
                    && !v.not_null
                    && (v.data_type == DataType::Text
                        || v.data_type == DataType::Binary
                        || v.data_type == DataType::Varbinary
                        || v.data_type == DataType::Blob
                        || v.data_type == DataType::ArrayInt
                        || v.data_type == DataType::ArrayString
                        || v.data_type == DataType::Json
                        || v.data_type == DataType::Jsonb
                        || v.data_type == DataType::Geometry)
            })
            .collect()
    }
    pub fn all_fields_except_json(&self) -> Vec<(&String, &FieldDef)> {
        self.merged_fields
            .iter()
            .filter(|(_k, v)| {
                !(v.data_type == DataType::ArrayInt
                    || v.data_type == DataType::ArrayString
                    || v.data_type == DataType::Json
                    || v.data_type == DataType::Jsonb
                    || v.data_type == DataType::Point
                    || v.data_type == DataType::GeoPoint
                    || v.data_type == DataType::Geometry)
            })
            .collect()
    }
    pub fn equivalence_cache_fields_except_json(&self) -> Vec<(&String, &FieldDef)> {
        self.merged_fields
            .iter()
            .filter(|(k, v)| {
                v.is_equivalence()
                    && !v.exclude_from_cache()
                    && !ConfigDef::version().eq(&**k)
                    && !ConfigDef::aggregation_type().eq(&**k)
            })
            .filter(|(_k, v)| {
                !(v.data_type == DataType::ArrayInt
                    || v.data_type == DataType::ArrayString
                    || v.data_type == DataType::Json
                    || v.data_type == DataType::Jsonb
                    || v.data_type == DataType::Point
                    || v.data_type == DataType::GeoPoint
                    || v.data_type == DataType::Geometry)
            })
            .collect()
    }
    pub fn comparable_cache_fields_except_json(&self) -> Vec<(&String, &FieldDef)> {
        self.merged_fields
            .iter()
            .filter(|(k, v)| {
                v.is_comparable()
                    && !v.exclude_from_cache()
                    && !ConfigDef::version().eq(&**k)
                    && !ConfigDef::aggregation_type().eq(&**k)
            })
            .filter(|(_k, v)| {
                !(v.data_type == DataType::ArrayInt
                    || v.data_type == DataType::ArrayString
                    || v.data_type == DataType::Json
                    || v.data_type == DataType::Jsonb
                    || v.data_type == DataType::Point
                    || v.data_type == DataType::GeoPoint
                    || v.data_type == DataType::Geometry)
            })
            .collect()
    }
    pub fn string_cache_fields(&self) -> Vec<(&String, &FieldDef)> {
        self.merged_fields
            .iter()
            .filter(|(k, v)| {
                v.is_comparable()
                    && !v.exclude_from_cache()
                    && !ConfigDef::version().eq(&**k)
                    && !ConfigDef::aggregation_type().eq(&**k)
            })
            .filter(|(_k, v)| {
                v.data_type == DataType::Char
                    || v.data_type == DataType::IdVarchar
                    || v.data_type == DataType::TextVarchar
                    || v.data_type == DataType::Text
            })
            .collect()
    }
    pub fn all_fields_only_json(&self) -> Vec<(&String, &FieldDef)> {
        self.merged_fields
            .iter()
            .filter(|(_k, v)| {
                v.data_type == DataType::ArrayInt
                    || v.data_type == DataType::ArrayString
                    || v.data_type == DataType::Json
                    || v.data_type == DataType::Jsonb
            })
            .collect()
    }
    pub fn all_fields_only_geo(&self) -> Vec<(&String, &FieldDef)> {
        self.merged_fields
            .iter()
            .filter(|(_k, v)| {
                v.data_type == DataType::Point
                    || v.data_type == DataType::GeoPoint
                    || v.data_type == DataType::Geometry
            })
            .collect()
    }

    pub fn auto_primary(&self) -> Vec<(&String, &FieldDef)> {
        self.merged_fields
            .iter()
            .filter(|(_k, v)| v.auto.is_some())
            .collect()
    }
    pub fn has_auto_primary(&self) -> bool {
        !self.auto_primary().is_empty()
    }
    pub fn auto_inc_or_seq(&self) -> Vec<(&String, &FieldDef)> {
        self.merged_fields
            .iter()
            .filter(|(_k, v)| {
                v.auto == Some(AutoGeneration::AutoIncrement)
                    || v.auto == Some(AutoGeneration::Sequence)
            })
            .collect()
    }
    pub fn auto_inc(&self) -> Vec<(&String, &FieldDef)> {
        self.merged_fields
            .iter()
            .filter(|(_k, v)| v.auto == Some(AutoGeneration::AutoIncrement))
            .collect()
    }
    pub fn is_auto_inc(&self) -> bool {
        self.merged_fields
            .iter()
            .any(|(_k, v)| v.auto == Some(AutoGeneration::AutoIncrement))
    }
    pub fn auto_seq(&self) -> Vec<(&String, &FieldDef)> {
        self.merged_fields
            .iter()
            .filter(|(_k, v)| v.auto == Some(AutoGeneration::Sequence))
            .collect()
    }
    pub fn auto_uuid(&self) -> Vec<(&String, &FieldDef)> {
        self.merged_fields
            .iter()
            .filter(|(_k, v)| v.auto == Some(AutoGeneration::Uuid))
            .collect()
    }
    #[allow(dead_code)]
    pub fn except_auto_increment(&self) -> Vec<(&String, &FieldDef)> {
        self.merged_fields
            .iter()
            .filter(|(_k, v)| v.auto.is_none())
            .collect()
    }
    pub fn for_factory(&self) -> Vec<(&String, &FieldDef)> {
        self.merged_fields
            .iter()
            .filter(|(_k, v)| !v.skip_factory())
            .collect()
    }
    pub fn non_auto_primary_for_factory(&self) -> Vec<(&String, &FieldDef)> {
        self.merged_fields
            .iter()
            .filter(|(_k, v)| !v.skip_factory() && v.auto.is_none())
            .collect()
    }
    pub fn for_cmp(&self) -> Vec<(&String, &FieldDef)> {
        self.merged_fields
            .iter()
            .filter(|(_k, v)| !v.skip_factory() && !v.primary)
            .collect()
    }
    pub fn for_api_response(&self) -> Vec<(&String, &FieldDef)> {
        self.merged_fields
            .iter()
            .filter(|(k, v)| {
                !v.exclude_from_cache() && ApiFieldDef::has(k) && ApiFieldDef::check(k, false)
            })
            .collect()
    }
    pub fn for_api_response_not_in_request(&self) -> Vec<(&String, &FieldDef)> {
        let pri: HashMap<_, _> = self.auto_primary().into_iter().collect();
        let req: HashMap<_, _> = self.for_api_request().into_iter().collect();
        self.for_api_response()
            .into_iter()
            .filter(|(k, v)| !pri.contains_key(k) && !req.contains_key(k) && !v.is_version)
            .collect()
    }
    pub fn for_api_request(&self) -> Vec<(&String, &FieldDef)> {
        self.merged_fields
            .iter()
            .filter(|(k, v)| {
                (!v.skip_factory() || v.is_version)
                    && v.auto.is_none()
                    && ApiFieldDef::has(k)
                    && ApiFieldDef::check(k, true)
                    && (v.primary || !v.is_cascade_on_delete())
            })
            .collect()
    }
    pub fn for_api_update_updater(&self) -> Vec<(&String, &FieldDef)> {
        self.merged_fields
            .iter()
            .filter(|(k, v)| {
                !v.skip_factory()
                    && v.auto.is_none()
                    && v.data_type != DataType::Binary
                    && v.data_type != DataType::Varbinary
                    && v.data_type != DataType::Blob
                    && ApiFieldDef::has(k)
                    && ApiFieldDef::check(k, true)
                    && !ApiFieldDef::disable_update(k)
                    && !v.is_cascade_on_delete()
                    && !v.primary
            })
            .collect()
    }
    pub fn for_api_request_except(&self, except: &[String]) -> Vec<(&String, &FieldDef)> {
        self.for_api_request()
            .into_iter()
            .filter(|(k, _v)| !except.contains(*k))
            .collect()
    }
    pub fn for_api_request_except_primary_and(
        &self,
        except: &[String],
    ) -> Vec<(&String, &FieldDef)> {
        self.for_api_request()
            .into_iter()
            .filter(|(k, _v)| !except.contains(*k))
            .filter(|(_, v)| !v.primary)
            .collect()
    }
    pub fn fields_with_default(&self) -> Vec<(&String, &FieldDef)> {
        self.for_api_request()
            .into_iter()
            .filter(|(k, field)| ApiFieldDef::default(k, field).is_some())
            .collect()
    }
    pub fn multi_index(&self, cache_only: bool) -> Vec<(String, IndexDef)> {
        let mut map = BTreeMap::new();
        for (index_name, def) in &self.merged_indexes {
            if def.fields.len() > 1
                && def.fields.iter().all(|(n, _)| {
                    let col = self.merged_fields.get(n).unwrap_or_else(|| {
                    error_exit!(
                        "The {n} field of the {index_name} index in the {} model does not exist.",
                        self.name
                    )
                });
                    if cache_only && col.exclude_from_cache() {
                        return false;
                    }
                    if !col.is_comparable() {
                        return false;
                    }
                    true
                })
            {
                let mut def = def.clone();
                def.fields.retain(|k, _| {
                    !ConfigDef::deleted_at().eq_ignore_ascii_case(k)
                        && !ConfigDef::deleted().eq_ignore_ascii_case(k)
                });
                for i in 2..=def.fields.len() {
                    let mut def = def.clone();
                    def.fields.truncate(i);
                    let fields: Vec<_> = def.fields.iter().map(|(n, _)| n.to_string()).collect();
                    let names: Vec<_> = fields.iter().map(|v| v.to_pascal()).collect();
                    map.insert(names.join("_"), def);
                }
            }
        }
        for (model, name, rel) in self.relations_belonging(false) {
            let local_id = rel.get_local_id(name, model);
            if local_id.len() > 1 {
                let fields: Vec<_> = local_id
                    .iter()
                    .map(|local_id| {
                        if let Some(local_col) = self.merged_fields.get(local_id) {
                            local_col.get_col_name(local_id).to_string()
                        } else {
                            local_id.clone()
                        }
                    })
                    .collect();
                let names: Vec<_> = fields.iter().map(|v| v.to_pascal()).collect();
                map.insert(
                    names.join("_"),
                    IndexDef {
                        fields: fields.into_iter().map(|v| (v, None)).collect(),
                        ..Default::default()
                    },
                );
            }
        }
        for (selector_name, selector) in &self.selectors {
            for (order_name, order) in &selector.orders {
                if order.fields.len() > 1
                    && order.fields.iter().all(|(n, _)| {
                        let col = self
                            .merged_fields
                            .get(n)
                            .unwrap_or_else(|| error_exit!("The {n} field of the {order_name} order of the {selector_name} selector in the {} model does not exist.", self.name));
                        if !col.is_comparable() {
                            error_exit!("The {n} field of the {order_name} order of the {selector_name} selector in the {} model cannot be sorted.", self.name);
                        }
                        true
                    })
                {
                    let fields: Vec<_> = order.fields.iter().map(|(n, _)| n.to_string()).collect();
                    let names: Vec<_> = fields.iter().map(|v| v.to_pascal()).collect();
                    map.insert(
                        names.join("_"),
                        IndexDef {
                            fields: fields.into_iter().map(|v| (v, None)).collect(),
                            ..Default::default()
                        },
                    );
                }
            }
        }
        map.into_iter().collect()
    }
    pub fn unique_index(&self) -> Vec<(&String, &IndexDef)> {
        self.merged_indexes
            .iter()
            .filter(|v| v.1.index_type == Some(IndexType::Unique))
            .map(|v| (v.0, v.1))
            .collect()
    }
    pub fn unique_key(&self) -> Vec<(&String, &FieldDef)> {
        let mut chk = HashSet::new();
        self.merged_indexes
            .iter()
            .filter(|v| v.1.index_type == Some(IndexType::Unique))
            .flat_map(|v| {
                let name = v.0;
                let index = v.1;
                let mut ret = Vec::new();
                if !index.fields.is_empty() {
                    for row in &index.fields {
                        if row.0 == ConfigDef::deleted().as_str() || chk.contains(row.0) {
                            continue;
                        }
                        let col = self
                            .merged_fields
                            .get(row.0)
                            .unwrap_or_else(|| error_exit!("{} index is not in fields", row.0));
                        chk.insert(row.0.clone());
                        ret.push((row.0, col));
                    }
                } else if !chk.contains(name) {
                    let col = self
                        .merged_fields
                        .get(name)
                        .unwrap_or_else(|| error_exit!("{} index is not in fields", name));
                    chk.insert(name.clone());
                    ret.push((name, col));
                }
                ret
            })
            .collect()
    }
    pub fn relations(&self) -> Vec<(&ModelDef, &String, &RelDef)> {
        self.merged_relations
            .iter()
            .filter(|v| !v.1.is_type_of_belongs_to_outer_db() && v.1.rel_type.is_some())
            .map(|v| (self, v.0, v.1))
            .collect()
    }
    pub fn relations_in_cache(&self) -> Vec<(&ModelDef, &String, &RelDef)> {
        self.merged_relations
            .iter()
            .filter(|v| v.1.is_type_of_has() && v.1.in_cache())
            .map(|v| (self, v.0, v.1))
            .collect()
    }
    pub fn belongs_to_outer_db(&self) -> Vec<(&String, &RelDef)> {
        self.merged_relations
            .iter()
            .filter(|v| v.1.is_type_of_belongs_to_outer_db())
            .map(|v| (v.0, v.1))
            .collect()
    }
    pub fn relations_one_and_belonging(
        &self,
        self_only: bool,
    ) -> Vec<(&ModelDef, &String, &RelDef)> {
        self.merged_relations
            .iter()
            .filter(|v| !self_only || !v.1.in_abstract)
            .filter(|v| v.1.is_type_of_belongs_to() || v.1.is_type_of_has_one())
            .map(|v| (self, v.0, v.1))
            .collect()
    }
    pub fn relations_belonging(&self, self_only: bool) -> Vec<(&ModelDef, &String, &RelDef)> {
        self.merged_relations
            .iter()
            .filter(|v| !self_only || !v.1.in_abstract)
            .filter(|v| ApiRelationDef::has(v.0))
            .filter(|v| v.1.is_type_of_belongs_to())
            .map(|v| (self, v.0, v.1))
            .collect()
    }
    pub fn relations_belonging_cache(&self, self_only: bool) -> Vec<(&ModelDef, &String, &RelDef)> {
        self.merged_relations
            .iter()
            .filter(|v| !self_only || !v.1.in_abstract)
            .filter(|v| v.1.is_type_of_belongs_to() && v.1.get_foreign_model().use_cache())
            .map(|v| (self, v.0, v.1))
            .collect()
    }
    pub fn relations_belonging_uncached(
        &self,
        self_only: bool,
    ) -> Vec<(&ModelDef, &String, &RelDef)> {
        self.merged_relations
            .iter()
            .filter(|v| !self_only || !v.1.in_abstract)
            .filter(|v| v.1.is_type_of_belongs_to() && !v.1.get_foreign_model().use_cache())
            .map(|v| (self, v.0, v.1))
            .collect()
    }
    pub fn relations_belonging_for_api_response(&self) -> Vec<(&ModelDef, &String, &RelDef)> {
        self.merged_relations
            .iter()
            .filter(|v| ApiRelationDef::check(v.0, false))
            .filter(|v| v.1.is_type_of_belongs_to())
            .map(|v| (self, v.0, v.1))
            .collect()
    }
    pub fn relations_belonging_outer_db(
        &self,
        self_only: bool,
    ) -> Vec<(&ModelDef, &String, &RelDef)> {
        self.merged_relations
            .iter()
            // .filter(|v| ApiRelationDef::has(v.0))
            .filter(|v| !self_only || !v.1.in_abstract)
            .filter(|v| v.1.is_type_of_belongs_to_outer_db())
            .map(|v| (self, v.0, v.1))
            .collect()
    }
    pub fn relations_one_and_many(&self, self_only: bool) -> Vec<(&ModelDef, &String, &RelDef)> {
        self.merged_relations
            .iter()
            .filter(|v| !self_only || !v.1.in_abstract)
            .filter(|v| v.1.is_type_of_has_one() || v.1.is_type_of_has_many())
            .map(|v| (self, v.0, v.1))
            .collect()
    }
    pub fn relations_one(&self, self_only: bool) -> Vec<(&ModelDef, &String, &RelDef)> {
        self.merged_relations
            .iter()
            .filter(|v| !self_only || !v.1.in_abstract)
            .filter(|v| ApiRelationDef::has(v.0))
            .filter(|v| v.1.is_type_of_has_one())
            .map(|v| (self, v.0, v.1))
            .collect()
    }
    pub fn relations_one_cache(&self, self_only: bool) -> Vec<(&ModelDef, &String, &RelDef)> {
        self.merged_relations
            .iter()
            .filter(|v| !self_only || !v.1.in_abstract)
            .filter(|v| v.1.is_type_of_has_one() && v.1.in_cache())
            .map(|v| (self, v.0, v.1))
            .collect()
    }
    pub fn relations_one_uncached(&self, self_only: bool) -> Vec<(&ModelDef, &String, &RelDef)> {
        self.merged_relations
            .iter()
            .filter(|v| !self_only || !v.1.in_abstract)
            .filter(|v| v.1.is_type_of_has_one() && !v.1.in_cache())
            .map(|v| (self, v.0, v.1))
            .collect()
    }
    pub fn relations_one_for_api_response(&self) -> Vec<(&ModelDef, &String, &RelDef)> {
        self.merged_relations
            .iter()
            .filter(|v| ApiRelationDef::check(v.0, false))
            .filter(|v| v.1.is_type_of_has_one())
            .map(|v| (self, v.0, v.1))
            .collect()
    }
    pub fn relations_many(&self, self_only: bool) -> Vec<(&ModelDef, &String, &RelDef)> {
        self.merged_relations
            .iter()
            .filter(|v| !self_only || !v.1.in_abstract)
            .filter(|v| ApiRelationDef::has(v.0))
            .filter(|v| v.1.is_type_of_has_many())
            .map(|v| (self, v.0, v.1))
            .collect()
    }
    pub fn relations_many_without_limit(&self) -> Vec<(&ModelDef, &String, &RelDef)> {
        self.merged_relations
            .iter()
            .filter(|v| v.1.is_type_of_has_many() && v.1.limit.is_none())
            .map(|v| (self, v.0, v.1))
            .collect()
    }
    pub fn relations_many_with_limit(&self) -> Vec<(&ModelDef, &String, &RelDef)> {
        self.merged_relations
            .iter()
            .filter(|v| v.1.is_type_of_has_many() && v.1.limit.is_some())
            .map(|v| (self, v.0, v.1))
            .collect()
    }
    pub fn relations_many_cache(&self, self_only: bool) -> Vec<(&ModelDef, &String, &RelDef)> {
        self.merged_relations
            .iter()
            .filter(|v| !self_only || !v.1.in_abstract)
            .filter(|v| v.1.is_type_of_has_many() && v.1.in_cache())
            .map(|v| (self, v.0, v.1))
            .collect()
    }
    pub fn relations_many_cache_without_limit(&self) -> Vec<(&ModelDef, &String, &RelDef)> {
        self.merged_relations
            .iter()
            .filter(|v| v.1.is_type_of_has_many() && v.1.in_cache() && v.1.limit.is_none())
            .map(|v| (self, v.0, v.1))
            .collect()
    }
    pub fn relations_many_cache_with_limit(&self) -> Vec<(&ModelDef, &String, &RelDef)> {
        self.merged_relations
            .iter()
            .filter(|v| v.1.is_type_of_has_many() && v.1.in_cache() && v.1.limit.is_some())
            .map(|v| (self, v.0, v.1))
            .collect()
    }
    pub fn relations_many_uncached(&self, self_only: bool) -> Vec<(&ModelDef, &String, &RelDef)> {
        self.merged_relations
            .iter()
            .filter(|v| !self_only || !v.1.in_abstract)
            .filter(|v| v.1.is_type_of_has_many() && !v.1.in_cache())
            .map(|v| (self, v.0, v.1))
            .collect()
    }
    pub fn relations_many_uncached_without_limit(&self) -> Vec<(&ModelDef, &String, &RelDef)> {
        self.merged_relations
            .iter()
            .filter(|v| v.1.is_type_of_has_many() && !v.1.in_cache() && v.1.limit.is_none())
            .map(|v| (self, v.0, v.1))
            .collect()
    }
    pub fn relations_many_uncached_with_limit(&self) -> Vec<(&ModelDef, &String, &RelDef)> {
        self.merged_relations
            .iter()
            .filter(|v| v.1.is_type_of_has_many() && !v.1.in_cache() && v.1.limit.is_some())
            .map(|v| (self, v.0, v.1))
            .collect()
    }
    pub fn relations_many_for_api_response(&self) -> Vec<(&ModelDef, &String, &RelDef)> {
        self.merged_relations
            .iter()
            .filter(|v| ApiRelationDef::check(v.0, false))
            .filter(|v| v.1.is_type_of_has_many())
            .map(|v| (self, v.0, v.1))
            .collect()
    }
    pub fn relations_many_for_api_request(&self) -> Vec<(&ModelDef, &String, &RelDef)> {
        self.merged_relations
            .iter()
            .filter(|v| ApiRelationDef::check(v.0, true))
            .filter(|v| v.1.is_type_of_has_many())
            .map(|v| (self, v.0, v.1))
            .collect()
    }
    pub fn relations_one_for_api_request(&self) -> Vec<(&ModelDef, &String, &RelDef)> {
        self.merged_relations
            .iter()
            .filter(|v| ApiRelationDef::check(v.0, true))
            .filter(|v| v.1.is_type_of_has_one())
            .map(|v| (self, v.0, v.1))
            .collect()
    }
    pub fn relations_one_for_api_request_with_replace_type(
        &self,
        is_replace: bool,
    ) -> Vec<(&ModelDef, &String, &RelDef)> {
        self.merged_relations
            .iter()
            .filter(|v| ApiRelationDef::check(v.0, true))
            .filter(|v| ApiRelationDef::get(v.0).unwrap().use_replace == is_replace)
            .filter(|v| v.1.is_type_of_has_one())
            .map(|v| (self, v.0, v.1))
            .collect()
    }
    pub fn relation_constraint(&self) -> Vec<(&ModelDef, &String, &RelDef)> {
        self.merged_relations
            .iter()
            .filter(|v| v.1.is_type_of_belongs_to() && !v.1.disable_index)
            .map(|v| (self, v.0, v.1))
            .collect()
    }
    pub fn outer_db_relation_constraint(&self) -> Vec<(&ModelDef, &String, &RelDef)> {
        self.merged_relations
            .iter()
            .filter(|v| v.1.is_type_of_belongs_to_outer_db() && !v.1.disable_index)
            .map(|v| (self, v.0, v.1))
            .collect()
    }

    pub fn relation_mods(&self) -> Vec<Vec<String>> {
        let mut mods: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for (_name, rel) in self
            .merged_relations
            .iter()
            .filter(|v| !v.1.is_type_of_belongs_to_outer_db() && v.1.rel_type.is_some())
        {
            let group_name = rel.get_group_name().to_snake();
            let mod_name = rel.get_mod_name();
            if let std::collections::btree_map::Entry::Vacant(e) = mods.entry(group_name.clone()) {
                let mut list = BTreeSet::new();
                list.insert(mod_name);
                e.insert(list);
            } else {
                mods.get_mut(&group_name).unwrap().insert(mod_name);
            }
        }
        let mut vec = Vec::new();
        for (group_name, list) in mods.into_iter() {
            for mod_name in list {
                vec.push(vec![group_name.clone(), mod_name]);
            }
        }
        vec
    }
    pub fn num_enums(&self, is_domain: bool) -> Vec<(&String, FieldDef)> {
        let mut vec: Vec<(&String, FieldDef)> = self
            .fields
            .iter()
            .map(|(k, v)| (k, v.exact()))
            .filter(|(_k, v)| {
                v.data_type == DataType::TinyInt
                    || v.data_type == DataType::SmallInt
                    || v.data_type == DataType::Int
                    || v.data_type == DataType::BigInt
            })
            .filter(|(_k, v)| v.enum_values.is_some())
            .filter(|(_k, v)| !is_domain || v.value_object.is_none())
            .collect();
        vec.sort_by(|a, b| a.0.cmp(b.0));
        vec
    }
    pub fn str_enums(&self, is_domain: bool) -> Vec<(&String, FieldDef)> {
        let mut vec: Vec<(&String, FieldDef)> = self
            .fields
            .iter()
            .map(|(k, v)| (k, v.exact()))
            .filter(|(_k, v)| {
                !(v.data_type == DataType::TinyInt
                    || v.data_type == DataType::SmallInt
                    || v.data_type == DataType::Int
                    || v.data_type == DataType::BigInt)
            })
            .filter(|(_k, v)| v.enum_values.is_some())
            .filter(|(_k, v)| !is_domain || v.value_object.is_none())
            .collect();
        vec.sort_by(|a, b| a.0.cmp(b.0));
        vec
    }
    pub fn parent(&self) -> Vec<Arc<ModelDef>> {
        let mut cur = self.inheritance.clone();
        let mut cur_group_name: String = self.group_name.clone();
        while let Some(ref inheritance) = cur {
            let model = RelDef::get_model_by_name(&inheritance.extends, &cur_group_name);
            cur_group_name = model.group_name.clone();
            cur.clone_from(&model.inheritance);
            if model.abstract_mode {
                return vec![model];
            }
        }
        Vec::new()
    }
    pub fn parents(&self) -> Vec<Arc<ModelDef>> {
        let mut vec = Vec::new();
        let mut cur = self.inheritance.clone();
        let mut cur_group_name: String = self.group_name.clone();
        while let Some(ref inheritance) = cur {
            let model = RelDef::get_model_by_name(&inheritance.extends, &cur_group_name);
            cur_group_name = model.group_name.clone();
            cur.clone_from(&model.inheritance);
            if model.abstract_mode {
                vec.push(model.clone());
            }
        }
        vec
    }
    pub fn downcast_simple(&self) -> Vec<Arc<ModelDef>> {
        let mut vec = Vec::new();
        let mut cur = self.inheritance.clone();
        let mut cur_group_name: String = self.group_name.clone();
        while let Some(ref inheritance) = cur {
            if inheritance._type == InheritanceType::Simple {
                let model = RelDef::get_model_by_name(&inheritance.extends, &cur_group_name);
                cur_group_name = model.group_name.clone();
                cur.clone_from(&model.inheritance);
                vec.push(model.clone());
            } else {
                break;
            }
        }
        if let Some(model) = vec.pop() {
            vec![model]
        } else {
            Vec::new()
        }
    }
    pub fn downcast_aggregation(&self) -> Vec<Arc<ModelDef>> {
        let mut vec = Vec::new();
        let mut cur = self.inheritance.clone();
        let mut cur_group_name: String = self.group_name.clone();
        while let Some(ref inheritance) = cur {
            if inheritance._type == InheritanceType::ColumnAggregation {
                let model = RelDef::get_model_by_name(&inheritance.extends, &cur_group_name);
                cur_group_name = model.group_name.clone();
                cur.clone_from(&model.inheritance);
                vec.push(model.clone());
            } else {
                break;
            }
        }
        if let Some(model) = vec.pop() {
            vec![model]
        } else {
            Vec::new()
        }
    }

    pub fn get_type_id(&self, target: &str) -> u64 {
        static TYPE_IDS: Lazy<std::sync::Mutex<HashSet<u64>>> =
            Lazy::new(|| std::sync::Mutex::new(HashSet::new()));
        let mut hash = hash(&format!(
            "{} {} {} {}",
            target, self.db, self.group_name, self.name
        ));
        let mut ids = TYPE_IDS.lock().unwrap();
        while ids.contains(&hash) {
            hash = hash.wrapping_add(1);
        }
        ids.insert(hash);
        hash
    }

    pub fn upsert_conflict_target(&self) -> String {
        if is_mysql_mode() {
            return String::new();
        }
        if let Some(upsert_conflict_target) = &self.upsert_conflict_target {
            if let Some(index) = self.merged_indexes.get(upsert_conflict_target) {
                if index.index_type != Some(IndexType::Unique) {
                    error_exit!("{upsert_conflict_target} is not a unique index.");
                }
                format!(
                    "({})",
                    index
                        .fields(upsert_conflict_target, self)
                        .iter()
                        .map(|(name, def)| format!("\"{}\"", def.get_col_name(name)))
                        .collect::<Vec<_>>()
                        .join(",")
                )
            } else {
                error_exit!("{upsert_conflict_target} is not a unique index.");
            }
        } else {
            format!(
                "({})",
                self.primaries()
                    .iter()
                    .map(|(name, def)| format!("\"{}\"", def.get_col_name(name)))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        }
    }
}
