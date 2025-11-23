use clap::ValueEnum;
use compact_str::CompactString;
use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use strum_macros::{AsRefStr, EnumString};

use crate::common::ToCase as _;

use super::{AGGREGATION_TYPE, CREATED_AT, DELETED, DELETED_AT, ModelDef, UPDATED_AT, VERSION};

/// ### データベース設定
#[derive(Debug, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConfigDef {
    /// ### 仕様書等のタイトル
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// ### 仕様書等の著者
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// ### データベースID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub db_id: Option<u64>,
    /// ### データベース
    pub db: DbType,
    /// ### デフォルトで外部キー制約をマイグレーションのDDLに出力しない
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub ignore_foreign_key: bool,
    /// ### リレーションのインデックスを自動生成しない
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub disable_relation_index: bool,
    /// ### テーブル名を複数形にする
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub plural_table_name: bool,
    /// ### この階層の名前を隠す
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub hidden_layer: bool,
    /// ### 論理削除のデフォルト設定
    #[serde(skip_serializing_if = "Option::is_none")]
    pub soft_delete: Option<SoftDelete>,
    /// ### リレーションの自動インデックスに論理削除カラムを追加する
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub add_soft_delete_column_to_relation_index: bool,
    /// ### デフォルトのタイムスタンプ設定
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestampable: Option<Timestampable>,
    /// ### 日時型のデフォルトのタイムゾーン設定
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_time_zone: Option<TimeZone>,
    /// ### タイムスタンプタイムゾーン
    /// created_at, updated_at, deleted_atに使用されるタイムゾーン
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp_time_zone: Option<TimeZone>,
    /// ### タイムスタンプキャッシュ無効化
    /// created_at, updated_atをキャッシュせず、APIでも取得しない
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub disable_timestamp_cache: bool,
    /// ### デフォルトでキャッシュを使用する
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub use_cache: bool,
    /// ### 高速キャッシュを使用する
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub use_fast_cache: bool,
    /// ### ストレージキャッシュを使用する(EXPERIMENTAL)
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub use_storage_cache: bool,
    /// ### デフォルトで全行キャッシュを使用する
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub use_all_rows_cache: bool,
    /// ### 全てのキャッシュを強制的に無効化
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub force_disable_cache: bool,
    /// ### デフォルトで更新時に常にすべてのキャッシュをクリアする
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub use_clear_whole_cache: bool,
    /// ### デフォルトで更新通知を使用する
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub use_update_notice: bool,
    /// ### デフォルトで遅延INSERTを使用する
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub use_insert_delayed: bool,
    /// ### デフォルトで遅延SAVEを使用する
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub use_save_delayed: bool,
    /// ### デフォルトで遅延UPDATEを使用する
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub use_update_delayed: bool,
    /// ### デフォルトで遅延UPSERTを使用する
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub use_upsert_delayed: bool,
    /// ### デフォルトで更新を無効化する
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub disable_update: bool,
    /// ### デフォルトで削除を無効化する
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub disable_delete: bool,
    /// ### シーケンスを使用する
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub use_sequence: bool,
    /// ### 更新トランザクション分離レベル
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_isolation: Option<Isolation>,
    /// ### 参照トランザクション分離レベル
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_tx_isolation: Option<Isolation>,
    /// ### ストレージエンジン
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,
    // /// ### 文字セット
    // #[serde(default, skip_serializing_if = "Option::is_none")]
    // pub character_set: Option<String>,
    /// ### ID用文字セット照合順序
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id_collation: Option<String>,
    /// ### テキスト用文字セット照合順序
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_collation: Option<String>,
    /// ### DDL出力時のカラム順序を維持する(MySQLのみ)
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub preserve_column_order: bool,
    /// ### ドメイン生成から除外
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub exclude_from_domain: bool,
    /// ### 論理名をカラムのSQLコメントとして使用
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub use_label_as_sql_comment: bool,
    /// ### 強制的にdatetimeを使用する(MySQLのみ)
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub force_datetime_on_mysql: bool,
    /// ### created_atに別名を使用
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rename_created_at: Option<String>,
    /// ### created_atのラベル
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_of_created_at: Option<String>,
    /// ### updated_atに別名を使用
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rename_updated_at: Option<String>,
    /// ### updated_atのラベル
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_of_updated_at: Option<String>,
    /// ### deleted_atに別名を使用
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rename_deleted_at: Option<String>,
    /// ### deleted_atのラベル
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_of_deleted_at: Option<String>,
    /// ### deletedに別名を使用
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rename_deleted: Option<String>,
    /// ### deletedのラベル
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_of_deleted: Option<String>,
    /// ### カラム集約の_typeに別名を使用
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rename_aggregation_type: Option<String>,
    /// ### カラム集約の_typeのラベル
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_of_aggregation_type: Option<String>,
    /// ### versionに別名を使用
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rename_version: Option<String>,
    /// ### versionのラベル
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_of_version: Option<String>,
    /// ### existsのNO_SEMIJOINを無効化(MySQLのみ)
    /// SEMIJOINはexistsをanyに変換するため、existsとanyの区別ができなくなる。
    /// 明示的にexistsとanyを区別するのではなく、MySQLに任せる場合にdisable_no_semijoinをtureにする。
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub disable_no_semijoin: bool,
    /// ### モデルグループ
    pub groups: IndexMap<String, Option<GroupDef>>,
}

/// ### データベース設定
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConfigJson {
    /// ### 仕様書等のタイトル
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// ### 仕様書等の著者
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// ### データベースID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub db_id: Option<u64>,
    /// ### データベース
    pub db: DbType,
    /// ### デフォルトで外部キー制約をマイグレーションのDDLに出力しない
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub ignore_foreign_key: bool,
    /// ### リレーションのインデックスを自動生成しない
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub disable_relation_index: bool,
    /// ### テーブル名を複数形にする
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub plural_table_name: bool,
    /// ### この階層の名前を隠す
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub hidden_layer: bool,
    /// ### 論理削除のデフォルト設定
    #[serde(skip_serializing_if = "Option::is_none")]
    pub soft_delete: Option<SoftDelete>,
    /// ### リレーションの自動インデックスに論理削除カラムを追加する
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub add_soft_delete_column_to_relation_index: bool,
    /// ### デフォルトのタイムスタンプ設定
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestampable: Option<Timestampable>,
    /// ### 日時型のデフォルトのタイムゾーン設定
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_time_zone: Option<TimeZone>,
    /// ### タイムスタンプタイムゾーン
    /// created_at, updated_at, deleted_atに使用されるタイムゾーン
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp_time_zone: Option<TimeZone>,
    /// ### タイムスタンプキャッシュ無効化
    /// created_at, updated_atをキャッシュせず、APIでも取得しない
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub disable_timestamp_cache: bool,
    /// ### デフォルトでキャッシュを使用する
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub use_cache: bool,
    /// ### 高速キャッシュを使用する
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub use_fast_cache: bool,
    /// ### ストレージキャッシュを使用する(EXPERIMENTAL)
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub use_storage_cache: bool,
    /// ### デフォルトで全行キャッシュを使用する
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub use_all_rows_cache: bool,
    /// ### 全てのキャッシュを強制的に無効化
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub force_disable_cache: bool,
    /// ### デフォルトで更新時に常にすべてのキャッシュをクリアする
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub use_clear_whole_cache: bool,
    /// ### デフォルトで更新通知を使用する
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub use_update_notice: bool,
    /// ### デフォルトで遅延INSERTを使用する
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub use_insert_delayed: bool,
    /// ### デフォルトで遅延SAVEを使用する
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub use_save_delayed: bool,
    /// ### デフォルトで遅延UPDATEを使用する
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub use_update_delayed: bool,
    /// ### デフォルトで遅延UPSERTを使用する
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub use_upsert_delayed: bool,
    /// ### デフォルトで更新を無効化する
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub disable_update: bool,
    /// ### デフォルトで削除を無効化する
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub disable_delete: bool,
    /// ### シーケンスを使用する
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub use_sequence: bool,
    /// ### 更新トランザクション分離レベル
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_isolation: Option<Isolation>,
    /// ### 参照トランザクション分離レベル
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_tx_isolation: Option<Isolation>,
    /// ### ストレージエンジン
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,
    // /// ### 文字セット
    // #[serde(default, skip_serializing_if = "Option::is_none")]
    // pub character_set: Option<String>,
    /// ### ID用文字セット照合順序
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id_collation: Option<String>,
    /// ### テキスト用文字セット照合順序
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_collation: Option<String>,
    /// ### DDL出力時のカラム順序を維持する(MySQLのみ)
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub preserve_column_order: bool,
    /// ### ドメイン生成から除外
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub exclude_from_domain: bool,
    /// ### 論理名をカラムのSQLコメントとして使用
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub use_label_as_sql_comment: bool,
    /// ### 強制的にdatetimeを使用する(MySQLのみ)
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub force_datetime_on_mysql: bool,
    /// ### created_atに別名を使用
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rename_created_at: Option<String>,
    /// ### created_atのラベル
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_of_created_at: Option<String>,
    /// ### updated_atに別名を使用
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rename_updated_at: Option<String>,
    /// ### updated_atのラベル
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_of_updated_at: Option<String>,
    /// ### deleted_atに別名を使用
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rename_deleted_at: Option<String>,
    /// ### deleted_atのラベル
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_of_deleted_at: Option<String>,
    /// ### deletedに別名を使用
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rename_deleted: Option<String>,
    /// ### deletedのラベル
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_of_deleted: Option<String>,
    /// ### カラム集約の_typeに別名を使用
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rename_aggregation_type: Option<String>,
    /// ### カラム集約の_typeのラベル
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_of_aggregation_type: Option<String>,
    /// ### versionに別名を使用
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rename_version: Option<String>,
    /// ### versionのラベル
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_of_version: Option<String>,
    /// ### existsのNO_SEMIJOINを無効化(MySQLのみ)
    /// SEMIJOINはexistsをanyに変換するため、existsとanyの区別ができなくなる。
    /// 明示的にexistsとanyを区別するのではなく、MySQLに任せる場合にdisable_no_semijoinをtureにする。
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub disable_no_semijoin: bool,
    /// ### モデルグループ
    pub groups: Vec<GroupJson>,
}

impl From<ConfigDef> for ConfigJson {
    fn from(value: ConfigDef) -> Self {
        Self {
            title: value.title,
            author: value.author,
            db_id: value.db_id,
            db: value.db,
            ignore_foreign_key: value.ignore_foreign_key,
            disable_relation_index: value.disable_relation_index,
            plural_table_name: value.plural_table_name,
            hidden_layer: value.hidden_layer,
            timestampable: value.timestampable,
            output_time_zone: value.output_time_zone,
            timestamp_time_zone: value.timestamp_time_zone,
            disable_timestamp_cache: value.disable_timestamp_cache,
            soft_delete: value.soft_delete,
            add_soft_delete_column_to_relation_index: value
                .add_soft_delete_column_to_relation_index,
            use_cache: value.use_cache,
            use_fast_cache: value.use_fast_cache,
            use_storage_cache: value.use_storage_cache,
            use_all_rows_cache: value.use_all_rows_cache,
            force_disable_cache: value.force_disable_cache,
            use_clear_whole_cache: value.use_clear_whole_cache,
            use_update_notice: value.use_update_notice,
            use_insert_delayed: value.use_insert_delayed,
            use_save_delayed: value.use_save_delayed,
            use_update_delayed: value.use_update_delayed,
            use_upsert_delayed: value.use_upsert_delayed,
            disable_update: value.disable_update,
            disable_delete: value.disable_delete,
            use_sequence: value.use_sequence,
            tx_isolation: value.tx_isolation,
            read_tx_isolation: value.read_tx_isolation,
            engine: value.engine,
            // character_set: value.character_set,
            id_collation: value.id_collation,
            text_collation: value.text_collation,
            preserve_column_order: value.preserve_column_order,
            exclude_from_domain: value.exclude_from_domain,
            use_label_as_sql_comment: value.use_label_as_sql_comment,
            force_datetime_on_mysql: value.force_datetime_on_mysql,
            rename_created_at: value.rename_created_at,
            label_of_created_at: value.label_of_created_at,
            rename_updated_at: value.rename_updated_at,
            label_of_updated_at: value.label_of_updated_at,
            rename_deleted_at: value.rename_deleted_at,
            label_of_deleted_at: value.label_of_deleted_at,
            rename_deleted: value.rename_deleted,
            label_of_deleted: value.label_of_deleted,
            rename_aggregation_type: value.rename_aggregation_type,
            label_of_aggregation_type: value.label_of_aggregation_type,
            rename_version: value.rename_version,
            label_of_version: value.label_of_version,
            disable_no_semijoin: value.disable_no_semijoin,
            groups: value
                .groups
                .into_iter()
                .map(|(k, v)| {
                    let mut v: GroupJson = v.unwrap_or_default().into();
                    v.name.clone_from(&k);
                    v._name = Some(k);
                    v
                })
                .collect(),
        }
    }
}

impl From<ConfigJson> for ConfigDef {
    fn from(value: ConfigJson) -> Self {
        Self {
            title: value.title,
            author: value.author,
            db_id: value.db_id,
            db: value.db,
            ignore_foreign_key: value.ignore_foreign_key,
            disable_relation_index: value.disable_relation_index,
            plural_table_name: value.plural_table_name,
            hidden_layer: value.hidden_layer,
            timestampable: value.timestampable,
            output_time_zone: value.output_time_zone,
            timestamp_time_zone: value.timestamp_time_zone,
            disable_timestamp_cache: value.disable_timestamp_cache,
            soft_delete: value.soft_delete,
            add_soft_delete_column_to_relation_index: value
                .add_soft_delete_column_to_relation_index,
            use_cache: value.use_cache,
            use_fast_cache: value.use_fast_cache,
            use_storage_cache: value.use_storage_cache,
            use_all_rows_cache: value.use_all_rows_cache,
            force_disable_cache: value.force_disable_cache,
            use_clear_whole_cache: value.use_clear_whole_cache,
            use_update_notice: value.use_update_notice,
            use_insert_delayed: value.use_insert_delayed,
            use_save_delayed: value.use_save_delayed,
            use_update_delayed: value.use_update_delayed,
            use_upsert_delayed: value.use_upsert_delayed,
            disable_update: value.disable_update,
            disable_delete: value.disable_delete,
            use_sequence: value.use_sequence,
            tx_isolation: value.tx_isolation,
            read_tx_isolation: value.read_tx_isolation,
            engine: value.engine,
            // character_set: value.character_set,
            id_collation: value.id_collation,
            text_collation: value.text_collation,
            preserve_column_order: value.preserve_column_order,
            exclude_from_domain: value.exclude_from_domain,
            use_label_as_sql_comment: value.use_label_as_sql_comment,
            force_datetime_on_mysql: value.force_datetime_on_mysql,
            rename_created_at: value.rename_created_at,
            label_of_created_at: value.label_of_created_at,
            rename_updated_at: value.rename_updated_at,
            label_of_updated_at: value.label_of_updated_at,
            rename_deleted_at: value.rename_deleted_at,
            label_of_deleted_at: value.label_of_deleted_at,
            rename_deleted: value.rename_deleted,
            label_of_deleted: value.label_of_deleted,
            rename_aggregation_type: value.rename_aggregation_type,
            label_of_aggregation_type: value.label_of_aggregation_type,
            rename_version: value.rename_version,
            label_of_version: value.label_of_version,
            disable_no_semijoin: value.disable_no_semijoin,
            groups: value
                .groups
                .into_iter()
                .map(|v| {
                    let name = v.name.clone();
                    let v: GroupDef = v.into();
                    if v == GroupDef::default() {
                        (name, None)
                    } else {
                        (name, Some(v))
                    }
                })
                .collect(),
        }
    }
}

impl ConfigDef {
    pub fn fix_static_vars(&self) {
        let mut v = CREATED_AT.write().unwrap();
        v.clear();
        v.push_str(self.rename_created_at.as_deref().unwrap_or("created_at"));

        let mut v = UPDATED_AT.write().unwrap();
        v.clear();
        v.push_str(self.rename_updated_at.as_deref().unwrap_or("updated_at"));

        let mut v = DELETED_AT.write().unwrap();
        v.clear();
        v.push_str(self.rename_deleted_at.as_deref().unwrap_or("deleted_at"));

        let mut v = DELETED.write().unwrap();
        v.clear();
        v.push_str(self.rename_deleted.as_deref().unwrap_or("deleted"));

        let mut v = AGGREGATION_TYPE.write().unwrap();
        v.clear();
        v.push_str(self.rename_aggregation_type.as_deref().unwrap_or("_type"));

        let mut v = VERSION.write().unwrap();
        v.clear();
        v.push_str(self.rename_version.as_deref().unwrap_or("_version"));
    }

    pub fn db_id(&self) -> u64 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64;
        self.db_id.unwrap_or(now)
    }

    pub fn db_type_short(&self) -> &str {
        match self.db {
            DbType::Mysql => "MySql",
            DbType::Postgres => "Pg",
        }
    }

    pub fn db_type_long(&self) -> &str {
        match self.db {
            DbType::Mysql => "MySql",
            DbType::Postgres => "Postgres",
        }
    }

    pub fn db_type_switch(&self, mysql: &'static str, pgsql: &'static str) -> &'static str {
        match self.db {
            DbType::Mysql => mysql,
            DbType::Postgres => pgsql,
        }
    }

    pub fn is_mysql(&self) -> bool {
        self.db == DbType::Mysql
    }

    pub fn signed_only(&self) -> bool {
        match self.db {
            DbType::Mysql => false,
            DbType::Postgres => true,
        }
    }

    pub fn use_fast_cache(&self) -> bool {
        self.use_fast_cache
    }

    pub fn created_at() -> CompactString {
        super::CREATED_AT.read().unwrap().clone()
    }

    pub fn updated_at() -> CompactString {
        super::UPDATED_AT.read().unwrap().clone()
    }

    pub fn deleted_at() -> CompactString {
        super::DELETED_AT.read().unwrap().clone()
    }

    pub fn deleted() -> CompactString {
        super::DELETED.read().unwrap().clone()
    }

    pub fn aggregation_type() -> CompactString {
        super::AGGREGATION_TYPE.read().unwrap().clone()
    }

    pub fn version() -> CompactString {
        super::VERSION.read().unwrap().clone()
    }

    pub fn max_db_str_len(&self) -> u64 {
        match self.db {
            DbType::Mysql => 4 * 1024 * 1024 * 1024 - 1,
            DbType::Postgres => 1024 * 1024 * 1024 - 1,
        }
    }

    pub fn outer_db(&self) -> BTreeSet<String> {
        let mut v = BTreeSet::new();
        let group_lock = super::GROUPS.read().unwrap();
        let groups = group_lock.as_ref().unwrap();
        for (_, models) in groups {
            for (_, model) in &models.1 {
                for (_, belongs) in &model.1.belongs_to_outer_db() {
                    v.insert(belongs.db().to_string());
                }
            }
        }
        v
    }

    pub fn layer_name(&self, db: &str, group_name: &str) -> String {
        let mut name = String::new();
        if !self.hidden_layer {
            name.push_str(&db.to_pascal());
        }
        let group = self
            .groups
            .get(group_name)
            .cloned()
            .unwrap_or_default()
            .unwrap_or_default();
        if !group.hidden_layer {
            name.push_str(&group_name.to_pascal());
        }
        name
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
/// ### グループ定義
pub struct GroupDef {
    /// ### 論理名
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// ### テーブル名にグループ名を使用しない
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub exclude_group_from_table_name: bool,
    /// ### この階層の名前を隠す
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub hidden_layer: bool,
    /// モデル数が少ない場合はここにモデルを記述可能
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub models: IndexMap<String, ModelDef>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
/// ### グループ定義
pub struct GroupJson {
    pub id: Option<u64>,
    /// ### グループ名
    /// スネークケース推奨
    #[schemars(regex(pattern = r"^[A-Za-z][_0-9A-Za-z]*(?<!_)$"))]
    pub name: String,
    pub _name: Option<String>,
    /// ### 論理名
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// ### テーブル名にグループ名を使用しない
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub exclude_group_from_table_name: bool,
    /// ### この階層の名前を隠す
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub hidden_layer: bool,
}

impl From<GroupDef> for GroupJson {
    fn from(value: GroupDef) -> Self {
        use crc::{CRC_64_ECMA_182, Crc};
        pub const CRC64: Crc<u64> = Crc::<u64>::new(&CRC_64_ECMA_182);

        Self {
            id: Some(CRC64.checksum(value.label.clone().unwrap_or_default().as_bytes())),
            name: String::new(),
            _name: None,
            label: value.label,
            exclude_group_from_table_name: value.exclude_group_from_table_name,
            hidden_layer: value.hidden_layer,
        }
    }
}

impl From<GroupJson> for GroupDef {
    fn from(value: GroupJson) -> Self {
        Self {
            label: value.label,
            exclude_group_from_table_name: value.exclude_group_from_table_name,
            hidden_layer: value.hidden_layer,
            models: IndexMap::new(),
        }
    }
}

#[derive(
    Debug,
    derive_more::Display,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Copy,
    Clone,
    Default,
    JsonSchema,
    ValueEnum,
)]
#[serde(rename_all = "lowercase")]
/// ### データベースタイプ
pub enum DbType {
    #[default]
    #[display("mysql")]
    Mysql,
    #[display("postgres")]
    Postgres,
}

impl DbType {
    pub fn is_mysql(&self) -> bool {
        *self == DbType::Mysql
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Copy, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
/// ### タイムスタンプ設定
pub enum Timestampable {
    /// ### タイムスタンプなし
    None,
    /// ### クエリー実行日時
    RealTime,
    /// ### アクセス日時
    FixedTime,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Copy, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
/// ### タイムゾーン
pub enum TimeZone {
    /// ### UTC
    /// 保存、取得ともにUTC、ISO 8601フォーマット
    Utc,
    /// ### ローカル
    /// 保存はUTC、取得はサーバのローカル、ISO 8601フォーマット
    Local,
}

#[derive(
    Debug, PartialEq, Eq, Serialize, Deserialize, Copy, Clone, JsonSchema, AsRefStr, EnumString,
)]
#[serde(rename_all = "snake_case")]
/// ### 論理削除
pub enum SoftDelete {
    /// ### 論理削除なし
    None,
    /// ### 日時型論理削除
    Time,
    /// ### フラグ型論理削除
    Flag,
    /// ### UNIXタイムスタンプ型論理削除
    /// ユニーク制約に使用するためのUNIXタイムスタンプ
    UnixTime,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Copy, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
/// ### トランザクション分離レベル
pub enum Isolation {
    /// ### REPEATABLE READ
    RepeatableRead,
    /// ### READ COMMITTED
    ReadCommitted,
    /// ### READ UNCOMMITTED
    ReadUncommitted,
    /// ### SERIALIZABLE
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
