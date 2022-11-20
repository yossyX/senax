use convert_case::{Case, Casing};
use indexmap::IndexMap;
use schemars::schema::{InstanceType, Schema, SchemaObject, SingleOrVec};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::collections::HashSet;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use crate::common::{hash, if_then_else};

use super::{
    AutoIncrement, ColumnDef, ColumnType, ColumnTypeOrDef, IndexDef, IndexType, ReferenceOption,
    RelDef, RelationsType, SoftDelete, Timestampable, CONFIG, DELETED, DELETED_AT, TYPE_IDS,
};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, JsonSchema)]
#[schemars(deny_unknown_fields)]
#[schemars(title = "Inheritance")]
pub struct Inheritance {
    /// 継承元
    pub extends: String,
    /// 継承タイプ
    #[serde(rename = "type")]
    pub type_def: InheritanceType,
    /// column_aggregationの場合のキーカラム
    pub key_field: Option<String>,
    /// column_aggregationの場合のキーの値
    #[schemars(default, schema_with = "value_schema")]
    pub key_value: Option<Value>,
}
fn value_schema(_: &mut schemars::gen::SchemaGenerator) -> Schema {
    let schema = SchemaObject {
        instance_type: Some(SingleOrVec::Vec(vec![
            InstanceType::Boolean,
            InstanceType::Number,
            InstanceType::String,
            InstanceType::Integer,
        ])),
        ..Default::default()
    };
    Schema::Object(schema)
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Copy, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(title = "Inheritance Type")]
pub enum InheritanceType {
    /// 単一テーブル継承
    /// 子テーブルのカラムも含めたすべてのカラムを親となるテーブルに格納する
    Simple,
    /// 具象テーブル継承
    /// 子クラスごとに共通のカラムとそれぞれのモデルのカラムをすべて含んだ状態で独立したテーブルを作成する
    Concrete,
    /// カラム集約テーブル継承
    /// 単一テーブル継承と似ているが、型を特定するための _type カラムがある
    ColumnAggregation,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[schemars(deny_unknown_fields)]
#[schemars(title = "Model Def")]
pub struct ModelDef {
    #[serde(skip)]
    pub db: String,
    #[serde(skip)]
    pub group_name: String,
    #[serde(skip)]
    pub name: String,
    #[serde(default, skip)]
    pub on_delete_list: BTreeSet<String>,
    #[serde(default, skip)]
    pub merged_columns: IndexMap<String, ColumnDef>,
    #[serde(default, skip)]
    pub merged_relations: IndexMap<String, Option<RelDef>>,
    #[serde(default, skip)]
    pub merged_indexes: IndexMap<String, IndexDef>,

    /// 仕様書等のためのタイトル
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// コメント
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// テーブル名
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table_name: Option<String>,
    /// falseの場合は外部キー制約をDDLに出力しない
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore_foreign_key: Option<bool>,
    /// タイムスタンプ設定
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestampable: Option<Timestampable>,
    /// created_atの無効化
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub disable_created_at: bool,
    /// updated_atの無効化
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub disable_updated_at: bool,
    /// 論理削除設定
    #[serde(skip_serializing_if = "Option::is_none")]
    pub soft_delete: Option<SoftDelete>,
    /// キャッシュ整合性のためのバージョンを使用するか
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub versioned: bool,
    /// save_delayedでカウンターを使用するカラム
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub counting: Option<String>,
    /// キャッシュを使用するか
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_cache: Option<bool>,
    /// 高速キャッシュを使用するか(experimental)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_fast_cache: Option<bool>,
    /// 全キャッシュを使用するか
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_cache_all: Option<bool>,
    /// 他サーバでinsertされたデータをキャッシュするか
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub ignore_propagated_insert_cache: bool,
    /// 物理削除時の_before_deleteと_after_deleteの呼び出しを行うか
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub on_delete_fn: bool,
    /// 抽象化モード
    #[serde(default, skip_serializing_if = "super::is_false")]
    #[serde(rename = "abstract")]
    pub abstract_mode: bool,
    /// 継承モード
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inheritance: Option<Inheritance>,
    /// MySQLのストレージエンジン
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,
    /// 文字セット
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub character_set: Option<String>,
    /// 文字セット照合順序
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collate: Option<String>,
    /// 名前にマルチバイトを使用した場合のmod名
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(regex(pattern = r"^[A-Za-z][0-9A-Z_a-z]*$"))]
    pub mod_name: Option<String>,

    /// カラム
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub columns: IndexMap<String, ColumnTypeOrDef>,
    /// リレーション
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub relations: IndexMap<String, Option<RelDef>>,
    /// インデックス
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub indexes: IndexMap<String, Option<IndexDef>>,
}

impl ModelDef {
    pub fn table_name(&self) -> String {
        match self.table_name {
            Some(ref n) => n.clone(),
            None => format!("{}_{}", &self.group_name, &self.name),
        }
    }

    pub fn has_table(&self) -> bool {
        !self.abstract_mode
            && (self.inheritance_type() == None
                || self.inheritance_type() == Some(InheritanceType::Concrete))
    }

    pub fn mod_name(&self) -> &str {
        self.mod_name.as_ref().unwrap_or(&self.name)
    }

    pub fn inheritance_type(&self) -> Option<InheritanceType> {
        self.inheritance
            .as_ref()
            .map(|inheritance| inheritance.type_def)
    }

    pub fn inheritance_cond(&self, param: &str) -> String {
        if let Some(ref inheritance) = self.inheritance {
            if inheritance.type_def == InheritanceType::ColumnAggregation {
                let key_value = match inheritance.key_value.as_ref().unwrap() {
                    Value::Null => "null".to_owned(),
                    Value::Bool(b) => if_then_else!(*b, "true", "false").to_owned(),
                    Value::Number(n) => format!("{}", n),
                    Value::String(s) => format!("{:?}", s),
                    Value::Sequence(_) => panic!("invalid key_value"),
                    Value::Mapping(_) => panic!("invalid key_value"),
                };
                format!(
                    "`{}`={}{}",
                    inheritance.key_field.as_ref().unwrap(),
                    key_value,
                    param
                )
            } else {
                "".to_owned()
            }
        } else {
            "".to_owned()
        }
    }

    pub fn inheritance_set(&self) -> String {
        if let Some(ref inheritance) = self.inheritance {
            if inheritance.type_def == InheritanceType::ColumnAggregation {
                let key_value = match inheritance.key_value.as_ref().unwrap() {
                    Value::Null => "null".to_owned(),
                    Value::Bool(b) => if_then_else!(*b, "true", "false").to_owned(),
                    Value::Number(n) => format!("{}", n),
                    Value::String(s) => format!("{:?}.to_string()", s),
                    Value::Sequence(_) => panic!("invalid key_value"),
                    Value::Mapping(_) => panic!("invalid key_value"),
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
            if inheritance.type_def == InheritanceType::ColumnAggregation {
                let key_value = match inheritance.key_value.as_ref().unwrap() {
                    Value::Null => "null".to_owned(),
                    Value::Bool(b) => if_then_else!(*b, "true", "false").to_owned(),
                    Value::Number(n) => format!("{}", n),
                    Value::String(s) => format!("{:?}", s),
                    Value::Sequence(_) => panic!("invalid key_value"),
                    Value::Mapping(_) => panic!("invalid key_value"),
                };
                format!(
                    "r#{} == {}",
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

    pub fn use_cache(&self) -> bool {
        self.use_cache
            .or(unsafe { CONFIG.get().unwrap() }.use_cache)
            .unwrap_or(false)
    }

    pub fn use_fast_cache(&self) -> bool {
        self.use_fast_cache.unwrap_or(false)
    }

    pub fn use_cache_all(&self) -> bool {
        self.use_cache_all
            .or(unsafe { CONFIG.get().unwrap() }.use_cache_all)
            .unwrap_or(false)
    }

    pub fn ignore_foreign_key(&self) -> bool {
        self.ignore_foreign_key
            .unwrap_or(unsafe { CONFIG.get().unwrap() }.ignore_foreign_key)
    }

    pub fn timestampable(&self) -> Option<Timestampable> {
        let timestampable = self
            .timestampable
            .or(unsafe { CONFIG.get().unwrap() }.timestampable);
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

    pub fn get_updated_at(&self) -> &ColumnDef {
        self.merged_columns.get("updated_at").unwrap()
    }

    pub fn soft_delete(&self) -> Option<SoftDelete> {
        let soft_delete = self
            .soft_delete
            .or(unsafe { CONFIG.get().unwrap() }.soft_delete);
        if soft_delete == Some(SoftDelete::None) {
            return None;
        }
        soft_delete
    }

    pub fn soft_delete_tpl(&self, none: &str, time: &str, flag: &str) -> String {
        let op = self.soft_delete();
        match op {
            None => none.to_owned(),
            Some(SoftDelete::None) => {
                none.replace("{pascal_name}", &self.name.to_case(Case::Pascal))
            }
            Some(SoftDelete::Time) => {
                let col = self.merged_columns.get(DELETED_AT).unwrap();
                time.replace("{cond_type}", &col.get_cond_type())
                    .replace("{pascal_name}", &self.name.to_case(Case::Pascal))
                    .replace(
                        "{val}",
                        if_then_else!(
                            self.timestampable() == Some(Timestampable::RealTime),
                            "SystemTime::now()",
                            "conn.time()"
                        ),
                    )
            }
            Some(SoftDelete::Flag) => {
                flag.replace("{pascal_name}", &self.name.to_case(Case::Pascal))
            }
            Some(SoftDelete::UnixTime) => {
                flag.replace("{pascal_name}", &self.name.to_case(Case::Pascal))
            }
        }
    }

    pub fn soft_delete_tpl2(&self, none: &str, time: &str, flag: &str, time_num: &str) -> String {
        let op = self.soft_delete();
        match op {
            None => none.to_owned(),
            Some(SoftDelete::None) => self.soft_delete_tpl(none, time, flag),
            Some(SoftDelete::Time) => self.soft_delete_tpl(none, time, flag),
            Some(SoftDelete::Flag) => self.soft_delete_tpl(none, time, flag),
            Some(SoftDelete::UnixTime) => {
                time_num.replace("{pascal_name}", &self.name.to_case(Case::Pascal))
            }
        }
    }

    pub fn get_counting(&self) -> &String {
        self.counting.as_ref().unwrap()
    }

    pub fn get_counting_col(&self) -> String {
        let name = self.counting.as_ref().unwrap();
        self.merged_columns
            .get(name)
            .unwrap_or_else(|| panic!("The {} model does not have a {} column.", &self.name, name))
            .get_col_name(name)
            .to_string()
    }

    pub fn get_counting_type(&self) -> String {
        let name = self.counting.as_ref().unwrap();
        self.merged_columns
            .get(name)
            .unwrap_or_else(|| panic!("The {} model does not have a {} column.", &self.name, name))
            .get_inner_type(&true)
    }

    pub fn all_columns(&self) -> Vec<(&String, &ColumnDef)> {
        self.merged_columns.iter().collect()
    }
    pub fn nullable(&self) -> Vec<(&String, &ColumnDef)> {
        self.merged_columns
            .iter()
            .filter(|(_k, v)| !v.not_null)
            .collect()
    }
    pub fn serializable(&self) -> Vec<(&String, &ColumnDef)> {
        self.merged_columns
            .iter()
            .filter(|(_k, v)| !v.not_serializable)
            .collect()
    }
    pub fn serializable_cache(&self) -> Vec<(&String, &ColumnDef)> {
        self.merged_columns
            .iter()
            .filter(|(_k, v)| !v.not_serializable && !v.exclude_from_cache)
            .collect()
    }
    pub fn id(&self) -> Vec<(&String, &ColumnDef)> {
        self.merged_columns
            .iter()
            .filter(|(_k, v)| v.main_primary)
            .collect()
    }
    pub fn id_auto_increment(&self) -> Vec<(&String, &ColumnDef)> {
        self.merged_columns
            .iter()
            .filter(|(_k, v)| v.main_primary && v.auto_increment == Some(AutoIncrement::Auto))
            .collect()
    }
    pub fn id_except_auto_increment(&self) -> Vec<(&String, &ColumnDef)> {
        self.merged_columns
            .iter()
            .filter(|(_k, v)| v.main_primary && v.auto_increment != Some(AutoIncrement::Auto))
            .collect()
    }
    pub fn id_name(&self) -> String {
        self.id().get(0).map(|v| v.0.clone()).unwrap_or_default()
    }
    pub fn primaries(&self) -> Vec<(&String, &ColumnDef)> {
        self.merged_columns
            .iter()
            .filter(|(_k, v)| v.primary)
            .collect()
    }
    pub fn non_primaries(&self) -> Vec<(&String, &ColumnDef)> {
        self.merged_columns
            .iter()
            .filter(|(_k, v)| !v.primary)
            .collect()
    }
    pub fn cache_cols_without_primary(&self) -> Vec<(&String, &ColumnDef)> {
        self.merged_columns
            .iter()
            .filter(|(_k, v)| !v.primary && !v.exclude_from_cache)
            .collect()
    }
    pub fn cache_cols(&self) -> Vec<(&String, &ColumnDef)> {
        self.merged_columns
            .iter()
            .filter(|(_k, v)| !v.exclude_from_cache)
            .collect()
    }
    pub fn cache_cols_not_null_sized(&self) -> Vec<(&String, &ColumnDef)> {
        self.merged_columns
            .iter()
            .filter(|(_k, v)| {
                !v.exclude_from_cache
                    && v.not_null
                    && (v.type_def == ColumnType::Text
                        || v.type_def == ColumnType::Blob
                        || v.type_def == ColumnType::ArrayInt
                        || v.type_def == ColumnType::ArrayString
                        || v.type_def == ColumnType::Json)
            })
            .collect()
    }
    pub fn cache_cols_null_sized(&self) -> Vec<(&String, &ColumnDef)> {
        self.merged_columns
            .iter()
            .filter(|(_k, v)| {
                !v.exclude_from_cache
                    && !v.not_null
                    && (v.type_def == ColumnType::Text
                        || v.type_def == ColumnType::Blob
                        || v.type_def == ColumnType::ArrayInt
                        || v.type_def == ColumnType::ArrayString
                        || v.type_def == ColumnType::Json)
            })
            .collect()
    }
    pub fn all_columns_without_json(&self) -> Vec<(&String, &ColumnDef)> {
        self.merged_columns
            .iter()
            .filter(|(_k, v)| {
                !(v.type_def == ColumnType::ArrayInt
                    || v.type_def == ColumnType::ArrayString
                    || v.type_def == ColumnType::Json
                    || v.type_def == ColumnType::Point)
            })
            .collect()
    }
    pub fn all_columns_only_json(&self) -> Vec<(&String, &ColumnDef)> {
        self.merged_columns
            .iter()
            .filter(|(_k, v)| {
                v.type_def == ColumnType::ArrayInt
                    || v.type_def == ColumnType::ArrayString
                    || v.type_def == ColumnType::Json
            })
            .collect()
    }

    pub fn auto_increments(&self) -> Vec<(&String, &ColumnDef)> {
        self.merged_columns
            .iter()
            .filter(|(_k, v)| v.auto_increment == Some(AutoIncrement::Auto))
            .collect()
    }
    #[allow(dead_code)]
    pub fn except_auto_increment(&self) -> Vec<(&String, &ColumnDef)> {
        self.merged_columns
            .iter()
            .filter(|(_k, v)| v.auto_increment != Some(AutoIncrement::Auto))
            .collect()
    }
    pub fn for_factory(&self) -> Vec<(&String, &ColumnDef)> {
        self.merged_columns
            .iter()
            .filter(|(_k, v)| !v.skip_factory)
            .collect()
    }
    pub fn for_cmp(&self) -> Vec<(&String, &ColumnDef)> {
        self.merged_columns
            .iter()
            .filter(|(_k, v)| {
                v.auto_increment != Some(AutoIncrement::Auto) && !v.skip_factory && !v.primary
            })
            .collect()
    }
    // pub fn indexes(&self) -> Vec<(&ModelDef, &String, &IndexDef)> {
    //     self.merged_indexes
    //         .iter()
    //         .map(|v| (self, v.0, v.1))
    //         .collect()
    // }
    #[allow(dead_code)]
    pub fn unique(&self) -> Vec<(&ModelDef, &String, &IndexDef)> {
        self.merged_indexes
            .iter()
            .filter(|v| {
                v.1.type_def == Some(IndexType::Unique)
                    && (v.1.fields.len() <= 1
                        || (v.1.fields.len() == 2 && v.1.fields.contains_key(DELETED)))
            })
            .map(|v| (self, v.0, v.1))
            .collect()
    }
    pub fn unique_index(&self) -> Vec<(&String, &IndexDef)> {
        self.merged_indexes
            .iter()
            .filter(|v| v.1.type_def == Some(IndexType::Unique))
            .map(|v| (v.0, v.1))
            .collect()
    }
    pub fn unique_key(&self) -> Vec<(&String, &ColumnDef)> {
        let mut chk = HashSet::new();
        self.merged_indexes
            .iter()
            .filter(|v| v.1.type_def == Some(IndexType::Unique))
            .flat_map(|v| {
                let name = v.0;
                let index = v.1;
                let mut ret = Vec::new();
                if !index.fields.is_empty() {
                    for row in &index.fields {
                        if row.0 == DELETED || chk.contains(row.0) {
                            continue;
                        }
                        let col = self
                            .merged_columns
                            .get(row.0)
                            .unwrap_or_else(|| panic!("{} index is not in columns", row.0));
                        chk.insert(row.0.clone());
                        ret.push((row.0, col));
                    }
                } else if !chk.contains(name) {
                    let col = self
                        .merged_columns
                        .get(name)
                        .unwrap_or_else(|| panic!("{} index is not in columns", name));
                    chk.insert(name.clone());
                    ret.push((name, col));
                }
                ret
            })
            .collect()
    }
    pub fn relations(&self) -> Vec<(&ModelDef, &String, &Option<RelDef>)> {
        self.merged_relations
            .iter()
            .map(|v| (self, v.0, v.1))
            .collect()
    }
    pub fn relations_cache(&self) -> Vec<(&ModelDef, &String, &Option<RelDef>)> {
        self.merged_relations
            .iter()
            .filter(|v| {
                v.1.as_ref()
                    .map(|v| v.in_cache && v.type_def != Some(RelationsType::One))
                    .unwrap_or(false)
            })
            .map(|v| (self, v.0, v.1))
            .collect()
    }
    pub fn relations_one(&self) -> Vec<(&ModelDef, &String, &Option<RelDef>)> {
        self.merged_relations
            .iter()
            .filter(|v| {
                v.1.as_ref().and_then(|v| v.type_def.as_ref()) != Some(&RelationsType::Many)
            })
            .map(|v| (self, v.0, v.1))
            .collect()
    }
    pub fn relations_one_except_cache(&self) -> Vec<(&ModelDef, &String, &Option<RelDef>)> {
        self.merged_relations
            .iter()
            .filter(|v| {
                v.1.as_ref()
                    .map(|v| {
                        !v.use_cache
                            && !v.use_cache_with_trashed
                            && v.type_def != Some(RelationsType::Many)
                    })
                    .unwrap_or(false)
            })
            .map(|v| (self, v.0, v.1))
            .collect()
    }
    pub fn relations_one_only_cache(&self) -> Vec<(&ModelDef, &String, &Option<RelDef>)> {
        self.merged_relations
            .iter()
            .filter(|v| {
                v.1.as_ref()
                    .map(|v| {
                        (v.use_cache || v.use_cache_with_trashed)
                            && v.type_def != Some(RelationsType::Many)
                    })
                    .unwrap_or(false)
            })
            .map(|v| (self, v.0, v.1))
            .collect()
    }
    pub fn relations_one_owner(&self) -> Vec<(&ModelDef, &String, &Option<RelDef>)> {
        // one-to-oneで自分が所有側の特殊型、相手側は主キーでなければならない
        self.merged_relations
            .iter()
            .filter(|v| v.1.as_ref().and_then(|v| v.type_def) == Some(RelationsType::OneToOne))
            .map(|v| (self, v.0, v.1))
            .collect()
    }
    pub fn relations_one_cache(&self) -> Vec<(&ModelDef, &String, &Option<RelDef>)> {
        self.merged_relations
            .iter()
            .filter(|v| {
                v.1.as_ref().and_then(|v| v.type_def.as_ref()) == Some(&RelationsType::OneToOne)
                    && v.1.as_ref().map(|v| v.in_cache).unwrap_or(false)
            })
            .map(|v| (self, v.0, v.1))
            .collect()
    }
    pub fn relations_many(&self) -> Vec<(&ModelDef, &String, &Option<RelDef>)> {
        self.merged_relations
            .iter()
            .filter(|v| {
                v.1.as_ref().and_then(|v| v.type_def.as_ref()) == Some(&RelationsType::Many)
            })
            .map(|v| (self, v.0, v.1))
            .collect()
    }
    pub fn relations_many_cache(&self) -> Vec<(&ModelDef, &String, &Option<RelDef>)> {
        self.merged_relations
            .iter()
            .filter(|v| {
                v.1.as_ref().and_then(|v| v.type_def.as_ref()) == Some(&RelationsType::Many)
                    && v.1.as_ref().map(|v| v.in_cache).unwrap_or(false)
            })
            .map(|v| (self, v.0, v.1))
            .collect()
    }
    pub fn relation_constraint(&self) -> Vec<(&ModelDef, &String, &Option<RelDef>)> {
        if self.ignore_foreign_key() {
            return Vec::new();
        }
        self.merged_relations
            .iter()
            .filter(|v| {
                v.1.as_ref().and_then(|v| v.type_def.as_ref()) != Some(&RelationsType::Many)
                    && (v.1.as_ref().and_then(|v| v.on_delete).is_some()
                        || !v.1.as_ref().map(|v| v.use_cache).unwrap_or(false)
                            && !v
                                .1
                                .as_ref()
                                .map(|v| v.use_cache_with_trashed)
                                .unwrap_or(false))
            })
            .map(|v| (self, v.0, v.1))
            .collect()
    }
    pub fn relations_on_delete_mod(&self) -> BTreeSet<(String,)> {
        self.merged_relations
            .iter()
            .filter(|v| v.1.as_ref().and_then(|v| v.on_delete).is_some())
            .map(|v| {
                let rel_name = v.0;
                let rel = v.1;
                let mod_name = RelDef::get_group_mod_name(rel, rel_name);
                (mod_name,)
            })
            .collect()
    }
    pub fn relations_on_delete_cascade(&self) -> Vec<(String, String)> {
        self.merged_relations
            .iter()
            .filter(|v| {
                v.1.as_ref().and_then(|v| v.type_def) != Some(RelationsType::Many)
                    && v.1.as_ref().and_then(|v| v.on_delete) == Some(ReferenceOption::Cascade)
            })
            .map(|v| {
                let rel_name = v.0;
                let rel = v.1;
                let mod_name = RelDef::get_group_mod_name(rel, rel_name);
                let local = RelDef::get_local_id(rel, rel_name, &self.id_name());
                (mod_name, local)
            })
            .collect()
    }
    pub fn relations_on_delete_restrict(&self) -> Vec<(String, String)> {
        self.merged_relations
            .iter()
            .filter(|v| {
                v.1.as_ref().and_then(|v| v.type_def) != Some(RelationsType::Many)
                    && v.1.as_ref().and_then(|v| v.on_delete).is_some()
                    && v.1.as_ref().and_then(|v| v.on_delete) == Some(ReferenceOption::Restrict)
            })
            .map(|v| {
                let rel_name = v.0;
                let rel = v.1;
                let mod_name = RelDef::get_group_mod_name(rel, rel_name);
                let local = RelDef::get_local_id(rel, rel_name, &self.id_name());
                (mod_name, local)
            })
            .collect()
    }
    pub fn relations_on_delete_not_cascade(&self) -> Vec<(String, String, &str, &str)> {
        self.merged_relations
            .iter()
            .filter(|v| {
                v.1.as_ref().and_then(|v| v.type_def) != Some(RelationsType::Many)
                    && v.1.as_ref().and_then(|v| v.on_delete).is_some()
                    && v.1.as_ref().and_then(|v| v.on_delete) != Some(ReferenceOption::Cascade)
                    && v.1.as_ref().and_then(|v| v.on_delete) != Some(ReferenceOption::Restrict)
            })
            .map(|v| {
                let rel_name = v.0;
                let rel = v.1;
                let mod_name = RelDef::get_group_mod_name(rel, rel_name);
                let local = RelDef::get_local_id(rel, rel_name, &self.id_name());
                let mode = rel.as_ref().and_then(|v| v.on_delete).unwrap();
                let val = if mode == ReferenceOption::SetNull {
                    "null"
                } else {
                    "0"
                };
                let val2 = if mode == ReferenceOption::SetNull {
                    "None"
                } else {
                    "0"
                };
                (mod_name, local, val, val2)
            })
            .collect()
    }

    pub fn relation_mods(&self) -> Vec<Vec<String>> {
        let mut mods: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for (name, rel) in self.merged_relations.iter() {
            let group_name = RelDef::get_group_name(rel, self);
            let mod_name = RelDef::get_mod_name(rel, name);
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
    pub fn enums(&self) -> Vec<(&String, ColumnDef)> {
        let mut vec: Vec<(&String, ColumnDef)> = self
            .columns
            .iter()
            .map(|(k, v)| (k, v.exact()))
            .filter(|(_k, v)| v.type_def == ColumnType::Enum && v.enum_model.is_none())
            .collect();
        vec.sort_by(|a, b| a.0.cmp(b.0));
        vec
    }
    pub fn db_enums(&self) -> Vec<(&String, ColumnDef)> {
        let mut vec: Vec<(&String, ColumnDef)> = self
            .columns
            .iter()
            .map(|(k, v)| (k, v.exact()))
            .filter(|(_k, v)| v.type_def == ColumnType::DbEnum || v.type_def == ColumnType::DbSet)
            .collect();
        vec.sort_by(|a, b| a.0.cmp(b.0));
        vec
    }
    pub fn parents(&self) -> Vec<Arc<ModelDef>> {
        let mut vec = Vec::new();
        let mut cur = self.inheritance.clone();
        while let Some(ref inheritance) = cur {
            let model = RelDef::get_model_by_name(&inheritance.extends);
            cur = model.inheritance.clone();
            if model.abstract_mode {
                vec.push(model.clone());
            }
        }
        vec
    }
    pub fn downcast_simple(&self) -> Vec<Arc<ModelDef>> {
        let mut vec = Vec::new();
        let mut cur = self.inheritance.clone();
        while let Some(ref inheritance) = cur {
            if inheritance.type_def == InheritanceType::Simple {
                let model = RelDef::get_model_by_name(&inheritance.extends);
                cur = model.inheritance.clone();
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
        while let Some(ref inheritance) = cur {
            if inheritance.type_def == InheritanceType::ColumnAggregation {
                let model = RelDef::get_model_by_name(&inheritance.extends);
                cur = model.inheritance.clone();
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
        let mut hash = hash(&format!(
            "{} {} {} {}",
            target, self.db, self.group_name, self.name
        ));
        while unsafe { TYPE_IDS.contains(&hash) } {
            hash = hash.wrapping_add(1);
        }
        unsafe { TYPE_IDS.insert(hash) };
        hash
    }

    pub fn not_optimized_tuple(&self) -> bool {
        false
        // let conf = unsafe { CONFIG.get().unwrap() };
        // let engine = self.engine.as_ref().or(conf.engine.as_ref());
        // conf.db == DbType::MariaDb
        //     && (engine.is_none()
        //         || engine
        //             .map(|v| v.eq_ignore_ascii_case("InnoDB"))
        //             .unwrap_or_default())
    }
}
