use anyhow::{Context as _, Result};
use chrono::Utc;
use convert_case::{Case, Casing};
use std::collections::HashMap;
use std::env;
use std::fmt::Write;
use std::fs;

use crate::ddl::parser::column;
use crate::ddl::parser::common::{Literal, ReferenceOption, SqlType, TableKey};
use crate::ddl::table::{Column, Constraint, Table};
use crate::schema::{self, AutoIncrement, CONFIG, GROUPS, MODEL, MODELS};
use crate::{ddl, MODELS_PATH};

pub async fn generate(db: &str, description: &Option<String>, revert: bool) -> Result<()> {
    schema::parse(db)?;
    let groups = unsafe { GROUPS.get().unwrap() }.clone();
    let mut new_tables = HashMap::new();
    for (_group_name, defs) in &groups {
        unsafe {
            MODELS.take();
            MODELS.set(defs.clone()).unwrap();
        }
        for (_model_name, def) in defs {
            unsafe {
                MODEL.take();
                MODEL.set(def.clone()).unwrap();
            }
            let table_name = def.table_name();
            if def.has_table() {
                let mut table = Table {
                    name: table_name.clone(),
                    engine: def
                        .engine
                        .clone()
                        .or_else(|| unsafe { CONFIG.get().unwrap() }.engine.clone()),
                    character_set: def.character_set.clone(),
                    collate: def.collate.clone(),
                    ..Default::default()
                };
                for (col_name, col) in &def.merged_columns {
                    let constraint = Constraint {
                        not_null: col.not_null,
                        // character_set: col.character_set.clone()
                        collation: col.collate.clone(),
                        auto_increment: col.auto_increment == Some(AutoIncrement::Auto),
                        // primary_key: col.primary
                        srid: col.srid,
                        ..Default::default()
                    };
                    let sql_type = match col.type_def {
                        schema::ColumnType::TinyInt if col.signed => SqlType::Tinyint,
                        schema::ColumnType::TinyInt => SqlType::UnsignedTinyint,
                        schema::ColumnType::SmallInt if col.signed => SqlType::Smallint,
                        schema::ColumnType::SmallInt => SqlType::UnsignedSmallint,
                        schema::ColumnType::Int if col.signed => SqlType::Int,
                        schema::ColumnType::Int => SqlType::UnsignedInt,
                        schema::ColumnType::BigInt if col.signed => SqlType::Bigint,
                        schema::ColumnType::BigInt => SqlType::UnsignedBigint,
                        schema::ColumnType::Float => SqlType::Float,
                        schema::ColumnType::Double => SqlType::Double,
                        schema::ColumnType::Varchar => {
                            SqlType::Varchar(col.length.unwrap_or(schema::DEFAULT_VARCHAR_LENGTH))
                        }
                        schema::ColumnType::Boolean => SqlType::Tinyint,
                        schema::ColumnType::Text if col.length.unwrap_or(65536) < 256 => {
                            SqlType::Tinytext
                        }
                        schema::ColumnType::Text if col.length.unwrap_or(65536) < 65536 => {
                            SqlType::Text
                        }
                        schema::ColumnType::Text => SqlType::Longtext,
                        schema::ColumnType::Blob if col.length.unwrap_or(65536) < 256 => {
                            SqlType::Tinyblob
                        }
                        schema::ColumnType::Blob if col.length.unwrap_or(65536) < 65536 => {
                            SqlType::Blob
                        }
                        schema::ColumnType::Blob => SqlType::Longblob,
                        schema::ColumnType::Timestamp => {
                            SqlType::Timestamp(col.precision.unwrap_or(0))
                        }
                        schema::ColumnType::DateTime => {
                            SqlType::DateTime(col.precision.unwrap_or(0))
                        }
                        schema::ColumnType::Date => SqlType::Date,
                        schema::ColumnType::Time => SqlType::Time,
                        schema::ColumnType::Decimal => SqlType::Decimal(
                            col.precision.unwrap_or(schema::DEFAULT_PRECISION),
                            col.scale.unwrap_or(schema::DEFAULT_SCALE),
                        ),
                        schema::ColumnType::ArrayInt => SqlType::Json,
                        schema::ColumnType::ArrayString => SqlType::Json,
                        schema::ColumnType::Json => SqlType::Json,
                        schema::ColumnType::Enum => SqlType::UnsignedTinyint,
                        schema::ColumnType::DbEnum => SqlType::Enum(
                            col.db_enum_values
                                .as_ref()
                                .unwrap_or(&Vec::new())
                                .iter()
                                .map(|v| Literal::String(v.name.clone()))
                                .collect(),
                        ),
                        schema::ColumnType::DbSet => SqlType::Set(
                            col.db_enum_values
                                .as_ref()
                                .unwrap_or(&Vec::new())
                                .iter()
                                .map(|v| Literal::String(v.name.clone()))
                                .collect(),
                        ),
                        schema::ColumnType::Point => SqlType::Point,
                        schema::ColumnType::UnSupported => todo!(),
                    };
                    table.columns.insert(
                        col.get_col_name(col_name).to_string(),
                        Column {
                            sql_type,
                            constraint,
                            default: col.default.clone(),
                            comment: col.sql_comment.clone(),
                        },
                    );
                }
                let cols: Vec<column::Column> = def
                    .primaries()
                    .iter()
                    .map(|(n, c)| column::Column {
                        name: c.get_col_name(n).to_string(),
                        query: None,
                        len: None,
                    })
                    .collect();
                if !cols.is_empty() {
                    table.primary = Some(TableKey::PrimaryKey(cols));
                }
                for (index_name, index) in &def.merged_indexes {
                    let cols: Vec<column::Column> = if !index.fields.is_empty() {
                        index
                            .fields
                            .iter()
                            .map(|(n, c)| {
                                let col = def
                                    .merged_columns
                                    .get(n)
                                    .unwrap_or_else(|| panic!("{} is not in columns", n));
                                let name = col.get_col_name(n).to_string();
                                let len = c.as_ref().and_then(|c| c.length);
                                let query = if col.type_def == schema::ColumnType::ArrayInt {
                                    Some(format!("CAST(`{}` AS UNSIGNED ARRAY)", name))
                                } else if col.type_def == schema::ColumnType::ArrayString {
                                    Some(format!(
                                        "CAST(`{}` AS CHAR({}) ARRAY)",
                                        name,
                                        len.unwrap_or(255)
                                    ))
                                } else {
                                    None
                                };
                                column::Column { name, query, len }
                            })
                            .collect()
                    } else {
                        let col = def
                            .merged_columns
                            .get(index_name)
                            .unwrap_or_else(|| panic!("{} is not in columns", index_name));
                        let name = col.get_col_name(index_name).to_string();
                        let query = if col.type_def == schema::ColumnType::ArrayInt {
                            Some(format!("CAST(`{}` AS UNSIGNED ARRAY)", name))
                        } else if col.type_def == schema::ColumnType::ArrayString {
                            Some(format!("CAST(`{}` AS CHAR({}) ARRAY)", name, 255))
                        } else {
                            None
                        };
                        vec![column::Column {
                            name,
                            query,
                            len: None,
                        }]
                    };
                    if index.type_def == Some(schema::IndexType::Unique) {
                        let index_name = format!("UQ_{}", index_name);
                        table
                            .indexes
                            .insert(index_name.clone(), TableKey::UniqueKey(index_name, cols));
                    } else if index.type_def == Some(schema::IndexType::Fulltext) {
                        let index_name = format!("FT_{}", index_name);
                        table.indexes.insert(
                            index_name.clone(),
                            TableKey::FulltextKey(
                                index_name,
                                cols,
                                index.parser.map(|v| v.to_string()),
                            ),
                        );
                    } else if index.type_def == Some(schema::IndexType::Spatial) {
                        let index_name = format!("SP_{}", index_name);
                        table
                            .indexes
                            .insert(index_name.clone(), TableKey::SpatialKey(index_name, cols));
                    } else {
                        let index_name = format!("IDX_{}", index_name);
                        table
                            .indexes
                            .insert(index_name.clone(), TableKey::Key(index_name, cols));
                    }
                }
                for (_model, name, rel) in def.relation_constraint() {
                    let local_id = schema::RelDef::get_local_id(rel, name, &def.id_name());
                    let foreign_table = schema::RelDef::get_foreign_table_name(rel, name);
                    let key_name = format!("FK_{}_{}_{}", &table_name, local_id, foreign_table);
                    let local_col_name = if let Some(local_col) = def.merged_columns.get(&local_id)
                    {
                        local_col.get_col_name(&local_id).to_string()
                    } else {
                        local_id.clone()
                    };
                    let local_cols = vec![column::Column {
                        name: local_col_name,
                        query: None,
                        len: None,
                    }];
                    let foreign = schema::RelDef::get_foreign_model(rel, name);
                    let foreign_primaries = foreign
                        .primaries()
                        .iter()
                        .map(|(n, c)| column::Column {
                            name: c.get_col_name(n).to_string(),
                            query: None,
                            len: None,
                        })
                        .collect();
                    table.constraints.insert(
                        key_name.clone(),
                        TableKey::Constraint(
                            key_name,
                            local_cols,
                            foreign_table,
                            foreign_primaries,
                            rel.as_ref().and_then(|r| ref_op(&r.on_delete)),
                            rel.as_ref().and_then(|r| ref_op(&r.on_update)),
                        ),
                    );
                }
                new_tables.insert(table_name, table);
            }
        }
    }
    let url_name = format!("{}_DB_URL", db.to_case(Case::Upper));
    let db_url =
        env::var(&url_name).with_context(|| format!("{} is not set in the .env file", url_name))?;
    let old_tables = ddl::table::parse(&db_url).await?;
    let ddl = make_ddl(&new_tables, &old_tables)?;
    if let Some(description) = description {
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
        let base_path = MODELS_PATH.get().unwrap().join(&db);
        let ddl_path = base_path.join("migrations");
        fs::create_dir_all(&ddl_path)?;
        let dt = Utc::now();
        let file_prefix = dt.format("%Y%m%d%H%M%S").to_string();
        if !revert {
            let file_path = ddl_path.join(format!("{}_{}.sql", file_prefix, description));
            println!("{}", file_path.display());
            fs::write(file_path, &ddl)?;
        } else {
            let file_path = ddl_path.join(format!("{}_{}.up.sql", file_prefix, description));
            println!("{}", file_path.display());
            fs::write(file_path, &ddl)?;

            let ddl = make_ddl(&old_tables, &new_tables)?;
            let file_path = ddl_path.join(format!("{}_{}.down.sql", file_prefix, description));
            println!("{}", file_path.display());
            fs::write(file_path, &ddl)?;
        }
    } else {
        println!("{}", &ddl);
    }
    Ok(())
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

fn escape(s: &String) -> String {
    format!("`{}`", s)
}

fn make_ddl(
    new_tables: &HashMap<String, Table>,
    old_tables: &HashMap<String, Table>,
) -> Result<String> {
    let mut result = String::new();
    for (table_name, new_table) in new_tables {
        if let Some(old_table) = old_tables.get(table_name) {
            for name in old_table.constraints.keys() {
                if !new_table.constraints.contains_key(name) {
                    // Delete foreign key constraints
                    writeln!(
                        &mut result,
                        "ALTER TABLE {} DROP FOREIGN KEY {};",
                        &escape(table_name),
                        &escape(name)
                    )?;
                }
            }
            for name in old_table.indexes.keys() {
                if !new_table.indexes.contains_key(name) {
                    // Delete indexes
                    writeln!(
                        &mut result,
                        "ALTER TABLE {} DROP INDEX {};",
                        &escape(table_name),
                        &escape(name)
                    )?;
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
            writeln!(&mut result, "DROP TABLES {};", &escape(table_name))?;
        }
    }
    for (table_name, new_table) in new_tables {
        if let Some(old_table) = old_tables.get(table_name) {
            let mut pos = " FIRST".to_string();
            for (name, new_field) in &new_table.columns {
                if !unsafe { CONFIG.get().unwrap() }.preserve_column_order {
                    pos = String::new();
                }
                if let Some(old_field) = old_table.columns.get(name) {
                    if new_field != old_field {
                        // fix columns
                        writeln!(
                            &mut result,
                            "ALTER TABLE {} CHANGE COLUMN {} {} {}{};",
                            &escape(table_name),
                            &escape(name),
                            &escape(name),
                            new_field,
                            &pos
                        )?;
                    }
                } else {
                    // add columns
                    writeln!(
                        &mut result,
                        "ALTER TABLE {} ADD COLUMN {} {}{};",
                        &escape(table_name),
                        &escape(name),
                        new_field,
                        &pos
                    )?;
                }
                pos = format!(" AFTER {}", &escape(name));
            }
            for (name, _old_field) in &old_table.columns {
                if !new_table.columns.contains_key(name) {
                    // Delete columns
                    writeln!(
                        &mut result,
                        "ALTER TABLE {} DROP COLUMN {};",
                        &escape(table_name),
                        &escape(name)
                    )?;
                }
            }
            if new_table.primary != old_table.primary {
                // Fix primary keys
                if let Some(ref primary) = new_table.primary {
                    writeln!(
                        &mut result,
                        "ALTER TABLE {} DROP PRIMARY KEY, ADD {};",
                        &escape(table_name),
                        primary
                    )?;
                } else {
                    writeln!(
                        &mut result,
                        "ALTER TABLE {} DROP PRIMARY KEY;",
                        &escape(table_name)
                    )?;
                }
            }
        } else {
            // add tables
            if table_name.starts_with('_') {
                // To prevent adding _sqlx_migrations
                continue;
            }
            writeln!(&mut result, "{}", &new_table)?;
        }
    }
    for (table_name, old_table) in old_tables {
        if let Some(new_table) = new_tables.get(table_name) {
            for (name, index) in &new_table.indexes {
                if let Some(old_index) = old_table.indexes.get(name) {
                    if old_index != index {
                        // fix indexes
                        writeln!(
                            &mut result,
                            "ALTER TABLE {} DROP INDEX {}, ADD {};",
                            &escape(table_name),
                            &escape(name),
                            index
                        )?;
                    }
                } else {
                    // add indexes
                    writeln!(
                        &mut result,
                        "ALTER TABLE {} ADD {};",
                        &escape(table_name),
                        index
                    )?;
                }
            }
            for (name, constraint) in &new_table.constraints {
                if let Some(old_constraint) = old_table.constraints.get(name) {
                    if old_constraint != constraint {
                        // fix foreign key constraints
                        writeln!(
                            &mut result,
                            "ALTER TABLE {} DROP FOREIGN KEY {}, ADD {};",
                            &escape(table_name),
                            &escape(name),
                            constraint
                        )?;
                    }
                } else {
                    // Add foreign key constraints
                    writeln!(
                        &mut result,
                        "ALTER TABLE {} ADD {};",
                        &escape(table_name),
                        constraint
                    )?;
                }
            }
        }
    }
    for (table_name, new_table) in new_tables {
        if !old_tables.contains_key(table_name) {
            // Add foreign key constraints when adding table
            for constraint in new_table.constraints.values() {
                writeln!(
                    &mut result,
                    "ALTER TABLE {} ADD {};",
                    &escape(table_name),
                    constraint
                )?;
            }
        }
    }
    Ok(result)
}
