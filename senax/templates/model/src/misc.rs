// This code is automatically generated by Senax and is always overwritten.

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use rust_decimal::Decimal;
use senax_common::cache::calc_mem_size;
use serde_json::Value;
use sqlx::query::{Query, QueryAs};
use std::convert::TryFrom;

use crate::connection::{DbArguments, DbType};

macro_rules! fetch {
    ( $conn:ident, $query:ident, $method:ident ) => {
        if $conn.has_read_tx() {
            $query.$method($conn.get_read_tx().await?.as_mut()).await?
@%- if !config.force_disable_cache %@
        } else if $conn.has_cache_tx() {
            $query.$method($conn.get_cache_tx().await?.as_mut()).await?
@%- endif %@
        } else if $conn.has_tx() {
            $query.$method($conn.get_tx().await?.as_mut()).await?
        } else {
            $query
                .$method($conn.get_reader().await?.as_mut())
                .await?
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

pub(crate) trait ColTr {
    fn name(&self) -> &'static str;
}
pub(crate) trait BindTr {
    fn name(&self) -> &'static str;
    fn placeholder(&self) -> &'static str {
        ""
    }
    fn len(&self) -> usize {
        1
    }
    fn query_as_bind<T>(
        self,
        query: QueryAs<'_, DbType, T, DbArguments>,
    ) -> QueryAs<'_, DbType, T, DbArguments>;
    fn query_bind(self, query: Query<'_, DbType, DbArguments>) -> Query<'_, DbType, DbArguments>;
}
#[allow(dead_code)]
pub(crate) trait BindArrayTr {
    fn query_as_each_bind<T>(
        self,
        query: QueryAs<'_, DbType, T, DbArguments>,
    ) -> QueryAs<'_, DbType, T, DbArguments>;
    fn query_each_bind(
        self,
        query: Query<'_, DbType, DbArguments>,
    ) -> Query<'_, DbType, DbArguments>;
}
pub(crate) trait ColRelTr {
    fn write_rel(&self, buf: &mut String, idx: usize, without_key: bool);
    fn write_key(&self, buf: &mut String);
    fn query_as_bind<T>(
        self,
        query: QueryAs<'_, DbType, T, DbArguments>,
    ) -> QueryAs<'_, DbType, T, DbArguments>;
    fn query_bind(self, query: Query<'_, DbType, DbArguments>) -> Query<'_, DbType, DbArguments>;
}
pub(crate) trait FilterTr
where
    Self: Sized,
{
    fn write(&self, buf: &mut String, idx: usize, trash_mode: &mut TrashMode);
    fn query_as_bind<T>(
        self,
        query: sqlx::query::QueryAs<'_, DbType, T, DbArguments>,
    ) -> sqlx::query::QueryAs<'_, DbType, T, DbArguments>;
    fn query_bind(
        self,
        query: sqlx::query::Query<'_, DbType, DbArguments>,
    ) -> sqlx::query::Query<'_, DbType, DbArguments>;
    fn write_where(
        filter: &Option<Self>,
        trash_mode: TrashMode,
        trashed_sql: &str,
        not_trashed_sql: &str,
        only_trashed_sql: &str,
    ) -> String;
}
pub(crate) trait OrderTr
where
    Self: Sized,
{
    fn write(&self, buf: &mut String);
    fn write_order(order: &Option<Vec<Self>>) -> String;
}

#[rustfmt::skip]
macro_rules! filter {
    ( $t:ty ) => {
        fn write(&self, buf: &mut String, idx: usize, trash_mode: &mut TrashMode) {
            match self {
                Filter_::WithTrashed => {
                    *trash_mode = TrashMode::With;
                }
                Filter_::OnlyTrashed => {
                    *trash_mode = TrashMode::Only;
                }
                Filter_::Match(cols, _v) => {
                    buf.push_str(" MATCH (");
                    for c in cols {
                        buf.push_str(c.name());
                        buf.push_str(",");
                    }
                    buf.truncate(buf.len() - 1);
                    buf.push_str(") AGAINST (?) AND ");
                }
                Filter_::MatchBoolean(cols, _v) => {
                    buf.push_str(" MATCH (");
                    for c in cols {
                        buf.push_str(c.name());
                        buf.push_str(",");
                    }
                    buf.truncate(buf.len() - 1);
                    buf.push_str(") AGAINST (?  IN BOOLEAN MODE) AND ");
                }
                Filter_::MatchExpansion(cols, _v) => {
                    buf.push_str(" MATCH (");
                    for c in cols {
                        buf.push_str(c.name());
                        buf.push_str(",");
                    }
                    buf.truncate(buf.len() - 1);
                    buf.push_str(") AGAINST (?  WITH QUERY EXPANSION) AND ");
                }
                Filter_::IsNull(c) => {
                    buf.push_str(c.name());
                    buf.push_str(" IS NULL AND ");
                }
                Filter_::IsNotNull(c) => {
                    buf.push_str(c.name());
                    buf.push_str(" IS NOT NULL AND ");
                }
                Filter_::Eq(c) => {
                    buf.push_str(c.name());
                    buf.push_str(" = ");
                    buf.push_str(c.placeholder());
                    buf.push_str(" AND ");
                }
                Filter_::EqKey(c) => {
                    buf.push_str(c.name());
                    buf.push_str(" = ? AND ");
                }
                Filter_::NotEq(c) => {
                    buf.push_str(c.name());
                    buf.push_str(" != ? AND ");
                }
                Filter_::Gt(c) => {
                    buf.push_str(c.name());
                    buf.push_str(" > ");
                    buf.push_str(c.placeholder());
                    buf.push_str(" AND ");
                }
                Filter_::Gte(c) => {
                    buf.push_str(c.name());
                    buf.push_str(" >= ");
                    buf.push_str(c.placeholder());
                    buf.push_str(" AND ");
                }
                Filter_::Lt(c) => {
                    buf.push_str(c.name());
                    buf.push_str(" < ");
                    buf.push_str(c.placeholder());
                    buf.push_str(" AND ");
                }
                Filter_::Lte(c) => {
                    buf.push_str(c.name());
                    buf.push_str(" <= ");
                    buf.push_str(c.placeholder());
                    buf.push_str(" AND ");
                }
                Filter_::Like(c) => {
                    buf.push_str(c.name());
                    buf.push_str(" LIKE ? AND ");
                }
                Filter_::AllBits(c) => {
                    buf.push_str(c.name());
                    buf.push_str(" & ? = ? AND ");
                }
                Filter_::AnyBits(c) => {
                    buf.push_str(c.name());
                    buf.push_str(" & ? != 0 AND ");
                }
                Filter_::In(c) => {
                    if c.len() > 0 {
                        buf.push_str(c.name());
                        buf.push_str(" IN (");
                        for _i in 0..c.len() {
                            buf.push_str(c.placeholder());
                            buf.push_str(",");
                        }
                        buf.truncate(buf.len() - 1);
                        buf.push_str(") AND ");
                    } else {
                        buf.push_str("false AND ");
                    }
                }
                Filter_::NotIn(c) => {
                    if c.len() > 0 {
                        buf.push_str(c.name());
                        buf.push_str(" NOT IN (");
                        for _i in 0..c.len() {
                            buf.push_str(c.placeholder());
                            buf.push_str(",");
                        }
                        buf.truncate(buf.len() - 1);
                        buf.push_str(") AND ");
                    }
                }
                Filter_::MemberOf(c, p) => {
                    buf.push_str("CAST(? AS JSON) MEMBER OF (");
                    if p.is_some() {
                        buf.push_str("JSON_EXTRACT(");
                        buf.push_str(c.name());
                        buf.push_str(", ?)");
                    } else {
                        buf.push_str(c.name());
                    }
                    buf.push_str(") AND ");
                }
                Filter_::Contains(c, p) => {
                    buf.push_str("JSON_CONTAINS(");
                    if p.is_some() {
                        buf.push_str("JSON_EXTRACT(");
                        buf.push_str(c.name());
                        buf.push_str(", ?)");
                    } else {
                        buf.push_str(c.name());
                    }
                    buf.push_str(", CAST(? AS JSON)) AND ");
                }
                Filter_::Overlaps(c, p) => {
                    buf.push_str("JSON_OVERLAPS(");
                    if p.is_some() {
                        buf.push_str("JSON_EXTRACT(");
                        buf.push_str(c.name());
                        buf.push_str(", ?)");
                    } else {
                        buf.push_str(c.name());
                    }
                    buf.push_str(", CAST(? AS JSON)) AND ");
                }
                Filter_::JsonIn(c, p) => {
                    buf.push_str("JSON_OVERLAPS(JSON_EXTRACT(");
                    buf.push_str(c.name());
                    buf.push_str(", ?)");
                    buf.push_str(", CAST(? AS JSON)) AND ");
                }
                Filter_::JsonContainsPath(c, _p) => {
                    buf.push_str("JSON_CONTAINS_PATH(");
                    buf.push_str(c.name());
                    buf.push_str(", 'one', ?) AND ");
                }
                Filter_::JsonEq(c, _p) => {
                    buf.push_str("JSON_EXTRACT(");
                    buf.push_str(c.name());
                    buf.push_str(", ?) = CAST(? AS JSON) AND ");
                }
                Filter_::JsonLt(c, _p) => {
                    buf.push_str("JSON_EXTRACT(");
                    buf.push_str(c.name());
                    buf.push_str(", ?) < CAST(? AS JSON) AND ");
                }
                Filter_::JsonLte(c, _p) => {
                    buf.push_str("JSON_EXTRACT(");
                    buf.push_str(c.name());
                    buf.push_str(", ?) <= CAST(? AS JSON) AND ");
                }
                Filter_::JsonGt(c, _p) => {
                    buf.push_str("JSON_EXTRACT(");
                    buf.push_str(c.name());
                    buf.push_str(", ?) > CAST(? AS JSON) AND ");
                }
                Filter_::JsonGte(c, _p) => {
                    buf.push_str("JSON_EXTRACT(");
                    buf.push_str(c.name());
                    buf.push_str(", ?) >= CAST(? AS JSON) AND ");
                }
                Filter_::Within(c) => {
                    buf.push_str("ST_Within(");
                    buf.push_str(c.name());
                    buf.push_str(", ST_GeomFromGeoJSON(?, 2, ?)) AND ");
                }
                Filter_::Intersects(c) => {
                    buf.push_str("ST_Intersects(");
                    buf.push_str(c.name());
                    buf.push_str(", ST_GeomFromGeoJSON(?, 2, ?)) AND ");
                }
                Filter_::Crosses(c) => {
                    buf.push_str("ST_Crosses(");
                    buf.push_str(c.name());
                    buf.push_str(", ST_GeomFromGeoJSON(?, 2, ?)) AND ");
                }
                Filter_::DWithin(c) => {
                    buf.push_str("ST_Distance(");
                    buf.push_str(c.name());
                    buf.push_str(", ST_GeomFromGeoJSON(?, 2, ?)) <= ? AND ");
                    buf.push_str("ST_Intersects(");
                    buf.push_str(c.name());
                    buf.push_str(", ST_Buffer(ST_GeomFromGeoJSON(?, 2, ?), ? * 1.1)) AND ");
                }
                Filter_::Not(c) => {
                    buf.push_str("NOT (");
                    c.write(buf, idx, trash_mode);
                    buf.truncate(buf.len() - 5);
                    buf.push_str(") AND ");
                }
                Filter_::And(v) => {
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
                Filter_::Or(v) => {
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
                Filter_::Exists(c) => {
                    buf.push_str("EXISTS (");
                    c.write_rel(buf, idx, false);
                    buf.push_str(") AND ");
                }
                Filter_::NotExists(c) => {
                    buf.push_str("NOT EXISTS (");
                    c.write_rel(buf, idx, false);
                    buf.push_str(") AND ");
                }
                Filter_::EqAny(c) => {
                    c.write_key(buf);
                    buf.push_str(" = ANY (");
                    c.write_rel(buf, idx, true);
                    buf.push_str(") AND ");
                }
                Filter_::NotAll(c) => {
                    c.write_key(buf);
                    buf.push_str(" <> ALL (");
                    c.write_rel(buf, idx, true);
                    buf.push_str(") AND ");
                }
                Filter_::Raw(raw) => {
                    buf.push_str("(");
                    buf.push_str(raw);
                    buf.push_str(") AND ");
                }
                Filter_::RawWithParam(raw, _param) => {
                    buf.push_str("(");
                    buf.push_str(raw);
                    buf.push_str(") AND ");
                }
                Filter_::Boolean(_) => {
                    buf.push_str("? AND ");
                }
            };
        }
        fn query_as_bind<T>(
            self,
            mut query: sqlx::query::QueryAs<'_, DbType, T, DbArguments>,
        ) -> sqlx::query::QueryAs<'_, DbType, T, DbArguments> {
            match self {
                Filter_::WithTrashed => query,
                Filter_::OnlyTrashed => query,
                Filter_::Match(_c, v) => query.bind(v),
                Filter_::MatchBoolean(_c, v) => query.bind(v),
                Filter_::MatchExpansion(_c, v) => query.bind(v),
                Filter_::IsNull(_c) => query,
                Filter_::IsNotNull(_c) => query,
                Filter_::Eq(c) => c.query_as_bind(query),
                Filter_::EqKey(c) => c.query_as_bind(query),
                Filter_::NotEq(c) => c.query_as_bind(query),
                Filter_::Gt(c) => c.query_as_bind(query),
                Filter_::Gte(c) => c.query_as_bind(query),
                Filter_::Lt(c) => c.query_as_bind(query),
                Filter_::Lte(c) => c.query_as_bind(query),
                Filter_::Like(c) => c.query_as_bind(query),
                Filter_::AllBits(c) => c.query_as_bind(query),
                Filter_::AnyBits(c) => c.query_as_bind(query),
                Filter_::In(c) => c.query_as_bind(query),
                Filter_::NotIn(c) => c.query_as_bind(query),
                Filter_::MemberOf(c, p) => if let Some(p) = p { c.query_as_bind(query).bind(p) } else { c.query_as_bind(query) },
                Filter_::Contains(c, p) => if let Some(p) = p { c.query_as_bind(query.bind(p)) } else { c.query_as_bind(query) },
                Filter_::Overlaps(c, p) => if let Some(p) = p { c.query_as_bind(query.bind(p)) } else { c.query_as_bind(query) },
                Filter_::JsonIn(c, p) => c.query_as_bind(query.bind(p)),
                Filter_::JsonContainsPath(c, p) => query.bind(p),
                Filter_::JsonEq(c, p) => c.query_as_bind(query.bind(p)),
                Filter_::JsonLt(c, p) => c.query_as_bind(query.bind(p)),
                Filter_::JsonLte(c, p) => c.query_as_bind(query.bind(p)),
                Filter_::JsonGt(c, p) => c.query_as_bind(query.bind(p)),
                Filter_::JsonGte(c, p) => c.query_as_bind(query.bind(p)),
                Filter_::Within(c) => c.query_as_bind(query),
                Filter_::Intersects(c) => c.query_as_bind(query),
                Filter_::Crosses(c) => c.query_as_bind(query),
                Filter_::DWithin(c) => c.query_as_bind(query),
                Filter_::Not(c) => c.query_as_bind(query),
                Filter_::And(v) => {for c in v { query = c.query_as_bind(query); } query},
                Filter_::Or(v) => {for c in v { query = c.query_as_bind(query); } query},
                Filter_::Exists(c) => c.query_as_bind(query),
                Filter_::NotExists(c) => c.query_as_bind(query),
                Filter_::EqAny(c) => c.query_as_bind(query),
                Filter_::NotAll(c) => c.query_as_bind(query),
                Filter_::Raw(_c) => query,
                Filter_::RawWithParam(_c, param) => {for v in param { query = query.bind(v); } query},
                Filter_::Boolean(v) => query.bind(v),
            }
        }
        fn query_bind(
            self,
            mut query: sqlx::query::Query<'_, DbType, DbArguments>,
        ) -> sqlx::query::Query<'_, DbType, DbArguments> {
            match self {
                Filter_::WithTrashed => query,
                Filter_::OnlyTrashed => query,
                Filter_::Match(_c, v) => query.bind(v),
                Filter_::MatchBoolean(_c, v) => query.bind(v),
                Filter_::MatchExpansion(_c, v) => query.bind(v),
                Filter_::IsNull(_c) => query,
                Filter_::IsNotNull(_c) => query,
                Filter_::Eq(c) => c.query_bind(query),
                Filter_::EqKey(c) => c.query_bind(query),
                Filter_::NotEq(c) => c.query_bind(query),
                Filter_::Gt(c) => c.query_bind(query),
                Filter_::Gte(c) => c.query_bind(query),
                Filter_::Lt(c) => c.query_bind(query),
                Filter_::Lte(c) => c.query_bind(query),
                Filter_::Like(c) => c.query_bind(query),
                Filter_::AllBits(c) => c.query_bind(query),
                Filter_::AnyBits(c) => c.query_bind(query),
                Filter_::In(c) => c.query_bind(query),
                Filter_::NotIn(c) => c.query_bind(query),
                Filter_::MemberOf(c, p) => if let Some(p) = p { c.query_bind(query).bind(p) } else { c.query_bind(query) },
                Filter_::Contains(c, p) => if let Some(p) = p { c.query_bind(query.bind(p)) } else { c.query_bind(query) },
                Filter_::Overlaps(c, p) => if let Some(p) = p { c.query_bind(query.bind(p)) } else { c.query_bind(query) },
                Filter_::JsonIn(c, p) => c.query_bind(query.bind(p)),
                Filter_::JsonContainsPath(c, p) => query.bind(p),
                Filter_::JsonEq(c, p) => c.query_bind(query.bind(p)),
                Filter_::JsonLt(c, p) => c.query_bind(query.bind(p)),
                Filter_::JsonLte(c, p) => c.query_bind(query.bind(p)),
                Filter_::JsonGt(c, p) => c.query_bind(query.bind(p)),
                Filter_::JsonGte(c, p) => c.query_bind(query.bind(p)),
                Filter_::Within(c) => c.query_bind(query),
                Filter_::Intersects(c) => c.query_bind(query),
                Filter_::Crosses(c) => c.query_bind(query),
                Filter_::DWithin(c) => c.query_bind(query),
                Filter_::Not(c) => c.query_bind(query),
                Filter_::And(v) => {for c in v { query = c.query_bind(query); } query},
                Filter_::Or(v) => {for c in v { query = c.query_bind(query); } query},
                Filter_::Exists(c) => c.query_bind(query),
                Filter_::NotExists(c) => c.query_bind(query),
                Filter_::EqAny(c) => c.query_bind(query),
                Filter_::NotAll(c) => c.query_bind(query),
                Filter_::Raw(_c) => query,
                Filter_::RawWithParam(_c, param) => {for v in param { query = query.bind(v); } query},
                Filter_::Boolean(v) => query.bind(v),
            }
        }
        fn write_where(
            filter: &Option<Filter_>,
            mut trash_mode: TrashMode,
            trashed_sql: &str,
            not_trashed_sql: &str,
            only_trashed_sql: &str,
        ) -> String {
            let mut s = String::with_capacity(100);
            s.push_str("WHERE ");
            if let Some(ref c) = filter {
                c.write(&mut s, 1, &mut trash_mode);
            }
            if trash_mode == TrashMode::Not {
                s.push_str(not_trashed_sql)
            } else if trash_mode == TrashMode::Only {
                s.push_str(only_trashed_sql)
            } else {
                s.push_str(trashed_sql)
            }
            if s.len() > "WHERE ".len() {
                s.truncate(s.len() - " AND ".len());
            } else {
                s.truncate(0);
            }
            s
        }
    };
}
pub(crate) use filter;

macro_rules! order {
    () => {
        fn write(&self, buf: &mut String) {
            match self {
                Order_::Asc(c) => {
                    buf.push_str(c.name());
                    buf.push_str(" ASC, ");
                }
                Order_::Desc(c) => {
                    buf.push_str(c.name());
                    buf.push_str(" DESC, ");
                }
                Order_::IsNullAsc(c) => {
                    buf.push_str(c.name());
                    buf.push_str(" IS NULL ASC, ");
                }
                Order_::IsNullDesc(c) => {
                    buf.push_str(c.name());
                    buf.push_str(" IS NULL DESC, ");
                }
            };
        }
        fn write_order(order: &Option<Vec<Order_>>) -> String {
            match order {
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
pub(crate) use order;

pub(crate) trait IntoJson<T> {
    fn _into_json(&self) -> String;
}

impl<T> IntoJson<T> for T
where
    T: serde::Serialize,
{
    fn _into_json(&self) -> String {
        let s = serde_json::to_string(self).unwrap();
        assert!(s.len() <= @{ config.max_db_str_len() }@, "Incorrect JSON length.");
        s
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

pub trait Updater {
    fn is_new(&self) -> bool;
    fn has_been_deleted(&self) -> bool;
    fn mark_for_delete(&mut self);
    fn unmark_for_delete(&mut self);
    fn will_be_deleted(&self) -> bool;
    fn mark_for_upsert(&mut self);
    fn is_updated(&self) -> bool;
    fn overwrite_except_skip(&mut self, updater: Self);
    fn overwrite_only_set(&mut self, updater: Self);
    fn overwrite_with(&mut self, updater: Self, set_only: bool);
}

#[derive(Clone, Debug)]
pub enum BindValue {
    Bool(Option<bool>),
    Enum(Option<i64>),
    Number(Option<Decimal>),
    String(Option<String>),
    DateTime(Option<NaiveDateTime>),
    Date(Option<NaiveDate>),
    Time(Option<NaiveTime>),
    Blob(Option<Vec<u8>>),
    Json(Option<Value>),
    Uuid(Option<uuid::fmt::Hyphenated>),
    BinaryUuid(Option<uuid::Uuid>),
}

macro_rules! impl_bind_value {
    ($T:ty, $U:ident) => {
        impl core::convert::From<$T> for BindValue {
            fn from(t: $T) -> Self {
                Self::$U(Some(t.into()))
            }
        }
        impl core::convert::From<Option<$T>> for BindValue {
            fn from(t: Option<$T>) -> Self {
                Self::$U(t.map(|v| v.into()))
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
impl_bind_value!(uuid::Uuid, Uuid);

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

pub mod arc_bytes {
    use serde::{Deserialize, Deserializer, Serializer};
    use serde_bytes::ByteBuf;
    use std::sync::Arc;

    pub fn serialize<S>(data: &Arc<Vec<u8>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(data.as_slice())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Arc<Vec<u8>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let buf = ByteBuf::deserialize(deserializer)?;
        Ok(Arc::new(buf.into_vec()))
    }
}
pub mod option_arc_bytes {
    use serde::{Deserialize, Deserializer, Serializer};
    use serde_bytes::ByteBuf;
    use std::sync::Arc;

    pub fn serialize<S>(data: &Option<Arc<Vec<u8>>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match data {
            Some(value) => serializer.serialize_bytes(value.as_slice()),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Arc<Vec<u8>>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        match Option::<ByteBuf>::deserialize(deserializer)? {
            Some(buf) => Ok(Some(Arc::new(buf.into_vec()))),
            None => Ok(None),
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Default, PartialEq)]
pub(crate) struct JsonBlob(std::sync::Arc<Vec<u8>>);
impl TryFrom<&str> for JsonBlob {
    type Error = Box<dyn std::error::Error + Send + Sync>;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Ok(Default::default());
        }
        let _: Value = serde_json::from_str(value)?;
        Ok(Self(zstd::stream::encode_all(value.as_bytes(), 3)?.into()))
    }
}
impl From<&JsonBlob> for String {
    fn from(value: &JsonBlob) -> Self {
        if value.0.is_empty() {
            return String::new();
        }
        let v = zstd::stream::decode_all(value.0.as_slice()).unwrap();
        unsafe { String::from_utf8_unchecked(v) }
    }
}
impl Size for JsonBlob {
    fn _size(&self) -> usize {
        calc_mem_size(self.0.capacity()) + std::mem::size_of::<usize>() * 4
    }
}
impl std::fmt::Debug for JsonBlob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self._into_json())
    }
}
impl JsonBlob {
    pub fn _into_json(&self) -> String {
        let s: String = self.into();
        assert!(s.len() <= @{ config.max_db_str_len() }@, "Incorrect JSON length.");
        s
    }
    pub fn _to_value<T: serde::de::DeserializeOwned>(&self) -> Option<T> {
        if self.0.is_empty() {
            return None;
        }
        let v = zstd::stream::decode_all(self.0.as_slice()).unwrap();
        Some(serde_json::from_slice(&v).unwrap())
    }
}
pub(crate) trait ToJsonBlob {
    fn _to_json_blob(&self) -> anyhow::Result<JsonBlob>;
}

impl<T> ToJsonBlob for T
where
    T: serde::Serialize,
{
    fn _to_json_blob(&self) -> anyhow::Result<JsonBlob> {
        let v = serde_json::to_string(self)?;
        Ok(JsonBlob(zstd::stream::encode_all(v.as_bytes(), 3)?.into()))
    }
}
@{-"\n"}@