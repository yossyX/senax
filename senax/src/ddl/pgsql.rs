use anyhow::{Result, bail};
use indexmap::IndexMap;
use senax_pgsql_parser::{ForeignKeyAction, PostgreSQLDataType};

use crate::ddl::sql_type::{IndexColumn, Literal, ReferenceOption, SqlType, TableKey};
use crate::ddl::table::{Column, Constraint, Table};

pub async fn get_pgsql_table_def_map(database_url: &str) -> Result<IndexMap<String, Table>> {
    let pool = senax_pgsql_parser::connect_to_database(database_url).await?;
    let schema = senax_pgsql_parser::get_database_schema(&pool).await?;

    let mut result = IndexMap::new();
    for table in schema.tables {
        let def = convert_table(&table)?;
        result.insert(table.table_name, def);
    }
    Ok(result)
}

fn convert_table(org_table: &senax_pgsql_parser::TableInfo) -> Result<Table> {
    let table = Table {
        name: org_table.table_name.clone(),
        old_name: None,
        old_soft_delete: None,
        columns: convert_columns(&org_table.columns)?,
        primary: convert_primary(&org_table.indexes),
        indexes: convert_indexes(&org_table.indexes),
        constraints: convert_constraints(&org_table.foreign_keys),
        comment: org_table.comment.clone(),
        engine: None,
        skip_ddl: false,
    };
    Ok(table)
}

fn convert_columns(org: &[senax_pgsql_parser::ColumnInfo]) -> Result<IndexMap<String, Column>> {
    let mut columns = IndexMap::new();
    for col in org {
        let (sql_type, alt_type) = convert_type(&col.data_type)?;
        let default = col.column_default.clone().and_then(|v| match v {
            senax_pgsql_parser::DefaultValue::String(v) => Some(Literal::String(v)),
            senax_pgsql_parser::DefaultValue::Integer(v) => Some(Literal::Integer(v)),
            senax_pgsql_parser::DefaultValue::Float(v) => Some(Literal::String(v.to_string())),
            senax_pgsql_parser::DefaultValue::Boolean(v) => Some(Literal::Boolean(v)),
            senax_pgsql_parser::DefaultValue::Null => Some(Literal::Null),
            senax_pgsql_parser::DefaultValue::CurrentTimestamp => Some(Literal::CurrentTimestamp),
            senax_pgsql_parser::DefaultValue::CurrentDate => Some(Literal::CurrentDate),
            senax_pgsql_parser::DefaultValue::CurrentTime => Some(Literal::CurrentTime),
            senax_pgsql_parser::DefaultValue::Expression(_) => None,
            senax_pgsql_parser::DefaultValue::Binary(_) => None,
        });
        let constraint = Constraint {
            not_null: !col.is_nullable,
            collation: col.collate.clone(),
            // default_value: default.clone(),
            auto_increment: is_auto_increment(&col.data_type),
            // primary_key: col.is_primary_key,
            // unique: false,
            srid: srid(&col.data_type),
            query: None,
        };
        let column = Column {
            old_name: None,
            sql_type: sql_type.clone(),
            alt_type: alt_type.unwrap_or(sql_type),
            constraint,
            default,
            comment: col.comment.clone(),
        };
        columns.insert(col.column_name.clone(), column);
    }
    Ok(columns)
}

fn convert_primary(org: &[senax_pgsql_parser::IndexInfo]) -> Option<(String, TableKey)> {
    for index in org {
        if index.is_primary {
            let cols: Vec<IndexColumn> = index
                .columns
                .iter()
                .map(|v| IndexColumn {
                    name: v.clone(),
                    ..Default::default()
                })
                .collect();
            return Some((index.index_name.clone(), TableKey::PrimaryKey(cols)));
        }
    }
    None
}

fn convert_indexes(org: &[senax_pgsql_parser::IndexInfo]) -> IndexMap<String, TableKey> {
    let mut indexes = IndexMap::new();
    for index in org {
        if senax_pgsql_parser::is_system_generated_index(index) {
            continue;
        }
        let cols: Vec<IndexColumn> = index
            .columns
            .iter()
            .map(|v| IndexColumn {
                name: v.clone(),
                ..Default::default()
            })
            .collect();

        if index.is_unique {
            indexes.insert(
                index.index_name.clone(),
                TableKey::UniqueKey(index.index_name.clone(), cols),
            );
        } else {
            indexes.insert(
                index.index_name.clone(),
                TableKey::Key(index.index_name.clone(), cols),
            );
        }
    }
    indexes
}

fn convert_constraints(
    org: &[senax_pgsql_parser::ForeignKeyConstraint],
) -> IndexMap<String, TableKey> {
    let mut constraints = IndexMap::new();
    for fk in org {
        constraints.insert(
            fk.constraint_name.clone(),
            TableKey::Constraint(
                fk.constraint_name.clone(),
                fk.columns
                    .iter()
                    .map(|v| IndexColumn {
                        name: v.clone(),
                        ..Default::default()
                    })
                    .collect(),
                fk.referenced_table.clone(),
                fk.referenced_columns
                    .iter()
                    .map(|v| IndexColumn {
                        name: v.clone(),
                        ..Default::default()
                    })
                    .collect(),
                fk.on_delete.clone().map(|v| v.into()),
                fk.on_update.clone().map(|v| v.into()),
            ),
        );
    }
    constraints
}

fn convert_type(typ: &PostgreSQLDataType) -> Result<(SqlType, Option<SqlType>)> {
    Ok(match typ {
        PostgreSQLDataType::SmallInt => (SqlType::Smallint, Some(SqlType::UnsignedSmallint)),
        PostgreSQLDataType::Integer => (SqlType::Int, Some(SqlType::UnsignedInt)),
        PostgreSQLDataType::BigInt => (SqlType::Bigint, Some(SqlType::UnsignedBigint)),
        PostgreSQLDataType::SmallSerial => (SqlType::Smallint, Some(SqlType::UnsignedSmallint)),
        PostgreSQLDataType::Serial => (SqlType::Int, Some(SqlType::UnsignedInt)),
        PostgreSQLDataType::BigSerial => (SqlType::Bigint, Some(SqlType::UnsignedBigint)),
        PostgreSQLDataType::Real => (SqlType::Float, None),
        PostgreSQLDataType::DoublePrecision => (SqlType::Double, None),
        PostgreSQLDataType::Numeric(precision, scale) => (
            SqlType::Decimal(
                precision.unwrap_or_default() as u16,
                scale.unwrap_or_default() as u16,
            ),
            None,
        ),
        PostgreSQLDataType::Decimal(precision, scale) => (
            SqlType::Decimal(
                precision.unwrap_or_default() as u16,
                scale.unwrap_or_default() as u16,
            ),
            None,
        ),
        PostgreSQLDataType::Char(len) => (SqlType::Char(len.unwrap_or_default() as u32), None),
        PostgreSQLDataType::Varchar(len) => {
            (SqlType::Varchar(len.unwrap_or_default() as u32), None)
        }
        PostgreSQLDataType::Text => (SqlType::Text, None),
        PostgreSQLDataType::Bytea => (SqlType::Varbinary(0), None),
        PostgreSQLDataType::Date => (SqlType::Date, None),
        PostgreSQLDataType::Time => (SqlType::Time, None),
        PostgreSQLDataType::TimeWithTimeZone => bail!("unsupported PostgreSQL TimeWithTimeZone"),
        PostgreSQLDataType::Timestamp => (SqlType::DateTime(0), None),
        PostgreSQLDataType::TimestampWithTimeZone => (SqlType::Timestamp(0), None),
        PostgreSQLDataType::Interval => bail!("PostgreSQL Interval"),
        PostgreSQLDataType::Boolean => (SqlType::Bool, None),
        PostgreSQLDataType::Uuid => (SqlType::Uuid, None),
        PostgreSQLDataType::Json => (SqlType::Json, None),
        PostgreSQLDataType::Jsonb => (SqlType::Json, None),
        PostgreSQLDataType::Array(_postgre_sqldata_type) => bail!("unsupported PostgreSQL Array"),
        PostgreSQLDataType::Geometry(info) => {
            if let Some(info) = info {
                if info.geometry_type == senax_pgsql_parser::GeometryType::Point {
                    (SqlType::Point, None)
                } else {
                    (SqlType::Geometry, None)
                }
            } else {
                bail!("unsupported PostgreSQL Geometry");
            }
        }
        PostgreSQLDataType::Geography(info) => {
            if let Some(info) = info {
                if info.geometry_type == senax_pgsql_parser::GeometryType::Point {
                    (SqlType::Point, None)
                } else {
                    (SqlType::Geometry, None)
                }
            } else {
                bail!("unsupported PostgreSQL Geography");
            }
        }
        PostgreSQLDataType::Custom(v) => bail!("unsupported PostgreSQL Custom: {}", v),
        PostgreSQLDataType::Unknown(v) => bail!("unsupported PostgreSQL Unknown: {}", v),
    })
}

fn srid(typ: &PostgreSQLDataType) -> Option<u32> {
    match typ {
        PostgreSQLDataType::Geography(geometry_info) => geometry_info
            .as_ref()
            .and_then(|v| v.srid)
            .map(|v| v as u32),
        _ => None,
    }
}

#[allow(clippy::match_like_matches_macro)]
fn is_auto_increment(typ: &PostgreSQLDataType) -> bool {
    match typ {
        PostgreSQLDataType::SmallSerial => true,
        PostgreSQLDataType::Serial => true,
        PostgreSQLDataType::BigSerial => true,
        _ => false,
    }
}

impl From<ForeignKeyAction> for ReferenceOption {
    fn from(value: ForeignKeyAction) -> Self {
        match value {
            ForeignKeyAction::Restrict => ReferenceOption::Restrict,
            ForeignKeyAction::Cascade => ReferenceOption::Cascade,
            ForeignKeyAction::SetNull => ReferenceOption::SetNull,
            ForeignKeyAction::NoAction => ReferenceOption::NoAction,
            ForeignKeyAction::SetDefault => ReferenceOption::SetDefault,
        }
    }
}
