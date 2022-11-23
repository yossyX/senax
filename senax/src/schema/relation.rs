use convert_case::{Case, Casing};
use inflector::string::pluralize::to_plural;
use inflector::string::singularize::to_singular;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::{to_id_name, ModelDef, GROUPS, MODEL, MODELS};
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Copy, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(title = "Relations Type")]
pub enum RelationsType {
    Many,
    One,
    OneToOne,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Copy, Clone, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(title = "Reference Option")]
pub enum ReferenceOption {
    Restrict,
    Cascade,
    SetNull,
    // NoAction,
    SetZero,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[schemars(deny_unknown_fields)]
#[schemars(title = "Relation Def")]
pub struct RelDef {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// 結合先のモデル　他のグループは::区切りで指定
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    // リレーションのタイプ　デフォルトはone
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_def: Option<RelationsType>,
    /// 結合するローカルのカラム名
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local: Option<String>,
    /// 結合先のカラム名
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foreign: Option<String>,
    /// manyあるいはone_to_oneの場合にリレーション先も一緒にキャッシュするか
    /// 結合深さは1代のみで子テーブルは親に含んだ状態で更新する必要がある
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub in_cache: bool,
    /// リレーションを取得する際の追加条件
    /// 記述例：rel_group_model::Cond::Eq(rel_group_model::ColOne::value(1))
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_cond: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_by: Option<String>,
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub desc: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub use_cache: bool,
    /// リレーション先が論理削除されていてもキャッシュを取得する
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub use_cache_with_trashed: bool,
    /// DBの外部キー制約による削除およびソフトウェア側での削除制御
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_delete: Option<ReferenceOption>,
    /// DBの外部キー制約による更新
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_update: Option<ReferenceOption>,
}
pub const MODEL_NAME_SPLITTER: &str = "::";
impl RelDef {
    pub fn get_stem_name(rel_def: &Option<RelDef>, name: &str) -> String {
        if let Some(ref rel) = rel_def {
            match rel.model {
                None => name,
                Some(ref name) => {
                    if name.contains(MODEL_NAME_SPLITTER) {
                        let (group_name, stem_name) = name.split_once(MODEL_NAME_SPLITTER).unwrap();
                        crate::common::check_name(group_name);
                        crate::common::check_name(stem_name);
                        stem_name
                    } else {
                        crate::common::check_name(name);
                        name
                    }
                }
            }
        } else {
            name
        }
        .to_owned()
    }
    pub fn get_foreign_class_name(rel_def: &Option<RelDef>, name: &str) -> String {
        format!(
            "_{}",
            Self::get_foreign_model_name(rel_def, name)
                .1
                .to_case(Case::Pascal)
        )
    }
    pub fn get_id_name(rel_def: &Option<RelDef>, name: &str) -> String {
        to_id_name(&Self::get_foreign_model_name(rel_def, name).1)
    }
    pub fn get_mod_name(rel_def: &Option<RelDef>, name: &str) -> String {
        Self::get_foreign_model_name(rel_def, name).0
    }
    pub fn get_group_mod_name(rel_def: &Option<RelDef>, name: &str) -> String {
        let model_def = unsafe { MODEL.get().unwrap() }.clone();
        format!(
            "{}_{}",
            RelDef::get_group_name(rel_def, model_def.as_ref()),
            RelDef::get_mod_name(rel_def, name)
        )
    }
    pub fn get_local_id(rel_def: &Option<RelDef>, name: &str, id_name: &str) -> String {
        if let Some(ref rel) = rel_def {
            match rel.local {
                None => {
                    if rel.type_def == Some(RelationsType::OneToOne) {
                        id_name.to_string()
                    } else {
                        format!("{}_id", name)
                    }
                }
                Some(ref name) => name.to_owned(),
            }
        } else {
            format!("{}_id", name)
        }
    }
    pub fn get_foreign_id(
        rel_def: &Option<RelDef>,
        self_model: &ModelDef,
        target_model: &Arc<ModelDef>,
    ) -> String {
        if let Some(ref rel) = rel_def {
            if let Some(ref name) = rel.foreign {
                return name.to_owned();
            }
        }
        let id = format!("{}_id", self_model.name);
        if target_model.merged_columns.contains_key(&id) {
            return id;
        }
        let id = format!("{}_id", to_singular(&self_model.name));
        if target_model.merged_columns.contains_key(&id) {
            return id;
        }
        "foreign id not found!".to_string()
    }
    pub fn foreign_is_not_null(
        rel_def: &Option<RelDef>,
        name: &str,
        self_model: &ModelDef,
    ) -> bool {
        let target_model = Self::get_foreign_model(rel_def, name);
        let foreign = Self::get_foreign_id(rel_def, self_model, &target_model);
        let column = target_model
            .merged_columns
            .get(&foreign)
            .unwrap_or_else(|| panic!("{} is not found in {}.", foreign, target_model.name));
        column.not_null
    }

    pub fn get_foreign_table_name(rel_def: &Option<RelDef>, name: &str) -> String {
        Self::get_foreign_model(rel_def, name).table_name()
    }

    pub fn get_foreign_model(rel_def: &Option<RelDef>, name: &str) -> Arc<ModelDef> {
        let group_name = rel_def
            .as_ref()
            .and_then(|v| v.model.as_ref())
            .and_then(|name| {
                if name.contains(MODEL_NAME_SPLITTER) {
                    let (group_name, _) = name.split_once(MODEL_NAME_SPLITTER).unwrap();
                    crate::common::check_name(group_name);
                    Some(group_name)
                } else {
                    None
                }
            });
        let stem_name = Self::get_stem_name(rel_def, name);
        get_model(group_name, &stem_name)
    }

    pub fn get_foreign_model_name(rel_def: &Option<RelDef>, name: &str) -> (String, String) {
        let group_name = rel_def
            .as_ref()
            .and_then(|v| v.model.as_ref())
            .and_then(|name| {
                if name.contains(MODEL_NAME_SPLITTER) {
                    let (group_name, _) = name.split_once(MODEL_NAME_SPLITTER).unwrap();
                    crate::common::check_name(group_name);
                    Some(group_name)
                } else {
                    None
                }
            });
        let stem_name = Self::get_stem_name(rel_def, name);
        get_model_name(group_name, &stem_name)
    }

    pub fn get_model_by_name(name: &str) -> Arc<ModelDef> {
        let (group_name, stem_name) = if name.contains(MODEL_NAME_SPLITTER) {
            let (group_name, stem_name) = name.split_once(MODEL_NAME_SPLITTER).unwrap();
            crate::common::check_name(group_name);
            crate::common::check_name(stem_name);
            (Some(group_name), stem_name)
        } else {
            (None, name)
        };
        get_model(group_name, stem_name)
    }

    pub fn get_group_name(rel_def: &Option<RelDef>, model_def: &ModelDef) -> String {
        if let Some(ref rel) = rel_def {
            match rel.model {
                None => model_def.group_name.to_owned(),
                Some(ref name) => {
                    if name.contains(MODEL_NAME_SPLITTER) {
                        let (group_name, stem_name) = name.split_once(MODEL_NAME_SPLITTER).unwrap();
                        crate::common::check_name(group_name);
                        crate::common::check_name(stem_name);
                        group_name.to_string()
                    } else {
                        crate::common::check_name(&model_def.group_name);
                        model_def.group_name.to_owned()
                    }
                }
            }
        } else {
            model_def.group_name.to_owned()
        }
    }
}

fn get_model(group_name: Option<&str>, stem_name: &str) -> Arc<ModelDef> {
    if let Some(group_name) = group_name {
        if let Some(model) = unsafe { GROUPS.get().unwrap() }
            .get(group_name)
            .unwrap_or_else(|| panic!("{} group is not defined", group_name))
            .get(stem_name)
        {
            return model.clone();
        }
        let plural_name = to_plural(stem_name);
        unsafe { GROUPS.get().unwrap() }
            .get(group_name)
            .unwrap_or_else(|| panic!("{} group is not defined", group_name))
            .get(&plural_name)
            .unwrap_or_else(|| panic!("{} model is not defined", stem_name))
            .clone()
    } else {
        if let Some(model) = unsafe { MODELS.get().unwrap() }.get(stem_name) {
            return model.clone();
        }
        let plural_name = to_plural(stem_name);
        unsafe { MODELS.get().unwrap() }
            .get(&plural_name)
            .unwrap_or_else(|| panic!("{} model is not defined", stem_name))
            .clone()
    }
}

fn get_model_name(group_name: Option<&str>, stem_name: &str) -> (String, String) {
    if let Some(group_name) = group_name {
        if let Some(model) = unsafe { GROUPS.get().unwrap() }
            .get(group_name)
            .unwrap_or_else(|| panic!("{} group is not defined", group_name))
            .get(stem_name)
        {
            return (model.mod_name().to_string(), model.name.clone());
        }
        let plural_name = to_plural(stem_name);
        let model = unsafe { GROUPS.get().unwrap() }
            .get(group_name)
            .unwrap_or_else(|| panic!("{} group is not defined", group_name))
            .get(&plural_name)
            .unwrap_or_else(|| panic!("{} model is not defined", stem_name));
        (model.mod_name().to_string(), model.name.clone())
    } else {
        if let Some(model) = unsafe { MODELS.get().unwrap() }.get(stem_name) {
            return (model.mod_name().to_string(), model.name.clone());
        }
        let plural_name = to_plural(stem_name);
        let model = unsafe { MODELS.get().unwrap() }
            .get(&plural_name)
            .unwrap_or_else(|| panic!("{} model is not defined", stem_name));
        (model.mod_name().to_string(), model.name.clone())
    }
}
