use anyhow::{Context as _, Result, bail, ensure};
use compact_str::CompactString;
use indexmap::IndexMap;
use once_cell::sync::Lazy;
use regex::Regex;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::{Arc, RwLock};

pub mod column;
pub use column::*;
pub mod config;
pub use config::*;
pub mod index;
pub use index::*;
pub mod model;
pub use model::*;
pub mod relation;
pub use relation::*;
pub mod selector;
pub use selector::*;
pub mod json_schema;

use crate::api_generator::schema::{ApiConfigDef, ApiDbDef, ApiModelDef};
use crate::common::ToCase as _;
use crate::common::{DEFAULT_SRID, parse_yml_file, to_singular};
use crate::{SCHEMA_PATH, SIMPLE_VALUE_OBJECTS_FILE};

static CREATED_AT: RwLock<CompactString> = RwLock::new(CompactString::const_new(""));
static UPDATED_AT: RwLock<CompactString> = RwLock::new(CompactString::const_new(""));
static DELETED_AT: RwLock<CompactString> = RwLock::new(CompactString::const_new(""));
static DELETED: RwLock<CompactString> = RwLock::new(CompactString::const_new(""));
static AGGREGATION_TYPE: RwLock<CompactString> = RwLock::new(CompactString::const_new(""));
static VERSION: RwLock<CompactString> = RwLock::new(CompactString::const_new(""));

pub type GroupsDef =
    IndexMap<String, (AtomicUsize, IndexMap<String, (AtomicUsize, Arc<ModelDef>)>)>;

pub static CONFIG: RwLock<Option<ConfigDef>> = RwLock::new(None);
pub static GROUPS: RwLock<Option<GroupsDef>> = RwLock::new(None);
pub static VALUE_OBJECTS: RwLock<Option<IndexMap<String, FieldDef>>> = RwLock::new(None);
pub static DOMAIN_MODE: AtomicBool = AtomicBool::new(false);

type GroupIndex = IndexMap<String, IndexMap<String, RefCell<ModelDef>>>;

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
/// ### スキーマ定義
pub struct SchemaDef {
    #[schemars(default)]
    conf: HashMap<String, ConfigDef>,
    #[schemars(default)]
    model: HashMap<String, ModelDef>,
    #[schemars(default)]
    simple_value_object: HashMap<String, FieldDef>,
    #[schemars(default)]
    api_config: ApiConfigDef,
    #[schemars(default)]
    api_db: ApiDbDef,
    #[schemars(default)]
    api_model: HashMap<String, Option<ApiModelDef>>,
}

pub fn is_false(val: &bool) -> bool {
    !(*val)
}

pub fn parse(db: &str, outer_crate: bool, config_only: bool) -> Result<(), anyhow::Error> {
    crate::common::check_ascii_name(db);
    let path = Path::new(SCHEMA_PATH).join(format!("{db}.yml"));
    let mut config: ConfigDef = parse_yml_file(&path)?;
    if config.force_disable_cache {
        config.use_cache = false;
        config.use_fast_cache = false;
        config.use_storage_cache = false;
        config.use_all_rows_cache = false;
    }
    config.fix_static_vars();

    for (name, def) in config.groups.iter_mut() {
        if let Some(def) = def {
            if def.label.is_none() {
                def.label = Some(name.to_title());
            }
        }
    }
    CONFIG.write().unwrap().replace(config.clone());
    if config_only {
        return Ok(());
    }

    let path = Path::new(SCHEMA_PATH).join(SIMPLE_VALUE_OBJECTS_FILE);
    let mut value_objects: IndexMap<String, FieldDef> = if path.exists() {
        parse_yml_file(&path)?
    } else {
        IndexMap::new()
    };
    for (name, def) in value_objects.iter_mut() {
        ensure!(
            def.data_type != DataType::ValueObject,
            "The type of value_object cannot be value_object.:{}",
            name
        );
        crate::common::check_struct_name(name);
        crate::common::check_ascii_name(name);
        def.not_null = true;
    }
    VALUE_OBJECTS.write().unwrap().replace(value_objects);

    let db_path = Path::new(SCHEMA_PATH).join(db);
    let mut groups = IndexMap::new();
    let mut name_check_set = HashSet::new();
    for (group_name, group_def) in &config.groups {
        crate::common::check_ascii_name(group_name);
        let group_def = group_def.clone().unwrap_or_default();
        let mut model_map = IndexMap::new();
        let defs: IndexMap<String, ModelDef> = if group_def.models.is_empty() {
            let path = db_path.join(format!("{group_name}.yml"));
            if path.exists() {
                parse_yml_file(&path)?
            } else {
                IndexMap::new()
            }
        } else {
            group_def.models.clone()
        };
        for (model_name, mut def) in defs.into_iter() {
            crate::common::check_struct_name(&model_name);
            crate::common::check_ascii_name(&model_name);
            let name_check = format!("{}_{}", group_name, model_name);
            if !name_check_set.insert(name_check) {
                bail!("{}_{} is duplicate", group_name, model_name);
            }
            def.db = db.to_string();
            def.group_name.clone_from(group_name);
            def.name = model_name.to_string();
            if def.dummy_always_present() {
                if def.use_all_rows_cache.is_none() {
                    def.use_all_rows_cache = Some(false);
                }
                if def.skip_ddl.is_none() {
                    def.skip_ddl = Some(true);
                }
            }
            for (name, _col) in def.fields.iter() {
                crate::common::check_column_name(name);
            }
            let non_primaries: Vec<String> = def
                .fields
                .iter()
                .filter(|(_k, v)| !v.exact().primary)
                .map(|(k, _v)| k.clone())
                .collect();
            if def.created_at_conf().is_some()
                && !def.fields.contains_key(ConfigDef::created_at().as_str())
                && !def.disable_created_at
            {
                let mut col: FieldDef = serde_json::from_str(
                    r#"{"type":"datetime","not_null":true,"skip_factory":true}"#,
                )?;
                col.label.clone_from(&config.label_of_created_at);
                col.time_zone = config.timestamp_time_zone;
                col.auto_gen = true;
                col.exclude_from_cache = Some(config.disable_timestamp_cache);
                col.is_timestamp = true;
                def.fields
                    .insert(ConfigDef::created_at().to_string(), col.into());
            }
            if def.updated_at_conf().is_some()
                && !def.fields.contains_key(ConfigDef::updated_at().as_str())
                && !def.disable_updated_at
            {
                let mut col: FieldDef = serde_json::from_str(
                    r#"{"type":"datetime","not_null":true,"skip_factory":true}"#,
                )?;
                col.label.clone_from(&config.label_of_updated_at);
                col.time_zone = config.timestamp_time_zone;
                col.auto_gen = true;
                col.exclude_from_cache = Some(config.disable_timestamp_cache);
                col.is_timestamp = true;
                def.fields
                    .insert(ConfigDef::updated_at().to_string(), col.into());
            }
            if let Some(soft_delete) = def.soft_delete() {
                match soft_delete {
                    SoftDelete::None => {}
                    SoftDelete::Time => {
                        if !def.fields.contains_key(ConfigDef::deleted_at().as_str()) {
                            let mut col: FieldDef =
                                serde_json::from_str(r#"{"type":"datetime","skip_factory":true}"#)?;
                            col.label.clone_from(&config.label_of_deleted_at);
                            col.time_zone = config.timestamp_time_zone;
                            col.auto_gen = true;
                            col.hidden = Some(true);
                            def.fields
                                .insert(ConfigDef::deleted_at().to_string(), col.into());
                        }
                    }
                    SoftDelete::Flag => {
                        if !def.fields.contains_key(ConfigDef::deleted().as_str()) {
                            let mut col: FieldDef = serde_json::from_str(
                                r#"{"type":"boolean","not_null":true,"skip_factory":true}"#,
                            )?;
                            col.label.clone_from(&config.label_of_deleted);
                            col.auto_gen = true;
                            col.hidden = Some(true);
                            def.fields
                                .insert(ConfigDef::deleted().to_string(), col.into());
                        }
                    }
                    SoftDelete::UnixTime => {
                        if !def.fields.contains_key(ConfigDef::deleted().as_str()) {
                            let mut col: FieldDef = serde_json::from_str(
                                r#"{"type":"int","not_null":true,"skip_factory":true}"#,
                            )?;
                            col.label.clone_from(&config.label_of_deleted);
                            col.auto_gen = true;
                            col.hidden = Some(true);
                            def.fields
                                .insert(ConfigDef::deleted().to_string(), col.into());
                        }
                    }
                }
            }
            if def.versioned {
                if def.counting.is_some() {
                    bail!(
                        "Both versioned and counting cannot be set for {}.",
                        def.name
                    );
                }
                if !def.fields.contains_key(ConfigDef::version().as_str()) {
                    let mut col: FieldDef = serde_json::from_str(
                        r#"{"type":"int","not_null":true,"skip_factory":true,"default":"0"}"#,
                    )?;
                    col.label.clone_from(&config.label_of_version);
                    col.auto_gen = true;
                    col.hidden = Some(true);
                    def.fields
                        .insert(ConfigDef::version().to_string(), col.into());
                }
            }
            if let Some(ref mut inheritance) = def.inheritance {
                if inheritance._type == InheritanceType::ColumnAggregation {
                    if let Some(ref key_field) = inheritance.key_field {
                        crate::common::check_name(key_field);
                    } else {
                        inheritance.key_field = Some(ConfigDef::aggregation_type().to_string());
                    }
                    if inheritance.key_value.is_none() {
                        inheritance.key_value = Some(Value::from(model_name.clone()));
                    }
                    let key_field = inheritance.key_field.as_ref().unwrap();
                    match def.fields.entry(key_field.to_string()) {
                        indexmap::map::Entry::Occupied(mut entry) => {
                            let mut col = entry.get().exact();
                            col.skip_factory = Some(true);
                            entry.insert(col.into());
                        }
                        indexmap::map::Entry::Vacant(entry) => {
                            let mut col: FieldDef = serde_json::from_str(
                                r#"{"type":"varchar","not_null":true, "skip_factory": true}"#,
                            )?;
                            col.label.clone_from(&config.label_of_aggregation_type);
                            col.auto_gen = true;
                            col.hidden = Some(true);
                            entry.insert(col.into());
                        }
                    }
                }
            }
            for name in non_primaries {
                def.fields.move_index(
                    def.fields.get_index_of(&name).unwrap(),
                    def.fields.len() - 1,
                );
            }
            for (col_name, column) in def.fields.clone().iter() {
                let mut column_def = column.exact();
                if column_def.primary {
                    column_def.not_null = true;
                    if column_def.auto == Some(AutoGeneration::Sequence) {
                        config.use_sequence = true;
                    }
                }
                if column_def.data_type == DataType::ValueObject {
                    let (vo_name, postfix) = if let Some(vo_name) = &column_def.value_object {
                        (vo_name.clone(), "".to_string())
                    } else {
                        static RE: Lazy<Regex> =
                            Lazy::new(|| Regex::new(r"^(.+?)_?([0-9]*)$").unwrap());
                        if let Some(c) = RE.captures(col_name) {
                            (
                                c.get(1).unwrap().as_str().to_string(),
                                c.get(2).unwrap().as_str().to_string(),
                            )
                        } else {
                            (col_name.clone(), "".to_string())
                        }
                    };
                    let v = VALUE_OBJECTS
                        .read()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .get(&vo_name)
                        .cloned()
                        .with_context(|| format!("undefined value_object:{vo_name}"))?;
                    let org = column_def;
                    column_def = v;
                    column_def.value_object = Some(vo_name);
                    column_def.overwrite(org, &postfix);
                }
                if column_def.enum_values.is_some() {
                    column_def.enum_class = Some(EnumClass {
                        outer_crate,
                        db: db.to_string(),
                        group: group_name.to_snake(),
                        mod_name: def.mod_name().to_string(),
                        name: col_name.to_pascal(),
                    });
                }
                if column_def.srid.is_none() {
                    if column_def.data_type == DataType::Point {
                        column_def.srid = Some(0);
                    } else if column_def.data_type == DataType::GeoPoint
                        || column_def.data_type == DataType::Geometry
                    {
                        column_def.srid = Some(DEFAULT_SRID);
                    }
                }
                def.fields.insert(col_name.clone(), column_def.into());
            }
            def.merged_fields = def.fields.iter().fold(IndexMap::new(), |mut map, (k, v)| {
                map.insert(k.clone(), v.exact());
                map
            });

            for (name, rel) in &def.has_one {
                def.relations
                    .insert(name.clone(), HasOneDef::convert(rel, group_name, name));
            }
            for (name, rel) in &def.has_many {
                def.relations
                    .insert(name.clone(), HasManyDef::convert(rel, group_name, name));
            }
            for (name, rel) in &def.belongs_to {
                def.relations
                    .insert(name.clone(), BelongsToDef::convert(rel, group_name, name));
            }
            for (name, rel) in &def.belongs_to_outer_db {
                def.relations.insert(
                    name.clone(),
                    BelongsToOuterDbDef::convert(rel, group_name, name),
                );
            }
            def.merged_relations.clone_from(&def.relations);
            def.merged_indexes =
                def.indexes
                    .iter()
                    .fold(IndexMap::new(), |mut map, (name, index)| {
                        map.insert(name.clone(), index.clone().unwrap_or_default());
                        map
                    });
            for (_, selector) in &mut def.selectors {
                for (name, order) in &mut selector.orders {
                    if order.direct_sql.is_none() && order.fields.is_empty() {
                        order.fields.insert(name.clone(), ());
                    }
                }
            }
            if def.exclude_group_from_table_name.is_none() {
                def.exclude_group_from_table_name = Some(group_def.exclude_group_from_table_name);
            }
            if config.force_disable_cache {
                def.use_cache = Some(false);
                def.use_all_rows_cache = Some(false);
                def.use_filtered_row_cache = Some(false);
            }
            model_map.insert(model_name, RefCell::new(def));
        }
        groups.insert(group_name.clone(), model_map);
    }
    for (cur_group_name, defs) in groups.iter() {
        for (cur_model_name, def) in defs.iter() {
            fix_inheritance(cur_group_name, cur_model_name, def, &groups)?;
        }
    }
    for (group_name, defs) in groups.iter() {
        for (cur_model_name, def) in defs.iter() {
            let mut model = def.borrow_mut();
            let main_pk_count = model
                .primaries()
                .iter()
                .filter(|(_, v)| v.main_primary)
                .count();
            ensure!(
                main_pk_count < 2,
                "There are too many main_primaries.:{}",
                cur_model_name
            );
            let mut main_primary = None;
            for (idx, (primary_name, p_def)) in model.primaries().into_iter().enumerate() {
                if p_def.auto == Some(AutoGeneration::AutoIncrement) && idx != 0 {
                    bail!("Auto increment cannot be used except for the first primary key.");
                }
                if p_def.main_primary
                    || main_pk_count == 0
                        && (p_def.auto.is_some()
                            || primary_name.eq("id")
                            || primary_name.eq(&format!("{}_id", cur_model_name)))
                {
                    main_primary = Some(primary_name.clone());
                    break;
                }
            }
            if main_primary.is_none() && model.primaries().len() == 1 {
                main_primary = Some(
                    model
                        .primaries()
                        .first()
                        .map(|(name, _)| name.to_string())
                        .unwrap(),
                );
            }
            if let Some(main_primary) = main_primary {
                let mod_name = model.mod_name().to_string();
                if let Some(column_def) = model.merged_fields.get_mut(&main_primary) {
                    column_def.id_class = Some(IdClass {
                        outer_crate,
                        db: db.to_string(),
                        group: group_name.to_snake(),
                        mod_name,
                        name: cur_model_name.to_pascal(),
                    });
                    column_def.main_primary = true;
                    column_def.value_object = None;
                }
            }
        }
    }
    for (cur_group_name, defs) in groups.iter() {
        for (cur_model_name, def) in defs.iter() {
            {
                let mut model = def.borrow_mut();
                for (rel_name, rel_def) in model.merged_relations.clone().iter() {
                    if rel_def.is_type_of_belongs_to_outer_db() {
                        let local_ids = rel_def.get_local_id(rel_name, &model);
                        if local_ids.len() == 1 {
                            let col_name = &local_ids[0];
                            if let Some(column_def) = model.merged_fields.get_mut(col_name) {
                                column_def.outer_db_rel = Some((rel_name.clone(), rel_def.clone()));
                                column_def.main_primary = false;
                                column_def.id_class = None;
                                column_def.enum_class = None;
                                column_def.value_object = None;
                            }
                        }
                    }
                    if rel_def.is_type_of_belongs_to() {
                        let local_ids = rel_def.get_local_id(rel_name, &model);
                        if local_ids.len() == 1 {
                            let col_name = &local_ids[0];
                            let self_local = model.relation_primaries(col_name.to_string());
                            let id = if model.full_name().eq(&rel_def.model) {
                                if !model.non_main_primaries().is_empty() {
                                    let rel_def = model.merged_relations.get_mut(rel_name).unwrap();
                                    rel_def.local = Some(self_local);
                                }
                                model.id().pop().map(|v| v.1.clone())
                            } else {
                                let rel_model =
                                    get_model(&rel_def.model, cur_group_name, &groups).borrow();
                                if !model.non_main_primaries().is_empty() {
                                    let rel_def = model.merged_relations.get_mut(rel_name).unwrap();
                                    rel_def.local =
                                        Some(rel_model.relation_primaries(col_name.to_string()));
                                }
                                rel_model.id().pop().map(|v| v.1.clone())
                            };
                            if let Some(column_def) = model.merged_fields.get_mut(col_name) {
                                if let Some(id) = id {
                                    if column_def.data_type == DataType::AutoFk {
                                        let org = column_def.clone();
                                        *column_def = id;
                                        column_def.auto = None;
                                        column_def.overwrite(org, "");
                                    } else if column_def.data_type != id.data_type
                                        || column_def.length != id.length
                                        || column_def.signed != id.signed
                                    {
                                        bail!(
                                            "The field type of the {} relation in the {} model is incorrect.",
                                            rel_name,
                                            cur_model_name
                                        );
                                    }
                                }
                                column_def.rel = Some((rel_name.clone(), rel_def.clone()));
                                column_def.main_primary = false;
                                column_def.id_class = None;
                                column_def.enum_class = None;
                                column_def.value_object = None;
                            }
                        }
                        let rel_def = model.merged_relations.get(rel_name).unwrap();
                        let local_ids = rel_def.get_local_id(rel_name, &model);
                        if !local_ids.is_empty() {
                            if model.full_name().eq(&rel_def.model) {
                                ensure!(
                                    local_ids.len() == model.primaries().len(),
                                    "There is an anomaly in the number of fields in the local property of the {} relation of the {} model.",
                                    rel_name,
                                    cur_model_name
                                );
                            } else {
                                let rel_model =
                                    get_model(&rel_def.model, cur_group_name, &groups).borrow();
                                ensure!(
                                    local_ids.len() == rel_model.primaries().len(),
                                    "There is an anomaly in the number of fields in the local property of the {} relation of the {} model.",
                                    rel_name,
                                    cur_model_name
                                );
                            }
                        }
                    }
                    if rel_def.is_type_of_has() {
                        let foreign_ids = rel_def.get_foreign_id(&model);
                        if foreign_ids.len() == 1 {
                            let col_name = &foreign_ids[0];
                            let self_foreign = model.relation_primaries(col_name.to_string());
                            if model.full_name().eq(&rel_def.model) {
                                if !model.non_main_primaries().is_empty() {
                                    let rel_def = model.merged_relations.get_mut(rel_name).unwrap();
                                    rel_def.foreign = Some(self_foreign);
                                }
                            } else {
                                let rel_model =
                                    get_model(&rel_def.model, cur_group_name, &groups).borrow();
                                if !model.non_main_primaries().is_empty() {
                                    let rel_def = model.merged_relations.get_mut(rel_name).unwrap();
                                    rel_def.foreign =
                                        Some(rel_model.relation_primaries(col_name.to_string()));
                                }
                            }
                        }
                        let rel_def = model.merged_relations.get(rel_name).unwrap();
                        let foreign_ids = rel_def.get_foreign_id(&model);
                        if !foreign_ids.is_empty() {
                            ensure!(
                                foreign_ids.len() == model.primaries().len(),
                                "There is an anomaly in the number of fields in the foreign property of the {} relation of the {} model.",
                                rel_name,
                                cur_model_name
                            );
                        }
                    }
                }
                for (k, v) in &model.merged_fields {
                    ensure!(
                        v.data_type != DataType::AutoFk,
                        "There is no definition of belongs_to relation corresponding to auto_fk in the {} column of the {} model.",
                        k,
                        model.name
                    );
                }
            }
            {
                if !def.borrow().abstract_mode
                    && def.borrow().inheritance_type() != Some(InheritanceType::Simple)
                    && def.borrow().inheritance_type() != Some(InheritanceType::ColumnAggregation)
                {
                    for (rel_name, rel_def) in def.clone().borrow().merged_relations.iter() {
                        if !rel_def.is_type_of_has_many() && rel_def.on_delete.is_some() {
                            let mut model =
                                get_model(&rel_def.model, cur_group_name, &groups).borrow_mut();
                            model.on_delete_list.insert(format!(
                                "{}::_base::_{}",
                                &_to_var_name(&cur_group_name.to_snake()),
                                &cur_model_name.to_snake()
                            ));
                        }
                        if rel_def.in_cache {
                            let ref_model = get_model(&rel_def.model, cur_group_name, &groups);
                            let mut ref_model = ref_model.borrow_mut();
                            if ref_model.use_cache() {
                                // let rel_id = rel_def.get_foreign_id(&def.borrow());
                                let rel_hash = crate::common::rel_hash(format!(
                                    "{}::{}::{}",
                                    &cur_group_name, &cur_model_name, rel_name
                                ));
                                ref_model.cache_owners.push((
                                    cur_group_name.to_string(),
                                    cur_model_name.to_string(),
                                    rel_name.to_string(),
                                    rel_hash,
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
    for (cur_group_name, defs) in groups.iter() {
        for (cur_model_name, def) in defs.iter() {
            let model = def.borrow();
            for (rel_name, rel_def) in model.merged_relations.iter() {
                if rel_def.is_type_of_belongs_to() {
                    if model.merged_fields.contains_key(rel_name) {
                        error_exit!(
                            "The same relation name as the {} field in the {} model cannot be used.",
                            rel_name,
                            model.name
                        );
                    }
                    if !model.full_name().eq(&rel_def.model) {
                        let mut rel_model =
                            get_model(&rel_def.model, cur_group_name, &groups).borrow_mut();
                        let m_name = format!("{}::{}", cur_group_name, cur_model_name);
                        if rel_def.on_delete.is_some()
                            && !rel_model
                                .merged_relations
                                .iter()
                                .any(|(__, v)| v.model == m_name)
                        {
                            let r = RelDef {
                                model: m_name,
                                ..Default::default()
                            };
                            rel_model
                                .merged_relations
                                .insert(format!("_{}_{}_", cur_group_name, cur_model_name), r);
                        }
                    }
                }
                if rel_def.is_type_of_has() {
                    if model.merged_fields.contains_key(rel_name) {
                        error_exit!(
                            "The same relation name as the {} field in the {} model cannot be used.",
                            rel_name,
                            model.name
                        );
                    }
                    let foreign_ids = rel_def.get_foreign_id(&model);
                    if model.full_name().eq(&rel_def.model) {
                        ensure!(
                            model.merged_relations.iter().any(|(k, v)| {
                                v.is_type_of_belongs_to()
                                    && v.model == model.full_name()
                                    && v.get_local_id(k, &model) == foreign_ids
                            }),
                            "The {} relation in the {} model requires a corresponding belongs_to.",
                            rel_name,
                            cur_model_name,
                        );
                    } else {
                        let rel_model = get_model(&rel_def.model, cur_group_name, &groups).borrow();
                        ensure!(
                            rel_model.merged_relations.iter().any(|(k, v)| {
                                v.is_type_of_belongs_to()
                                    && v.model == model.full_name()
                                    && v.get_local_id(k, &rel_model) == foreign_ids
                            }),
                            "The {} relation in the {} model requires a corresponding belongs_to.",
                            rel_name,
                            cur_model_name,
                        );
                    }
                }
            }
        }
    }
    let groups: GroupsDef = groups.into_iter().fold(IndexMap::new(), |mut map, (k, v)| {
        let v = v.into_iter().fold(IndexMap::new(), |mut map, (k, v)| {
            let mut model = v.into_inner();
            let non_primaries: Vec<String> = model
                .merged_fields
                .iter()
                .filter(|(_k, v)| !v.primary && !v.auto_gen)
                .map(|(k, _v)| k.clone())
                .collect();
            // Move auto-generated fields to the top.
            for name in non_primaries {
                model.merged_fields.move_index(
                    model.merged_fields.get_index_of(&name).unwrap(),
                    model.merged_fields.len() - 1,
                );
            }
            map.insert(k, (AtomicUsize::new(0), Arc::new(model)));
            map
        });
        map.insert(k, (AtomicUsize::new(0), v));
        map
    });
    CONFIG.write().unwrap().replace(config);
    GROUPS.write().unwrap().replace(groups);
    Ok(())
}

fn fix_inheritance(
    cur_group_name: &str,
    cur_model_name: &String,
    def: &RefCell<ModelDef>,
    groups: &GroupIndex,
) -> Result<()> {
    let mut sql_model_name = cur_model_name.clone();
    let mut cur_model = def;
    let mut cur_g = cur_group_name.to_owned();
    while let Some(ref inheritance) = cur_model.clone().borrow().inheritance {
        let mut model = def.borrow_mut();
        let base_model_refcell = get_model(&inheritance.extends, &cur_g, groups);
        let mut base_model = base_model_refcell.borrow_mut();
        cur_g.clone_from(&base_model.group_name);

        let mut merged_fields = IndexMap::new();
        for (name, col) in &base_model.fields {
            let mut col = col.exact();
            col.in_abstract = base_model.abstract_mode;
            merged_fields.insert(name.clone(), col);
        }
        for (name, def1) in &model.merged_fields {
            if let Some(mut def2) = merged_fields.get(name).cloned() {
                def2.in_abstract = def1.in_abstract;
                if !def2.eq(def1) {
                    bail!("{} column is already defined in {}.", name, cur_model_name);
                }
            } else {
                merged_fields.insert(name.clone(), def1.clone());
            }
        }
        model.merged_fields = merged_fields;

        if inheritance._type != InheritanceType::Concrete {
            sql_model_name.clone_from(&inheritance.extends);
            for (name, def) in &model.fields {
                if let Some(base) = base_model.merged_fields.get(name) {
                    if !base.eq(&def.exact()) {
                        bail!("{} column is already defined in {}.", name, cur_model_name);
                    }
                } else {
                    base_model.merged_fields.insert(name.clone(), def.exact());
                }
            }
        }

        let mut merged_relations = IndexMap::new();
        for (name, mut relation) in base_model.relations.clone() {
            relation.in_abstract = base_model.abstract_mode;
            merged_relations.insert(name, relation);
        }
        for (name, def1) in &model.merged_relations {
            if let Some(mut def2) = merged_relations.get(name).cloned() {
                def2.in_abstract = def1.in_abstract;
                if !def2.eq(def1) {
                    bail!("{} relation is already defined.", name);
                }
            } else {
                merged_relations.insert(name.clone(), def1.clone());
            }
        }
        model.merged_relations = merged_relations;

        if inheritance._type != InheritanceType::Concrete {
            for (name, def) in &model.relations {
                if let Some(base) = base_model.merged_relations.get(name) {
                    if !base.eq(def) {
                        bail!("{} relation is already defined.", name);
                    }
                } else {
                    base_model
                        .merged_relations
                        .insert(name.clone(), def.clone());
                }
            }
        }

        let mut merged_indexes =
            base_model
                .indexes
                .iter()
                .fold(IndexMap::new(), |mut map, (name, index)| {
                    map.insert(name.clone(), index.clone().unwrap_or_default());
                    map
                });
        for (name, def1) in &model.merged_indexes {
            if let Some(def2) = merged_indexes.get(name) {
                if !def2.eq(def1) {
                    bail!("{} index is already defined.", name);
                }
            } else {
                merged_indexes.insert(name.clone(), def1.clone());
            }
        }
        model.merged_indexes = merged_indexes;

        if inheritance._type != InheritanceType::Concrete {
            for (name, index) in &model.indexes {
                if let Some(base) = base_model.merged_indexes.get(name) {
                    if !base.eq(&index.clone().unwrap_or_default()) {
                        bail!("{} index is already defined.", name);
                    }
                } else {
                    base_model
                        .merged_indexes
                        .insert(name.clone(), index.clone().unwrap_or_default());
                }
            }
        }
        cur_model = base_model_refcell;
    }
    let model = get_model(&sql_model_name, cur_group_name, groups);
    let table_name = model.borrow().table_name();
    let model = get_model(cur_model_name, cur_group_name, groups);
    model.borrow_mut().table_name = Some(table_name);
    Ok(())
}

pub fn get_model<'a>(
    model_name: &str,
    cur_group_name: &str,
    groups: &'a GroupIndex,
) -> &'a RefCell<ModelDef> {
    let (group_name, stem_name) = if model_name.contains(MODEL_NAME_SPLITTER) {
        let (group_name, model_name) = model_name.split_once(MODEL_NAME_SPLITTER).unwrap();
        (group_name.to_string(), model_name.to_string())
    } else {
        (cur_group_name.to_owned(), model_name.to_owned())
    };
    let group = groups
        .get(&group_name)
        .unwrap_or_else(|| error_exit!("{} group is not defined", group_name));

    if let Some(model) = group.get(&stem_name) {
        return model;
    }
    let singular_name = to_singular(&stem_name);
    let model = group
        .get(&singular_name)
        .unwrap_or_else(|| error_exit!("{} {} model is not defined", group_name, stem_name));
    model
}

pub fn set_domain_mode(mode: bool) -> bool {
    DOMAIN_MODE.store(mode, std::sync::atomic::Ordering::SeqCst);
    true
}
pub fn domain_mode() -> bool {
    DOMAIN_MODE.load(std::sync::atomic::Ordering::SeqCst)
}

pub fn to_id_name(name: &str) -> String {
    to_id_name_wo_pascal(&name.to_pascal())
}
pub fn to_id_name_wo_pascal(name: &str) -> String {
    if domain_mode() {
        format!("{}Id", name)
    } else {
        format!("_{}Id", name)
    }
}
pub fn _to_var_name(s: &str) -> String {
    let name = s;
    if BAD_KEYWORDS.iter().any(|&x| x == name) {
        format!("_{}", name)
    } else if KEYWORDS.iter().any(|&x| x == name) {
        format!("r#{}", name)
    } else {
        name.to_owned()
    }
}
pub fn _to_raw_var_name(s: &str) -> String {
    let name = s;
    if BAD_KEYWORDS.iter().any(|&x| x == name) {
        format!("_{}", name)
    } else {
        name.to_owned()
    }
}
static KEYWORDS: &[&str] = &[
    "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn", "for",
    "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
    "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where",
    "while", "async", "await", "dyn", "abstract", "become", "box", "do", "final", "macro",
    "override", "priv", "typeof", "unsized", "virtual", "yield", "try", "gen",
];
pub static BAD_KEYWORDS: &[&str] = &["super", "self", "Self", "extern", "crate"];
