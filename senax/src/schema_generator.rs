use anyhow::{Context as _, Result, ensure};
use indexmap::IndexMap;
use once_cell::sync::Lazy;
use regex::Regex;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::env;
use std::path::Path;

use crate::SCHEMA_PATH;
use crate::common::ToCase as _;
use crate::common::{fs_write, simplify_yml, to_plural, to_singular};
use crate::ddl::sql_type::{IndexColumn, ReferenceOption, SqlType, TableKey};
use crate::ddl::table::parse;
use crate::schema::{
    self, BelongsToDef, CONFIG, ConfigDef, DataType, EnumValue, FieldDef, FieldDefOrSubsetType,
    HasManyDef, HasOneDef, IndexDef, ModelDef, SoftDelete, StringOrArray,
};
use crate::schema::{IndexFieldDef, IndexType, Parser};

pub async fn generate(
    db: &str,
    db_url: &Option<String>,
    use_label_as_sql_comment: bool,
    ignore_timestampable: bool,
    interpret_tinyint_as_boolean: bool,
) -> Result<()> {
    let non_snake_case = crate::common::check_non_snake_case()?;
    schema::parse(db, false, true)?;
    let config = CONFIG.read().unwrap().as_ref().unwrap().clone();
    ensure!(!config.groups.is_empty(), "The groups are empty.");
    let url_name = format!("{}_DB_URL", db.to_upper_snake());
    let db_url = if let Some(db_url) = db_url {
        db_url.to_owned()
    } else {
        env::var(&url_name)
            .with_context(|| format!("{} is required in the .env file.", url_name))?
    };
    let tables = parse(&db_url).await?;
    let mut defs: IndexMap<String, ModelDef> = IndexMap::new();
    let mut has_many: HashMap<String, Vec<HasManyDef>> = HashMap::new();
    let mut has_one: HashMap<String, Vec<HasOneDef>> = HashMap::new();
    for (table_name, table) in &tables {
        if table_name.starts_with('_') {
            continue;
        }
        let mut model = ModelDef::default();
        let mut pk = Vec::new();
        let mut indexes = IndexMap::new();
        let mut belongs_to: IndexMap<String, Option<BelongsToDef>> = IndexMap::new();
        let singular_name = to_singular_name(&config, table_name, non_snake_case);
        model.table_name = Some(table_name.clone());
        if use_label_as_sql_comment || config.use_label_as_sql_comment {
            if let Some(comment) = &table.comment {
                if let Some((label, comment)) = comment.trim().split_once('\n') {
                    model.label = Some(label.trim().to_string());
                    model.comment = Some(comment.to_string());
                } else {
                    model.label = Some(comment.trim().to_string());
                }
            }
        } else {
            model.comment = table.comment.as_ref().map(|v| v.trim().to_string());
        }
        if table.engine.as_deref() != Some("InnoDB") {
            model.engine = table.engine.clone();
        }
        if config.collation != table.collation {
            model.collation = table.collation.clone();
        }
        model._before_rename_name = Some(model.table_name());
        if let Some((_, primary)) = &table.primary {
            match primary {
                TableKey::PrimaryKey(cols) => {
                    pk = cols.iter().map(|c| c.name.clone()).collect();
                }
                _ => unimplemented!(),
            }
        }
        let mut created_at = false;
        let mut updated_at = false;
        let mut soft_delete = SoftDelete::None;
        for (name, column) in &table.columns {
            if name == ConfigDef::created_at().as_str() {
                created_at = true;
                continue;
            }
            if name == ConfigDef::updated_at().as_str() {
                updated_at = true;
                continue;
            }
            if name == ConfigDef::deleted_at().as_str() {
                soft_delete = SoftDelete::Time;
                continue;
            }
            if name == ConfigDef::deleted().as_str() {
                if column.sql_type == SqlType::Bool || column.sql_type == SqlType::Tinyint {
                    soft_delete = SoftDelete::Flag;
                } else {
                    soft_delete = SoftDelete::UnixTime;
                }
                continue;
            }
            if name == ConfigDef::version().as_str() {
                model.versioned = true;
                continue;
            }
            let mut field = FieldDef::default();
            match column.sql_type {
                SqlType::Bool => {
                    field.data_type = DataType::Boolean;
                }
                SqlType::Char(len) => {
                    if len == schema::UUID_LENGTH {
                        field.data_type = DataType::Uuid;
                    } else {
                        field.data_type = DataType::Char;
                        field.length = Some(len);
                    }
                }
                SqlType::Varchar(len) => {
                    if len == schema::UUID_LENGTH {
                        field.data_type = DataType::Uuid;
                    } else {
                        field.data_type = DataType::Varchar;
                        field.length = Some(len);
                    }
                }
                SqlType::Int => {
                    field.data_type = DataType::Int;
                    field.signed = true;
                }
                SqlType::UnsignedInt => {
                    field.data_type = DataType::Int;
                }
                SqlType::Smallint => {
                    field.data_type = DataType::SmallInt;
                    field.signed = true;
                }
                SqlType::UnsignedSmallint => {
                    field.data_type = DataType::SmallInt;
                }
                SqlType::Bigint => {
                    field.data_type = DataType::BigInt;
                    field.signed = true;
                }
                SqlType::UnsignedBigint => {
                    field.data_type = DataType::BigInt;
                }
                SqlType::Tinyint if interpret_tinyint_as_boolean => {
                    field.data_type = DataType::Boolean;
                }
                SqlType::Tinyint => {
                    field.data_type = DataType::TinyInt;
                    field.signed = true;
                }
                SqlType::UnsignedTinyint => {
                    field.data_type = DataType::TinyInt;
                }
                SqlType::Blob => {
                    field.data_type = DataType::Blob;
                    // field.length = Some(65535);
                }
                SqlType::Longblob => {
                    field.data_type = DataType::Blob;
                }
                SqlType::Mediumblob => {
                    field.data_type = DataType::Blob;
                    field.length = Some(16777215);
                }
                SqlType::Tinyblob => {
                    field.data_type = DataType::Blob;
                    field.length = Some(255);
                }
                SqlType::Double => {
                    field.data_type = DataType::Double;
                    field.signed = true;
                }
                SqlType::Float => {
                    field.data_type = DataType::Float;
                    field.signed = true;
                }
                SqlType::Real => {
                    field.data_type = DataType::Double;
                    field.signed = true;
                }
                SqlType::Tinytext => {
                    field.data_type = DataType::Text;
                    field.length = Some(255);
                }
                SqlType::Mediumtext => {
                    field.data_type = DataType::Text;
                    field.length = Some(16777215);
                }
                SqlType::Longtext => {
                    field.data_type = DataType::Text;
                }
                SqlType::Text => {
                    field.data_type = DataType::Text;
                    // field.length = Some(65535);
                }
                SqlType::Date => {
                    field.data_type = DataType::Date;
                }
                SqlType::Time => {
                    field.data_type = DataType::Time;
                }
                SqlType::DateTime(precision) => {
                    field.data_type = DataType::UtcDateTime;
                    if precision > 0 {
                        field.precision = Some(precision)
                    }
                }
                SqlType::Timestamp(precision) => {
                    field.data_type = DataType::TimestampWithTimeZone;
                    if precision > 0 {
                        field.precision = Some(precision)
                    }
                }
                SqlType::Binary(len) => {
                    if len == schema::BINARY_UUID_LENGTH {
                        field.data_type = DataType::BinaryUuid;
                    } else {
                        field.data_type = DataType::Binary;
                        field.length = Some(len as u32);
                    }
                }
                SqlType::Varbinary(len) => {
                    if len == schema::BINARY_UUID_LENGTH {
                        field.data_type = DataType::BinaryUuid;
                    } else {
                        field.data_type = DataType::Varbinary;
                        field.length = Some(len as u32);
                    }
                }
                SqlType::Enum(ref values) => {
                    field.data_type = DataType::DbEnum;
                    field.enum_values = Some(
                        values
                            .iter()
                            .map(|v| EnumValue {
                                name: v.to_raw_string(),
                                label: None,
                                comment: None,
                                value: None,
                            })
                            .collect(),
                    );
                }
                SqlType::Set(ref values) => {
                    field.data_type = DataType::DbSet;
                    field.enum_values = Some(
                        values
                            .iter()
                            .map(|v| EnumValue {
                                name: v.to_raw_string(),
                                label: None,
                                comment: None,
                                value: None,
                            })
                            .collect(),
                    );
                }
                SqlType::Decimal(precision, scale) => {
                    field.data_type = DataType::Decimal;
                    if precision > 0 {
                        field.precision = Some(precision)
                    }
                    if scale > 0 {
                        field.scale = Some(scale)
                    }
                }
                SqlType::Json => {
                    field.data_type = DataType::Json;
                }
                SqlType::Point => {
                    field.data_type = DataType::GeoPoint;
                    field.srid = column.constraint.srid;
                }
                SqlType::Geometry => {
                    field.data_type = DataType::Geometry;
                    field.srid = column.constraint.srid;
                }
                SqlType::UnSupported => unimplemented!(),
                SqlType::Uuid => {
                    field.data_type = DataType::Uuid;
                }
            };
            // field.character_set = column.constraint.character_set.clone();
            if config.collation != column.constraint.collation {
                field.collation = column.constraint.collation.clone();
            }
            // field.primary = column.constraint.primary_key || pk.contains(name);
            field.primary = pk.contains(name);
            if column.constraint.auto_increment {
                field.auto = Some(schema::AutoGeneration::AutoIncrement);
            }
            if !field.primary {
                field.not_null = column.constraint.not_null;
            }
            if use_label_as_sql_comment || config.use_label_as_sql_comment {
                if let Some(comment) = &column.comment {
                    if let Some((label, comment)) = comment.split_once('\n') {
                        field.label = Some(label.trim().to_string());
                        field.comment = Some(comment.to_string());
                    } else {
                        field.label = Some(comment.trim().to_string());
                    }
                }
            } else {
                field.sql_comment = column.comment.clone();
            }
            field._before_rename_name = Some(field.get_col_name(name).to_string());
            model
                .fields
                .insert(name.clone(), FieldDefOrSubsetType::Exact(field));
        }
        if !ignore_timestampable {
            if config.timestampable.is_some() {
                if !created_at && !updated_at {
                    model.timestampable = Some(schema::Timestampable::None);
                } else if !created_at {
                    model.disable_created_at = true;
                } else if !updated_at {
                    model.disable_updated_at = true;
                }
            } else if created_at || updated_at {
                model.timestampable = Some(schema::Timestampable::FixedTime);
                if !created_at {
                    model.disable_created_at = true;
                }
                if !updated_at {
                    model.disable_updated_at = true;
                }
            }
        }
        if !ignore_timestampable || soft_delete != SoftDelete::None {
            match config.soft_delete {
                None | Some(SoftDelete::None) => {
                    if soft_delete != SoftDelete::None {
                        model.soft_delete = Some(soft_delete)
                    }
                }
                Some(s) => {
                    if soft_delete != s {
                        model.soft_delete = Some(soft_delete)
                    }
                }
            }
        }
        let mut constraint_idx = HashSet::new();
        let pk = if pk.len() == 1 {
            StringOrArray::One(pk[0].clone())
        } else {
            StringOrArray::Many(pk.to_vec())
        };
        for constraint in table.constraints.values() {
            match constraint {
                TableKey::Constraint(
                    _name,
                    columns,
                    table,
                    _foreign_cols,
                    on_delete,
                    on_update,
                ) => {
                    let parent_singular_name = to_singular_name(&config, table, non_snake_case);
                    let mut def = BelongsToDef {
                        model: Some(to_combined_name(
                            &config,
                            &parent_singular_name,
                            &singular_name,
                        )),
                        ..Default::default()
                    };
                    let local = if columns.len() == 1 {
                        StringOrArray::One(columns[0].name.clone())
                    } else {
                        StringOrArray::Many(columns.iter().map(|v| v.name.clone()).collect())
                    };
                    def.local = Some(local.clone());
                    constraint_idx.insert(local.to_vec().join(","));
                    if soft_delete == SoftDelete::Time {
                        constraint_idx.insert(format!(
                            "{},{}",
                            local.to_vec().join(","),
                            ConfigDef::deleted_at()
                        ));
                    }
                    if soft_delete == SoftDelete::Flag || soft_delete == SoftDelete::UnixTime {
                        constraint_idx.insert(format!(
                            "{},{}",
                            local.to_vec().join(","),
                            ConfigDef::deleted()
                        ));
                    }
                    def.on_delete = on_delete.as_ref().map(|v| match v {
                        ReferenceOption::Restrict => schema::ReferenceOption::Restrict,
                        ReferenceOption::Cascade => schema::ReferenceOption::Cascade,
                        ReferenceOption::SetNull => schema::ReferenceOption::SetNull,
                        ReferenceOption::NoAction => schema::ReferenceOption::Restrict,
                        ReferenceOption::SetDefault => schema::ReferenceOption::SetZero,
                    });
                    def.on_update = on_update.as_ref().map(|v| match v {
                        ReferenceOption::Restrict => schema::ReferenceOption::Restrict,
                        ReferenceOption::Cascade => schema::ReferenceOption::Cascade,
                        ReferenceOption::SetNull => schema::ReferenceOption::SetNull,
                        ReferenceOption::NoAction => schema::ReferenceOption::Restrict,
                        ReferenceOption::SetDefault => schema::ReferenceOption::SetZero,
                    });

                    if pk == local {
                        let name = parent_singular_name.clone();
                        belongs_to.insert(name, Some(def));
                        let foreign = if local
                            == StringOrArray::One(format!("{}_id", parent_singular_name))
                        {
                            None
                        } else {
                            Some(local.clone())
                        };
                        let def = HasOneDef {
                            model: Some(to_combined_name(
                                &config,
                                &singular_name,
                                &parent_singular_name,
                            )),
                            foreign,
                            ..Default::default()
                        };
                        has_one.entry(parent_singular_name).or_default().push(def);
                    } else {
                        let name = &columns.last().unwrap().name;
                        let name = if name.ends_with("_id") {
                            name.trim_end_matches("_id")
                        } else {
                            &parent_singular_name
                        };
                        belongs_to.insert(name.to_string(), Some(def));
                        let foreign = if local
                            == StringOrArray::One(format!("{}_id", parent_singular_name))
                        {
                            None
                        } else {
                            Some(local.clone())
                        };
                        let def = HasManyDef {
                            model: Some(to_combined_name(
                                &config,
                                &singular_name,
                                &parent_singular_name,
                            )),
                            foreign,
                            ..Default::default()
                        };
                        has_many.entry(parent_singular_name).or_default().push(def);
                    }
                }
                _ => unimplemented!(),
            }
        }
        if config.ignore_foreign_key {
            for (name, _col) in &table.columns {
                if name.ends_with("_id") {
                    let parent_singular_name = name.trim_end_matches("_id");
                    let parent = if config.plural_table_name {
                        to_plural(parent_singular_name)
                    } else {
                        parent_singular_name.to_string()
                    };
                    if parent == *table_name {
                        continue;
                    }
                    let mut table_exists = tables.get(&parent).is_some();
                    for (group, _) in &config.groups {
                        let n = format!("{}_{}", group, &parent);
                        table_exists =
                            table_exists || (n != *table_name && tables.get(&n).is_some());
                    }
                    if table_exists {
                        let mut def = BelongsToDef {
                            model: Some(to_combined_name(
                                &config,
                                parent_singular_name,
                                &singular_name,
                            )),
                            ..Default::default()
                        };
                        // if parent_singular_name == def.model.as_ref().unwrap() {
                        //     def.model = None;
                        // } else {
                        //     def.local = Some(name.clone());
                        // }
                        let local = StringOrArray::One(name.clone());
                        def.local = Some(local.clone());
                        constraint_idx.insert(name.clone());
                        if soft_delete == SoftDelete::Time {
                            constraint_idx.insert(format!("{},{}", name, ConfigDef::deleted_at()));
                        }
                        if soft_delete == SoftDelete::Flag || soft_delete == SoftDelete::UnixTime {
                            constraint_idx.insert(format!("{},{}", name, ConfigDef::deleted()));
                        }
                        def.on_delete = Some(schema::ReferenceOption::Cascade);
                        belongs_to.insert(parent_singular_name.to_string(), Some(def));

                        if pk == local {
                            let def = HasOneDef {
                                model: Some(to_combined_name(
                                    &config,
                                    &singular_name,
                                    parent_singular_name,
                                )),
                                ..Default::default()
                            };
                            has_one
                                .entry(parent_singular_name.to_string())
                                .or_default()
                                .push(def);
                        } else {
                            let def = HasManyDef {
                                model: Some(to_combined_name(
                                    &config,
                                    &singular_name,
                                    parent_singular_name,
                                )),
                                ..Default::default()
                            };
                            has_many
                                .entry(parent_singular_name.to_string())
                                .or_default()
                                .push(def);
                        }
                    }
                }
            }
        }
        for index in table.indexes.values() {
            match index {
                TableKey::UniqueKey(name, cols) => {
                    let mut def = IndexDef {
                        fields: cols.iter().fold(IndexMap::new(), |mut map, col| {
                            map.insert(
                                make_name(col),
                                if col.len.is_some() || col.query.is_some() {
                                    Some(IndexFieldDef {
                                        direction: None,
                                        length: col.len,
                                        query: col.query.clone(),
                                    })
                                } else {
                                    None
                                },
                            );
                            map
                        }),
                        index_type: Some(IndexType::Unique),
                        parser: None,
                        force_index_on: Default::default(),
                    };
                    let name = name.trim_start_matches("UQ_").to_string();
                    if let Some(first) = def.fields.first()
                        && def.fields.len() == 1
                        && *first.0 == name
                        && first.1.is_none()
                    {
                        def.fields.clear();
                    }
                    indexes.insert(name, Some(def));
                }
                TableKey::FulltextKey(name, cols, parser) => {
                    let mut def = IndexDef {
                        fields: cols.iter().fold(IndexMap::new(), |mut map, col| {
                            map.insert(
                                make_name(col),
                                if col.len.is_some() || col.query.is_some() {
                                    Some(IndexFieldDef {
                                        direction: None,
                                        length: col.len,
                                        query: col.query.clone(),
                                    })
                                } else {
                                    None
                                },
                            );
                            map
                        }),
                        index_type: Some(IndexType::Fulltext),
                        parser: parser.as_ref().map(Parser::from),
                        force_index_on: Default::default(),
                    };
                    let name = name.trim_start_matches("FT_").to_string();
                    if let Some(first) = def.fields.first()
                        && def.fields.len() == 1
                        && *first.0 == name
                        && first.1.is_none()
                    {
                        def.fields.clear();
                    }
                    indexes.insert(name, Some(def));
                }
                TableKey::Key(name, cols) => {
                    let chk = cols
                        .iter()
                        .map(|v| v.name.clone())
                        .collect::<Vec<_>>()
                        .join(",");
                    if constraint_idx.contains(&chk) {
                        continue;
                    }
                    let mut def = IndexDef {
                        fields: cols.iter().fold(IndexMap::new(), |mut map, col| {
                            map.insert(
                                make_name(col),
                                if col.len.is_some() || col.query.is_some() {
                                    Some(IndexFieldDef {
                                        direction: None,
                                        length: col.len,
                                        query: col.query.clone(),
                                    })
                                } else {
                                    None
                                },
                            );
                            map
                        }),
                        index_type: None,
                        parser: None,
                        force_index_on: Default::default(),
                    };
                    let name = name.trim_start_matches("IDX_").to_string();
                    if let Some(first) = def.fields.first()
                        && def.fields.len() == 1
                        && *first.0 == name
                        && first.1.is_none()
                    {
                        def.fields.clear();
                    }
                    if def == IndexDef::default() {
                        indexes.insert(name, None);
                    } else {
                        indexes.insert(name, Some(def));
                    }
                }
                TableKey::SpatialKey(name, cols) => {
                    let mut def = IndexDef {
                        fields: cols.iter().fold(IndexMap::new(), |mut map, col| {
                            map.insert(
                                make_name(col),
                                if col.len.is_some() || col.query.is_some() {
                                    Some(IndexFieldDef {
                                        direction: None,
                                        length: col.len,
                                        query: col.query.clone(),
                                    })
                                } else {
                                    None
                                },
                            );
                            map
                        }),
                        index_type: Some(IndexType::Spatial),
                        parser: None,
                        force_index_on: Default::default(),
                    };
                    let name = name.trim_start_matches("SP_").to_string();
                    if let Some(first) = def.fields.first()
                        && def.fields.len() == 1
                        && *first.0 == name
                        && first.1.is_none()
                    {
                        def.fields.clear();
                    }
                    indexes.insert(name, Some(def));
                }
                _ => unimplemented!(),
            }
        }
        model.belongs_to = belongs_to;
        model.indexes = indexes;
        let (group_name, model_name) = to_group_and_model_name(&config, &singular_name);
        model.group_name = group_name;
        model.name = model_name;
        defs.insert(singular_name, model);
    }
    for (singular_name, relations) in has_one {
        if let Some(def) = defs.get_mut(&singular_name) {
            for mut relation in relations {
                let names: Vec<_> = relation.model.as_ref().unwrap().split("::").collect();
                let name = names.last().unwrap().to_string();
                if relation.foreign == Some(StringOrArray::One(format!("{}_id", &def.name))) {
                    relation.foreign = None;
                }
                def.has_one.insert(name, Some(relation));
            }
        }
    }
    for (singular_name, relations) in has_many {
        if let Some(def) = defs.get_mut(&singular_name) {
            let mut counter = HashMap::new();
            for relation in &relations {
                counter
                    .entry(relation.model.clone().unwrap())
                    .and_modify(|v| *v += 1)
                    .or_insert(1);
            }
            for mut relation in relations {
                let names: Vec<_> = relation.model.as_ref().unwrap().split("::").collect();
                let mut name = String::new();
                if relation.foreign == Some(StringOrArray::One(format!("{}_id", &def.name))) {
                    relation.foreign = None;
                    name.push_str(&to_plural(names.last().unwrap()));
                } else {
                    if *counter.get(relation.model.as_ref().unwrap()).unwrap() > 1 {
                        name.push_str(
                            relation
                                .foreign
                                .as_ref()
                                .unwrap()
                                .last()
                                .trim_end_matches("_id")
                                .trim_end_matches(&format!("_{}", names.last().unwrap())),
                        );
                        name.push('_');
                    }
                    name.push_str(&to_plural(names.last().unwrap()));
                }
                def.has_many.insert(name, Some(relation));
            }
        }
    }
    let mut groups: IndexMap<String, IndexMap<String, ModelDef>> = IndexMap::new();
    for (_, mut model) in defs {
        let group = config.groups.get(&model.group_name).unwrap();
        let exclude = group.as_ref().map(|v| v.exclude_group_from_table_name);
        if model.table_name == Some(model.derive_table_name(exclude)) {
            model.table_name = None;
        }
        groups
            .entry(model.group_name.clone())
            .or_default()
            .insert(model.name.clone(), model);
    }
    let path = Path::new(SCHEMA_PATH).join(db);
    for (group, defs) in groups {
        let file_path = path.join(format!("{}.yml", group));
        let mut buf =
            "# yaml-language-server: $schema=../../senax-schema.json#properties/model\n\n"
                .to_string();
        buf.push_str(&simplify_yml(serde_yaml::to_string(&defs)?)?);
        fs_write(file_path, &buf)?;
    }
    Ok(())
}

fn make_name(col: &IndexColumn) -> String {
    if !col.name.is_empty() {
        col.name.clone()
    } else if let Some(ref query) = col.query {
        use std::fmt::Write;
        Sha256::digest(query)
            .iter()
            .take(4)
            .fold(String::new(), |mut output, x| {
                let _ = write!(output, "{:02X}", x);
                output
            })
    } else {
        panic!("no name");
    }
}

fn to_group_and_model_name(config: &ConfigDef, singular_name: &str) -> (String, String) {
    let mut group = config.groups.first().unwrap().0.to_string();
    let mut model_name = singular_name.to_string();
    let mut len = 0;
    for (name, _) in &config.groups {
        let prefix = format!("{}_", name);
        if singular_name.starts_with(&prefix) && name.len() > len {
            group = name.clone();
            len = name.len();
            model_name = singular_name.trim_start_matches(&prefix).to_string();
        }
    }
    (group, model_name)
}

fn to_combined_name(config: &ConfigDef, singular_name: &str, another: &str) -> String {
    let (group, model_name) = to_group_and_model_name(config, singular_name);
    let (another_group, _) = to_group_and_model_name(config, another);
    if group == another_group {
        model_name
    } else {
        format!("{}::{}", group, model_name)
    }
}

fn to_singular_name(config: &ConfigDef, table_name: &str, non_snake_case: bool) -> String {
    let mut singular_name = if config.plural_table_name {
        to_singular(table_name)
    } else {
        table_name.to_string()
    };
    static UPPER: Lazy<Regex> = Lazy::new(|| Regex::new(r"[A-Z]").unwrap());
    static LOWER: Lazy<Regex> = Lazy::new(|| Regex::new(r"[a-z]").unwrap());
    if !non_snake_case && UPPER.is_match(&singular_name) {
        if LOWER.is_match(&singular_name) {
            singular_name = singular_name.to_snake();
        } else {
            singular_name = singular_name.to_lowercase();
        }
    }
    singular_name
}
