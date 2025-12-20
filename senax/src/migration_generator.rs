use anyhow::{Context as _, Result};
use chrono::Utc;
use derive_more::Display;
use indexmap::IndexMap;
use std::collections::{BTreeMap, HashSet};
use std::env;
use std::fmt::Write;
use std::path::Path;
use std::str::FromStr;

use crate::common::fs_write;
use crate::common::{ToCase as _, escape_db_identifier};
use crate::ddl::sql_type::{IndexColumn, Literal, ReferenceOption, SqlType, TableKey};
use crate::ddl::table::{Column, Constraint, Table};
use crate::schema::{
    self, AutoGeneration, CONFIG, GROUPS, SoftDelete, SortDirection, is_mysql_mode,
};
use crate::{DB_PATH, ddl};

pub const UTF8_BYTE_LEN: u32 = 4;
pub const MYSQL_UUID_COLLATION: &str = "ascii_general_ci";

pub async fn generate(
    db: &str,
    description: &Option<String>,
    empty: bool,
    skip_empty: bool,
    use_test_db: bool,
) -> Result<()> {
    schema::parse(db, false, false)?;
    let config = CONFIG.read().unwrap().as_ref().unwrap().clone();
    let mut new_tables = IndexMap::new();
    {
        let group_lock = GROUPS.read().unwrap();
        let groups = group_lock.as_ref().unwrap();
        for (_group_name, (_, defs, _)) in groups {
            for (_model_name, (_, def)) in defs {
                if def.has_table() {
                    let (table_name, table, _) = make_table_def(def, &config)?;
                    new_tables.insert(table_name, table);
                }
            }
        }
    }
    let url_name = if use_test_db {
        format!("{}_TEST_DB_URL", db.to_upper_snake())
    } else {
        format!("{}_DB_URL", db.to_upper_snake())
    };
    let db_url = env::var(&url_name)
        .with_context(|| format!("{} is required in the .env file.", url_name))?;
    let old_tables = ddl::table::parse(&db_url).await?;
    let cli_mode = description.is_none();
    let (mut ddl, mut ddl_list) = make_ddl(new_tables, old_tables, cli_mode)?;
    if empty {
        ddl.clear();
        ddl_list.clear();
    }
    if skip_empty && ddl.is_empty() {
        return Ok(());
    }
    if let Some(description) = description {
        if ddl.is_empty() {
            ddl.push_str("-- TODO: Fix this file.\n");
        }
        ddl_list.insert(0, ddl);
        let description: String = description
            .chars()
            .map(|c| {
                if c.is_control() || c.is_whitespace() {
                    '_'
                } else {
                    c
                }
            })
            .collect();
        let base_path = Path::new(DB_PATH).join(db);
        let ddl_path = base_path.join("migrations");
        let dt = Utc::now();
        let mut file_prefix: u64 = dt.format("%Y%m%d%H%M%S").to_string().parse().unwrap();
        ddl_list.reverse();
        loop {
            let file_path = ddl_path.join(format!("{}_*.sql", file_prefix));
            if glob::glob(file_path.to_str().unwrap())?.count() > 0 {
                file_prefix += 1;
                continue;
            }
            let ddl = if let Some(ddl) = ddl_list.pop() {
                ddl
            } else {
                break;
            };
            let file_path = ddl_path.join(format!("{}_{}.sql", file_prefix, description));
            fs_write(file_path, &ddl)?;
        }
    } else if !ddl.is_empty() || !ddl_list.is_empty() {
        println!("-- {}", &db);
        println!("{}", &ddl);
        for line in ddl_list {
            println!("{}", &line);
        }
    }
    Ok(())
}

pub fn make_table_def(
    def: &std::sync::Arc<schema::ModelDef>,
    config: &schema::ConfigDef,
) -> Result<(String, Table, IndexMap<String, String>)> {
    let table_name = def.table_name();
    let mut table = Table {
        name: table_name.clone(),
        old_name: def._before_rename_name.clone(),
        engine: def.engine.clone().or_else(|| config.engine.clone()),
        skip_ddl: def.skip_ddl,
        ..Default::default()
    };
    let old_soft_delete = def._soft_delete.as_ref().and_then(|v| {
        let v: Vec<_> = v.split(',').collect();
        if let [col, typ] = v[..] {
            Some((col.to_string(), SoftDelete::from_str(typ).unwrap()))
        } else {
            None
        }
    });
    let soft_delete = def
        .soft_delete()
        .map(|typ| (def.soft_delete_col().unwrap().to_string(), typ));
    if old_soft_delete != soft_delete {
        table.old_soft_delete = old_soft_delete;
    }
    for (col_name, col) in &def.merged_fields {
        let mut constraint = Constraint {
            not_null: col.not_null,
            collation: col.collation.clone(),
            // default_value: Default::default(),
            auto_increment: col.auto == Some(AutoGeneration::AutoIncrement),
            // primary_key: col.primary
            // primary_key: Default::default(),
            // unique: Default::default(),
            srid: col.srid,
            query: col
                .query
                .clone()
                .map(|v| (v, col.generated_is_stored.unwrap_or_default())),
        };
        let sql_type = match col.data_type {
            schema::DataType::TinyInt if col.signed => SqlType::Tinyint,
            schema::DataType::TinyInt => SqlType::UnsignedTinyint,
            schema::DataType::SmallInt if col.signed => SqlType::Smallint,
            schema::DataType::SmallInt => SqlType::UnsignedSmallint,
            schema::DataType::Int if col.signed => SqlType::Int,
            schema::DataType::Int => SqlType::UnsignedInt,
            schema::DataType::BigInt if col.signed => SqlType::Bigint,
            schema::DataType::BigInt => SqlType::UnsignedBigint,
            schema::DataType::Float => SqlType::Float,
            schema::DataType::Double => SqlType::Double,
            schema::DataType::Char => SqlType::Char(
                col.length
                    .with_context(|| format!("length is required: {:?}", col_name))?,
            ),
            schema::DataType::IdVarchar => {
                SqlType::Varchar(col.length.unwrap_or(schema::DEFAULT_VARCHAR_LENGTH))
            }
            schema::DataType::TextVarchar => {
                SqlType::Varchar(col.length.unwrap_or(schema::DEFAULT_VARCHAR_LENGTH))
            }
            schema::DataType::Boolean if is_mysql_mode() => SqlType::Tinyint,
            schema::DataType::Boolean => SqlType::Bool,
            schema::DataType::Text if col.length.unwrap_or(65536) * UTF8_BYTE_LEN < 256 => {
                SqlType::Tinytext
            }
            schema::DataType::Text if col.length.unwrap_or(65536) * UTF8_BYTE_LEN < 65536 => {
                SqlType::Text
            }
            schema::DataType::Text if is_mysql_mode() => SqlType::Longtext,
            schema::DataType::Text => SqlType::Text,
            schema::DataType::Uuid if is_mysql_mode() => SqlType::Char(schema::UUID_LENGTH),
            schema::DataType::Uuid => SqlType::Uuid,
            schema::DataType::BinaryUuid => SqlType::Binary(schema::BINARY_UUID_LENGTH),
            schema::DataType::Binary => SqlType::Binary(
                col.length
                    .with_context(|| format!("length is required: {:?}", col_name))?
                    .try_into()?,
            ),
            schema::DataType::Varbinary => SqlType::Varbinary(
                col.length
                    .unwrap_or(schema::DEFAULT_VARCHAR_LENGTH)
                    .try_into()?,
            ),
            schema::DataType::Blob if col.length.unwrap_or(65536) < 256 => SqlType::Tinyblob,
            schema::DataType::Blob if col.length.unwrap_or(65536) < 65536 => SqlType::Blob,
            schema::DataType::Blob => SqlType::Longblob,
            schema::DataType::NaiveDateTime => SqlType::DateTime(col.precision.unwrap_or(0)),
            schema::DataType::UtcDateTime => SqlType::DateTime(col.precision.unwrap_or(0)),
            schema::DataType::TimestampWithTimeZone => {
                SqlType::Timestamp(col.precision.unwrap_or(0))
            }
            schema::DataType::Date => SqlType::Date,
            schema::DataType::Time => SqlType::Time,
            schema::DataType::Decimal => SqlType::Decimal(
                col.precision.unwrap_or(schema::DEFAULT_PRECISION),
                col.scale.unwrap_or(schema::DEFAULT_SCALE),
            ),
            schema::DataType::ArrayInt => SqlType::Json,
            schema::DataType::ArrayString => SqlType::Json,
            schema::DataType::Json => SqlType::Json,
            schema::DataType::Jsonb => SqlType::Jsonb,
            schema::DataType::DbEnum => SqlType::Enum(
                col.enum_values
                    .as_ref()
                    .unwrap_or(&Vec::new())
                    .iter()
                    .map(|v| Literal::String(v.name.clone()))
                    .collect(),
            ),
            schema::DataType::DbSet => SqlType::Set(
                col.enum_values
                    .as_ref()
                    .unwrap_or(&Vec::new())
                    .iter()
                    .map(|v| Literal::String(v.name.clone()))
                    .collect(),
            ),
            schema::DataType::Point => SqlType::Point,
            schema::DataType::GeoPoint => SqlType::Point,
            schema::DataType::Geometry => SqlType::Geometry,
            schema::DataType::ValueObject => unimplemented!(),
            schema::DataType::AutoFk => unimplemented!(),
            schema::DataType::UnSupported => unimplemented!(),
        };
        let alt_type = if col.data_type == schema::DataType::Uuid {
            SqlType::Varchar(schema::UUID_LENGTH)
        } else if col.data_type == schema::DataType::BinaryUuid {
            SqlType::Varbinary(schema::BINARY_UUID_LENGTH)
        } else if is_mysql_mode() && col.data_type == schema::DataType::Jsonb {
            SqlType::Json
        } else {
            sql_type.clone()
        };
        if constraint.collation.is_none() {
            if col.data_type == schema::DataType::Char
                || col.data_type == schema::DataType::IdVarchar
            {
                constraint.collation = config.id_collation.clone();
            }
            if col.data_type == schema::DataType::TextVarchar
                || col.data_type == schema::DataType::Text
            {
                constraint.collation = config.text_collation.clone();
            }
            if col.data_type == schema::DataType::Uuid && is_mysql_mode() {
                constraint.collation = Some(MYSQL_UUID_COLLATION.to_string());
            }
        }
        let mut sql_comment = col.sql_comment.clone();
        if sql_comment.is_none() && config.use_label_as_sql_comment {
            sql_comment.clone_from(&col.label);
        }
        let default = col.default.clone();
        table.columns.insert(
            col.get_col_name(col_name).to_string(),
            Column {
                old_name: col._before_rename_name.clone(),
                sql_type: sql_type.clone(),
                alt_type,
                constraint,
                default: default
                    .as_ref()
                    .map(|v| ddl::table::parse_default_value(v, &sql_type))
                    .transpose()
                    .with_context(|| format!("default value parse error: {:?}", default))?,
                comment: sql_comment,
            },
        );
    }
    let cols: Vec<IndexColumn> = def
        .primaries()
        .iter()
        .map(|(n, c)| IndexColumn {
            name: c.get_col_name(n).to_string(),
            query: None,
            len: None,
            desc: false,
        })
        .collect();
    if !cols.is_empty() {
        table.primary = Some((String::new(), TableKey::PrimaryKey(cols)));
    }
    let mut idx_check = HashSet::new();
    let mut idx_map = IndexMap::new();
    for (org_index_name, index) in &def.merged_indexes {
        let fields = if !index.fields.is_empty() {
            index.fields.clone()
        } else {
            let mut fields = IndexMap::new();
            fields.insert(org_index_name.clone(), None);
            fields
        };
        let cols: Vec<IndexColumn> = fields
            .iter()
            .map(|(n, c)| {
                let col = def
                    .merged_fields
                    .get(n)
                    .unwrap_or_else(|| error_exit!("{} is not in columns", n));
                let name = col.get_col_name(n).to_string();
                let len = c.as_ref().and_then(|c| c.length);
                let query = if let Some(Some(query)) = c.as_ref().map(|c| &c.query) {
                    Some(query.clone())
                } else if col.data_type == schema::DataType::ArrayInt {
                    Some(format!("CAST(`{}` AS UNSIGNED ARRAY)", name))
                } else if col.data_type == schema::DataType::ArrayString {
                    Some(format!(
                        "CAST(`{}` AS CHAR({}) ARRAY)",
                        name,
                        len.unwrap_or(255)
                    ))
                } else {
                    None
                };
                if !col.not_null
                    && index.index_type == Some(schema::IndexType::Geometry)
                    && is_mysql_mode()
                {
                    error_exit!("All parts of a SPATIAL index must be NOT NULL: {}", n)
                }
                let desc = c
                    .as_ref()
                    .map(|c| c.direction == Some(SortDirection::Desc))
                    .unwrap_or_default();
                IndexColumn {
                    name,
                    query,
                    len,
                    desc,
                }
            })
            .collect();
        let query = cols
            .iter()
            .filter_map(|v| v.query.clone())
            .collect::<Vec<_>>()
            .join(",");
        let mut index_name = if is_mysql_mode() {
            org_index_name.clone()
        } else {
            format!("{}_{}", &table_name, org_index_name)
        };
        if !query.is_empty() {
            index_name.push('_');
            index_name.push_str(&crate::common::hex_digest(&query));
        }
        if index.index_type == Some(schema::IndexType::Unique) {
            let mut check = String::new();
            for col in &cols {
                check.push_str(&format!("{},", col.name));
                idx_check.insert(check.clone());
            }
            let index_name = format!("UQ_{}", index_name);
            idx_map.insert(org_index_name.clone(), index_name.clone());
            table
                .indexes
                .insert(index_name.clone(), TableKey::UniqueKey(index_name, cols));
        } else if index.index_type == Some(schema::IndexType::Fulltext) {
            let index_name = format!("FT_{}", index_name);
            idx_map.insert(org_index_name.clone(), index_name.clone());
            table.indexes.insert(
                index_name.clone(),
                TableKey::FulltextKey(index_name, cols, index.parser.map(|v| v.to_string())),
            );
        } else if index.index_type == Some(schema::IndexType::Geometry) {
            let index_name = format!("SP_{}", index_name);
            idx_map.insert(org_index_name.clone(), index_name.clone());
            table
                .indexes
                .insert(index_name.clone(), TableKey::GeometryKey(index_name, cols));
        } else {
            let mut check = String::new();
            for col in &cols {
                check.push_str(&format!("{},", col.name));
                idx_check.insert(check.clone());
            }
            let index_name = format!("IDX_{}", index_name);
            idx_map.insert(org_index_name.clone(), index_name.clone());
            table
                .indexes
                .insert(index_name.clone(), TableKey::Key(index_name, cols));
        }
    }
    for (model, name, rel) in def.relation_constraint() {
        let local_id = rel.get_local_id(name, model);
        let foreign_table = rel.get_foreign_table_name();
        let key_name = format!("FK_{}_{}_{}", &table_name, name, foreign_table);
        let local_cols: Vec<_> = local_id
            .iter()
            .map(|local_id| {
                let local_col_name = if let Some(local_col) = def.merged_fields.get(local_id) {
                    local_col.get_col_name(local_id).to_string()
                } else {
                    local_id.clone()
                };
                IndexColumn {
                    name: local_col_name,
                    query: None,
                    len: None,
                    desc: false,
                }
            })
            .collect();
        let foreign = rel.get_foreign_model();
        let foreign_primaries = foreign
            .primaries()
            .iter()
            .map(|(n, c)| IndexColumn {
                name: c.get_col_name(n).to_string(),
                query: None,
                len: None,
                desc: false,
            })
            .collect();
        let index_name = format!("IDX_FK_{}", &name);
        let mut cols = local_cols.clone();
        if config.add_soft_delete_column_to_relation_index
            && let Some(col) = def.soft_delete_col()
        {
            cols.push(IndexColumn {
                name: col.to_string(),
                query: None,
                len: None,
                desc: false,
            });
        }
        let check = cols.iter().fold(String::new(), |mut output, v| {
            let _ = write!(output, "{},", v.name);
            output
        });
        if !config.disable_relation_index {
            if idx_check.contains(&check) {
                table
                    .indexes
                    .insert(index_name.clone(), TableKey::Key(index_name, vec![]));
            } else {
                table
                    .indexes
                    .insert(index_name.clone(), TableKey::Key(index_name, cols));
            }
        }
        if !def.ignore_foreign_key() {
            table.constraints.insert(
                key_name.clone(),
                TableKey::Constraint(
                    key_name,
                    local_cols,
                    foreign_table,
                    foreign_primaries,
                    ref_op(&rel.on_delete),
                    ref_op(&rel.on_update),
                ),
            );
        }
    }
    for (model, name, rel) in def.outer_db_relation_constraint() {
        let local_id = rel.get_local_id(name, model);
        let local_cols: Vec<_> = local_id
            .iter()
            .map(|local_id| {
                let local_col_name = if let Some(local_col) = def.merged_fields.get(local_id) {
                    local_col.get_col_name(local_id).to_string()
                } else {
                    local_id.clone()
                };
                IndexColumn {
                    name: local_col_name,
                    query: None,
                    len: None,
                    desc: false,
                }
            })
            .collect();
        let index_name = format!("IDX_FK_{}", &name);
        let mut cols = local_cols.clone();
        if config.add_soft_delete_column_to_relation_index
            && let Some(col) = def.soft_delete_col()
        {
            cols.push(IndexColumn {
                name: col.to_string(),
                query: None,
                len: None,
                desc: false,
            });
        }
        let check = cols.iter().fold(String::new(), |mut output, v| {
            let _ = write!(output, "{},", v.name);
            output
        });
        if !config.disable_relation_index {
            if idx_check.contains(&check) {
                table
                    .indexes
                    .insert(index_name.clone(), TableKey::Key(index_name, vec![]));
            } else {
                table
                    .indexes
                    .insert(index_name.clone(), TableKey::Key(index_name, cols));
            }
        }
    }
    Ok((table_name, table, idx_map))
}

fn ref_op(r: &Option<schema::ReferenceOption>) -> Option<ReferenceOption> {
    match r {
        Some(schema::ReferenceOption::Restrict) => Some(ReferenceOption::Restrict),
        Some(schema::ReferenceOption::Cascade) => Some(ReferenceOption::Cascade),
        Some(schema::ReferenceOption::SetNull) => Some(ReferenceOption::SetNull),
        // Some(schema::ReferenceOption::NoAction) => " Some(ReferenceOption::NoAction),
        Some(schema::ReferenceOption::SetZero) => Some(ReferenceOption::SetDefault),
        None => None,
    }
}

#[derive(Display, Eq, PartialOrd, Ord, PartialEq)]
enum Type {
    AddTable,
    DropTable,
    RenameTable,
    AddColumn,
    ChangeColumn,
    DropColumn,
    RenameColumn,
    ChangePrimary,
    DropPrimary,
    AddIndex,
    ChangeIndex,
    DropIndex,
    AddForeign,
    ChangeForeign,
    DropForeign,
}

fn make_ddl(
    mut new_tables: IndexMap<String, Table>,
    mut old_tables: IndexMap<String, Table>,
    cli_mode: bool,
) -> Result<(String, Vec<String>)> {
    let mut result1 = String::new();
    let mut result2 = Vec::new();
    let mut history: BTreeMap<Type, IndexMap<String, Vec<String>>> = BTreeMap::new();
    while {
        let mut found = false;
        for (table_name, new_table) in new_tables.iter_mut() {
            if new_table.skip_ddl {
                continue;
            }
            if let Some(old_name) = &new_table.old_name
                && table_name != old_name
                && old_tables.contains_key(old_name)
                && !old_tables.contains_key(table_name)
            {
                history
                    .entry(Type::RenameTable)
                    .or_default()
                    .entry(table_name.clone())
                    .or_default()
                    .push(old_name.clone());
                if is_mysql_mode() {
                    writeln!(
                        &mut result1,
                        "RENAME TABLE {} TO {};",
                        &escape_db_identifier(old_name),
                        &escape_db_identifier(table_name)
                    )?;
                } else {
                    writeln!(
                        &mut result1,
                        "ALTER TABLE {} RENAME TO {};",
                        &escape_db_identifier(old_name),
                        &escape_db_identifier(table_name)
                    )?;
                }
                let table = old_tables.swap_remove(old_name).unwrap();
                old_tables.insert(table_name.clone(), table);
                new_table.old_name = None;
                found = true;
            }
        }
        found
    } {}
    for (table_name, new_table) in &new_tables {
        if new_table.skip_ddl {
            continue;
        }
        if let Some(old_name) = &new_table.old_name
            && table_name != old_name
        {
            anyhow::bail!("Illegal rename of {} table detected.", table_name);
        }
    }
    for (table_name, new_table) in new_tables.iter_mut() {
        if new_table.skip_ddl {
            continue;
        }
        while {
            let mut found = false;
            if let Some(old_table) = old_tables.get_mut(table_name) {
                if let Some(old_soft_delete) = &new_table.old_soft_delete {
                    let (name, typ) = old_soft_delete;
                    match *typ {
                        SoftDelete::Time => {
                            writeln!(
                                &mut result1,
                                "DELETE FROM {} WHERE {} IS NOT NULL;",
                                &escape_db_identifier(table_name),
                                &escape_db_identifier(name),
                            )?;
                        }
                        SoftDelete::Flag | SoftDelete::UnixTime => {
                            writeln!(
                                &mut result1,
                                "DELETE FROM {} WHERE {} <> 0;",
                                &escape_db_identifier(table_name),
                                &escape_db_identifier(name),
                            )?;
                        }
                        SoftDelete::None => {}
                    }
                }
                for (name, new_field) in new_table.columns.iter_mut() {
                    if let Some(old_name) = &new_field.old_name
                        && name != old_name
                        && old_table.columns.contains_key(old_name)
                        && !old_table.columns.contains_key(name)
                    {
                        if !new_field.has_query()
                            && !old_table.columns.get(old_name).unwrap().has_query()
                        {
                            history
                                .entry(Type::RenameColumn)
                                .or_default()
                                .entry(table_name.clone())
                                .or_default()
                                .push(name.clone());
                            writeln!(
                                &mut result1,
                                "ALTER TABLE {} RENAME COLUMN {} TO {};",
                                &escape_db_identifier(table_name),
                                &escape_db_identifier(old_name),
                                &escape_db_identifier(name),
                            )?;
                            let column = old_table.columns.swap_remove(old_name).unwrap();
                            old_table.columns.insert(name.clone(), column);
                        }
                        new_field.old_name = None;
                        found = true;
                    }
                }
            }
            found
        } {}
    }
    for (table_name, old_table) in &old_tables {
        if let Some(new_table) = new_tables.get(table_name) {
            if new_table.skip_ddl {
                continue;
            }
            for (name, constraint) in &new_table.constraints {
                if let Some(old_constraint) = old_table.constraints.get(name)
                    && old_constraint != constraint
                {
                    // fix foreign key constraints
                    if is_mysql_mode() {
                        writeln!(
                            &mut result1,
                            "ALTER TABLE {} DROP FOREIGN KEY {};",
                            &escape_db_identifier(table_name),
                            &escape_db_identifier(name)
                        )?;
                    } else {
                        writeln!(
                            &mut result1,
                            "ALTER TABLE {} DROP CONSTRAINT {};",
                            &escape_db_identifier(table_name),
                            &escape_db_identifier(name)
                        )?;
                    }
                }
            }
            for (name, index) in &new_table.indexes {
                if let Some(old_index) = old_table.indexes.get(name)
                    && old_index != index
                    && !matches!(index, TableKey::Key(_, x) if x.is_empty())
                {
                    // fix indexes
                    if is_mysql_mode() {
                        writeln!(
                            &mut result1,
                            "ALTER TABLE {} DROP INDEX {};",
                            &escape_db_identifier(table_name),
                            &escape_db_identifier(name)
                        )?;
                    } else {
                        writeln!(&mut result1, "DROP INDEX {};", &escape_db_identifier(name))?;
                    }
                }
            }
        }
    }
    for (table_name, new_table) in &new_tables {
        if new_table.skip_ddl {
            continue;
        }
        if let Some(old_table) = old_tables.get(table_name) {
            for name in old_table.constraints.keys() {
                if !new_table.constraints.contains_key(name) {
                    // Delete foreign key constraints
                    history
                        .entry(Type::DropForeign)
                        .or_default()
                        .entry(table_name.clone())
                        .or_default()
                        .push(name.clone());
                    if is_mysql_mode() {
                        writeln!(
                            &mut result1,
                            "ALTER TABLE {} DROP FOREIGN KEY {};",
                            &escape_db_identifier(table_name),
                            &escape_db_identifier(name)
                        )?;
                    } else {
                        writeln!(
                            &mut result1,
                            "ALTER TABLE {} DROP CONSTRAINT {};",
                            &escape_db_identifier(table_name),
                            &escape_db_identifier(name)
                        )?;
                    }
                }
            }
            for name in old_table.indexes.keys() {
                if !new_table.indexes.contains_key(name) {
                    // Delete indexes
                    if !name.starts_with("IDX_FK_") {
                        history
                            .entry(Type::DropIndex)
                            .or_default()
                            .entry(table_name.clone())
                            .or_default()
                            .push(name.clone());
                    }
                    if is_mysql_mode() {
                        writeln!(
                            &mut result1,
                            "ALTER TABLE {} DROP INDEX {};",
                            &escape_db_identifier(table_name),
                            &escape_db_identifier(name)
                        )?;
                    } else {
                        writeln!(&mut result1, "DROP INDEX {};", &escape_db_identifier(name))?;
                    }
                }
            }
        }
    }
    for table_name in old_tables.keys() {
        if !new_tables.contains_key(table_name) {
            // Delete tables
            if table_name.starts_with('_') {
                // To prevent deletion of _sqlx_migrations
                continue;
            }
            if table_name.eq("spatial_ref_sys") {
                continue;
            }
            history
                .entry(Type::DropTable)
                .or_default()
                .entry(table_name.clone())
                .or_default();
            writeln!(
                &mut result1,
                "DROP TABLE {};",
                &escape_db_identifier(table_name)
            )?;
        }
    }
    for (table_name, new_table) in &new_tables {
        if new_table.skip_ddl {
            continue;
        }
        if let Some(old_table) = old_tables.get(table_name) {
            let mut alter_columns = Vec::new();
            let mut after_alter_table = Vec::new();
            if new_table.primary.as_ref().map(|v| &v.1) != old_table.primary.as_ref().map(|v| &v.1)
            {
                // Fix primary keys
                if is_mysql_mode() {
                    alter_columns.push("DROP PRIMARY KEY".to_string());
                } else if let Some(old_primary_key) = old_table.primary.as_ref().map(|v| &v.0) {
                    alter_columns.push(format!("DROP CONSTRAINT {}", old_primary_key));
                }
            }
            for (name, _old_field) in &old_table.columns {
                if !new_table.columns.contains_key(name) {
                    // Delete columns
                    history
                        .entry(Type::DropColumn)
                        .or_default()
                        .entry(table_name.clone())
                        .or_default()
                        .push(name.clone());
                    alter_columns.push(format!("DROP COLUMN {}", &escape_db_identifier(name)));
                }
            }
            let mut pos = " FIRST".to_string();
            for (name, new_field) in &new_table.columns {
                if !CONFIG
                    .read()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .preserve_column_order
                    || !is_mysql_mode()
                {
                    pos = String::new();
                }
                if let Some(old_field) = old_table.columns.get(name) {
                    if new_field != old_field {
                        // fix columns
                        history
                            .entry(Type::ChangeColumn)
                            .or_default()
                            .entry(table_name.clone())
                            .or_default()
                            .push(name.clone());
                        if is_mysql_mode() {
                            alter_columns.push(format!(
                                "CHANGE COLUMN {} {} {}{}",
                                &escape_db_identifier(name),
                                &escape_db_identifier(name),
                                new_field,
                                &pos
                            ));
                        } else {
                            if !new_field.is_same_type(old_field) {
                                alter_columns.push(format!(
                                    "ALTER COLUMN {} TYPE {:.0}",
                                    &escape_db_identifier(name),
                                    new_field,
                                ));
                            }
                            if old_field.constraint.not_null && !new_field.constraint.not_null {
                                alter_columns.push(format!(
                                    "ALTER COLUMN {} DROP NOT NULL",
                                    &escape_db_identifier(name),
                                ));
                            }
                            if !old_field.constraint.not_null && new_field.constraint.not_null {
                                alter_columns.push(format!(
                                    "ALTER COLUMN {} SET NOT NULL",
                                    &escape_db_identifier(name),
                                ));
                            }
                            if old_field.constraint.auto_increment
                                && !new_field.constraint.auto_increment
                            {
                                alter_columns.push(format!(
                                    "ALTER COLUMN {} DROP DEFAULT",
                                    &escape_db_identifier(name),
                                ));
                            }
                            if !old_field.constraint.auto_increment
                                && new_field.constraint.auto_increment
                            {
                                alter_columns.push(format!(
                                    "ALTER COLUMN {} SET DEFAULT nextval('{}_{}_seq'::regclass)",
                                    &escape_db_identifier(name),
                                    table_name,
                                    name
                                ));
                            }
                            if old_field.default.is_some() && new_field.default.is_none() {
                                alter_columns.push(format!(
                                    "ALTER COLUMN {} DROP DEFAULT",
                                    &escape_db_identifier(name),
                                ));
                            } else if old_field.default != new_field.default {
                                alter_columns.push(format!(
                                    "ALTER COLUMN {} SET DEFAULT {}",
                                    &escape_db_identifier(name),
                                    &new_field
                                        .default
                                        .as_ref()
                                        .map(|v| v.to_string())
                                        .unwrap_or_default()
                                ));
                            }
                            if old_field.comment != new_field.comment {
                                after_alter_table.push(format!(
                                    "COMMENT ON COLUMN {}.{} IS '{}';",
                                    &escape_db_identifier(table_name),
                                    &escape_db_identifier(name),
                                    &new_field
                                        .comment
                                        .as_ref()
                                        .map(|v| v.replace('\'', "''"))
                                        .unwrap_or_default()
                                ));
                            }
                        }
                    }
                } else {
                    // add columns
                    history
                        .entry(Type::AddColumn)
                        .or_default()
                        .entry(table_name.clone())
                        .or_default()
                        .push(name.clone());
                    alter_columns.push(format!(
                        "ADD COLUMN {} {}{}",
                        &escape_db_identifier(name),
                        new_field,
                        &pos
                    ));
                    if !is_mysql_mode() && new_field.comment.is_some() {
                        after_alter_table.push(format!(
                            "COMMENT ON COLUMN {}.{} IS '{}';",
                            &escape_db_identifier(table_name),
                            &escape_db_identifier(name),
                            &new_field
                                .comment
                                .as_ref()
                                .map(|v| v.replace('\'', "''"))
                                .unwrap_or_default()
                        ));
                    }
                }
                pos = format!(" AFTER {}", &escape_db_identifier(name));
            }
            if new_table.primary.as_ref().map(|v| &v.1) != old_table.primary.as_ref().map(|v| &v.1)
            {
                // Fix primary keys
                if let Some(ref primary) = new_table.primary {
                    history
                        .entry(Type::ChangePrimary)
                        .or_default()
                        .entry(table_name.clone())
                        .or_default();
                    alter_columns.push(format!("ADD {}", primary.1));
                } else {
                    history
                        .entry(Type::DropPrimary)
                        .or_default()
                        .entry(table_name.clone())
                        .or_default();
                }
            }
            if let Some(engine) = &new_table.engine
                && old_table
                    .engine
                    .as_ref()
                    .map(|v| !v.eq_ignore_ascii_case(engine))
                    .unwrap_or(true)
                && is_mysql_mode()
            {
                alter_columns.push(format!("ENGINE={}", engine));
            }
            if !alter_columns.is_empty() {
                writeln!(
                    &mut result1,
                    "ALTER TABLE {} {};",
                    &escape_db_identifier(table_name),
                    &alter_columns.join(", ")
                )?;
            }
            for line in after_alter_table {
                result1.push_str(&line);
            }
        } else {
            // add tables
            if table_name.starts_with('_') {
                // To prevent adding _sqlx_migrations
                continue;
            }
            history
                .entry(Type::AddTable)
                .or_default()
                .entry(table_name.clone())
                .or_default();
            writeln!(&mut result1, "{}", &new_table)?;
            if !is_mysql_mode() {
                for (name, new_field) in &new_table.columns {
                    if let Some(comment) = &new_field.comment {
                        writeln!(
                            &mut result1,
                            "COMMENT ON COLUMN {}.{} IS '{}';",
                            &escape_db_identifier(table_name),
                            &escape_db_identifier(name),
                            &comment.replace('\'', "''")
                        )?;
                    }
                }
                for (_name, index) in &new_table.indexes {
                    if let TableKey::Key(index_name, cols) = index
                        && !cols.is_empty()
                    {
                        writeln!(
                            &mut result1,
                            "CREATE INDEX {} ON {} ({});",
                            &escape_db_identifier(index_name),
                            &escape_db_identifier(table_name),
                            cols.iter()
                                .map(|v| v.to_string())
                                .collect::<Vec<_>>()
                                .join(",")
                        )?;
                    }
                    if let TableKey::UniqueKey(index_name, cols) = index {
                        writeln!(
                            &mut result1,
                            "CREATE UNIQUE INDEX {} ON {} ({});",
                            &escape_db_identifier(index_name),
                            &escape_db_identifier(table_name),
                            cols.iter()
                                .map(|v| v.to_string())
                                .collect::<Vec<_>>()
                                .join(",")
                        )?;
                    }
                    if let TableKey::GeometryKey(index_name, cols) = index {
                        writeln!(
                            &mut result1,
                            "CREATE INDEX {} ON {} USING GIST ({});",
                            &escape_db_identifier(index_name),
                            &escape_db_identifier(table_name),
                            cols.iter()
                                .map(|v| v.to_string())
                                .collect::<Vec<_>>()
                                .join(",")
                        )?;
                    }
                }
            }
        }
    }
    for (table_name, old_table) in &old_tables {
        if let Some(new_table) = new_tables.get(table_name) {
            if new_table.skip_ddl {
                continue;
            }
            for (name, index) in &new_table.indexes {
                let mut add_index = || {
                    if is_mysql_mode() {
                        writeln!(
                            &mut result1,
                            "ALTER TABLE {} ADD {};",
                            &escape_db_identifier(table_name),
                            index
                        )?;
                    } else {
                        if let TableKey::Key(index_name, cols) = index {
                            result2.push(format!(
                                "CREATE INDEX CONCURRENTLY {} ON {} ({});",
                                &escape_db_identifier(index_name),
                                &escape_db_identifier(table_name),
                                cols.iter()
                                    .map(|v| v.to_string())
                                    .collect::<Vec<_>>()
                                    .join(",")
                            ));
                        }
                        if let TableKey::UniqueKey(index_name, cols) = index {
                            result2.push(format!(
                                "CREATE UNIQUE INDEX CONCURRENTLY {} ON {} ({});",
                                &escape_db_identifier(index_name),
                                &escape_db_identifier(table_name),
                                cols.iter()
                                    .map(|v| v.to_string())
                                    .collect::<Vec<_>>()
                                    .join(",")
                            ));
                        }
                        if let TableKey::GeometryKey(index_name, cols) = index {
                            result2.push(format!(
                                "CREATE INDEX CONCURRENTLY {} ON {} USING GIST ({});",
                                &escape_db_identifier(index_name),
                                &escape_db_identifier(table_name),
                                cols.iter()
                                    .map(|v| v.to_string())
                                    .collect::<Vec<_>>()
                                    .join(",")
                            ));
                        }
                    }
                    Ok::<_, anyhow::Error>(())
                };
                if let Some(old_index) = old_table.indexes.get(name) {
                    if old_index != index && !matches!(index, TableKey::Key(_, x) if x.is_empty()) {
                        // fix indexes
                        if !name.starts_with("IDX_FK_") {
                            history
                                .entry(Type::ChangeIndex)
                                .or_default()
                                .entry(table_name.clone())
                                .or_default()
                                .push(name.clone());
                        }
                        add_index()?;
                    }
                } else if !matches!(index, TableKey::Key(_, x) if x.is_empty()) {
                    // add indexes
                    if !name.starts_with("IDX_FK_") {
                        history
                            .entry(Type::AddIndex)
                            .or_default()
                            .entry(table_name.clone())
                            .or_default()
                            .push(name.clone());
                    }
                    add_index()?;
                }
            }
            for (name, constraint) in &new_table.constraints {
                if let Some(old_constraint) = old_table.constraints.get(name) {
                    if old_constraint != constraint {
                        // fix foreign key constraints
                        history
                            .entry(Type::ChangeForeign)
                            .or_default()
                            .entry(table_name.clone())
                            .or_default()
                            .push(name.clone());
                        writeln!(
                            &mut result1,
                            "ALTER TABLE {} ADD {};",
                            &escape_db_identifier(table_name),
                            constraint
                        )?;
                    }
                } else {
                    // Add foreign key constraints
                    history
                        .entry(Type::AddForeign)
                        .or_default()
                        .entry(table_name.clone())
                        .or_default()
                        .push(name.clone());
                    writeln!(
                        &mut result1,
                        "ALTER TABLE {} ADD {};",
                        &escape_db_identifier(table_name),
                        constraint
                    )?;
                }
            }
        }
    }
    for (table_name, new_table) in &new_tables {
        if new_table.skip_ddl {
            continue;
        }
        if !old_tables.contains_key(table_name) {
            // Add foreign key constraints when adding table
            for constraint in new_table.constraints.values() {
                writeln!(
                    &mut result1,
                    "ALTER TABLE {} ADD {};",
                    &escape_db_identifier(table_name),
                    constraint
                )?;
            }
        }
    }
    let mut buf = String::new();
    if !is_mysql_mode()
        && result1.is_empty()
        && let Some(first) = result2.pop()
    {
        if !cli_mode {
            buf.push_str("-- no-transaction\n");
        }
        result1.push_str(&first);
    }
    if !cli_mode {
        for ddl in result2.iter_mut() {
            ddl.insert_str(0, "-- no-transaction\n");
        }
        for (typ, tables) in history {
            for (table, columns) in tables {
                let columns = columns.join(", ");
                writeln!(&mut buf, "-- [{typ}:{table}:{columns}]")?;
            }
        }
    }
    if !result1.is_empty() {
        if is_mysql_mode() {
            buf.push_str("SET foreign_key_checks = 0;\n");
            buf.push_str(&result1);
            buf.push_str("SET foreign_key_checks = 1;\n");
        } else {
            if result1.contains("geography(") {
                buf.push_str("CREATE EXTENSION IF NOT EXISTS postgis;\n");
            }
            buf.push_str(&result1);
        }
    }
    Ok((buf, result2))
}
