use anyhow::{bail, Result};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use sqlx::migrate::MigrateDatabase;
use sqlx::{MySql, MySqlPool, Row};
use std::collections::HashMap;
use std::fmt;

use super::parser::column::ColumnConstraint;
use super::parser::common::{SqlType, TableKey};
use super::parser::create::{creation, CreateTableStatement};
use super::parser::keywords::escape;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Table {
    pub name: String,
    pub columns: IndexMap<String, Column>,
    pub primary: Option<TableKey>,
    pub indexes: HashMap<String, TableKey>,
    pub constraints: HashMap<String, TableKey>,
    pub engine: Option<String>,
    pub character_set: Option<String>,
    pub collate: Option<String>,
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
                .iter()
                .map(|(_name, index)| format!(",\n    {}", index))
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
        if let Some(ref character_set) = self.character_set {
            write!(f, " CHARACTER SET='{character_set}'")?;
        }
        if let Some(ref collate) = self.collate {
            write!(f, " COLLATE='{collate}'")?;
        }
        write!(f, ";")
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct Column {
    pub sql_type: SqlType,
    pub constraint: Constraint,
    pub default: Option<String>,
    pub comment: Option<String>,
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
            write!(f, " DEFAULT '{}'", mysql_escape(default))?;
        }
        if let Some(ref comment) = self.comment {
            write!(f, " COMMENT '{}'", mysql_escape(comment))?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Constraint {
    pub not_null: bool,
    // pub character_set: Option<String>,
    pub collation: Option<String>,
    // pub default_value: Option<Literal>,
    pub auto_increment: bool,
    pub primary_key: bool,
    pub unique: bool,
    pub srid: Option<u32>,
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
        // if let Some(ref v) = self.default_value {
        //     write!(f, " DEFAULT {}", v.to_string())?;
        // }
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
        Ok(())
    }
}

impl From<Vec<ColumnConstraint>> for Constraint {
    fn from(v: Vec<ColumnConstraint>) -> Self {
        let mut constraint = Constraint::default();
        for row in v {
            match row {
                ColumnConstraint::NotNull => {
                    constraint.not_null = true;
                }
                ColumnConstraint::CharacterSet(_v) => {
                    // constraint.character_set = Some(v);
                }
                ColumnConstraint::Collation(v) => {
                    constraint.collation = Some(v);
                }
                ColumnConstraint::DefaultValue(_v) => {
                    // constraint.default_value = Some(v);
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
            let default = spec.default_value();
            map.insert(
                spec.column.name,
                Column {
                    sql_type: spec.sql_type,
                    constraint: spec.constraints.into(),
                    default,
                    comment: spec.comment,
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
            table.indexes.remove(name);
        }
    }
    table
}

pub async fn parse(database_url: &str) -> Result<HashMap<String, Table>> {
    if !MySql::database_exists(database_url).await? {
        return Ok(Default::default());
    }
    let pool = MySqlPool::connect(database_url).await?;
    let mut conn = pool.acquire().await?;

    let rows = sqlx::query("show full tables where Table_Type != 'VIEW'")
        .fetch_all(&mut conn)
        .await?;
    let tables: Vec<String> = rows.iter().map(|v| v.get(0)).collect();
    let mut result = HashMap::new();
    for table in tables {
        let row = sqlx::query(&format!("show create table `{}`;", &table))
            .fetch_one(&mut conn)
            .await?;
        let def: String = row.get(1);
        let def = match creation(def.as_bytes()) {
            Ok((_, o)) => o,
            Err(e) => match e {
                nom::Err::Incomplete(_e) => {
                    bail!(format!("failed to incomplete query:\n{}", def));
                }
                nom::Err::Error(e) => {
                    bail!(format!(
                        "failed to parse query:\n{}",
                        String::from_utf8(e.input.to_vec()).unwrap()
                    ));
                }
                nom::Err::Failure(e) => {
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
            PRIMARY KEY (`todo_id`),
            UNIQUE KEY `UQ_description` (`description`(20)),
            UNIQUE KEY `UQ_name_id` (`name_id`) USING BTREE,
            KEY `IDX_name_index` (`time1`,`time2`),
            KEY `IDX_xxxxx` (`gggggg`),
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
