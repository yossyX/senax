use anyhow::Result;
use indexmap::IndexMap;
use regex::Regex;
use std::collections::HashMap;

use crate::ddl::parser::common::{ReferenceOption, SqlType};
use crate::schema::{DbEnumValue, IndexFieldDef, IndexType, Parser, RelDef, RelationsType};
use crate::{
    ddl::{parser::common::TableKey, table::parse},
    schema::{self, ColumnDef, ColumnType, ColumnTypeOrDef, IndexDef, ModelDef},
};

pub async fn generate(db_url: &str) -> Result<()> {
    let tables = parse(db_url).await?;
    let re1 = Regex::new(r"`(.+)`").unwrap();
    let re2 = Regex::new(r"(?i)CHAR\((\d+)\)").unwrap();
    let mut defs: IndexMap<String, ModelDef> = IndexMap::new();
    let mut many_relations = HashMap::new();
    for (table_name, table) in &tables {
        let mut model = ModelDef::default();
        let mut pk = Vec::new();
        let mut indexes = IndexMap::new();
        let mut relations = IndexMap::new();
        if let Some(primary) = &table.primary {
            match primary {
                TableKey::PrimaryKey(cols) => {
                    pk = cols.iter().map(|c| c.name.clone()).collect();
                }
                _ => unimplemented!(),
            }
        }
        for index in table.indexes.values() {
            match index {
                TableKey::UniqueKey(name, cols) => {
                    let def = IndexDef {
                        fields: cols.iter().fold(IndexMap::new(), |mut map, col| {
                            map.insert(
                                col.name.clone(),
                                col.len.map(|v| IndexFieldDef {
                                    sorting: None,
                                    length: Some(v as u32),
                                }),
                            );
                            map
                        }),
                        type_def: Some(IndexType::Unique),
                        parser: None,
                    };
                    let name = name.trim_start_matches("UQ_").to_string();
                    indexes.insert(name, Some(def));
                }
                TableKey::FulltextKey(name, cols, parser) => {
                    let def = IndexDef {
                        fields: cols.iter().fold(IndexMap::new(), |mut map, col| {
                            map.insert(
                                col.name.clone(),
                                col.len.map(|v| IndexFieldDef {
                                    sorting: None,
                                    length: Some(v as u32),
                                }),
                            );
                            map
                        }),
                        type_def: Some(IndexType::Fulltext),
                        parser: parser.as_ref().map(Parser::from),
                    };
                    let name = name.trim_start_matches("FT_").to_string();
                    indexes.insert(name, Some(def));
                }
                TableKey::Key(name, cols) => {
                    let def = IndexDef {
                        fields: cols.iter().fold(IndexMap::new(), |mut map, col| {
                            if let Some(ref query) = col.query {
                                if let Some(caps) = re1.captures(query) {
                                    let name = caps.get(1).unwrap().as_str().to_string();
                                    let length = re2.captures(query).and_then(|v| {
                                        v.get(1).unwrap().as_str().parse::<u32>().ok()
                                    });
                                    map.insert(
                                        name,
                                        length.map(|v| IndexFieldDef {
                                            sorting: None,
                                            length: Some(v),
                                        }),
                                    );
                                }
                            } else {
                                map.insert(
                                    col.name.clone(),
                                    col.len.map(|v| IndexFieldDef {
                                        sorting: None,
                                        length: Some(v as u32),
                                    }),
                                );
                            }
                            map
                        }),
                        type_def: None,
                        parser: None,
                    };
                    let name = name.trim_start_matches("IDX_").to_string();
                    indexes.insert(name, Some(def));
                }
                TableKey::SpatialKey(name, cols) => {
                    let def = IndexDef {
                        fields: cols.iter().fold(IndexMap::new(), |mut map, col| {
                            map.insert(
                                col.name.clone(),
                                col.len.map(|v| IndexFieldDef {
                                    sorting: None,
                                    length: Some(v as u32),
                                }),
                            );
                            map
                        }),
                        type_def: Some(IndexType::Spatial),
                        parser: None,
                    };
                    let name = name.trim_start_matches("SP_").to_string();
                    indexes.insert(name, Some(def));
                }
                _ => unimplemented!(),
            }
        }
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
                    let mut def = RelDef {
                        model: Some(table.clone()),
                        ..Default::default()
                    };
                    if columns.len() == 1 {
                        let col = columns.get(0).unwrap();
                        let name = col.name.trim_end_matches("_id").to_string();
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
                        relations.insert(name.clone(), Some(def));

                        if !many_relations.contains_key(table) {
                            many_relations.insert(table.clone(), Vec::new());
                        }
                        let def = RelDef {
                            type_def: Some(RelationsType::Many),
                            model: Some(table_name.clone()),
                            foreign: Some(col.name.clone()),
                            ..Default::default()
                        };
                        many_relations.get_mut(table).unwrap().push(def);
                    }
                }
                _ => unimplemented!(),
            }
        }
        model.indexes = indexes;
        model.relations = relations;
        for (name, column) in &table.columns {
            let mut col = ColumnDef::default();
            match column.sql_type {
                SqlType::Bool => {
                    col.type_def = ColumnType::Boolean;
                }
                SqlType::Char(len) => {
                    col.type_def = ColumnType::Varchar;
                    col.length = Some(len as u32);
                }
                SqlType::Varchar(len) => {
                    col.type_def = ColumnType::Varchar;
                    col.length = Some(len as u32);
                }
                SqlType::Int => {
                    col.type_def = ColumnType::Int;
                    col.signed = true;
                }
                SqlType::UnsignedInt => {
                    col.type_def = ColumnType::Int;
                }
                SqlType::Smallint => {
                    col.type_def = ColumnType::SmallInt;
                    col.signed = true;
                }
                SqlType::UnsignedSmallint => {
                    col.type_def = ColumnType::SmallInt;
                }
                SqlType::Bigint => {
                    col.type_def = ColumnType::BigInt;
                    col.signed = true;
                }
                SqlType::UnsignedBigint => {
                    col.type_def = ColumnType::BigInt;
                }
                SqlType::Tinyint => {
                    col.type_def = ColumnType::TinyInt;
                    col.signed = true;
                }
                SqlType::UnsignedTinyint => {
                    col.type_def = ColumnType::TinyInt;
                }
                SqlType::Blob => {
                    col.type_def = ColumnType::Blob;
                    col.length = Some(65535);
                }
                SqlType::Longblob => {
                    col.type_def = ColumnType::Blob;
                }
                SqlType::Mediumblob => {
                    col.type_def = ColumnType::Blob;
                    col.length = Some(16777215);
                }
                SqlType::Tinyblob => {
                    col.type_def = ColumnType::Blob;
                    col.length = Some(255);
                }
                SqlType::Double => {
                    col.type_def = ColumnType::Double;
                }
                SqlType::Float => {
                    col.type_def = ColumnType::Float;
                }
                SqlType::Real => {
                    col.type_def = ColumnType::Double;
                }
                SqlType::Tinytext => {
                    col.type_def = ColumnType::Text;
                    col.length = Some(255);
                }
                SqlType::Mediumtext => {
                    col.type_def = ColumnType::Text;
                    col.length = Some(16777215);
                }
                SqlType::Longtext => {
                    col.type_def = ColumnType::Text;
                }
                SqlType::Text => {
                    col.type_def = ColumnType::Text;
                    col.length = Some(65535);
                }
                SqlType::Date => {
                    col.type_def = ColumnType::Date;
                }
                SqlType::Time => {
                    col.type_def = ColumnType::Time;
                }
                SqlType::DateTime(precision) => {
                    col.type_def = ColumnType::DateTime;
                    if precision > 0 {
                        col.precision = Some(precision)
                    }
                }
                SqlType::Timestamp(precision) => {
                    col.type_def = ColumnType::Timestamp;
                    if precision > 0 {
                        col.precision = Some(precision)
                    }
                }
                SqlType::Binary(len) => {
                    col.type_def = ColumnType::Blob;
                    col.length = Some(len as u32);
                }
                SqlType::Varbinary(len) => {
                    col.type_def = ColumnType::Blob;
                    col.length = Some(len as u32);
                }
                SqlType::Enum(ref values) => {
                    col.type_def = ColumnType::DbEnum;
                    col.db_enum_values = Some(
                        values
                            .iter()
                            .map(|v| DbEnumValue {
                                name: v.to_raw_string(),
                                title: None,
                                comment: None,
                            })
                            .collect(),
                    );
                }
                SqlType::Set(ref values) => {
                    col.type_def = ColumnType::DbSet;
                    col.db_enum_values = Some(
                        values
                            .iter()
                            .map(|v| DbEnumValue {
                                name: v.to_raw_string(),
                                title: None,
                                comment: None,
                            })
                            .collect(),
                    );
                }
                SqlType::Decimal(precision, scale) => {
                    col.type_def = ColumnType::Decimal;
                    if precision > 0 {
                        col.precision = Some(precision)
                    }
                    if scale > 0 {
                        col.scale = Some(scale)
                    }
                }
                SqlType::Json => {
                    col.type_def = ColumnType::Json;
                }
                SqlType::Point => {
                    col.type_def = ColumnType::Point;
                    col.srid = column.constraint.srid;
                }
            };
            col.not_null = column.constraint.not_null;
            // col.character_set = column.constraint.character_set.clone();
            col.collate = column.constraint.collation.clone();
            col.primary = column.constraint.primary_key || pk.contains(name);
            if column.constraint.auto_increment {
                col.auto_increment = Some(schema::AutoIncrement::Auto);
            }
            col.sql_comment = column.comment.clone();
            model
                .columns
                .insert(name.clone(), ColumnTypeOrDef::Exact(col));
        }

        defs.insert(table_name.clone(), model);
    }
    for (name, relations) in many_relations {
        if let Some(def) = defs.get_mut(&name) {
            for relation in relations {
                let name = relation.model.as_ref().unwrap().clone();
                def.relations.insert(name, Some(relation));
            }
        }
    }
    println!("{}", &serde_yaml::to_string(&defs)?);
    Ok(())
}
