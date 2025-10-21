use serde::Deserialize;
use serde::Serialize;
use std::fmt;
use std::str;

use senax_mysql_parser::column::Column as MysqlColumn;
use senax_mysql_parser::common::Literal as MysqlLiteral;
use senax_mysql_parser::common::ReferenceOption as MysqlReferenceOption;
use senax_mysql_parser::common::SqlType as MysqlSqlType;
use senax_mysql_parser::common::TableKey as MysqlTableKey;

use crate::common::escape_db_identifier;
use crate::ddl::table::mysql_escape;
use crate::schema::is_mysql_mode;

#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum SqlType {
    Bool,
    Char(u32),
    Varchar(u32),
    Int,
    UnsignedInt,
    Smallint,
    UnsignedSmallint,
    Bigint,
    UnsignedBigint,
    Tinyint,
    UnsignedTinyint,
    Blob,
    Longblob,
    Mediumblob,
    Tinyblob,
    Double,
    Float,
    Real,
    Tinytext,
    Mediumtext,
    Longtext,
    Text,
    Date,
    Time,
    DateTime(u16),
    Timestamp(u16),
    Binary(u16),
    Varbinary(u16),
    Enum(Vec<Literal>),
    Set(Vec<Literal>),
    Decimal(u16, u16),
    Json,
    Point,
    Geometry,
    UnSupported,
    // PostgreSQL
    Uuid,
}

impl SqlType {
    pub fn write(
        &self,
        f: &mut fmt::Formatter,
        constraint: &super::table::Constraint,
    ) -> fmt::Result {
        if is_mysql_mode() {
            let auto_inc = if constraint.auto_increment {
                " AUTO_INCREMENT"
            } else {
                ""
            };
            let collation = if let Some(ref c) = constraint.collation {
                format!(" COLLATE {}", c)
            } else {
                "".to_owned()
            };
            let srid = if let Some(srid) = constraint.srid {
                format!(" /*!80003 SRID {} */", srid)
            } else {
                "".to_owned()
            };
            match *self {
                SqlType::Bool => write!(f, "BOOL"),
                SqlType::Char(len) => write!(f, "CHAR({}){collation}", len),
                SqlType::Varchar(len) => write!(f, "VARCHAR({}){collation}", len),
                SqlType::Int => write!(f, "INT{auto_inc}"),
                SqlType::UnsignedInt => write!(f, "INT UNSIGNED{auto_inc}"),
                SqlType::Smallint => write!(f, "SMALLINT{auto_inc}"),
                SqlType::UnsignedSmallint => write!(f, "SMALLINT UNSIGNED{auto_inc}"),
                SqlType::Bigint => write!(f, "BIGINT{auto_inc}"),
                SqlType::UnsignedBigint => write!(f, "BIGINT UNSIGNED{auto_inc}"),
                SqlType::Tinyint => write!(f, "TINYINT{auto_inc}"),
                SqlType::UnsignedTinyint => write!(f, "TINYINT UNSIGNED{auto_inc}"),
                SqlType::Blob => write!(f, "BLOB"),
                SqlType::Longblob => write!(f, "LONGBLOB"),
                SqlType::Mediumblob => write!(f, "MEDIUMBLOB"),
                SqlType::Tinyblob => write!(f, "TINYBLOB"),
                SqlType::Double => write!(f, "DOUBLE"),
                SqlType::Float => write!(f, "FLOAT"),
                SqlType::Real => write!(f, "REAL"),
                SqlType::Tinytext => write!(f, "TINYTEXT{collation}"),
                SqlType::Mediumtext => write!(f, "MEDIUMTEXT{collation}"),
                SqlType::Longtext => write!(f, "LONGTEXT{collation}"),
                SqlType::Text => write!(f, "TEXT{collation}"),
                SqlType::Date => write!(f, "DATE"),
                SqlType::Time => write!(f, "TIME"),
                SqlType::DateTime(len) => {
                    if len > 0 {
                        write!(f, "DATETIME({})", len)
                    } else {
                        write!(f, "DATETIME")
                    }
                }
                SqlType::Timestamp(len) => {
                    if len > 0 {
                        write!(f, "TIMESTAMP({})", len)
                    } else {
                        write!(f, "TIMESTAMP")
                    }
                }
                SqlType::Binary(len) => write!(f, "BINARY({})", len),
                SqlType::Varbinary(len) => write!(f, "VARBINARY({})", len),
                SqlType::Enum(ref v) => write!(
                    f,
                    "ENUM({})",
                    v.iter()
                        .map(|v| v.to_string())
                        .collect::<Vec<String>>()
                        .join(",")
                ),
                SqlType::Set(ref v) => write!(
                    f,
                    "SET({})",
                    v.iter()
                        .map(|v| v.to_string())
                        .collect::<Vec<String>>()
                        .join(",")
                ),
                SqlType::Decimal(m, d) => write!(f, "DECIMAL({}, {})", m, d),
                SqlType::Json => write!(f, "JSON"),
                SqlType::Point => write!(f, "POINT{srid}"),
                SqlType::Geometry => write!(f, "GEOMETRY{srid}"),
                SqlType::UnSupported => unimplemented!("SqlType::UnSupported"),
                SqlType::Uuid => unimplemented!("SqlType::Uuid"),
            }
        } else {
            let collation = if let Some(ref c) = constraint.collation {
                format!(" COLLATE \"{}\"", c)
            } else {
                "".to_owned()
            };
            let srid = if let Some(srid) = constraint.srid {
                format!(", {}", srid)
            } else {
                "".to_owned()
            };
            match *self {
                SqlType::Bool => write!(f, "boolean"),
                SqlType::Char(0) => write!(f, "char{collation}"),
                SqlType::Char(len) => write!(f, "char({}){collation}", len),
                SqlType::Varchar(0) => write!(f, "varchar{collation}"),
                SqlType::Varchar(len) => write!(f, "varchar({}){collation}", len),
                SqlType::Int if constraint.auto_increment => write!(f, "serial"),
                SqlType::Int => write!(f, "integer"),
                SqlType::UnsignedInt if constraint.auto_increment => write!(f, "serial"),
                SqlType::UnsignedInt => write!(f, "integer"),
                SqlType::Smallint if constraint.auto_increment => write!(f, "smallserial"),
                SqlType::Smallint => write!(f, "smallint"),
                SqlType::UnsignedSmallint if constraint.auto_increment => write!(f, "smallserial"),
                SqlType::UnsignedSmallint => write!(f, "smallint"),
                SqlType::Bigint if constraint.auto_increment => write!(f, "bigserial"),
                SqlType::Bigint => write!(f, "bigint"),
                SqlType::UnsignedBigint if constraint.auto_increment => write!(f, "bigserial"),
                SqlType::UnsignedBigint => write!(f, "bigint"),
                SqlType::Tinyint => write!(f, "smallint"),
                SqlType::UnsignedTinyint => write!(f, "smallint"),
                SqlType::Blob => write!(f, "bytea"),
                SqlType::Longblob => write!(f, "bytea"),
                SqlType::Mediumblob => write!(f, "bytea"),
                SqlType::Tinyblob => write!(f, "bytea"),
                SqlType::Double => write!(f, "double precision"),
                SqlType::Float => write!(f, "real"),
                SqlType::Real => write!(f, "real"),
                SqlType::Tinytext => write!(f, "text{collation}"),
                SqlType::Mediumtext => write!(f, "text{collation}"),
                SqlType::Longtext => write!(f, "text{collation}"),
                SqlType::Text => write!(f, "text{collation}"),
                SqlType::Date => write!(f, "date"),
                SqlType::Time => write!(f, "time"),
                SqlType::DateTime(_) => write!(f, "timestamp"),
                SqlType::Timestamp(_) => write!(f, "timestamptz"),
                SqlType::Binary(_) => write!(f, "bytea"),
                SqlType::Varbinary(_) => write!(f, "bytea"),
                SqlType::Enum(ref _v) => unimplemented!("SqlType::Enum"),
                SqlType::Set(ref _v) => unimplemented!("SqlType::Set"),
                SqlType::Decimal(m, d) => write!(f, "numeric({}, {})", m, d),
                SqlType::Json => write!(f, "jsonb"),
                SqlType::Point => write!(f, "geography(POINT{srid})"),
                SqlType::Geometry => write!(f, "geography(GEOMETRYCOLLECTION{srid})"),
                SqlType::UnSupported => unimplemented!("SqlType::UnSupported"),
                SqlType::Uuid => write!(f, "uuid"),
            }
        }
    }
}

impl From<MysqlSqlType> for SqlType {
    fn from(value: MysqlSqlType) -> Self {
        match value {
            MysqlSqlType::Bool => SqlType::Bool,
            MysqlSqlType::Char(v) => SqlType::Char(v),
            MysqlSqlType::Varchar(v) => SqlType::Varchar(v),
            MysqlSqlType::Int => SqlType::Int,
            MysqlSqlType::UnsignedInt => SqlType::UnsignedInt,
            MysqlSqlType::Smallint => SqlType::Smallint,
            MysqlSqlType::UnsignedSmallint => SqlType::UnsignedSmallint,
            MysqlSqlType::Bigint => SqlType::Bigint,
            MysqlSqlType::UnsignedBigint => SqlType::UnsignedBigint,
            MysqlSqlType::Tinyint => SqlType::Tinyint,
            MysqlSqlType::UnsignedTinyint => SqlType::UnsignedTinyint,
            MysqlSqlType::Blob => SqlType::Blob,
            MysqlSqlType::Longblob => SqlType::Longblob,
            MysqlSqlType::Mediumblob => SqlType::Mediumblob,
            MysqlSqlType::Tinyblob => SqlType::Tinyblob,
            MysqlSqlType::Double => SqlType::Double,
            MysqlSqlType::Float => SqlType::Float,
            MysqlSqlType::Real => SqlType::Real,
            MysqlSqlType::Tinytext => SqlType::Tinytext,
            MysqlSqlType::Mediumtext => SqlType::Mediumtext,
            MysqlSqlType::Longtext => SqlType::Longtext,
            MysqlSqlType::Text => SqlType::Text,
            MysqlSqlType::Date => SqlType::Date,
            MysqlSqlType::Time => SqlType::Time,
            MysqlSqlType::DateTime(v) => SqlType::DateTime(v),
            MysqlSqlType::Timestamp(v) => SqlType::Timestamp(v),
            MysqlSqlType::Binary(v) => SqlType::Binary(v),
            MysqlSqlType::Varbinary(v) => SqlType::Varbinary(v),
            MysqlSqlType::Enum(v) => SqlType::Enum(v.into_iter().map(|v| v.into()).collect()),
            MysqlSqlType::Set(v) => SqlType::Set(v.into_iter().map(|v| v.into()).collect()),
            MysqlSqlType::Decimal(v1, v2) => SqlType::Decimal(v1, v2),
            MysqlSqlType::Json => SqlType::Json,
            MysqlSqlType::Point => SqlType::Point,
            MysqlSqlType::Geometry => SqlType::Geometry,
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum Literal {
    Null,
    Integer(i64),
    UnsignedInteger(u64),
    FixedPoint(i32, i32),
    String(String),
    Blob(Vec<u8>),
    CurrentTime,
    CurrentDate,
    CurrentTimestamp,
    Boolean(bool),
    // Placeholder(ItemPlaceholder),
}
impl From<MysqlLiteral> for Literal {
    fn from(value: MysqlLiteral) -> Self {
        match value {
            MysqlLiteral::Null => Literal::Null,
            MysqlLiteral::Integer(v) => Literal::Integer(v),
            MysqlLiteral::UnsignedInteger(v) => Literal::UnsignedInteger(v),
            MysqlLiteral::FixedPoint(v) => Literal::FixedPoint(v.integral, v.fractional),
            MysqlLiteral::String(v) => Literal::String(v),
            MysqlLiteral::Blob(v) => Literal::Blob(v),
            MysqlLiteral::CurrentTime => Literal::CurrentTime,
            MysqlLiteral::CurrentDate => Literal::CurrentDate,
            MysqlLiteral::CurrentTimestamp => Literal::CurrentTimestamp,
            MysqlLiteral::Placeholder(_) => unimplemented!(),
        }
    }
}
#[allow(clippy::to_string_trait_impl)]
impl ToString for Literal {
    fn to_string(&self) -> String {
        match *self {
            Literal::Null => "NULL".to_string(),
            Literal::Integer(ref i) => format!("{}", i),
            Literal::UnsignedInteger(ref i) => format!("{}", i),
            Literal::FixedPoint(integral, fractional) => format!("{}.{}", integral, fractional),
            Literal::String(ref s) => {
                if is_mysql_mode() {
                    format!("'{}'", mysql_escape(s))
                } else {
                    format!("'{}'", s.replace('\'', "''"))
                }
            }
            Literal::Blob(ref bv) => bv
                .iter()
                .map(|v| format!("{:x}", v))
                .collect::<Vec<String>>()
                .join(" "),
            Literal::CurrentTime => "CURRENT_TIME".to_string(),
            Literal::CurrentDate => "CURRENT_DATE".to_string(),
            Literal::CurrentTimestamp => "CURRENT_TIMESTAMP".to_string(),
            Literal::Boolean(b) => b.to_string(),
            // Literal::Placeholder(ref item) => item.to_string(),
        }
    }
}

impl Literal {
    pub fn to_raw_string(&self) -> String {
        match *self {
            Literal::String(ref s) => s.clone(),
            _ => self.to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TableKey {
    PrimaryKey(Vec<IndexColumn>),
    UniqueKey(String, Vec<IndexColumn>),
    FulltextKey(String, Vec<IndexColumn>, Option<String>),
    Key(String, Vec<IndexColumn>),
    SpatialKey(String, Vec<IndexColumn>),
    Constraint(
        String,
        Vec<IndexColumn>,
        String,
        Vec<IndexColumn>,
        Option<ReferenceOption>,
        Option<ReferenceOption>,
    ),
}

impl From<MysqlTableKey> for TableKey {
    fn from(value: MysqlTableKey) -> Self {
        match value {
            MysqlTableKey::PrimaryKey(v) => {
                TableKey::PrimaryKey(v.into_iter().map(|v| v.into()).collect())
            }
            MysqlTableKey::UniqueKey(v, c) => {
                TableKey::UniqueKey(v, c.into_iter().map(|v| v.into()).collect())
            }
            MysqlTableKey::FulltextKey(v1, c, v2) => {
                TableKey::FulltextKey(v1, c.into_iter().map(|v| v.into()).collect(), v2)
            }
            MysqlTableKey::Key(v, c) => TableKey::Key(v, c.into_iter().map(|v| v.into()).collect()),
            MysqlTableKey::SpatialKey(v, c) => {
                TableKey::SpatialKey(v, c.into_iter().map(|v| v.into()).collect())
            }
            MysqlTableKey::Constraint(v1, c1, v2, c2, r1, r2) => TableKey::Constraint(
                v1,
                c1.into_iter().map(|v| v.into()).collect(),
                v2,
                c2.into_iter().map(|v| v.into()).collect(),
                r1.map(|v| v.into()),
                r2.map(|v| v.into()),
            ),
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize, derive_more::Display)]
pub enum ReferenceOption {
    #[display("RESTRICT")]
    Restrict,
    #[display("CASCADE")]
    Cascade,
    #[display("SET NULL")]
    SetNull,
    #[display("NO ACTION")]
    NoAction,
    #[display("SET DEFAULT")]
    SetDefault,
}

impl From<MysqlReferenceOption> for ReferenceOption {
    fn from(value: MysqlReferenceOption) -> Self {
        match value {
            MysqlReferenceOption::Restrict => ReferenceOption::Restrict,
            MysqlReferenceOption::Cascade => ReferenceOption::Cascade,
            MysqlReferenceOption::SetNull => ReferenceOption::SetNull,
            MysqlReferenceOption::NoAction => ReferenceOption::NoAction,
            MysqlReferenceOption::SetDefault => ReferenceOption::SetDefault,
        }
    }
}

impl fmt::Display for TableKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            TableKey::PrimaryKey(ref columns) => {
                write!(f, "PRIMARY KEY ")?;
                write!(
                    f,
                    "({})",
                    columns
                        .iter()
                        .map(|c| c.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            TableKey::UniqueKey(ref name, ref columns) => {
                write!(f, "UNIQUE KEY {} ", escape_db_identifier(name))?;
                write!(
                    f,
                    "({})",
                    columns
                        .iter()
                        .map(|c| c.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            TableKey::FulltextKey(ref name, ref columns, ref parser) => {
                write!(f, "FULLTEXT KEY {} ", escape_db_identifier(name))?;
                write!(
                    f,
                    "({})",
                    columns
                        .iter()
                        .map(|c| c.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )?;
                if let Some(parser) = parser {
                    write!(f, "/*!50100 WITH PARSER `{}` */", parser)?;
                }
                Ok(())
            }
            TableKey::Key(ref name, ref columns) => {
                write!(f, "KEY {} ", escape_db_identifier(name))?;
                write!(
                    f,
                    "({})",
                    columns
                        .iter()
                        .map(|c| c.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            TableKey::SpatialKey(ref name, ref columns) => {
                write!(f, "SPATIAL KEY {} ", escape_db_identifier(name))?;
                write!(
                    f,
                    "({})",
                    columns
                        .iter()
                        .map(|c| c.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            TableKey::Constraint(
                ref name,
                ref columns,
                ref table,
                ref foreign,
                ref on_delete,
                ref on_update,
            ) => {
                write!(f, "CONSTRAINT {} FOREIGN KEY ", escape_db_identifier(name))?;
                write!(
                    f,
                    "({})",
                    columns
                        .iter()
                        .map(|c| c.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )?;
                write!(f, " REFERENCES {} ", escape_db_identifier(table))?;
                write!(
                    f,
                    "({})",
                    foreign
                        .iter()
                        .map(|c| c.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )?;
                if let Some(on_delete) = on_delete {
                    write!(f, " ON DELETE {}", &on_delete.to_string())?;
                }
                if let Some(on_update) = on_update {
                    write!(f, " ON UPDATE {}", &on_update.to_string())?;
                }
                Ok(())
            }
        }
    }
}

#[derive(Clone, Debug, Default, Eq, Serialize, Deserialize)]
pub struct IndexColumn {
    pub name: String,
    pub query: Option<String>,
    pub len: Option<u32>,
    pub desc: bool,
}
impl From<MysqlColumn> for IndexColumn {
    fn from(value: MysqlColumn) -> Self {
        IndexColumn {
            name: value.name,
            query: value.query,
            len: value.len,
            desc: value.desc,
        }
    }
}

impl PartialEq for IndexColumn {
    fn eq(&self, other: &Self) -> bool {
        if self.query.is_some() && other.query.is_some() {
            true
        } else if self.query.is_some() || other.query.is_some() {
            false
        } else {
            self.name == other.name && self.len == other.len && self.desc == other.desc
        }
    }
}

impl fmt::Display for IndexColumn {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(ref query) = self.query {
            write!(f, "({})", query)?;
        } else {
            write!(f, "{}", escape_db_identifier(&self.name))?;
            if is_mysql_mode()
                && let Some(ref len) = self.len
            {
                write!(f, "({})", len)?;
            }
        }
        if self.desc {
            write!(f, " DESC")?;
        }
        Ok(())
    }
}
