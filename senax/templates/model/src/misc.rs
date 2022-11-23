// This code is auto-generated and will always be overwritten.

use anyhow::Result;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use rust_decimal::Decimal;
use senax_common::cache::calc_mem_size;
use serde_json::Value;
use std::convert::TryFrom;
use validator::ValidationError;

macro_rules! fetch {
    ( $conn:ident, $query:ident, $method:ident ) => {
        if $conn.has_read_tx() {
            $query.$method($conn.get_read_tx().await?).await?
        } else if $conn.has_cache_tx() {
            $query.$method($conn.get_cache_tx().await?).await?
        } else if $conn.has_tx() {
            $query.$method($conn.get_tx().await?).await?
        } else {
            $query.$method($conn.get_replica_conn().await?).await?
        }
    };
}
pub(crate) use fetch;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) enum TrashMode {
    #[default]
    Not,
    With,
    Only,
}

macro_rules! condition {
    ( $t:ty ) => {
        pub(crate) fn write(&self, buf: &mut String, idx: i32, trash_mode: &mut TrashMode) {
            match self {
                Cond::WithTrashed => {
                    *trash_mode = TrashMode::With;
                }
                Cond::OnlyTrashed => {
                    *trash_mode = TrashMode::Only;
                }
                Cond::IsNull(c) => {
                    buf.push_str(c.name());
                    buf.push_str(" IS NULL AND ");
                }
                Cond::IsNotNull(c) => {
                    buf.push_str(c.name());
                    buf.push_str(" IS NOT NULL AND ");
                }
                Cond::Eq(c) => {
                    buf.push_str(c.name());
                    buf.push_str(" = ? AND ");
                }
                Cond::EqKey(c) => {
                    buf.push_str(c.name());
                    buf.push_str(" = ? AND ");
                }
                Cond::NotEq(c) => {
                    buf.push_str(c.name());
                    buf.push_str(" != ? AND ");
                }
                Cond::NullSafeEq(c) => {
                    buf.push_str(c.name());
                    buf.push_str(" <=> ? AND ");
                }
                Cond::Gt(c) => {
                    buf.push_str(c.name());
                    buf.push_str(" > ? AND ");
                }
                Cond::Gte(c) => {
                    buf.push_str(c.name());
                    buf.push_str(" >= ? AND ");
                }
                Cond::Lt(c) => {
                    buf.push_str(c.name());
                    buf.push_str(" < ? AND ");
                }
                Cond::Lte(c) => {
                    buf.push_str(c.name());
                    buf.push_str(" <= ? AND ");
                }
                Cond::Like(c) => {
                    buf.push_str(c.name());
                    buf.push_str(" LIKE ? AND ");
                }
                Cond::AllBits(c) => {
                    buf.push_str(c.name());
                    buf.push_str(" & ? = ? AND ");
                }
                Cond::AnyBits(c) => {
                    buf.push_str(c.name());
                    buf.push_str(" & ? != 0 AND ");
                }
                Cond::In(c) => {
                    if c.len() > 0 {
                        buf.push_str(c.name());
                        buf.push_str(" IN (");
                        for _i in 0..c.len() {
                            buf.push_str("?,");
                        }
                        buf.truncate(buf.len() - 1);
                        buf.push_str(") AND ");
                    } else {
                        buf.push_str("false AND ");
                    }
                }
                Cond::NotIn(c) => {
                    if c.len() > 0 {
                        buf.push_str(c.name());
                        buf.push_str(" NOT IN (");
                        for _i in 0..c.len() {
                            buf.push_str("?,");
                        }
                        buf.truncate(buf.len() - 1);
                        buf.push_str(") AND ");
                    }
                }
                Cond::MemberOf(c) => {
                    buf.push_str("? MEMBER OF (");
                    buf.push_str(c.name());
                    buf.push_str(") AND ");
                }
                Cond::Contains(c) => {
                    buf.push_str("JSON_CONTAINS (");
                    buf.push_str(c.name());
                    buf.push_str(",?) AND ");
                }
                Cond::Overlaps(c) => {
                    buf.push_str("JSON_OVERLAPS (");
                    buf.push_str(c.name());
                    buf.push_str(",?) AND ");
                }
                Cond::Not(c) => {
                    buf.push_str("NOT (");
                    c.write(buf, idx, trash_mode);
                    buf.truncate(buf.len() - 5);
                    buf.push_str(") AND ");
                }
                Cond::And(v) => {
                    if !v.is_empty() {
                        buf.push_str("(");
                        for c in v.iter() {
                            c.write(buf, idx, trash_mode);
                        }
                        buf.truncate(buf.len() - 5);
                        buf.push_str(") AND ");
                    } else {
                        buf.push_str("true AND ");
                    }
                }
                Cond::Or(v) => {
                    if !v.is_empty() {
                        buf.push_str("(");
                        for c in v.iter() {
                            c.write(buf, idx, trash_mode);
                            buf.truncate(buf.len() - 5);
                            buf.push_str(" OR ");
                        }
                        buf.truncate(buf.len() - 4);
                        buf.push_str(") AND ");
                    } else {
                        buf.push_str("false AND ");
                    }
                }
                Cond::Exists(c) => {
                    buf.push_str("EXISTS (");
                    c.write(buf, idx);
                    buf.push_str(") AND ");
                }
                Cond::NotExists(c) => {
                    buf.push_str("NOT EXISTS (");
                    c.write(buf, idx);
                    buf.push_str(") AND ");
                }
            };
        }
        pub(crate) fn bind<T>(
            self,
            query: sqlx::query::QueryAs<'_, DbType, T, DbArguments>,
        ) -> sqlx::query::QueryAs<'_, DbType, T, DbArguments> {
            match self {
                Cond::WithTrashed => query,
                Cond::OnlyTrashed => query,
                Cond::IsNull(_c) => query,
                Cond::IsNotNull(_c) => query,
                Cond::Eq(c) => c.bind(query),
                Cond::EqKey(c) => c.bind(query),
                Cond::NotEq(c) => c.bind(query),
                Cond::NullSafeEq(c) => c.bind(query),
                Cond::Gt(c) => c.bind(query),
                Cond::Gte(c) => c.bind(query),
                Cond::Lt(c) => c.bind(query),
                Cond::Lte(c) => c.bind(query),
                Cond::Like(c) => c.bind(query),
                Cond::AllBits(c) => c.bind(query),
                Cond::AnyBits(c) => c.bind(query),
                Cond::In(c) => c.bind(query),
                Cond::NotIn(c) => c.bind(query),
                Cond::MemberOf(c) => c.bind(query),
                Cond::Contains(c) => c.bind(query),
                Cond::Overlaps(c) => c.bind(query),
                Cond::Not(c) => c.bind(query),
                Cond::And(v) => v.into_iter().fold(query, |query, c| c.bind(query)),
                Cond::Or(v) => v.into_iter().fold(query, |query, c| c.bind(query)),
                Cond::Exists(c) => c.bind(query),
                Cond::NotExists(c) => c.bind(query),
            }
        }
        fn write_where(
            condition: &Option<Cond>,
            mut trash_mode: TrashMode,
            trashed_sql: &str,
            not_trashed_sql: &str,
            only_trashed_sql: &str,
        ) -> String {
            let mut s = String::with_capacity(100);
            s.push_str("WHERE ");
            match condition {
                Some(ref c) => {
                    c.write(&mut s, 1, &mut trash_mode);
                }
                _ => {}
            }
            if trash_mode == TrashMode::Not {
                s.push_str(not_trashed_sql)
            } else if trash_mode == TrashMode::Only {
                s.push_str(only_trashed_sql)
            } else {
                s.push_str(trashed_sql)
            }
            if s.len() > "WHERE ".len() {
                s.truncate(s.len() - 5);
            } else {
                s.truncate(0);
            }
            s
        }
    };
}
pub(crate) use condition;

macro_rules! order_by {
    () => {
        fn write(&self, buf: &mut String) {
            match self {
                OrderBy::Asc(c) => {
                    buf.push_str(c.name());
                    buf.push_str(" ASC, ");
                }
                OrderBy::Desc(c) => {
                    buf.push_str(c.name());
                    buf.push_str(" DESC, ");
                }
                OrderBy::IsNullAsc(c) => {
                    buf.push_str(c.name());
                    buf.push_str(" IS NULL ASC, ");
                }
                OrderBy::IsNullDesc(c) => {
                    buf.push_str(c.name());
                    buf.push_str(" IS NULL DESC, ");
                }
            };
        }
        pub(crate) fn write_order_by(order_by: &Option<Vec<OrderBy>>) -> String {
            match order_by {
                Some(ref v) if !v.is_empty() => {
                    let mut s = String::with_capacity(100);
                    s.push_str("ORDER BY ");
                    for o in v {
                        o.write(&mut s);
                    }
                    s.truncate(s.len() - 2);
                    s
                }
                _ => String::new(),
            }
        }
    };
}
pub(crate) use order_by;

pub(crate) trait IntoJson<T> {
    fn _into_json(self) -> sqlx::types::Json<T>;
}

impl<T> IntoJson<T> for T {
    fn _into_json(self) -> sqlx::types::Json<T> {
        sqlx::types::Json(self)
    }
}

pub(crate) trait Size {
    fn _size(&self) -> usize;
}

impl Size for String {
    fn _size(&self) -> usize {
        calc_mem_size(self.capacity()) + std::mem::size_of::<usize>() * 4
    }
}

impl Size for Vec<u8> {
    fn _size(&self) -> usize {
        calc_mem_size(self.capacity()) + std::mem::size_of::<usize>() * 4
    }
}

impl Size for Vec<u32> {
    fn _size(&self) -> usize {
        calc_mem_size(self.capacity() * std::mem::size_of::<u32>())
            + std::mem::size_of::<usize>() * 4
    }
}

impl Size for Vec<u64> {
    fn _size(&self) -> usize {
        calc_mem_size(self.capacity() * std::mem::size_of::<u64>())
            + std::mem::size_of::<usize>() * 4
    }
}

impl Size for Vec<String> {
    fn _size(&self) -> usize {
        calc_mem_size(self.capacity() * std::mem::size_of::<usize>())
            + std::mem::size_of::<usize>() * 2
            + self.iter().fold(0, |i, v| {
                i + v.capacity() + std::mem::size_of::<usize>() * 2
            })
    }
}

impl Size for Value {
    fn _size(&self) -> usize {
        match self {
            Value::String(v) => calc_mem_size(v.capacity()) + std::mem::size_of::<usize>() * 4,
            Value::Array(v) => {
                (calc_mem_size(v.capacity() * std::mem::size_of::<Value>())
                    + std::mem::size_of::<usize>() * 4)
                    + v.iter()
                        .fold(0, |i, v| i + v._size() + std::mem::size_of::<usize>() * 2)
            }
            Value::Object(v) => v.iter().fold(0, |i, (k, v)| {
                i + (calc_mem_size(k.capacity()) + std::mem::size_of::<usize>() * 4)
                    + v._size()
                    + std::mem::size_of::<usize>() * 2
            }),
            _ => 0,
        }
    }
}

pub trait ForUpdateTr {
    fn _is_new(&self) -> bool;
    fn _has_been_deleted(&self) -> bool;
    fn _delete(&mut self);
    fn _will_be_deleted(&self) -> bool;
    fn _upsert(&mut self);
    fn _is_updated(&self) -> bool;
}

#[derive(Clone, Debug)]
pub enum BindValue {
    Bool(Option<bool>),
    Enum(Option<u8>),
    Number(Option<Decimal>),
    String(Option<String>),
    DateTime(Option<NaiveDateTime>),
    Date(Option<NaiveDate>),
    Time(Option<NaiveTime>),
    Blob(Option<Vec<u8>>),
    Json(Option<Value>),
}

macro_rules! impl_bind_value {
    ($T:ty, $U:ident) => {
        impl core::convert::From<$T> for BindValue {
            fn from(t: $T) -> Self {
                Self::$U(Some(t))
            }
        }
        impl core::convert::From<Option<$T>> for BindValue {
            fn from(t: Option<$T>) -> Self {
                Self::$U(t)
            }
        }
    };
}
impl_bind_value!(bool, Bool);
impl_bind_value!(String, String);
impl_bind_value!(NaiveDateTime, DateTime);
impl_bind_value!(NaiveDate, Date);
impl_bind_value!(NaiveTime, Time);
impl_bind_value!(Vec<u8>, Blob);
impl_bind_value!(Value, Json);

macro_rules! impl_decimal {
    ($T:ty) => {
        impl core::convert::From<$T> for BindValue {
            fn from(t: $T) -> Self {
                Self::Number(Some(Decimal::from(t)))
            }
        }
        impl core::convert::From<Option<$T>> for BindValue {
            fn from(t: Option<$T>) -> Self {
                Self::Number(t.map(|v| Decimal::from(v)))
            }
        }
    };
}

macro_rules! impl_try_decimal {
    ($T:ty) => {
        impl core::convert::From<$T> for BindValue {
            fn from(t: $T) -> Self {
                Self::Number(Some(Decimal::try_from(t).unwrap()))
            }
        }
        impl core::convert::From<Option<$T>> for BindValue {
            fn from(t: Option<$T>) -> Self {
                Self::Number(t.map(|v| Decimal::try_from(v).unwrap()))
            }
        }
    };
}

impl_decimal!(isize);
impl_decimal!(i8);
impl_decimal!(i16);
impl_decimal!(i32);
impl_decimal!(i64);
impl_decimal!(usize);
impl_decimal!(u8);
impl_decimal!(u16);
impl_decimal!(u32);
impl_decimal!(u64);
impl_decimal!(i128);
impl_decimal!(u128);
impl_decimal!(Decimal);
impl_try_decimal!(f32);
impl_try_decimal!(f64);

#[allow(dead_code)]
pub(crate) fn validate_tinytext_length(value: &str) -> Result<(), ValidationError> {
    if value.len() > 255 {
        return Err(ValidationError::new("length"));
    }
    Ok(())
}

#[allow(dead_code)]
pub(crate) fn validate_text_length(value: &str) -> Result<(), ValidationError> {
    if value.len() > 65535 {
        return Err(ValidationError::new("length"));
    }
    Ok(())
}

#[allow(dead_code)]
pub(crate) fn validate_unsigned_decimal(
    value: &rust_decimal::Decimal,
) -> Result<(), ValidationError> {
    if value.is_sign_negative() {
        let mut err = ValidationError::new("range");
        err.add_param(::std::borrow::Cow::from("min"), &0.0);
        return Err(err);
    }
    Ok(())
}
@{-"\n"}@