use serde::Deserialize;
use serde::Serialize;
use std::fmt;
use std::str;

use super::common::{Literal, SqlType};
use super::keywords::escape;

#[derive(Clone, Debug, Eq, Serialize, Deserialize)]
pub struct Column {
    pub name: String,
    pub query: Option<String>,
    pub len: Option<u32>,
}

impl PartialEq for Column {
    fn eq(&self, other: &Self) -> bool {
        if self.query.is_some() && other.query.is_some() {
            true
        } else if self.query.is_some() || other.query.is_some() {
            false
        } else {
            self.name == other.name && self.len == other.len
        }
    }
}

impl fmt::Display for Column {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(ref query) = self.query {
            write!(f, "({})", query)?;
        } else {
            write!(f, "{}", escape(&self.name))?;
            if let Some(ref len) = self.len {
                write!(f, "({})", len)?;
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColumnConstraint {
    NotNull,
    CharacterSet(String),
    Collation(String),
    DefaultValue(Literal),
    AutoIncrement,
    PrimaryKey,
    Unique,
    Srid(u32),
    Generated(String),
}

impl fmt::Display for ColumnConstraint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ColumnConstraint::NotNull => write!(f, "NOT NULL"),
            ColumnConstraint::CharacterSet(ref charset) => write!(f, "CHARACTER SET {}", charset),
            ColumnConstraint::Collation(ref collation) => write!(f, "COLLATE {}", collation),
            ColumnConstraint::DefaultValue(ref literal) => {
                write!(f, "DEFAULT {}", literal.to_string())
            }
            ColumnConstraint::AutoIncrement => write!(f, "AUTO_INCREMENT"),
            ColumnConstraint::PrimaryKey => write!(f, "PRIMARY KEY"),
            ColumnConstraint::Unique => write!(f, "UNIQUE"),
            ColumnConstraint::Srid(srid) => {
                write!(f, "/*!80003 SRID {} */", srid)
            }
            ColumnConstraint::Generated(query) => {
                write!(f, "GENERATED ALWAYS AS ({}) VIRTUAL", query)
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColumnSpecification {
    pub column: Column,
    pub sql_type: SqlType,
    pub constraints: Vec<ColumnConstraint>,
    pub comment: Option<Literal>,
}

impl ColumnSpecification {
    pub fn default_value(&self) -> Option<String> {
        self.constraints
            .iter()
            .map(|v| match v {
                ColumnConstraint::DefaultValue(x) => Some(x.to_raw_string()),
                _ => None,
            })
            .find(|v| v.is_some())
            .flatten()
    }
}

impl fmt::Display for ColumnSpecification {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {}", escape(&self.column.name), self.sql_type)?;
        for constraint in self.constraints.iter() {
            write!(f, " {}", constraint)?;
        }
        if let Some(ref comment) = self.comment {
            write!(f, " COMMENT {}", comment.to_string())?;
        }
        Ok(())
    }
}
