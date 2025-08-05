use anyhow::{Result, bail};
use indexmap::IndexMap;
use senax_mysql_parser::NomErr;
use senax_mysql_parser::column::ColumnConstraint;
use senax_mysql_parser::common::{Literal, SqlType, TableKey};
use senax_mysql_parser::create::{CreateTableStatement, creation};
use senax_mysql_parser::create_table_options::TableOption;
use senax_mysql_parser::keywords::escape;
use serde::{Deserialize, Serialize};
use sqlx::migrate::MigrateDatabase;
use sqlx::{MySql, MySqlPool, Row};
use std::fmt;

use crate::common::yaml_value_to_str;
use crate::schema::SoftDelete;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Table {
    pub name: String,
    pub old_name: Option<String>,
    pub old_soft_delete: Option<(String, SoftDelete)>,
    pub columns: IndexMap<String, Column>,
    pub primary: Option<TableKey>,
    pub indexes: IndexMap<String, TableKey>,
    pub constraints: IndexMap<String, TableKey>,
    pub comment: Option<String>,
    pub engine: Option<String>,
    // pub character_set: Option<String>,
    pub collation: Option<String>,
    pub skip_ddl: bool,
}

impl fmt::Display for Table {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "CREATE TABLE {} (\n    ", escape(&self.name))?;
        write!(
            f,
            "{}",
            self.columns
                .iter()
                .map(|(name, column)| format!("{} {}", escape(name), column))
                .collect::<Vec<_>>()
                .join(",\n    ")
        )?;
        if let Some(ref primary) = self.primary {
            write!(f, ",\n    {}", primary)?;
        }
        write!(
            f,
            "{}",
            self.indexes
                .values()
                .filter(|index| !matches!(index, TableKey::Key(_, x) if x.is_empty()))
                .map(|index| format!(",\n    {}", index))
                .collect::<Vec<_>>()
                .join("")
        )?;
        // write!(
        //     f,
        //     "{}",
        //     self.constraints
        //         .iter()
        //         .map(|(name, constraint)| format!(",\n    {}", constraint))
        //         .collect::<Vec<_>>()
        //         .join("")
        // )?;
        write!(f, "\n)")?;
        if let Some(ref engine) = self.engine {
            write!(f, " ENGINE={engine}")?;
        }
        // if let Some(ref character_set) = self.character_set {
        //     write!(f, " CHARACTER SET='{character_set}'")?;
        // }
        if let Some(ref collation) = self.collation {
            write!(f, " COLLATE='{collation}'")?;
        }
        write!(f, ";")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Column {
    pub old_name: Option<String>,
    pub sql_type: SqlType,
    pub alt_type: SqlType,
    pub constraint: Constraint,
    pub default: Option<Literal>,
    pub comment: Option<String>,
}

impl PartialEq for Column {
    fn eq(&self, other: &Self) -> bool {
        (self.sql_type == other.sql_type
            || self.alt_type == other.sql_type
            || self.sql_type == other.alt_type)
            && self.constraint == other.constraint
            && comp_literal(&self.default, &other.default)
            && self.comment == other.comment
    }
}

impl Column {
    pub fn has_query(&self) -> bool {
        self.constraint.query.is_some()
    }
}

fn comp_literal(value: &Option<Literal>, other: &Option<Literal>) -> bool {
    if let Some(value) = value {
        if let Some(other) = other {
            return value.to_raw_string() == other.to_raw_string();
        }
    }
    value.is_some() == other.is_some()
}

fn mysql_escape(v: &str) -> String {
    v.replace('\\', "\\\\")
        .replace(0 as char, "\\0")
        .replace('\'', "\\'")
        .replace('"', "\\\"")
        .replace(8 as char, "\\b")
        .replace('\r', "\\r")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
        .replace(26 as char, "\\z")
}

impl fmt::Display for Column {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.sql_type)?;
        write!(f, "{}", self.constraint)?;
        if let Some(ref default) = self.default {
            write!(f, " DEFAULT {}", default.to_string())?;
        }
        if let Some(ref comment) = self.comment {
            write!(f, " COMMENT '{}'", mysql_escape(comment))?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Constraint {
    pub not_null: bool,
    // pub character_set: Option<String>,
    pub default_collation: Option<String>,
    pub collation: Option<String>,
    pub default_value: Option<Literal>,
    pub auto_increment: bool,
    pub primary_key: bool,
    pub unique: bool,
    pub srid: Option<u32>,
    pub query: Option<(String, bool)>,
}

impl PartialEq for Constraint {
    fn eq(&self, other: &Self) -> bool {
        self.not_null == other.not_null
            && (self.collation == other.collation
                || (self.default_collation == other.collation && self.collation.is_none())
                || (self.collation == other.default_collation && other.collation.is_none()))
            && self.default_value == other.default_value
            && self.auto_increment == other.auto_increment
            && self.primary_key == other.primary_key
            && self.unique == other.unique
            && self.srid == other.srid
            && self.query.is_none() == other.query.is_none()
            && self.query.clone().unwrap_or_default().1 == other.query.clone().unwrap_or_default().1
    }
}

impl fmt::Display for Constraint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.not_null {
            write!(f, " NOT NULL")?;
        }
        // if let Some(ref c) = self.character_set {
        //     write!(f, " CHARACTER SET {}", c)?;
        // }
        if let Some(ref c) = self.collation {
            write!(f, " COLLATE {}", c)?;
        }
        if let Some(ref v) = self.default_value {
            write!(f, " DEFAULT {}", v.to_string())?;
        }
        if self.auto_increment {
            write!(f, " AUTO_INCREMENT")?;
        }
        if self.primary_key {
            write!(f, " PRIMARY KEY")?;
        }
        if self.unique {
            write!(f, " UNIQUE")?;
        }
        if let Some(srid) = self.srid {
            write!(f, " /*!80003 SRID {} */", srid)?;
        }
        if let Some((query, stored)) = &self.query {
            write!(
                f,
                " GENERATED ALWAYS AS ({}) {}",
                query,
                if *stored { "STORED" } else { "VIRTUAL" }
            )?;
        }
        Ok(())
    }
}

impl From<Vec<ColumnConstraint>> for Constraint {
    fn from(list: Vec<ColumnConstraint>) -> Self {
        let mut constraint = Constraint::default();
        for row in list {
            match row {
                ColumnConstraint::NotNull => {
                    constraint.not_null = true;
                }
                ColumnConstraint::CharacterSet(_v) => {
                    // constraint.character_set = Some(v);
                }
                ColumnConstraint::Collation(v) => {
                    constraint.default_collation = Some(v.clone());
                    constraint.collation = Some(v);
                }
                ColumnConstraint::DefaultValue(v) => {
                    constraint.default_value = Some(v);
                }
                ColumnConstraint::AutoIncrement => {
                    constraint.auto_increment = true;
                }
                ColumnConstraint::PrimaryKey => {
                    constraint.primary_key = true;
                }
                ColumnConstraint::Unique => {
                    constraint.unique = true;
                }
                ColumnConstraint::Srid(v) => {
                    constraint.srid = Some(v);
                }
                ColumnConstraint::Generated(q, s) => {
                    constraint.query = Some((q, s));
                }
            }
        }
        constraint
    }
}

fn conv(def: CreateTableStatement) -> Table {
    let mut table = Table::default();
    table.name = def.table;
    table.columns = def
        .fields
        .into_iter()
        .fold(IndexMap::new(), |mut map, spec| {
            let default = spec
                .constraints
                .iter()
                .map(|v| match v {
                    ColumnConstraint::DefaultValue(x) => Some(x.clone()),
                    _ => None,
                })
                .find(|v| v.is_some())
                .flatten();
            let constraints: Vec<_> = spec
                .constraints
                .into_iter()
                .filter(|v| !matches!(v, ColumnConstraint::DefaultValue(_)))
                .collect();
            map.insert(
                spec.column.name,
                Column {
                    old_name: None,
                    sql_type: spec.sql_type.clone(),
                    alt_type: spec.sql_type,
                    constraint: constraints.into(),
                    default,
                    comment: spec.comment.map(|v| v.to_raw_string()),
                },
            );
            map
        });
    if let Some(keys) = def.keys {
        for key in keys {
            match &key {
                TableKey::PrimaryKey(_) => {
                    table.primary = Some(key);
                }
                TableKey::UniqueKey(n, _) => {
                    table.indexes.insert(n.to_string(), key);
                }
                TableKey::FulltextKey(n, _, _) => {
                    table.indexes.insert(n.to_string(), key);
                }
                TableKey::Key(n, _) => {
                    table.indexes.insert(n.to_string(), key);
                }
                TableKey::SpatialKey(n, _) => {
                    table.indexes.insert(n.to_string(), key);
                }
                TableKey::Constraint(n, _, _, _, _, _) => {
                    table.constraints.insert(n.to_string(), key);
                }
            }
        }
        for name in table.constraints.keys() {
            table.indexes.swap_remove(name);
        }
    }
    for option in def.options {
        match option {
            TableOption::Comment(comment) => table.comment = Some(comment.to_raw_string()),
            TableOption::Collation(collation) => table.collation = Some(collation),
            TableOption::Engine(engine) => table.engine = Some(engine),
            TableOption::Another => {}
        }
    }
    table
}

pub async fn parse(database_url: &str) -> Result<IndexMap<String, Table>> {
    if !MySql::database_exists(database_url).await? {
        return Ok(Default::default());
    }
    let pool = MySqlPool::connect(database_url).await?;
    let mut conn = pool.acquire().await?;

    let rows = sqlx::query("show full tables where Table_Type != 'VIEW'")
        .fetch_all(conn.as_mut())
        .await?;
    let tables: Vec<String> = rows.iter().map(|v| v.get(0)).collect();
    let mut result = IndexMap::new();
    for table in tables {
        let row = sqlx::query(&format!("show create table `{}`;", &table))
            .fetch_one(conn.as_mut())
            .await?;
        let def: String = row.get(1);
        let def = match creation(def.as_bytes()) {
            Ok((_, o)) => o,
            Err(e) => match e {
                NomErr::Incomplete(_e) => {
                    bail!(format!("failed to incomplete query:\n{}", def));
                }
                NomErr::Error(e) => {
                    bail!(format!(
                        "failed to parse query:\n{}",
                        String::from_utf8(e.input.to_vec()).unwrap()
                    ));
                }
                NomErr::Failure(e) => {
                    bail!(format!(
                        "failed to parse query:\n{}",
                        String::from_utf8(e.input.to_vec()).unwrap()
                    ));
                }
            },
        };
        let def = conv(def);
        result.insert(table, def);
    }
    Ok(result)
}

pub fn parse_default_value(value: &serde_yaml::Value, sql_type: &SqlType) -> Result<Literal> {
    let value: String = yaml_value_to_str(value)?;
    match sql_type {
        SqlType::Bool => {
            if value.eq_ignore_ascii_case("true") || value.eq("1") {
                Ok(Literal::Integer(1))
            } else if value.eq_ignore_ascii_case("false") || value.eq("0") {
                Ok(Literal::Integer(0))
            } else {
                anyhow::bail!("{:?} is not bool", value);
            }
        }
        SqlType::Char(_) => Ok(Literal::String(value)),
        SqlType::Varchar(_) => Ok(Literal::String(value)),
        SqlType::Int => Ok(Literal::Integer(value.parse()?)),
        SqlType::UnsignedInt => Ok(Literal::UnsignedInteger(value.parse()?)),
        SqlType::Smallint => Ok(Literal::Integer(value.parse()?)),
        SqlType::UnsignedSmallint => Ok(Literal::UnsignedInteger(value.parse()?)),
        SqlType::Bigint => Ok(Literal::Integer(value.parse()?)),
        SqlType::UnsignedBigint => Ok(Literal::UnsignedInteger(value.parse()?)),
        SqlType::Tinyint => {
            let v = if value.eq_ignore_ascii_case("true") {
                1
            } else if value.eq_ignore_ascii_case("false") {
                0
            } else {
                value.parse()?
            };
            Ok(Literal::Integer(v))
        }
        SqlType::UnsignedTinyint => {
            let v = if value.eq_ignore_ascii_case("true") {
                1
            } else if value.eq_ignore_ascii_case("false") {
                0
            } else {
                value.parse()?
            };
            Ok(Literal::UnsignedInteger(v))
        }
        SqlType::Blob => Ok(Literal::Null),
        SqlType::Longblob => Ok(Literal::Null),
        SqlType::Mediumblob => Ok(Literal::Null),
        SqlType::Tinyblob => Ok(Literal::Null),
        SqlType::Double => Ok(Literal::String(value)),
        SqlType::Float => Ok(Literal::String(value)),
        SqlType::Real => Ok(Literal::String(value)),
        SqlType::Tinytext => Ok(Literal::String(value)),
        SqlType::Mediumtext => Ok(Literal::String(value)),
        SqlType::Longtext => Ok(Literal::String(value)),
        SqlType::Text => Ok(Literal::String(value)),
        SqlType::Date => Ok(Literal::Null),
        SqlType::Time => Ok(Literal::Null),
        SqlType::DateTime(_) => {
            if "CURRENT_TIMESTAMP".eq_ignore_ascii_case(&value) {
                Ok(Literal::CurrentTimestamp)
            } else {
                Ok(Literal::Null)
            }
        }
        SqlType::Timestamp(_) => {
            if "CURRENT_TIMESTAMP".eq_ignore_ascii_case(&value) {
                Ok(Literal::CurrentTimestamp)
            } else {
                Ok(Literal::Null)
            }
        }
        SqlType::Binary(_) => Ok(Literal::Null),
        SqlType::Varbinary(_) => Ok(Literal::Null),
        SqlType::Enum(_) => Ok(Literal::String(value)),
        SqlType::Set(_) => Ok(Literal::String(value)),
        SqlType::Decimal(_, _) => Ok(Literal::Integer(value.parse()?)),
        SqlType::Json => Ok(Literal::Null),
        SqlType::Point => Ok(Literal::Null),
        SqlType::Geometry => Ok(Literal::Null),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sql_types() {
        let def = r#"CREATE TABLE `todos` (
            `todo_id` int unsigned NOT NULL AUTO_INCREMENT,
            `created_at` datetime NOT NULL,
            `updated_at` datetime NOT NULL,
            `deleted_at` datetime DEFAULT NULL,
            `version` int unsigned NOT NULL,
            `description` tinytext CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci NOT NULL COMMENT '詳細',
            `varchar` varchar(255) DEFAULT NULL,
            `done` tinyint DEFAULT NULL,
            `delete` tinyint DEFAULT NULL,
            `name_id` int unsigned DEFAULT NULL,
            `parent_id` int unsigned DEFAULT NULL,
            `value1` SMALLINT unsigned NOT NULL,
            `value2` int unsigned DEFAULT NULL,
            `color` tinyint unsigned NOT NULL,
            `color2` tinyint unsigned NOT NULL,
            `timestamp1` timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
            `timestamp2` timestamp NULL DEFAULT NULL,
            `datetime1` datetime NOT NULL,
            `datetime2` datetime DEFAULT NULL,
            `date1` date NOT NULL,
            `date2` date DEFAULT NULL,
            `time1` time NOT NULL,
            `time2` time DEFAULT NULL,
            `decimal1` decimal(36,9) NOT NULL,
            `decimal2` decimal(36,9) DEFAULT NULL,
            `json1` json NOT NULL,
            `json2` json DEFAULT NULL,
            `json3` json NOT NULL,
            `json4` json DEFAULT NULL,
            `json5` json NOT NULL,
            `json6` json DEFAULT NULL,
            `gggggg` int unsigned DEFAULT NULL,
            `double` double DEFAULT NULL,
            `point1` point /*!80003 SRID 4326 */ DEFAULT NULL,
            `point2` point NOT NULL /*!80003 SRID 4326 */,
            `text` longtext NOT NULL,
            `sidec` double GENERATED ALWAYS AS (sqrt(((`sidea` * `sidea`) + (`sideb` * `sideb`)))) VIRTUAL,
            PRIMARY KEY (`todo_id`),
            UNIQUE KEY `UQ_description` (`description`(20)),
            UNIQUE KEY `UQ_name_id` (`name_id`) USING BTREE,
            KEY `IDX_name_index` (`time1`,`time2`),
            KEY `IDX_xxxxx` (`gggggg` DESC),
            KEY `IDX_json1` ((cast(`json1` as unsigned array))),
            KEY `IDX_json3a` (`name_id`,(cast(`json3` as char(21) array))),
            FULLTEXT KEY `FT_text` (`text`) /*!50100 WITH PARSER `ngram` */ ,
            SPATIAL KEY `SP_point2` (`point2`),
            CONSTRAINT `FK_todos_name_id_space_todo_name` FOREIGN KEY (`name_id`) REFERENCES `space_todo_name` (`todo_name_id`) ON DELETE CASCADE,
            CONSTRAINT `FK_todos_parent_id_todos` FOREIGN KEY (`parent_id`) REFERENCES `todos` (`todo_id`) ON DELETE CASCADE
          ) ENGINE=InnoDB AUTO_INCREMENT=3 DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_0900_ai_ci"#;
        let def = match creation(def.as_bytes()) {
            Ok((_, o)) => Ok(o),
            Err(_) => Err(format!("failed to parse query: {}", def)),
        };
        println!("{}", &def.unwrap());
    }
}
