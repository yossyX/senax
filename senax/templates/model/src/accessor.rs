// This code is auto-generated and will always be overwritten.

#![allow(dead_code)]

use crate::misc::IntoJson;
use anyhow::{ensure, Result};
use derive_more::Display;
use log::kv::ToValue;
use num_traits::{CheckedAdd, CheckedSub, Float, SaturatingAdd, SaturatingSub};
use rust_decimal::Decimal;
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::fmt::Debug;
use std::ops::{BitAnd, BitOr};
use std::{fmt, fmt::Display, marker::PhantomData};

#[derive(
    Serialize_repr, Deserialize_repr, Eq, PartialEq, Clone, Copy, Debug, Display, Default, Hash,
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
    pub fn get_sql(&self, col: &str, nullable: bool, ps: &str) -> String {
        match self {
            Op::None => "".to_string(),
            Op::Skip => "".to_string(),
            Op::Set => format!("{}={}", col, ps),
            Op::Add if nullable => format!("{}=IFNULL({}, 0)+?", col, col),
            Op::Add => format!("{}={}+?", col, col),
            Op::Sub if nullable => format!("{}=IFNULL({}, 0)-?", col, col),
            Op::Sub => format!("{}={}-?", col, col),
            Op::Max if nullable => format!("{}=IF(IFNULL({}, ?)<?,?,{})", col, col, col),
            Op::Max => format!("{}=IF({}<?,?,{})", col, col, col),
            Op::Min if nullable => format!("{}=IF(IFNULL({}, ?)>?,?,{})", col, col, col),
            Op::Min => format!("{}=IF({}>?,?,{})", col, col, col),
            Op::BitAnd => format!("{}={}&?", col, col),
            Op::BitOr if nullable => format!("{}=IFNULL({}, 0)|?", col, col),
            Op::BitOr => format!("{}={}|?", col, col),
        }
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

macro_rules! assignment_sql_no_cache_update {
    ( $obj:ident, $vec:ident, $col:ident, $name:expr, $nullable:expr, $ph:expr ) => {
        if $obj._op.$col != Op::None && $obj._op.$col != Op::Skip {
            $vec.push($obj._op.$col.get_sql($name, $nullable, $ph));
        }
    };
}
pub(crate) use assignment_sql_no_cache_update;

macro_rules! assignment_sql {
    ( $obj:ident, $vec:ident, $col:ident, $name:expr, $nullable:expr, $update_cache: ident, $ph:expr ) => {
        if $obj._op.$col != Op::None && $obj._op.$col != Op::Skip {
            $vec.push($obj._op.$col.get_sql($name, $nullable, $ph));
            $update_cache = true;
        }
    };
}
pub(crate) use assignment_sql;

macro_rules! bind_sql {
    ( $obj:ident, $query:ident, $col:ident, $nullable:expr ) => {
        for _n in 0..$obj._op.$col.get_bind_num($nullable) {
            $query = $query.bind(&$obj._update.$col);
        }
    };
}
pub(crate) use bind_sql;

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
    pub fn is_zero_len<T>(val: &Vec<T>) -> bool {
        val.is_empty()
    }
    pub fn is_zero_json_len<T>(val: &sqlx::types::Json<Vec<T>>) -> bool {
        val.is_empty()
    }
    pub fn is_default_json<T: Default + PartialEq>(val: &sqlx::types::Json<T>) -> bool {
        **val == T::default()
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

pub struct AccessorPrimary<'a, I: Clone + Debug, O> {
    pub(crate) val: &'a I,
    pub(crate) _phantom: PhantomData<O>,
}
impl<'a, I: Clone + Debug, O> AccessorPrimary<'a, I, O>
where
    I: From<O>,
    O: From<I>,
{
    pub fn get(&self) -> O {
        self.val.clone().into()
    }

    pub(crate) fn _write_insert(f: &mut fmt::Formatter<'_>, col: &str, value: &I) -> fmt::Result {
        write!(f, "{}={:?}, ", col, value)
    }
}

pub struct AccessorNotNull<'a, I: Clone + Debug, O> {
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

    pub(crate) fn _write_insert(f: &mut fmt::Formatter<'_>, col: &str, value: &I) -> fmt::Result {
        write!(f, "{}={:?}, ", col, value)
    }

    pub(crate) fn _write_update(
        f: &mut fmt::Formatter<'_>,
        col: &str,
        op: Op,
        value: &I,
    ) -> fmt::Result {
        if op != Op::None && op != Op::Skip {
            write!(f, "{}={}({:?}), ", col, op, value)?;
        }
        Ok(())
    }
}
pub struct AccessorNull<'a, I: Clone + Debug, O> {
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
        col: &str,
        value: &Option<I>,
    ) -> fmt::Result {
        if value.is_none() {
            write!(f, "{}=null, ", col)
        } else {
            write!(f, "{}={:?}, ", col, value.as_ref().unwrap())
        }
    }

    pub(crate) fn _write_update(
        f: &mut fmt::Formatter<'_>,
        col: &str,
        op: Op,
        value: &Option<I>,
    ) -> fmt::Result {
        if op != Op::None && op != Op::Skip {
            if value.is_none() {
                write!(f, "{}={}(null), ", col, op)?;
            } else {
                write!(f, "{}={}({:?}), ", col, op, value.as_ref().unwrap())?;
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

    pub(crate) fn _write_insert(f: &mut fmt::Formatter<'_>, col: &str, value: &i8) -> fmt::Result {
        write!(f, "{}={:?}, ", col, value)
    }

    pub(crate) fn _write_update(
        f: &mut fmt::Formatter<'_>,
        col: &str,
        op: Op,
        value: &i8,
    ) -> fmt::Result {
        if op != Op::None && op != Op::Skip {
            write!(f, "{}={}({:?}), ", col, op, value)?;
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
        col: &str,
        value: &Option<i8>,
    ) -> fmt::Result {
        if value.is_none() {
            write!(f, "{}=null, ", col)
        } else {
            write!(f, "{}={:?}, ", col, value.as_ref().unwrap())
        }
    }

    pub(crate) fn _write_update(
        f: &mut fmt::Formatter<'_>,
        col: &str,
        op: Op,
        value: &Option<i8>,
    ) -> fmt::Result {
        if op != Op::None && op != Op::Skip {
            if value.is_none() {
                write!(f, "{}={}(null), ", col, op)?;
            } else {
                write!(f, "{}={}({:?}), ", col, op, value.as_ref().unwrap())?;
            }
        }
        Ok(())
    }
}

pub struct AccessorNotNullString<'a> {
    pub(crate) op: &'a mut Op,
    pub(crate) val: &'a mut String,
    pub(crate) update: &'a mut String,
    pub(crate) _phantom: PhantomData<String>,
}
impl<'a> AccessorNotNullString<'a> {
    pub fn get(&self) -> &str {
        &*self.val
    }
    pub fn set(&mut self, val: &str) {
        *self.op = Op::Set;
        *self.val = val.to_string();
        *self.update = val.to_string();
    }
    pub(crate) fn _set(op: Op, prop: &mut String, update: &str) {
        if op == Op::Set {
            *prop = update.to_owned();
        }
    }

    pub(crate) fn _write_insert(
        f: &mut fmt::Formatter<'_>,
        col: &str,
        value: &String,
    ) -> fmt::Result {
        write!(f, "{}={:?}, ", col, value)
    }

    pub(crate) fn _write_update(
        f: &mut fmt::Formatter<'_>,
        col: &str,
        op: Op,
        value: &String,
    ) -> fmt::Result {
        if op != Op::None && op != Op::Skip {
            write!(f, "{}={}({:?}), ", col, op, value)?;
        }
        Ok(())
    }
}
pub struct AccessorNullString<'a> {
    pub(crate) op: &'a mut Op,
    pub(crate) val: &'a mut Option<String>,
    pub(crate) update: &'a mut Option<String>,
    pub(crate) _phantom: PhantomData<String>,
}
impl<'a> AccessorNullString<'a> {
    pub fn get(&self) -> Option<&str> {
        self.val.as_deref()
    }
    pub fn set(&mut self, val: Option<&str>) {
        *self.op = Op::Set;
        *self.val = val.map(|v| v.to_string());
        *self.update = val.map(|v| v.to_string());
    }
    pub(crate) fn _set(op: Op, prop: &mut Option<String>, update: &Option<String>) {
        if op == Op::Set {
            *prop = update.clone();
        }
    }

    pub(crate) fn _write_insert(
        f: &mut fmt::Formatter<'_>,
        col: &str,
        value: &Option<String>,
    ) -> fmt::Result {
        if value.is_none() {
            write!(f, "{}=null, ", col)
        } else {
            write!(f, "{}={:?}, ", col, value.as_ref().unwrap())
        }
    }

    pub(crate) fn _write_update(
        f: &mut fmt::Formatter<'_>,
        col: &str,
        op: Op,
        value: &Option<String>,
    ) -> fmt::Result {
        if op != Op::None && op != Op::Skip {
            if value.is_none() {
                write!(f, "{}={}(null), ", col, op)?;
            } else {
                write!(f, "{}={}({:?}), ", col, op, value.as_ref().unwrap())?;
            }
        }
        Ok(())
    }
}

pub struct AccessorNotNullRef<'a, I: Clone + Debug> {
    pub(crate) op: &'a mut Op,
    pub(crate) val: &'a mut I,
    pub(crate) update: &'a mut I,
    pub(crate) _phantom: PhantomData<I>,
}
impl<'a, I: Clone + Debug> AccessorNotNullRef<'a, I> {
    pub fn get(&self) -> &I {
        self.val
    }
    pub fn set(&mut self, val: I) {
        *self.op = Op::Set;
        *self.val = val.clone();
        *self.update = val;
    }
    pub(crate) fn _set(op: Op, prop: &mut I, update: &I) {
        if op == Op::Set {
            *prop = update.clone();
        }
    }

    pub(crate) fn _write_insert(f: &mut fmt::Formatter<'_>, col: &str, value: &I) -> fmt::Result {
        write!(f, "{}={:?}, ", col, value)
    }

    pub(crate) fn _write_update(
        f: &mut fmt::Formatter<'_>,
        col: &str,
        op: Op,
        value: &I,
    ) -> fmt::Result {
        if op != Op::None && op != Op::Skip {
            write!(f, "{}={}({:?}), ", col, op, value)?;
        }
        Ok(())
    }
}
pub struct AccessorNullRef<'a, I: Clone + Debug> {
    pub(crate) op: &'a mut Op,
    pub(crate) val: &'a mut Option<I>,
    pub(crate) update: &'a mut Option<I>,
    pub(crate) _phantom: PhantomData<I>,
}
impl<'a, I: Clone + Debug> AccessorNullRef<'a, I> {
    pub fn get(&self) -> Option<&I> {
        self.val.as_ref()
    }
    pub fn set(&mut self, val: Option<I>) {
        *self.op = Op::Set;
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
        col: &str,
        value: &Option<I>,
    ) -> fmt::Result {
        if value.is_none() {
            write!(f, "{}=null, ", col)
        } else {
            write!(f, "{}={:?}, ", col, value.as_ref().unwrap())
        }
    }

    pub(crate) fn _write_update(
        f: &mut fmt::Formatter<'_>,
        col: &str,
        op: Op,
        value: &Option<I>,
    ) -> fmt::Result {
        if op != Op::None && op != Op::Skip {
            if value.is_none() {
                write!(f, "{}={}(null), ", col, op)?;
            } else {
                write!(f, "{}={}({:?}), ", col, op, value.as_ref().unwrap())?;
            }
        }
        Ok(())
    }
}

pub struct AccessorNotNullJson<'a, I: Clone + Debug> {
    pub(crate) op: &'a mut Op,
    pub(crate) val: &'a mut sqlx::types::Json<I>,
    pub(crate) update: &'a mut sqlx::types::Json<I>,
    pub(crate) _phantom: PhantomData<I>,
}
impl<'a, I: Clone + Debug> AccessorNotNullJson<'a, I> {
    pub fn get(&self) -> &I {
        self.val.as_ref()
    }
    pub fn set(&mut self, val: I) {
        *self.op = Op::Set;
        let val = val._into_json();
        *self.val = val.clone();
        *self.update = val;
    }
    pub(crate) fn _set(op: Op, prop: &mut sqlx::types::Json<I>, update: &sqlx::types::Json<I>) {
        if op == Op::Set {
            *prop = update.clone();
        }
    }

    pub(crate) fn _write_insert(
        f: &mut fmt::Formatter<'_>,
        col: &str,
        value: &sqlx::types::Json<I>,
    ) -> fmt::Result {
        write!(f, "{}={:?}, ", col, value)
    }

    pub(crate) fn _write_update(
        f: &mut fmt::Formatter<'_>,
        col: &str,
        op: Op,
        value: &sqlx::types::Json<I>,
    ) -> fmt::Result {
        if op != Op::None && op != Op::Skip {
            write!(f, "{}={}({:?}), ", col, op, value)?;
        }
        Ok(())
    }
}
pub struct AccessorNullJson<'a, I: Clone + Debug> {
    pub(crate) op: &'a mut Op,
    pub(crate) val: &'a mut Option<sqlx::types::Json<I>>,
    pub(crate) update: &'a mut Option<sqlx::types::Json<I>>,
    pub(crate) _phantom: PhantomData<I>,
}
impl<'a, I: Clone + Debug> AccessorNullJson<'a, I> {
    pub fn get(&self) -> Option<&I> {
        self.val.as_deref()
    }
    pub fn set(&mut self, val: Option<I>) {
        *self.op = Op::Set;
        let val = val.map(|v| v._into_json());
        *self.val = val.clone();
        *self.update = val;
    }
    pub(crate) fn _set(
        op: Op,
        prop: &mut Option<sqlx::types::Json<I>>,
        update: &Option<sqlx::types::Json<I>>,
    ) {
        if op == Op::Set {
            *prop = update.clone();
        }
    }

    pub(crate) fn _write_insert(
        f: &mut fmt::Formatter<'_>,
        col: &str,
        value: &Option<sqlx::types::Json<I>>,
    ) -> fmt::Result {
        if value.is_none() {
            write!(f, "{}=null, ", col)
        } else {
            write!(f, "{}={:?}, ", col, value.as_ref().unwrap())
        }
    }

    pub(crate) fn _write_update(
        f: &mut fmt::Formatter<'_>,
        col: &str,
        op: Op,
        value: &Option<sqlx::types::Json<I>>,
    ) -> fmt::Result {
        if op != Op::None && op != Op::Skip {
            if value.is_none() {
                write!(f, "{}={}(null), ", col, op)?;
            } else {
                write!(f, "{}={}({:?}), ", col, op, value.as_ref().unwrap())?;
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
    pub fn skip_update(&mut self) {
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

    pub(crate) fn _write_insert(f: &mut fmt::Formatter<'_>, col: &str, value: &I) -> fmt::Result {
        write!(f, "{}={:?}, ", col, value)
    }

    pub(crate) fn _write_update(
        f: &mut fmt::Formatter<'_>,
        col: &str,
        op: Op,
        value: &I,
    ) -> fmt::Result {
        if op != Op::None && op != Op::Skip {
            write!(f, "{}={}({:?}), ", col, op, value)?;
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
        col: &str,
        value: &Option<I>,
    ) -> fmt::Result {
        if value.is_none() {
            write!(f, "{}=null, ", col)
        } else {
            write!(f, "{}={:?}, ", col, value.as_ref().unwrap())
        }
    }

    pub(crate) fn _write_update(
        f: &mut fmt::Formatter<'_>,
        col: &str,
        op: Op,
        value: &Option<I>,
    ) -> fmt::Result {
        if op != Op::None && op != Op::Skip {
            if value.is_none() {
                write!(f, "{}={}(null), ", col, op)?;
            } else {
                write!(f, "{}={}({:?}), ", col, op, value.as_ref().unwrap())?;
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
        ensure!(!overflow, format!("overflow (base {}, add {})", base, val));
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
        ensure!(!overflow, format!("overflow (base {}, sub {})", base, val));
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

    pub(crate) fn _write_insert(f: &mut fmt::Formatter<'_>, col: &str, value: &I) -> fmt::Result {
        write!(f, "{}={:?}, ", col, value)
    }

    pub(crate) fn _write_update(
        f: &mut fmt::Formatter<'_>,
        col: &str,
        op: Op,
        value: &I,
    ) -> fmt::Result {
        if op != Op::None && op != Op::Skip {
            write!(f, "{}={}({:?}), ", col, op, value)?;
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
        ensure!(
            !overflow,
            format!("overflow (base {}, add {})", self_val, val)
        );
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
        ensure!(
            !overflow,
            format!("overflow (base {}, sub {})", self_val, val)
        );
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
        col: &str,
        value: &Option<I>,
    ) -> fmt::Result {
        if value.is_none() {
            write!(f, "{}=null, ", col)
        } else {
            write!(f, "{}={:?}, ", col, value.as_ref().unwrap())
        }
    }

    pub(crate) fn _write_update(
        f: &mut fmt::Formatter<'_>,
        col: &str,
        op: Op,
        value: &Option<I>,
    ) -> fmt::Result {
        if op != Op::None && op != Op::Skip {
            if value.is_none() {
                write!(f, "{}={}(null), ", col, op)?;
            } else {
                write!(f, "{}={}({:?}), ", col, op, value.as_ref().unwrap())?;
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

    pub(crate) fn _write_insert(f: &mut fmt::Formatter<'_>, col: &str, value: &I) -> fmt::Result {
        write!(f, "{}={:?}, ", col, value)
    }

    pub(crate) fn _write_update(
        f: &mut fmt::Formatter<'_>,
        col: &str,
        op: Op,
        value: &I,
    ) -> fmt::Result {
        if op != Op::None && op != Op::Skip {
            write!(f, "{}={}({:?}), ", col, op, value)?;
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
        col: &str,
        value: &Option<I>,
    ) -> fmt::Result {
        if value.is_none() {
            write!(f, "{}=null, ", col)
        } else {
            write!(f, "{}={:?}, ", col, value.as_ref().unwrap())
        }
    }

    pub(crate) fn _write_update(
        f: &mut fmt::Formatter<'_>,
        col: &str,
        op: Op,
        value: &Option<I>,
    ) -> fmt::Result {
        if op != Op::None && op != Op::Skip {
            if value.is_none() {
                write!(f, "{}={}(null), ", col, op)?;
            } else {
                write!(f, "{}={}({:?}), ", col, op, value.as_ref().unwrap())?;
            }
        }
        Ok(())
    }
}

pub struct AccessorOneToOne<'a, I> {
    pub(crate) name: &'static str,
    pub(crate) val: &'a mut Option<Option<Box<I>>>,
}
impl<'a, I> AccessorOneToOne<'a, I> {
    pub fn get(&mut self) -> Option<&mut Box<I>> {
        let name = self.name;
        self.val
            .as_mut()
            .unwrap_or_else(|| panic!("{} is not fetched.", name))
            .as_mut()
    }
    pub fn set(&mut self, val: I) {
        *self.val = Some(Some(Box::new(val)));
    }
}

pub struct AccessorMany<'a, I> {
    pub(crate) name: &'static str,
    pub(crate) val: &'a mut Option<Vec<I>>,
}
impl<'a, I> AccessorMany<'a, I>
where
    I: crate::misc::ForUpdateTr,
{
    pub fn iter(&mut self) -> impl Iterator<Item = &mut I> {
        let name = self.name;
        self.val
            .as_mut()
            .unwrap_or_else(|| panic!("{} is not fetched.", name))
            .iter_mut()
            .filter(|v| !v._will_be_deleted())
    }
    pub fn insert(&mut self, val: I) {
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