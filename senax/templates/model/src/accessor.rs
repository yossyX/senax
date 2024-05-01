// This code is auto-generated and will always be overwritten.
// Senax v@{ ""|senax_version }@

#![allow(dead_code)]

use anyhow::{ensure, Result};
use derive_more::Display;
use log::kv::ToValue;
use num_traits::{CheckedAdd, CheckedSub, Float, SaturatingAdd, SaturatingSub};
use rust_decimal::Decimal;
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::fmt::Debug;
use std::ops::{BitAnd, BitOr};
use std::sync::Arc;
use std::{fmt, fmt::Display, marker::PhantomData};

#[derive(
    Serialize_repr, Deserialize_repr, PartialEq, Eq, Clone, Copy, Debug, Display, Default, Hash,
)]
#[repr(u8)]
pub(crate) enum Op {
    #[default]
    None,
    Skip,
    Set,
    Add,
    Sub,
    Max,
    Min,
    BitAnd,
    BitOr,
}

impl Op {
    pub fn get_sql(&self, col: &str, nullable: bool, ph: &str) -> String {
        let mut buf = String::with_capacity(100);
        match self {
            Op::None => {}
            Op::Skip => {}
            Op::Set => {
                buf.push_str(col);
                buf.push('=');
                buf.push_str(ph);
            }
            Op::Add if nullable => {
                buf.push_str(col);
                buf.push_str("=IFNULL(");
                buf.push_str(col);
                buf.push_str(", 0)+");
                buf.push_str(ph);
            }
            Op::Add => {
                buf.push_str(col);
                buf.push('=');
                buf.push_str(col);
                buf.push('+');
                buf.push_str(ph);
            }
            Op::Sub if nullable => {
                buf.push_str(col);
                buf.push_str("=IFNULL(");
                buf.push_str(col);
                buf.push_str(", 0)-");
                buf.push_str(ph);
            }
            Op::Sub => {
                buf.push_str(col);
                buf.push('=');
                buf.push_str(col);
                buf.push('-');
                buf.push_str(ph);
            }
            Op::Max if nullable => {
                buf.push_str(col);
                buf.push_str("=IF(IFNULL(");
                buf.push_str(col);
                buf.push_str(", ");
                buf.push_str(ph);
                buf.push_str(")<");
                buf.push_str(ph);
                buf.push(',');
                buf.push_str(ph);
                buf.push(',');
                buf.push_str(col);
                buf.push(')');
            }
            Op::Max => {
                buf.push_str(col);
                buf.push_str("=IF(");
                buf.push_str(col);
                buf.push('<');
                buf.push_str(ph);
                buf.push(',');
                buf.push_str(ph);
                buf.push(',');
                buf.push_str(col);
                buf.push(')');
            }
            Op::Min if nullable => {
                buf.push_str(col);
                buf.push_str("=IF(IFNULL(");
                buf.push_str(col);
                buf.push_str(", ");
                buf.push_str(ph);
                buf.push_str(")>");
                buf.push_str(ph);
                buf.push(',');
                buf.push_str(ph);
                buf.push(',');
                buf.push_str(col);
                buf.push(')');
            }
            Op::Min => {
                buf.push_str(col);
                buf.push_str("=IF(");
                buf.push_str(col);
                buf.push('>');
                buf.push_str(ph);
                buf.push(',');
                buf.push_str(ph);
                buf.push(',');
                buf.push_str(col);
                buf.push(')');
            }
            Op::BitAnd => {
                buf.push_str(col);
                buf.push('=');
                buf.push_str(col);
                buf.push('&');
                buf.push_str(ph);
            }
            Op::BitOr if nullable => {
                buf.push_str(col);
                buf.push_str("=IFNULL(");
                buf.push_str(col);
                buf.push_str(", 0)|");
                buf.push_str(ph);
            }
            Op::BitOr => {
                buf.push_str(col);
                buf.push('=');
                buf.push_str(col);
                buf.push('|');
                buf.push_str(ph);
            }
        };
        buf
    }

    pub fn get_bind_num(&self, nullable: bool) -> u32 {
        match self {
            Op::None => 0,
            Op::Skip => 0,
            Op::Set => 1,
            Op::Add => 1,
            Op::Sub => 1,
            Op::Max if nullable => 3,
            Op::Max => 2,
            Op::Min if nullable => 3,
            Op::Min => 2,
            Op::BitAnd => 1,
            Op::BitOr => 1,
        }
    }

    pub fn is_none(&self) -> bool {
        *self == Op::None
    }
}

#[allow(unused_macros)]
macro_rules! assign_sql_no_cache_update {
    ( $obj:ident, $vec:ident, $col:ident, $name:expr, $nullable:expr, $ph:expr ) => {
        if $obj._op.$col != Op::None && $obj._op.$col != Op::Skip {
            $vec.push($obj._op.$col.get_sql($name, $nullable, $ph));
        }
    };
}
#[allow(unused_imports)]
pub(crate) use assign_sql_no_cache_update;

#[allow(unused_macros)]
macro_rules! assign_sql {
    ( $obj:ident, $vec:ident, $col:ident, $name:expr, $nullable:expr, $update_cache: ident, $ph:expr ) => {
        if $obj._op.$col != Op::None && $obj._op.$col != Op::Skip {
            $vec.push($obj._op.$col.get_sql($name, $nullable, $ph));
            $update_cache = true;
        }
    };
}
#[allow(unused_imports)]
pub(crate) use assign_sql;

pub(crate) struct Empty;
impl Empty {
    pub fn is_zero_u8(val: &u8) -> bool {
        *val == 0
    }
    pub fn is_zero_i8(val: &i8) -> bool {
        *val == 0
    }
    pub fn is_zero_u16(val: &u16) -> bool {
        *val == 0
    }
    pub fn is_zero_i16(val: &i16) -> bool {
        *val == 0
    }
    pub fn is_zero_u32(val: &u32) -> bool {
        *val == 0
    }
    pub fn is_zero_i32(val: &i32) -> bool {
        *val == 0
    }
    pub fn is_zero_u64(val: &u64) -> bool {
        *val == 0
    }
    pub fn is_zero_i64(val: &i64) -> bool {
        *val == 0
    }
    pub fn is_zero_f32(val: &f32) -> bool {
        *val == 0.0
    }
    pub fn is_zero_f64(val: &f64) -> bool {
        *val == 0.0
    }
    pub fn is_zero_decimal(val: &Decimal) -> bool {
        val.is_zero()
    }
    pub fn is_zero_len<T>(val: &std::sync::Arc<Vec<T>>) -> bool {
        val.is_empty()
    }
    pub fn is_zero_json_len<T>(val: &Vec<T>) -> bool {
        val.is_empty()
    }
    pub fn is_default<T: Default + PartialEq>(val: &T) -> bool {
        *val == T::default()
    }
    pub fn is_default_utc_date_time(val: &chrono::DateTime<chrono::offset::Utc>) -> bool {
        *val == chrono::DateTime::<chrono::offset::Utc>::default()
    }
    pub fn is_default_local_date_time(val: &chrono::DateTime<chrono::offset::Local>) -> bool {
        *val == chrono::DateTime::<chrono::offset::Local>::default()
    }
    pub fn is_default_date(val: &chrono::NaiveDate) -> bool {
        *val == chrono::NaiveDate::default()
    }
    pub fn is_default_time(val: &chrono::NaiveTime) -> bool {
        *val == chrono::NaiveTime::default()
    }
}

pub struct AccessorPrimary<'a, I: Clone + Debug, O>
where
    O: From<I>,
{
    pub(crate) val: &'a I,
    pub(crate) _phantom: PhantomData<O>,
}
impl<'a, I: Clone + Debug, O> AccessorPrimary<'a, I, O>
where
    O: From<I>,
{
    pub fn get(&self) -> O {
        self.val.clone().into()
    }
    pub fn mark_for_skip(&self) {}
    pub fn mark_for_set(&mut self) {}

    pub(crate) fn _write_insert(f: &mut fmt::Formatter<'_>, comma: &str, col: &str, value: &I) -> fmt::Result {
        write!(f, "{comma}{col}: {:?}", value)
    }

    pub(crate) fn _write_update(
        f: &mut fmt::Formatter<'_>,
        comma: &str,
        col: &str,
        _op: Op,
        value: &I,
    ) -> fmt::Result {
        write!(f, "{comma}{col}: {:?}", value)
    }
}

pub struct AccessorNotNull<'a, I: Clone + Debug, O>
where
    I: From<O>,
    O: From<I>,
{
    pub(crate) op: &'a mut Op,
    pub(crate) val: &'a mut I,
    pub(crate) update: &'a mut I,
    pub(crate) _phantom: PhantomData<O>,
}
impl<'a, I: Clone + Debug, O> AccessorNotNull<'a, I, O>
where
    I: From<O>,
    O: From<I>,
{
    pub fn get(&self) -> O {
        self.val.clone().into()
    }
    pub fn mark_for_skip(&mut self) {
        *self.op = Op::Skip;
    }
    pub fn mark_for_set(&mut self) {
        *self.op = Op::Set;
    }
    pub fn set(&mut self, val: O) {
        *self.op = Op::Set;
        let val: I = val.into();
        *self.val = val.clone();
        *self.update = val;
    }
    pub(crate) fn _set(op: Op, prop: &mut I, update: &I) {
        if op == Op::Set {
            *prop = update.clone();
        }
    }

    pub(crate) fn _write_insert(f: &mut fmt::Formatter<'_>, comma: &str, col: &str, value: &I) -> fmt::Result {
        write!(f, "{comma}{col}: {:?}", value)
    }

    pub(crate) fn _write_update(
        f: &mut fmt::Formatter<'_>,
        comma: &str,
        col: &str,
        op: Op,
        value: &I,
    ) -> fmt::Result {
        if op != Op::None && op != Op::Skip {
            write!(f, "{comma}{col}: {{{op}: {:?}}}", value)?;
        }
        Ok(())
    }
}
pub struct AccessorNull<'a, I: Clone + Debug, O>
where
    I: From<O>,
    O: From<I>,
{
    pub(crate) op: &'a mut Op,
    pub(crate) val: &'a mut Option<I>,
    pub(crate) update: &'a mut Option<I>,
    pub(crate) _phantom: PhantomData<O>,
}
impl<'a, I: Clone + Debug, O> AccessorNull<'a, I, O>
where
    I: From<O>,
    O: From<I>,
{
    pub fn get(&self) -> Option<O> {
        self.val.as_ref().map(|v| v.clone().into())
    }
    pub fn mark_for_skip(&mut self) {
        *self.op = Op::Skip;
    }
    pub fn mark_for_set(&mut self) {
        *self.op = Op::Set;
    }
    pub fn set(&mut self, val: Option<O>) {
        *self.op = Op::Set;
        let val = val.map(|v| v.into());
        *self.val = val.clone();
        *self.update = val;
    }
    pub(crate) fn _set(op: Op, prop: &mut Option<I>, update: &Option<I>) {
        if op == Op::Set {
            *prop = update.clone();
        }
    }

    pub(crate) fn _write_insert(
        f: &mut fmt::Formatter<'_>,
        comma: &str,
        col: &str,
        value: &Option<I>,
    ) -> fmt::Result {
        if let Some(value) = value {
            write!(f, "{comma}{col}: {:?}", value)
        } else {
            write!(f, "{comma}{col}: null")
        }
    }

    pub(crate) fn _write_update(
        f: &mut fmt::Formatter<'_>,
        comma: &str,
        col: &str,
        op: Op,
        value: &Option<I>,
    ) -> fmt::Result {
        if op != Op::None && op != Op::Skip {
            if let Some(value) = value {
                write!(f, "{comma}{col}: {{{op}: {:?}}}", value)?;
            } else {
                write!(f, "{comma}{col}: {{{op}: null}}")?;
            }
        }
        Ok(())
    }
}

pub struct AccessorNotNullBool<'a> {
    pub(crate) op: &'a mut Op,
    pub(crate) val: &'a mut i8,
    pub(crate) update: &'a mut i8,
    pub(crate) _phantom: PhantomData<i8>,
}
impl<'a> AccessorNotNullBool<'a> {
    pub fn get(&self) -> bool {
        (*self.val) == 1
    }
    pub fn mark_for_skip(&mut self) {
        *self.op = Op::Skip;
    }
    pub fn mark_for_set(&mut self) {
        *self.op = Op::Set;
    }
    pub fn set(&mut self, val: bool) {
        *self.op = Op::Set;
        *self.val = val.into();
        *self.update = val.into();
    }
    pub(crate) fn _set(op: Op, prop: &mut i8, update: &i8) {
        if op == Op::Set {
            *prop = *update;
        }
    }

    pub(crate) fn _write_insert(f: &mut fmt::Formatter<'_>, comma: &str, col: &str, value: &i8) -> fmt::Result {
        write!(f, "{comma}{col}: {:?}", value)
    }

    pub(crate) fn _write_update(
        f: &mut fmt::Formatter<'_>,
        comma: &str,
        col: &str,
        op: Op,
        value: &i8,
    ) -> fmt::Result {
        if op != Op::None && op != Op::Skip {
            write!(f, "{comma}{col}: {{{op}: {:?}}}", value)?;
        }
        Ok(())
    }
}
pub struct AccessorNullBool<'a> {
    pub(crate) op: &'a mut Op,
    pub(crate) val: &'a mut Option<i8>,
    pub(crate) update: &'a mut Option<i8>,
    pub(crate) _phantom: PhantomData<i8>,
}
impl<'a> AccessorNullBool<'a> {
    pub fn get(&self) -> Option<bool> {
        self.val.map(|v| v == 1)
    }
    pub fn mark_for_skip(&mut self) {
        *self.op = Op::Skip;
    }
    pub fn mark_for_set(&mut self) {
        *self.op = Op::Set;
    }
    pub fn set(&mut self, val: Option<bool>) {
        *self.op = Op::Set;
        let val = val.map(|v| v.into());
        *self.val = val;
        *self.update = val;
    }
    pub(crate) fn _set(op: Op, prop: &mut Option<i8>, update: &Option<i8>) {
        if op == Op::Set {
            *prop = *update;
        }
    }

    pub(crate) fn _write_insert(
        f: &mut fmt::Formatter<'_>,
        comma: &str,
        col: &str,
        value: &Option<i8>,
    ) -> fmt::Result {
        if let Some(value) = value {
            write!(f, "{comma}{col}: {:?}", value)
        } else {
            write!(f, "{comma}{col}: null")
        }
    }

    pub(crate) fn _write_update(
        f: &mut fmt::Formatter<'_>,
        comma: &str,
        col: &str,
        op: Op,
        value: &Option<i8>,
    ) -> fmt::Result {
        if op != Op::None && op != Op::Skip {
            if let Some(value) = value {
                write!(f, "{comma}{col}: {{{op}: {:?}}}", value)?;
            } else {
                write!(f, "{comma}{col}: {{{op}: null}}")?;
            }
        }
        Ok(())
    }
}

pub struct AccessorNotNullArc<'a, I: Debug> {
    pub(crate) op: &'a mut Op,
    pub(crate) val: &'a mut Arc<I>,
    pub(crate) update: &'a mut Arc<I>,
    pub(crate) _phantom: PhantomData<I>,
}
impl<'a, I: Debug> AccessorNotNullArc<'a, I> {
    pub fn get(&self) -> Arc<I> {
        self.val.clone()
    }
    pub fn mark_for_skip(&mut self) {
        *self.op = Op::Skip;
    }
    pub fn mark_for_set(&mut self) {
        *self.op = Op::Set;
    }
    pub fn set(&mut self, val: Arc<I>) {
        *self.op = Op::Set;
        *self.val = val.clone();
        *self.update = val;
    }
    pub(crate) fn _set(op: Op, prop: &mut Arc<I>, update: &Arc<I>) {
        if op == Op::Set {
            *prop = update.clone();
        }
    }

    pub(crate) fn _write_insert(
        f: &mut fmt::Formatter<'_>,
        comma: &str,
        col: &str,
        value: &Arc<I>,
    ) -> fmt::Result {
        write!(f, "{comma}{col}: {:?}", value.as_ref())
    }

    pub(crate) fn _write_update(
        f: &mut fmt::Formatter<'_>,
        comma: &str,
        col: &str,
        op: Op,
        value: &Arc<I>,
    ) -> fmt::Result {
        if op != Op::None && op != Op::Skip {
            write!(f, "{comma}{col}: {{{op}: {:?}}}", value.as_ref())?;
        }
        Ok(())
    }
}
pub struct AccessorNullArc<'a, I: Debug> {
    pub(crate) op: &'a mut Op,
    pub(crate) val: &'a mut Option<Arc<I>>,
    pub(crate) update: &'a mut Option<Arc<I>>,
    pub(crate) _phantom: PhantomData<I>,
}
impl<'a, I: Debug> AccessorNullArc<'a, I> {
    pub fn get(&self) -> Option<Arc<I>> {
        self.val.clone()
    }
    pub fn mark_for_skip(&mut self) {
        *self.op = Op::Skip;
    }
    pub fn mark_for_set(&mut self) {
        *self.op = Op::Set;
    }
    pub fn set(&mut self, val: Option<Arc<I>>) {
        *self.op = Op::Set;
        *self.val = val.clone();
        *self.update = val;
    }
    pub(crate) fn _set(op: Op, prop: &mut Option<Arc<I>>, update: &Option<Arc<I>>) {
        if op == Op::Set {
            *prop = update.clone();
        }
    }

    pub(crate) fn _write_insert(
        f: &mut fmt::Formatter<'_>,
        comma: &str,
        col: &str,
        value: &Option<Arc<I>>,
    ) -> fmt::Result {
        if let Some(value) = value {
            write!(f, "{comma}{col}: {:?}", value.as_ref())
        } else {
            write!(f, "{comma}{col}: null")
        }
    }

    pub(crate) fn _write_update(
        f: &mut fmt::Formatter<'_>,
        comma: &str,
        col: &str,
        op: Op,
        value: &Option<Arc<I>>,
    ) -> fmt::Result {
        if op != Op::None && op != Op::Skip {
            if let Some(value) = value {
                write!(f, "{comma}{col}: {{{op}: {:?}}}", value.as_ref())?;
            } else {
                write!(f, "{comma}{col}: {{{op}: null}}")?;
            }
        }
        Ok(())
    }
}

pub struct AccessorNotNullBlob<'a> {
    pub(crate) op: &'a mut Op,
    pub(crate) val: &'a mut Arc<Vec<u8>>,
    pub(crate) update: &'a mut Arc<Vec<u8>>,
    pub(crate) _phantom: PhantomData<Vec<u8>>,
}
impl<'a> AccessorNotNullBlob<'a> {
    pub fn get(&self) -> Arc<Vec<u8>> {
        self.val.clone()
    }
    pub fn mark_for_skip(&mut self) {
        *self.op = Op::Skip;
    }
    pub fn mark_for_set(&mut self) {
        *self.op = Op::Set;
    }
    pub fn set(&mut self, val: Arc<Vec<u8>>) {
        *self.op = Op::Set;
        *self.val = val.clone();
        *self.update = val;
    }
    pub(crate) fn _set(op: Op, prop: &mut Arc<Vec<u8>>, update: &Arc<Vec<u8>>) {
        if op == Op::Set {
            *prop = update.clone();
        }
    }

    pub(crate) fn _write_insert(
        f: &mut fmt::Formatter<'_>,
        comma: &str,
        col: &str,
        _value: &Arc<Vec<u8>>,
    ) -> fmt::Result {
        write!(f, "{comma}{col}: BLOB")
    }

    pub(crate) fn _write_update(
        f: &mut fmt::Formatter<'_>,
        comma: &str,
        col: &str,
        op: Op,
        _value: &Arc<Vec<u8>>,
    ) -> fmt::Result {
        if op != Op::None && op != Op::Skip {
            write!(f, "{comma}{col}: {{{op}: BLOB}}")?;
        }
        Ok(())
    }
}
pub struct AccessorNullBlob<'a> {
    pub(crate) op: &'a mut Op,
    pub(crate) val: &'a mut Option<Arc<Vec<u8>>>,
    pub(crate) update: &'a mut Option<Arc<Vec<u8>>>,
    pub(crate) _phantom: PhantomData<Vec<u8>>,
}
impl<'a> AccessorNullBlob<'a> {
    pub fn get(&self) -> Option<&Arc<Vec<u8>>> {
        self.val.as_ref()
    }
    pub fn mark_for_skip(&mut self) {
        *self.op = Op::Skip;
    }
    pub fn mark_for_set(&mut self) {
        *self.op = Op::Set;
    }
    pub fn set(&mut self, val: Option<Arc<Vec<u8>>>) {
        *self.op = Op::Set;
        *self.val = val.clone();
        *self.update = val;
    }
    pub(crate) fn _set(op: Op, prop: &mut Option<Arc<Vec<u8>>>, update: &Option<Arc<Vec<u8>>>) {
        if op == Op::Set {
            *prop = update.clone();
        }
    }

    pub(crate) fn _write_insert(
        f: &mut fmt::Formatter<'_>,
        comma: &str,
        col: &str,
        value: &Option<Arc<Vec<u8>>>,
    ) -> fmt::Result {
        if value.is_some() {
            write!(f, "{comma}{col}: BLOB")
        } else {
            write!(f, "{comma}{col}: null")
        }
    }

    pub(crate) fn _write_update(
        f: &mut fmt::Formatter<'_>,
        comma: &str,
        col: &str,
        op: Op,
        value: &Option<Arc<Vec<u8>>>,
    ) -> fmt::Result {
        if op != Op::None && op != Op::Skip {
            if value.is_some() {
                write!(f, "{comma}{col}: {{{op}: BLOB}}")?;
            } else {
                write!(f, "{comma}{col}: {{{op}: null}}")?;
            }
        }
        Ok(())
    }
}

pub struct AccessorNotNullOrd<'a, I: Clone + Ord + Debug + Default> {
    pub(crate) op: &'a mut Op,
    pub(crate) val: &'a mut I,
    pub(crate) update: &'a mut I,
    pub(crate) _phantom: PhantomData<I>,
}
impl<'a, I: Clone + Ord + Debug + Default> AccessorNotNullOrd<'a, I> {
    pub fn get(&self) -> I {
        self.val.clone()
    }
    pub fn mark_for_skip(&mut self) {
        *self.op = Op::Skip;
    }
    pub fn mark_for_set(&mut self) {
        *self.op = Op::Set;
    }
    pub(crate) fn skip_and_empty(&mut self) {
        *self.op = Op::Skip;
        *self.update = I::default();
    }
    pub fn set(&mut self, val: I) {
        *self.op = Op::Set;
        *self.val = val.clone();
        *self.update = val
    }
    pub fn max(&mut self, val: I) {
        if val > *self.val {
            *self.val = val.clone();
        }
        if *self.op == Op::None {
            *self.op = Op::Max;
            *self.update = val;
        } else if *self.op == Op::Max {
            *self.update = val.max(self.update.clone());
        } else {
            panic!("operation error!");
        }
    }
    pub fn min(&mut self, val: I) {
        if val < *self.val {
            *self.val = val.clone();
        }
        if *self.op == Op::None {
            *self.op = Op::Min;
            *self.update = val;
        } else if *self.op == Op::Min {
            *self.update = val.min(self.update.clone());
        } else {
            panic!("operation error!");
        }
    }
    pub(crate) fn _set(op: Op, prop: &mut I, update: &I) {
        match op {
            Op::Set => {
                *prop = update.clone();
            }
            Op::Max => {
                if *update > *prop {
                    *prop = update.clone();
                }
            }
            Op::Min => {
                if *update < *prop {
                    *prop = update.clone();
                }
            }
            _ => {}
        }
    }

    pub(crate) fn _write_insert(f: &mut fmt::Formatter<'_>, comma: &str, col: &str, value: &I) -> fmt::Result {
        write!(f, "{comma}{col}: {:?}", value)
    }

    pub(crate) fn _write_update(
        f: &mut fmt::Formatter<'_>,
        comma: &str,
        col: &str,
        op: Op,
        value: &I,
    ) -> fmt::Result {
        if op != Op::None && op != Op::Skip {
            write!(f, "{comma}{col}: {{{op}: {:?}}}", value)?;
        }
        Ok(())
    }
}
pub struct AccessorNullOrd<'a, I: Clone + Ord + Debug> {
    pub(crate) op: &'a mut Op,
    pub(crate) val: &'a mut Option<I>,
    pub(crate) update: &'a mut Option<I>,
    pub(crate) _phantom: PhantomData<I>,
}
impl<'a, I: Clone + Ord + Debug> AccessorNullOrd<'a, I> {
    pub fn get(&self) -> Option<I> {
        self.val.clone()
    }
    pub fn mark_for_skip(&mut self) {
        *self.op = Op::Skip;
    }
    pub fn mark_for_set(&mut self) {
        *self.op = Op::Set;
    }
    pub fn set(&mut self, val: Option<I>) {
        *self.op = Op::Set;
        *self.val = val.clone();
        *self.update = val;
    }
    pub fn max(&mut self, val: I) {
        if self.val.is_none() || val > *self.val.as_ref().unwrap() {
            *self.val = Some(val.clone());
        }
        if *self.op == Op::None {
            *self.op = Op::Max;
            *self.update = Some(val);
        } else if *self.op == Op::Max {
            *self.update = Some(val.max(self.update.clone().unwrap()));
        } else {
            panic!("operation error!");
        }
    }
    pub fn min(&mut self, val: I) {
        if self.val.is_none() || val < *self.val.as_ref().unwrap() {
            *self.val = Some(val.clone());
        }
        if *self.op == Op::None {
            *self.op = Op::Min;
            *self.update = Some(val);
        } else if *self.op == Op::Min {
            *self.update = Some(val.min(self.update.clone().unwrap()));
        } else {
            panic!("operation error!");
        }
    }
    pub(crate) fn _set(op: Op, prop: &mut Option<I>, update: &Option<I>) {
        match op {
            Op::Set => {
                *prop = update.clone();
            }
            Op::Max => {
                if prop.is_none() || *update.as_ref().unwrap() > *prop.as_ref().unwrap() {
                    *prop = update.clone();
                }
            }
            Op::Min => {
                if prop.is_none() || *update.as_ref().unwrap() < *prop.as_ref().unwrap() {
                    *prop = update.clone();
                }
            }
            _ => {}
        }
    }

    pub(crate) fn _write_insert(
        f: &mut fmt::Formatter<'_>,
        comma: &str,
        col: &str,
        value: &Option<I>,
    ) -> fmt::Result {
        if let Some(value) = value {
            write!(f, "{comma}{col}: {:?}", value)
        } else {
            write!(f, "{comma}{col}: null")
        }
    }

    pub(crate) fn _write_update(
        f: &mut fmt::Formatter<'_>,
        comma: &str,
        col: &str,
        op: Op,
        value: &Option<I>,
    ) -> fmt::Result {
        if op != Op::None && op != Op::Skip {
            if let Some(value) = value {
                write!(f, "{comma}{col}: {{{op}: {:?}}}", value)?;
            } else {
                write!(f, "{comma}{col}: {{{op}: null}}")?;
            }
        }
        Ok(())
    }
}

pub struct AccessorNotNullNum<
    'a,
    I: Copy
        + Ord
        + BitAnd<Output = I>
        + BitOr<Output = I>
        + CheckedAdd
        + SaturatingAdd
        + CheckedSub
        + SaturatingSub
        + Debug
        + Display
        + ToValue,
> {
    pub(crate) op: &'a mut Op,
    pub(crate) val: &'a mut I,
    pub(crate) update: &'a mut I,
    pub(crate) _phantom: PhantomData<I>,
}
impl<
        'a,
        I: Copy
            + Ord
            + BitAnd<Output = I>
            + BitOr<Output = I>
            + CheckedAdd
            + SaturatingAdd
            + CheckedSub
            + SaturatingSub
            + Debug
            + Display
            + ToValue,
    > AccessorNotNullNum<'a, I>
{
    pub fn get(&self) -> I {
        *self.val
    }
    pub fn mark_for_skip(&mut self) {
        *self.op = Op::Skip;
    }
    pub fn mark_for_set(&mut self) {
        *self.op = Op::Set;
    }
    pub fn set(&mut self, val: I) {
        *self.op = Op::Set;
        *self.val = val;
        *self.update = val;
    }
    pub fn add(&mut self, val: I) -> Result<()> {
        let mut overflow = false;
        let base = *self.val;
        *self.val = self.val.checked_add(&val).unwrap_or_else(|| {
            overflow = true;
            self.val.saturating_add(&val)
        });
        if *self.op == Op::None {
            *self.op = Op::Add;
            *self.update = val;
        } else if *self.op == Op::Add {
            *self.update = *self.update + val;
        } else {
            panic!("operation error!");
        }
        ensure!(!overflow, "overflow (base {}, add {})", base, val);
        Ok(())
    }
    pub fn sub(&mut self, val: I) -> Result<()> {
        let mut overflow = false;
        let base = *self.val;
        *self.val = self.val.checked_sub(&val).unwrap_or_else(|| {
            overflow = true;
            self.val.saturating_sub(&val)
        });
        if *self.op == Op::None {
            *self.op = Op::Sub;
            *self.update = val;
        } else if *self.op == Op::Sub {
            *self.update = *self.update + val;
        } else {
            panic!("operation error!");
        }
        ensure!(!overflow, "overflow (base {}, sub {})", base, val);
        Ok(())
    }
    pub fn max(&mut self, val: I) {
        *self.val = val.max(*self.val);
        if *self.op == Op::None {
            *self.op = Op::Max;
            *self.update = val;
        } else if *self.op == Op::Max {
            *self.update = val.max(*self.update);
        } else {
            panic!("operation error!");
        }
    }
    pub fn min(&mut self, val: I) {
        *self.val = val.min(*self.val);
        if *self.op == Op::None {
            *self.op = Op::Min;
            *self.update = val;
        } else if *self.op == Op::Min {
            *self.update = val.min(*self.update);
        } else {
            panic!("operation error!");
        }
    }
    pub fn bit_and(&mut self, val: I) {
        *self.val = *self.val & val;
        if *self.op == Op::None {
            *self.op = Op::BitAnd;
            *self.update = val;
        } else if *self.op == Op::BitAnd {
            *self.update = *self.update & val;
        } else {
            panic!("operation error!");
        }
    }
    pub fn bit_or(&mut self, val: I) {
        *self.val = *self.val | val;
        if *self.op == Op::None {
            *self.op = Op::BitOr;
            *self.update = val;
        } else if *self.op == Op::BitOr {
            *self.update = *self.update | val;
        } else {
            panic!("operation error!");
        }
    }
    pub(crate) fn _set(op: Op, prop: &mut I, update: &I) {
        match op {
            Op::Set => {
                *prop = *update;
            }
            Op::Add => {
                *prop = prop.saturating_add(update);
            }
            Op::Sub => {
                *prop = prop.saturating_sub(update);
            }
            Op::Max => {
                if *update > *prop {
                    *prop = *update;
                }
            }
            Op::Min => {
                if *update < *prop {
                    *prop = *update;
                }
            }
            Op::BitAnd => {
                *prop = *prop & *update;
            }
            Op::BitOr => {
                *prop = *prop | *update;
            }
            _ => {}
        }
    }

    pub(crate) fn _write_insert(f: &mut fmt::Formatter<'_>, comma: &str, col: &str, value: &I) -> fmt::Result {
        write!(f, "{comma}{col}: {:?}", value)
    }

    pub(crate) fn _write_update(
        f: &mut fmt::Formatter<'_>,
        comma: &str,
        col: &str,
        op: Op,
        value: &I,
    ) -> fmt::Result {
        if op != Op::None && op != Op::Skip {
            write!(f, "{comma}{col}: {{{op}: {:?}}}", value)?;
        }
        Ok(())
    }
}
pub struct AccessorNullNum<
    'a,
    I: Copy
        + Ord
        + BitAnd<Output = I>
        + BitOr<Output = I>
        + CheckedAdd
        + SaturatingAdd
        + CheckedSub
        + SaturatingSub
        + Debug
        + Display
        + Default
        + ToValue,
> {
    pub(crate) op: &'a mut Op,
    pub(crate) val: &'a mut Option<I>,
    pub(crate) update: &'a mut Option<I>,
    pub(crate) _phantom: PhantomData<I>,
}
impl<
        'a,
        I: Copy
            + Ord
            + BitAnd<Output = I>
            + BitOr<Output = I>
            + CheckedAdd
            + SaturatingAdd
            + CheckedSub
            + SaturatingSub
            + Debug
            + Display
            + Default
            + ToValue,
    > AccessorNullNum<'a, I>
{
    pub fn get(&self) -> Option<I> {
        *self.val
    }
    pub fn mark_for_skip(&mut self) {
        *self.op = Op::Skip;
    }
    pub fn mark_for_set(&mut self) {
        *self.op = Op::Set;
    }
    pub fn set(&mut self, val: Option<I>) {
        *self.op = Op::Set;
        *self.val = val;
        *self.update = val;
    }
    pub fn add(&mut self, val: I) -> Result<()> {
        let mut overflow = false;
        let self_val = self.val.unwrap_or_default();
        *self.val = Some(self_val.checked_add(&val).unwrap_or_else(|| {
            overflow = true;
            self_val.saturating_add(&val)
        }));
        if *self.op == Op::None {
            *self.op = Op::Add;
            *self.update = Some(val);
        } else if *self.op == Op::Add {
            *self.update = Some(self.update.unwrap() + val);
        } else {
            panic!("operation error!");
        }
        ensure!(!overflow, "overflow (base {}, add {})", self_val, val);
        Ok(())
    }
    pub fn sub(&mut self, val: I) -> Result<()> {
        let mut overflow = false;
        let self_val = self.val.unwrap_or_default();
        *self.val = Some(self_val.checked_sub(&val).unwrap_or_else(|| {
            overflow = true;
            self_val.saturating_sub(&val)
        }));
        if *self.op == Op::None {
            *self.op = Op::Sub;
            *self.update = Some(val);
        } else if *self.op == Op::Sub {
            *self.update = Some(self.update.unwrap() + val);
        } else {
            panic!("operation error!");
        }
        ensure!(!overflow, "overflow (base {}, sub {})", self_val, val);
        Ok(())
    }
    pub fn max(&mut self, val: I) {
        *self.val = Some(val.max(self.val.unwrap_or_default()));
        if *self.op == Op::None {
            *self.op = Op::Max;
            *self.update = Some(val);
        } else if *self.op == Op::Max {
            *self.update = Some(val.max(self.update.unwrap_or_default()));
        } else {
            panic!("operation error!");
        }
    }
    pub fn min(&mut self, val: I) {
        *self.val = Some(val.min(self.val.unwrap_or_default()));
        if *self.op == Op::None {
            *self.op = Op::Min;
            *self.update = Some(val);
        } else if *self.op == Op::Min {
            *self.update = Some(val.min(self.update.unwrap_or_default()));
        } else {
            panic!("operation error!");
        }
    }
    pub fn bit_and(&mut self, val: I) {
        *self.val = Some(self.val.unwrap_or_default() & val);
        if *self.op == Op::None {
            *self.op = Op::BitAnd;
            *self.update = Some(val);
        } else if *self.op == Op::BitAnd {
            *self.update = Some(self.update.unwrap_or_default() & val);
        } else {
            panic!("operation error!");
        }
    }
    pub fn bit_or(&mut self, val: I) {
        *self.val = Some(self.val.unwrap_or_default() | val);
        if *self.op == Op::None {
            *self.op = Op::BitOr;
            *self.update = Some(val);
        } else if *self.op == Op::BitOr {
            *self.update = Some(self.update.unwrap_or_default() | val);
        } else {
            panic!("operation error!");
        }
    }
    pub(crate) fn _set(op: Op, prop: &mut Option<I>, update: &Option<I>) {
        match op {
            Op::Set => {
                *prop = *update;
            }
            Op::Add => {
                *prop = Some(
                    prop.unwrap_or_default()
                        .saturating_add(&update.unwrap_or_default()),
                );
            }
            Op::Sub => {
                *prop = Some(
                    prop.unwrap_or_default()
                        .saturating_sub(&update.unwrap_or_default()),
                );
            }
            Op::Max => {
                if prop.is_none() || *update.as_ref().unwrap() > *prop.as_ref().unwrap() {
                    *prop = *update;
                }
            }
            Op::Min => {
                if prop.is_none() || *update.as_ref().unwrap() < *prop.as_ref().unwrap() {
                    *prop = *update;
                }
            }
            Op::BitAnd => {
                *prop = Some(prop.unwrap_or_default() & update.unwrap_or_default());
            }
            Op::BitOr => {
                *prop = Some(prop.unwrap_or_default() | update.unwrap_or_default());
            }
            _ => {}
        }
    }

    pub(crate) fn _write_insert(
        f: &mut fmt::Formatter<'_>,
        comma: &str,
        col: &str,
        value: &Option<I>,
    ) -> fmt::Result {
        if let Some(value) = value {
            write!(f, "{comma}{col}: {:?}", value)
        } else {
            write!(f, "{comma}{col}: null")
        }
    }

    pub(crate) fn _write_update(
        f: &mut fmt::Formatter<'_>,
        comma: &str,
        col: &str,
        op: Op,
        value: &Option<I>,
    ) -> fmt::Result {
        if op != Op::None && op != Op::Skip {
            if let Some(value) = value {
                write!(f, "{comma}{col}: {{{op}: {:?}}}", value)?;
            } else {
                write!(f, "{comma}{col}: {{{op}: null}}")?;
            }
        }
        Ok(())
    }
}

pub struct AccessorNotNullFloat<'a, I: Copy + PartialOrd + Float + Debug + Display + ToValue> {
    pub(crate) op: &'a mut Op,
    pub(crate) val: &'a mut I,
    pub(crate) update: &'a mut I,
    pub(crate) _phantom: PhantomData<I>,
}
impl<'a, I: Copy + PartialOrd + Float + Debug + Display + ToValue> AccessorNotNullFloat<'a, I> {
    pub fn get(&self) -> I {
        *self.val
    }
    pub fn mark_for_skip(&mut self) {
        *self.op = Op::Skip;
    }
    pub fn mark_for_set(&mut self) {
        *self.op = Op::Set;
    }
    pub fn set(&mut self, val: I) {
        *self.op = Op::Set;
        *self.val = val;
        *self.update = val;
    }
    pub fn add(&mut self, val: I) {
        *self.val = *self.val + val;
        if *self.op == Op::None {
            *self.op = Op::Add;
            *self.update = val;
        } else if *self.op == Op::Add {
            *self.update = *self.update + val;
        } else {
            panic!("operation error!");
        }
    }
    pub fn sub(&mut self, val: I) {
        *self.val = *self.val - val;
        if *self.op == Op::None {
            *self.op = Op::Sub;
            *self.update = val;
        } else if *self.op == Op::Sub {
            *self.update = *self.update + val;
        } else {
            panic!("operation error!");
        }
    }
    pub fn max(&mut self, val: I) {
        *self.val = val.max(*self.val);
        if *self.op == Op::None {
            *self.op = Op::Max;
            *self.update = val;
        } else if *self.op == Op::Max {
            *self.update = val.max(*self.update);
        } else {
            panic!("operation error!");
        }
    }
    pub fn min(&mut self, val: I) {
        *self.val = val.min(*self.val);
        if *self.op == Op::None {
            *self.op = Op::Min;
            *self.update = val;
        } else if *self.op == Op::Min {
            *self.update = val.min(*self.update);
        } else {
            panic!("operation error!");
        }
    }
    pub(crate) fn _set(op: Op, prop: &mut I, update: &I) {
        match op {
            Op::Set => {
                *prop = *update;
            }
            Op::Add => {
                *prop = *prop + *update;
            }
            Op::Sub => {
                *prop = *prop - *update;
            }
            Op::Max => {
                if *update > *prop {
                    *prop = *update;
                }
            }
            Op::Min => {
                if *update < *prop {
                    *prop = *update;
                }
            }
            _ => {}
        }
    }

    pub(crate) fn _write_insert(f: &mut fmt::Formatter<'_>, comma: &str, col: &str, value: &I) -> fmt::Result {
        write!(f, "{comma}{col}: {:?}", value)
    }

    pub(crate) fn _write_update(
        f: &mut fmt::Formatter<'_>,
        comma: &str,
        col: &str,
        op: Op,
        value: &I,
    ) -> fmt::Result {
        if op != Op::None && op != Op::Skip {
            write!(f, "{comma}{col}: {{{op}: {:?}}}", value)?;
        }
        Ok(())
    }
}
pub struct AccessorNullFloat<'a, I: Copy + PartialOrd + Float + Debug + Display + Default + ToValue>
{
    pub(crate) op: &'a mut Op,
    pub(crate) val: &'a mut Option<I>,
    pub(crate) update: &'a mut Option<I>,
    pub(crate) _phantom: PhantomData<I>,
}
impl<'a, I: Copy + PartialOrd + Float + Debug + Display + Default + ToValue>
    AccessorNullFloat<'a, I>
{
    pub fn get(&self) -> Option<I> {
        *self.val
    }
    pub fn mark_for_skip(&mut self) {
        *self.op = Op::Skip;
    }
    pub fn mark_for_set(&mut self) {
        *self.op = Op::Set;
    }
    pub fn set(&mut self, val: Option<I>) {
        *self.op = Op::Set;
        *self.val = val;
        *self.update = val;
    }
    pub fn add(&mut self, val: I) {
        let self_val = self.val.unwrap_or_default();
        *self.val = Some(self_val + val);
        if *self.op == Op::None {
            *self.op = Op::Add;
            *self.update = Some(val);
        } else if *self.op == Op::Add {
            *self.update = Some(self.update.unwrap() + val);
        } else {
            panic!("operation error!");
        }
    }
    pub fn sub(&mut self, val: I) {
        let self_val = self.val.unwrap_or_default();
        *self.val = Some(self_val - val);
        if *self.op == Op::None {
            *self.op = Op::Sub;
            *self.update = Some(val);
        } else if *self.op == Op::Sub {
            *self.update = Some(self.update.unwrap() + val);
        } else {
            panic!("operation error!");
        }
    }
    pub fn max(&mut self, val: I) {
        *self.val = Some(val.max(self.val.unwrap_or_default()));
        if *self.op == Op::None {
            *self.op = Op::Max;
            *self.update = Some(val);
        } else if *self.op == Op::Max {
            *self.update = Some(val.max(self.update.unwrap_or_default()));
        } else {
            panic!("operation error!");
        }
    }
    pub fn min(&mut self, val: I) {
        *self.val = Some(val.min(self.val.unwrap_or_default()));
        if *self.op == Op::None {
            *self.op = Op::Min;
            *self.update = Some(val);
        } else if *self.op == Op::Min {
            *self.update = Some(val.min(self.update.unwrap_or_default()));
        } else {
            panic!("operation error!");
        }
    }
    pub(crate) fn _set(op: Op, prop: &mut Option<I>, update: &Option<I>) {
        match op {
            Op::Set => {
                *prop = *update;
            }
            Op::Add => {
                *prop = Some(prop.unwrap_or_default() + update.unwrap_or_default());
            }
            Op::Sub => {
                *prop = Some(prop.unwrap_or_default() - update.unwrap_or_default());
            }
            Op::Max => {
                if prop.is_none() || *update.as_ref().unwrap() > *prop.as_ref().unwrap() {
                    *prop = *update;
                }
            }
            Op::Min => {
                if prop.is_none() || *update.as_ref().unwrap() < *prop.as_ref().unwrap() {
                    *prop = *update;
                }
            }
            _ => {}
        }
    }

    pub(crate) fn _write_insert(
        f: &mut fmt::Formatter<'_>,
        comma: &str,
        col: &str,
        value: &Option<I>,
    ) -> fmt::Result {
        if let Some(value) = value {
            write!(f, "{comma}{col}: {:?}", value)
        } else {
            write!(f, "{comma}{col}: null")
        }
    }

    pub(crate) fn _write_update(
        f: &mut fmt::Formatter<'_>,
        comma: &str,
        col: &str,
        op: Op,
        value: &Option<I>,
    ) -> fmt::Result {
        if op != Op::None && op != Op::Skip {
            if let Some(value) = value {
                write!(f, "{comma}{col}: {{{op}: {:?}}}", value)?;
            } else {
                write!(f, "{comma}{col}: {{{op}: null}}")?;
            }
        }
        Ok(())
    }
}

pub struct AccessorHasOne<'a, I>
where
    I: crate::misc::Updater,
{
    pub(crate) name: &'static str,
    pub(crate) val: &'a mut Option<Vec<I>>,
}
impl<'a, I> AccessorHasOne<'a, I>
where
    I: crate::misc::Updater,
{
    pub fn get(&mut self) -> Option<&mut I> {
        let name = self.name;
        self.val
            .as_mut()
            .unwrap_or_else(|| panic!("{} is not fetched.", name))
            .last_mut()
    }
    pub fn set(&mut self, val: I) {
        if self.val.is_none() {
            *self.val = Some(Vec::new());
        }
        let list = self.val.as_mut().unwrap();
        if let Some(old) = list.last_mut() {
            old.mark_for_delete();
        }
        list.push(val);
    }
}

pub struct AccessorHasMany<'a, I>
where
    I: crate::misc::Updater,
{
    pub(crate) name: &'static str,
    pub(crate) val: &'a mut Option<Vec<I>>,
}
impl<'a, I> AccessorHasMany<'a, I>
where
    I: crate::misc::Updater,
{
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut I> {
        let name = self.name;
        self.val
            .as_mut()
            .unwrap_or_else(|| panic!("{} is not fetched.", name))
            .iter_mut()
            .filter(|v| !v.will_be_deleted())
    }
    pub fn push(&mut self, val: I) {
        if self.val.is_none() {
            *self.val = Some(Vec::new());
        }
        self.val.as_mut().unwrap().push(val);
    }
    pub fn take(&mut self) -> Option<Vec<I>> {
        self.val.take()
    }
    pub fn replace(&mut self, vec: Vec<I>) -> Option<Vec<I>> {
        self.val.replace(vec)
    }
}
@{-"\n"}@