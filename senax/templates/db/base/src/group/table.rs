use ahash::AHasher;
use anyhow::{Context as _, Result};
use arc_swap::ArcSwapOption;
use bytes::Bytes;
use core::option::Option;
use crossbeam::queue::SegQueue;
use derive_more::Display;
use fxhash::{FxHashMap, FxHasher64};
use once_cell::sync::OnceCell;
use senax_common::cache::db_cache::{CacheVal, HashVal};
use senax_common::cache::msec::MSec;
use senax_common::cache::calc_mem_size;
use senax_common::ShardId;
use senax_common::{types::blob::*, types::geo_point::*, types::point::*, SqlColumns};
use senax_encoder::{Pack, Unpack};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::boxed::Box;
use std::collections::{BTreeMap, HashMap};
use std::convert::TryInto;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::SystemTime;
use std::vec::Vec;
use std::{cmp, fmt};
use strum::{EnumMessage, EnumString, FromRepr, IntoStaticStr};
use tokio::sync::{Mutex, RwLock};
use zstd::{decode_all, encode_all};
@%- if !config.force_disable_cache %@

use crate::cache::Cache;
@% endif %@
use crate::connection::{DbArguments, DbConn, DbRow, DbType};
use crate::misc::ToJsonRawValue as _;
use crate::misc::{BindValue, Updater, Size, TrashMode};
use crate::models::USE_FAST_CACHE;
use crate::{self as db, accessor::*, BULK_INSERT_MAX_SIZE, IN_CONDITION_LIMIT};
@%- if !config.exclude_from_domain %@
use base_domain as domain;
#[allow(unused_imports)]
use domain::value_objects;
@%- endif %@
@%- for mod_name in def.relation_mods() %@
use crate::models::@{ mod_name[0]|ident }@::@{ mod_name[1]|ident }@ as rel_@{ mod_name[0] }@_@{ mod_name[1] }@;
@%- endfor %@
@%- for (name, rel_def) in def.belongs_to_outer_db() %@
use db_@{ rel_def.db()|snake }@::models::@{ rel_def.get_group_mod_path() }@ as rel_@{ rel_def.get_group_mod_name() }@;
@%- endfor %@

static PRIMARY_TYPE_ID: u64 = @{ def.get_type_id("PRIMARY_TYPE_ID") }@;
static VERSION_TYPE_ID: u64 = @{ def.get_type_id("VERSION_TYPE_ID") }@;
static CACHE_SYNC_TYPE_ID: u64 = @{ def.get_type_id("CACHE_SYNC_TYPE_ID") }@;
static CACHE_TYPE_ID: u64 = @{ def.get_type_id("CACHE_TYPE_ID") }@;
pub static COL_KEY_TYPE_ID: u64 = @{ def.get_type_id("COL_KEY_TYPE_ID") }@;

#[derive(Pack, Unpack, Clone, Debug)]
pub enum CacheOp {
    #[senax(id = 1)]
    None,
@%- if def.act_as_job_queue() %@
    #[senax(id = 2)]
    Queued,
@%- endif %@
@%- if !config.force_disable_cache && !def.use_clear_whole_cache() && !def.act_as_job_queue() %@
    #[senax(id = 3)]
    Insert {
        #[senax(id = 1)]
        overwrite: bool,
        #[senax(id = 2)]
        shard_id: ShardId,
        #[senax(id = 3)]
        data: Data,
@{- def.relations_one_cache(false)|fmt_rel_join("
        _{rel_name}: Option<Vec<rel_{class_mod}::CacheOp>>,", "") }@
@{- def.relations_many_cache(false)|fmt_rel_join("
        _{rel_name}: Option<Vec<rel_{class_mod}::CacheOp>>,", "") }@
    },
    #[senax(id = 4)]
    BulkInsert {
        #[senax(id = 1)]
        replace: bool,
        #[senax(id = 2)]
        overwrite: bool,
        #[senax(id = 3)]
        ignore: bool,
        #[senax(id = 4)]
        shard_id: ShardId,
        #[senax(id = 5)]
        list: Vec<ForInsert>,
    },
    #[senax(id = 5)]
    Update {
        #[senax(id = 1)]
        id: InnerPrimary,
        #[senax(id = 2)]
        shard_id: ShardId,
        #[senax(id = 3)]
        update: Data,
        #[senax(id = 4)]
        op: OpData,
@{- def.relations_one_cache(false)|fmt_rel_join("
        _{rel_name}: Option<Vec<rel_{class_mod}::CacheOp>>,", "") }@
@{- def.relations_many_cache(false)|fmt_rel_join("
        _{rel_name}: Option<Vec<rel_{class_mod}::CacheOp>>,", "") }@
    },
    #[senax(id = 6)]
    UpdateMany {
        #[senax(id = 1)]
        ids: Vec<InnerPrimary>,
        #[senax(id = 2)]
        shard_id: ShardId,
        #[senax(id = 3)]
        update: Data,
        #[senax(id = 4)]
        data_list: Vec<Data>,
        #[senax(id = 5)]
        op: OpData,
    },
    #[senax(id = 7)]
    BulkUpsert {
        #[senax(id = 1)]
        shard_id: ShardId,
        #[senax(id = 2)]
        data_list: Vec<Data>,
        #[senax(id = 3)]
        update: Data,
        #[senax(id = 4)]
        op: OpData,
    },
    #[senax(id = 8)]
    Delete {
        #[senax(id = 1)]
        id: InnerPrimary,
        #[senax(id = 2)]
        shard_id: ShardId,
    },
    #[senax(id = 9)]
    DeleteMany {
        #[senax(id = 1)]
        ids: Vec<InnerPrimary>,
        #[senax(id = 2)]
        shard_id: ShardId,
    },
    #[senax(id = 10)]
    DeleteAll {
        #[senax(id = 1)]
        shard_id: ShardId,
    },
    #[senax(id = 11)]
    Cascade {
        #[senax(id = 1)]
        ids: Vec<InnerPrimary>,
        #[senax(id = 2)]
        shard_id: ShardId,
    },
    #[senax(id = 12)]
    Invalidate {
        #[senax(id = 1)]
        id: InnerPrimary,
        #[senax(id = 2)]
        shard_id: ShardId,
    },
    #[senax(id = 13)]
    Notify {
        #[senax(id = 1)]
        id: InnerPrimary,
        #[senax(id = 2)]
        shard_id: ShardId,
    },
@%- for (mod_name, rel_name, local, val, val2, rel) in def.relations_on_delete_not_cascade() %@
    Reset@{ rel_name|pascal }@@{ val|pascal }@ {
        #[senax(id = 1)]
        ids: Vec<InnerPrimary>,
        #[senax(id = 2)]
        shard_id: ShardId,
    },
@%- endfor %@
@%- endif %@
    #[senax(id = 14)]
    InvalidateAll,
}

#[cfg(not(feature="cache_update_only"))]
impl CacheOp {
    @%- if !config.force_disable_cache && !def.use_clear_whole_cache() && !def.act_as_job_queue() %@
    pub fn update(mut obj: CacheData, update: &Data, op: &OpData) -> CacheData {
        @{- def.cache_cols_except_primary()|fmt_join("
        Accessor{accessor_with_sep_type}::_set(op.{var}, &mut obj.{var}, &update.{var});", "") }@
        obj
    }
    @%- endif %@
    pub fn wrap(self) -> crate::CacheOp {
        crate::CacheOp::@{ group_name|to_pascal_name }@(crate::models::@{ group_name|snake|ident }@::CacheOp::@{ model_name|to_pascal_name }@(self))
    }
}

@% for (name, column_def) in def.id_except_auto_increment() -%@
#[derive(Deserialize, Serialize, Hash, PartialEq, Eq, PartialOrd, Ord, Clone,@% if column_def.is_copyable() %@ Copy,@% endif %@ Display, Debug, Default)]
#[cfg_attr(feature = "seeder", derive(::schemars::JsonSchema))]
#[serde(transparent)]
@%- if !column_def.is_displayable() %@
#[display("{:?}", _0)]
@%- endif %@
pub struct @{ id_name }@(pub @{ column_def.get_inner_type(false, false) }@);
@% endfor -%@
@% for (name, column_def) in def.id_auto_inc_or_seq() -%@
#[derive(Serialize, Hash, PartialEq, Eq, PartialOrd, Ord, Clone,@% if column_def.is_copyable() %@ Copy,@% endif %@ Display, Debug, Default)]
#[cfg_attr(feature = "seeder", derive(::schemars::JsonSchema))]
#[serde(transparent)]
@%- if !column_def.is_displayable() %@
#[display("{:?}", _0)]
@%- endif %@
pub struct @{ id_name }@(
    #[cfg_attr(feature = "seeder", schemars(schema_with = "crate::misc::id_schema"))]
    pub @{ column_def.get_inner_type(true, false) }@
);

pub static GENERATED_IDS: OnceCell<std::sync::RwLock<HashMap<String, @{ id_name }@>>> = OnceCell::new();

#[allow(clippy::unnecessary_cast)]
impl<'de> serde::Deserialize<'de> for @{ id_name }@ {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Visitor;

        struct IdVisitor;

        impl Visitor<'_> for IdVisitor {
            type Value = @{ id_name }@;

            #[inline]
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("an integer")
            }

            #[inline]
            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let ids = GENERATED_IDS.get().map(|ids| ids.read().unwrap());
                if let Some(id) = ids.and_then(|ids| ids.get(v).copied()) {
                    return Ok(id);
                }
                Err(serde::de::Error::invalid_value(serde::de::Unexpected::Str(v), &self))
            }

            #[inline]
            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(@{ id_name }@(v as @{ column_def.get_inner_type(true, false) }@))
            }
        }
        deserializer.deserialize_u64(IdVisitor)
    }
}

@% endfor -%@
#[derive(Clone, Hash, PartialEq, Eq)]
pub struct Primary(@{ def.primaries()|fmt_join("pub {outer_owned}", ", ") }@);
impl Primary {
    pub fn cols() -> &'static str {
        r#"@{ def.primaries()|fmt_join("{col_esc}", ", ") }@"#
    }
    pub fn cols_with_paren() -> &'static str {
        r#"@{ def.primaries()|fmt_join_with_paren("{col_esc}", ", ") }@"#
    }
    pub fn cols_with_idx(idx: usize) -> String {
        format!(r#"@{ def.primaries()|fmt_join_with_paren("_t{}.{col_esc}", ", ") }@"#, @{ def.primaries()|fmt_join("idx", ", ") }@)
    }
}
#[derive(
    Hash, PartialEq, Eq, Serialize, Pack, Unpack, Clone, Debug, PartialOrd, Ord,
)]
pub struct InnerPrimary(@{ def.primaries()|fmt_join("pub {inner}", ", ") }@);
impl sqlx::FromRow<'_, DbRow> for InnerPrimary {
    fn from_row(row: &DbRow) -> sqlx::Result<Self> {
        use sqlx::Row;
        Ok(InnerPrimary (
            @{- def.primaries()|fmt_join("{from_row}", ", ") }@
        ))
    }
}
impl SqlColumns for InnerPrimary {
    fn _sql_cols(_is_mysql: bool) -> &'static str {
        r#"@{ def.primaries()|fmt_join("{col_query}", ", ") }@"#
    }
}

#[derive(Hash, PartialEq, Eq, Pack, Unpack, Clone)]
pub struct PrimaryHasher(pub InnerPrimary, pub ShardId);
@%- if !config.force_disable_cache %@

impl HashVal for PrimaryHasher {
    fn hash_val(&self, shard_id: ShardId) -> u128 {
        let mut hasher = FxHasher64::default();
        PRIMARY_TYPE_ID.hash(&mut hasher);
        shard_id.hash(&mut hasher);
        self.0.hash(&mut hasher);
        let hash = (hasher.finish() as u128) << 64;

        let mut hasher = AHasher::default();
        PRIMARY_TYPE_ID.hash(&mut hasher);
        shard_id.hash(&mut hasher);
        self.0.hash(&mut hasher);
        hash | (hasher.finish() as u128)
    }
}
@%- endif %@

impl PrimaryHasher {
    pub fn _shard_id(&self) -> ShardId {
        self.1
    }
    pub fn to_wrapper(&self, time: MSec) -> PrimaryWrapper {
        PrimaryWrapper(self.0.clone(), self.1, time)
    }
}

#[derive(Pack, Unpack, Clone, Debug)]
pub struct PrimaryWrapper(pub InnerPrimary, pub ShardId, pub MSec);
@%- if !config.force_disable_cache %@

impl CacheVal for PrimaryWrapper {
    fn _size(&self) -> u32 {
        let size = calc_mem_size(std::mem::size_of::<Self>());
        size.try_into().unwrap_or(u32::MAX)
    }
    fn _type_id(&self) -> u64 {
        Self::__type_id()
    }
    fn __type_id() -> u64 {
        PRIMARY_TYPE_ID
    }
    fn _shard_id(&self) -> ShardId {
        self.1
    }
    fn _time(&self) -> MSec {
        self.2
    }
    fn _estimate() -> usize {
        calc_mem_size(std::mem::size_of::<Self>())
    }
    fn _encode(&self) -> Result<Vec<u8>> {
        Ok(senax_encoder::pack(self)?.to_vec())
    }
    fn _decode(v: &[u8]) -> Result<Self> {
        let mut bytes = Bytes::from(v.to_vec());
        Ok(senax_encoder::unpack(&mut bytes)?)
    }
}
@%- endif %@

#[derive(Pack, Unpack, PartialEq, Clone, Debug, senax_macros::SqlCol)]
pub struct Data {
@{ def.all_fields()|fmt_join("{column_query}    pub {var}: {inner},\n", "") -}@
}
impl Data {
    fn validate(&self) -> Result<(), validator::ValidationErrors> {
        use std::borrow::Cow;
        let mut errors = validator::ValidationErrors::new();
        @{- def.all_fields()|fmt_join("{validate}", "") }@
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}
impl sqlx::FromRow<'_, DbRow> for Data {
    fn from_row(row: &DbRow) -> sqlx::Result<Self> {
        use sqlx::Row;
        Ok(Data {
            @{ def.all_fields()|fmt_join("{var}: {from_row},", "
            ") }@
        })
    }
}

#[allow(clippy::derivable_impls)]
impl Default for Data {
    fn default() -> Self {
        Self {
@{- def.all_fields()|fmt_join("
            {var}: {default},", "") }@
        }
    }
}

impl fmt::Display for Data {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        @{- def.all_except_secret()|fmt_join("
        Accessor{accessor_with_sep_type}::_write_insert(f, \"{comma}\", \"{raw_name}\", &self.{var})?;", "") }@
        write!(f, "}}")?;
        Ok(())
    }
}

impl Data {
    #[allow(clippy::let_and_return)]
    pub fn _size(&self) -> usize {
        let mut size = std::mem::size_of::<Self>();
        @{- def.cache_cols_not_null_sized()|fmt_join("
        size += self.{var}._size();", "") }@
        @{- def.cache_cols_null_sized()|fmt_join("
        size += self.{var}.as_ref().map(|v| v._size()).unwrap_or(0);", "") }@
        size
    }
}

#[derive(Pack, Unpack, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct OpData {
@{- def.all_fields()|fmt_join("
    pub {var}: Op,", "") }@
}
@%- if !config.force_disable_cache %@

#[derive(Pack, Unpack, Clone, Debug, Default, senax_macros::SqlCol)]
pub struct CacheData {
@{ def.cache_cols()|fmt_join("{column_query}    pub {var}: {inner},\n", "") -}@
}
impl sqlx::FromRow<'_, DbRow> for CacheData {
    fn from_row(row: &DbRow) -> sqlx::Result<Self> {
        use sqlx::Row;
        Ok(CacheData {
            @{ def.cache_cols()|fmt_join("{var}: {from_row},", "
            ") }@
        })
    }
}
@%- endif %@

@% for (name, column_def) in def.num_enums(false) -%@
@% let values = column_def.enum_values.as_ref().unwrap() -%@
#[derive(Pack, Unpack, Serialize_repr, Deserialize_repr, sqlx::Type, Hash, PartialEq, Eq, Clone, Copy, Debug, Default, strum::Display, FromRepr, EnumMessage, EnumString, IntoStaticStr)]
#[cfg_attr(feature = "seeder", derive(::schemars::JsonSchema))]
#[repr(@{ column_def.get_inner_type(true, true) }@)]
#[allow(non_camel_case_types)]
#[allow(clippy::upper_case_acronyms)]
pub enum _@{ name|pascal }@ {
@% for row in values -%@@{ row.label|label4 }@@{ row.comment|comment4 }@@{ row.label|strum_message4 }@@{ row.comment|strum_detailed4 }@    @% if loop.first %@#[default]@% endif %@@{ row.name|ident }@@{ row.value_str() }@,
@% endfor -%@
}
#[allow(non_snake_case)]
impl _@{ name|pascal }@ {
    pub fn inner(&self) -> @{ column_def.get_inner_type(true, true) }@ {
        *self as @{ column_def.get_inner_type(true, true) }@
    }
@%- for row in values %@
    pub fn is_@{ row.name }@(&self) -> bool {
        self == &Self::@{ row.name|ident }@
    }
@%- endfor %@
}
impl From<@{ column_def.get_inner_type(true, true) }@> for _@{ name|pascal }@ {
    fn from(val: @{ column_def.get_inner_type(true, true) }@) -> Self {
        if let Some(val) = Self::from_repr(val) {
            val
        } else {
            panic!("{} is a value outside the range of _@{ name|pascal }@.", val)
        }
    }
}
impl From<_@{ name|pascal }@> for @{ column_def.get_inner_type(true, true) }@ {
    fn from(val: _@{ name|pascal }@) -> Self {
        val.inner()
    }
}
impl From<_@{ name|pascal }@> for BindValue {
    fn from(val: _@{ name|pascal }@) -> Self {
        Self::Enum(Some(val.inner() as i64))
    }
}
impl From<Option<_@{ name|pascal }@>> for BindValue {
    fn from(val: Option<_@{ name|pascal }@>) -> Self {
        Self::Enum(val.map(|t| t.inner() as i64))
    }
}
@%- if !config.exclude_from_domain %@
@%- let a = crate::schema::set_domain_mode(true) %@
impl From<@{ column_def.get_filter_type(true) }@> for _@{ name|pascal }@ {
    fn from(v: @{ column_def.get_filter_type(true) }@) -> Self {
        match v {
@%- for row in values %@
            @{ column_def.get_filter_type(true) }@::@{ row.name }@ => _@{ name|pascal }@::@{ row.name }@,
@%- endfor %@
        }
    }
}
impl From<_@{ name|pascal }@> for @{ column_def.get_filter_type(true) }@ {
    fn from(v: _@{ name|pascal }@) -> Self {
        match v {
@%- for row in values %@
            _@{ name|pascal }@::@{ row.name }@ => @{ column_def.get_filter_type(true) }@::@{ row.name }@,
@%- endfor %@
        }
    }
}
@%- let a = crate::schema::set_domain_mode(false) %@
@%- endif %@

@% endfor -%@
@% for (name, column_def) in def.str_enums(false) -%@
@% let values = column_def.enum_values.as_ref().unwrap() -%@
#[derive(Pack, Unpack, Serialize, Deserialize, Hash, PartialEq, Eq, Clone, Copy, Debug, Default, strum::Display, EnumMessage, EnumString, IntoStaticStr)]
#[cfg_attr(feature = "seeder", derive(::schemars::JsonSchema))]
#[allow(non_camel_case_types)]
#[allow(clippy::upper_case_acronyms)]
pub enum _@{ name|pascal }@ {
@% for row in values -%@@{ row.label|label4 }@@{ row.comment|comment4 }@@{ row.label|strum_message4 }@@{ row.comment|strum_detailed4 }@    @% if loop.first %@#[default]@% endif %@@{ row.name|ident }@,
@% endfor -%@
}
#[allow(non_snake_case)]
impl _@{ name|pascal }@ {
    pub fn as_static_str(&self) -> &'static str {
        Into::<&'static str>::into(self)
    }
@%- for row in values %@
    pub fn is_@{ row.name }@(&self) -> bool {
        self == &Self::@{ row.name|ident }@
    }
@%- endfor %@
}
@%- if !config.exclude_from_domain %@
@%- let a = crate::schema::set_domain_mode(true) %@
impl From<@{ column_def.get_filter_type(true) }@> for _@{ name|pascal }@ {
    fn from(v: @{ column_def.get_filter_type(true) }@) -> Self {
        match v {
@%- for row in values %@
            @{ column_def.get_filter_type(true) }@::@{ row.name }@ => _@{ name|pascal }@::@{ row.name }@,
@%- endfor %@
        }
    }
}
impl From<_@{ name|pascal }@> for @{ column_def.get_filter_type(true) }@ {
    fn from(v: _@{ name|pascal }@) -> Self {
        match v {
@%- for row in values %@
            _@{ name|pascal }@::@{ row.name }@ => @{ column_def.get_filter_type(true) }@::@{ row.name }@,
@%- endfor %@
        }
    }
}
@%- let a = crate::schema::set_domain_mode(false) %@
@%- endif %@

@% endfor -%@
@{ def.label|label0 -}@
@{ def.comment|comment0 -}@
#[derive(Clone, Debug)]
pub struct _@{ pascal_name }@ {
    pub _inner: Data,
    pub _filter_flag: BTreeMap<&'static str, bool>,
@{ def.relations_one(false)|fmt_rel_join("    pub {rel_name}: Option<Option<Box<rel_{class_mod}::{class}>>>,\n", "") -}@
@{ def.relations_many(false)|fmt_rel_join("    pub {rel_name}: Option<Vec<rel_{class_mod}::{class}>>,\n", "") -}@
@{ def.relations_belonging(false)|fmt_rel_join("    pub {rel_name}: Option<Option<Box<rel_{class_mod}::{class}>>>,\n", "") -}@
@{ def.relations_belonging_outer_db(false)|fmt_rel_outer_db_join("    pub {rel_name}: Option<Option<Box<rel_{class_mod}::{class}>>>,\n", "") -}@
}

#[derive(Hash, PartialEq, Eq, Clone, Copy, strum::Display, EnumMessage, EnumString, IntoStaticStr, strum_macros::EnumIter, strum_macros::EnumProperty)]
#[allow(non_camel_case_types)]
pub enum _@{ pascal_name }@Info {
@%- for (col_name, column_def) in def.all_fields() %@
@{ column_def.label|strum_message4 }@@{ column_def.comment|strum_detailed4 }@@{ column_def|strum_props4 }@    @{ col_name|ident }@,
@%- endfor %@
}
@%- if !config.force_disable_cache %@

#[derive(Pack, Unpack, Clone, Debug)]
pub struct CacheWrapper {
    pub _inner: CacheData,
    _shard_id: ShardId,
    _time: MSec,
@{ def.relations_one_cache(false)|fmt_rel_join("    pub {rel_name}: Option<Option<Arc<rel_{class_mod}::CacheWrapper>>>,\n", "") -}@
@{ def.relations_many_cache(false)|fmt_rel_join("    pub {rel_name}: Option<Vec<Arc<rel_{class_mod}::CacheWrapper>>>,\n", "") -}@
}

#[derive(Clone, Debug)]
pub struct _@{ pascal_name }@Cache {
    pub _wrapper: Arc<CacheWrapper>,
    pub _filter_flag: BTreeMap<&'static str, bool>,
@{ def.relations_one_cache(false)|fmt_rel_join("    pub {rel_name}: Option<Option<Box<rel_{class_mod}::{class}Cache>>>,\n", "") -}@
@{ def.relations_one_uncached(false)|fmt_rel_join("    pub {rel_name}: Option<Option<Box<rel_{class_mod}::{class}>>>,\n", "") -}@
@{ def.relations_many_cache(false)|fmt_rel_join("    pub {rel_name}: Option<Vec<rel_{class_mod}::{class}Cache>>,\n", "") -}@
@{ def.relations_many_uncached(false)|fmt_rel_join("    pub {rel_name}: Option<Vec<rel_{class_mod}::{class}>>,\n", "") -}@
@{ def.relations_belonging_cache(false)|fmt_rel_join("    pub {rel_name}: Option<Option<Box<rel_{class_mod}::{class}Cache>>>,\n", "") -}@
@{ def.relations_belonging_uncached(false)|fmt_rel_join("    pub {rel_name}: Option<Option<Box<rel_{class_mod}::{class}>>>,\n", "") -}@
@{ def.relations_belonging_outer_db(false)|fmt_rel_outer_db_join("    pub {rel_name}: Option<Option<Box<rel_{class_mod}::{class}>>>,\n", "") -}@
}

#[derive(Pack, Unpack, Clone, Debug)]
pub struct VersionWrapper {
    pub id: InnerPrimary,
    pub shard_id: ShardId,
    pub time: MSec,
    pub version: @{ config.u32() }@,
}

impl CacheVal for VersionWrapper {
    fn _size(&self) -> u32 {
        let size = calc_mem_size(std::mem::size_of::<Self>());
        size.try_into().unwrap()
    }
    fn _type_id(&self) -> u64 {
        Self::__type_id()
    }
    fn __type_id() -> u64 {
        VERSION_TYPE_ID
    }
    fn _shard_id(&self) -> ShardId {
        self.shard_id
    }
    fn _time(&self) -> MSec {
        self.time
    }
    fn _estimate() -> usize {
        calc_mem_size(std::mem::size_of::<Self>())
    }
    fn _encode(&self) -> Result<Vec<u8>> {
        Ok(senax_encoder::pack(self)?.to_vec())
    }
    fn _decode(v: &[u8]) -> Result<Self> {
        let mut bytes = Bytes::from(v.to_vec());
        Ok(senax_encoder::unpack(&mut bytes)?)
    }
}
impl HashVal for VersionWrapper {
    fn hash_val(&self, shard_id: ShardId) -> u128 {
        let mut hasher = FxHasher64::default();
        VERSION_TYPE_ID.hash(&mut hasher);
        shard_id.hash(&mut hasher);
        self.id.hash(&mut hasher);
        let hash = (hasher.finish() as u128) << 64;

        let mut hasher = AHasher::default();
        VERSION_TYPE_ID.hash(&mut hasher);
        shard_id.hash(&mut hasher);
        self.id.hash(&mut hasher);
        hash | (hasher.finish() as u128)
    }
}

#[derive(Pack, Unpack, Clone, Debug)]
pub struct CacheSyncWrapper {
    pub id: InnerPrimary,
    pub shard_id: ShardId,
    pub time: MSec,
    pub sync: u64,
}

impl CacheVal for CacheSyncWrapper {
    fn _size(&self) -> u32 {
        let size = calc_mem_size(std::mem::size_of::<Self>());
        size.try_into().unwrap()
    }
    fn _type_id(&self) -> u64 {
        Self::__type_id()
    }
    fn __type_id() -> u64 {
        CACHE_SYNC_TYPE_ID
    }
    fn _shard_id(&self) -> ShardId {
        self.shard_id
    }
    fn _time(&self) -> MSec {
        self.time
    }
    fn _estimate() -> usize {
        calc_mem_size(std::mem::size_of::<Self>())
    }
    fn _encode(&self) -> Result<Vec<u8>> {
        Ok(senax_encoder::pack(self)?.to_vec())
    }
    fn _decode(v: &[u8]) -> Result<Self> {
        let mut bytes = Bytes::from(v.to_vec());
        Ok(senax_encoder::unpack(&mut bytes)?)
    }
}
impl HashVal for CacheSyncWrapper {
    fn hash_val(&self, shard_id: ShardId) -> u128 {
        let mut hasher = FxHasher64::default();
        CACHE_SYNC_TYPE_ID.hash(&mut hasher);
        shard_id.hash(&mut hasher);
        self.id.hash(&mut hasher);
        let hash = (hasher.finish() as u128) << 64;

        let mut hasher = AHasher::default();
        CACHE_SYNC_TYPE_ID.hash(&mut hasher);
        shard_id.hash(&mut hasher);
        self.id.hash(&mut hasher);
        hash | (hasher.finish() as u128)
    }
}
@%- endif %@

#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "seeder", derive(::schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct _@{ pascal_name }@Factory {
@{ def.for_factory()|fmt_join("{label}{comment}{factory_default}    pub {var}: {factory},", "\n") }@
@{ def.relations_one(false)|fmt_rel_join("    pub {rel_name}: Option<rel_{class_mod}::{class}Factory>,\n", "") -}@
@{ def.relations_many(false)|fmt_rel_join("    pub {rel_name}: Option<Vec<rel_{class_mod}::{class}Factory>>,\n", "") -}@
}

#[derive(Clone, Debug)]
pub struct _@{ pascal_name }@Updater {
    pub _data: Data,
    pub _update: Data,
    pub _filter_flag: BTreeMap<&'static str, bool>,
    pub _is_new: bool,
    pub _do_delete: bool,
    pub _upsert: bool,
    pub _is_loaded: bool,
    pub _op: OpData,
@{ def.relations_one(false)|fmt_rel_join("    pub {rel_name}: Option<Vec<rel_{class_mod}::{class}Updater>>,\n", "") -}@
@{ def.relations_many(false)|fmt_rel_join("    pub {rel_name}: Option<Vec<rel_{class_mod}::{class}Updater>>,\n", "") -}@
@{ def.relations_belonging(false)|fmt_rel_join("    pub {rel_name}: Option<Option<Box<rel_{class_mod}::{class}>>>,\n", "") -}@
@{ def.relations_belonging_outer_db(false)|fmt_rel_outer_db_join("    pub {rel_name}: Option<Option<Box<rel_{class_mod}::{class}>>>,\n", "") -}@
}

#[derive(Pack, Unpack, Clone, Debug)]
pub struct ForInsert {
    #[senax(id = 1)]
    pub _data: Data,
    #[senax(id = 2)]
    pub _is_new: bool,
@{- def.relations_one(false)|fmt_rel_join("
    pub {rel_name}: Option<Option<Box<rel_{class_mod}::ForInsert>>>,", "") }@
@{- def.relations_many(false)|fmt_rel_join("
    pub {rel_name}: Option<Vec<rel_{class_mod}::ForInsert>>,", "") }@
}

impl From<_@{ pascal_name }@Updater> for ForInsert {
    fn from(v: _@{ pascal_name }@Updater) -> Self {
        Self {
            _data: v._data,
            _is_new: v._is_new,
            @{- def.relations_one(false)|fmt_rel_join("
            {rel_name}: v.{rel_name}.map(|v| v.into_iter().filter(|v| !v.will_be_deleted()).next_back().map(|v| Box::new(v.into()))),", "") }@
            @{- def.relations_many(false)|fmt_rel_join("
            {rel_name}: v.{rel_name}.map(|v| v.into_iter().map(|v| v.into()).collect()),", "") }@
        }
    }
}

impl From<Box<_@{ pascal_name }@Updater>> for Box<ForInsert> {
    fn from(v: Box<_@{ pascal_name }@Updater>) -> Self {
        Box::new(ForInsert {
            _data: v._data,
            _is_new: v._is_new,
            @{- def.relations_one(false)|fmt_rel_join("
            {rel_name}: v.{rel_name}.map(|v| v.into_iter().filter(|v| !v.will_be_deleted()).next_back().map(|v| Box::new(v.into()))),", "") }@
            @{- def.relations_many(false)|fmt_rel_join("
            {rel_name}: v.{rel_name}.map(|v| v.into_iter().map(|v| v.into()).collect()),", "") }@
        })
    }
}

impl fmt::Display for ForInsert {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{INSERT: {}}}", &self._data)?;
        Ok(())
    }
}

pub trait _@{ pascal_name }@Getter: Send + Sync + 'static {
@{ def.all_fields()|fmt_join("{label}{comment}    fn _{raw_name}(&self) -> {outer};
", "") -}@
@{ def.relations_one_and_belonging(false)|fmt_rel_join("{label}{comment}    fn _{raw_rel_name}(&self) -> Result<Option<&rel_{class_mod}::{class}>>;
", "") -}@
@{ def.relations_many(false)|fmt_rel_join("{label}{comment}    fn _{raw_rel_name}(&self) -> Result<&Vec<rel_{class_mod}::{class}>>;
", "") -}@
@{ def.relations_belonging_outer_db(false)|fmt_rel_outer_db_join("{label}{comment}    fn _{raw_rel_name}(&self) -> Result<Option<&rel_{class_mod}::{class}>>;
", "") -}@
}
@#
@%- for (model, rel_name, rel) in def.relations_belonging(false) %@
struct RelCol@{ rel_name|pascal }@;
impl RelCol@{ rel_name|pascal }@ {
    fn cols() -> &'static str {
        r#"@{ rel.get_local_cols(rel_name, def)|fmt_join_with_paren("{col_esc}", ", ") }@"#
    }
    fn cols_with_idx(idx: usize) -> String {
        format!(r#"@{ rel.get_local_cols(rel_name, def)|fmt_join_with_paren("_t{}.{col_esc}", ", ") }@"#, @{ rel.get_local_cols(rel_name, def)|fmt_join("idx", ", ") }@)
    }
}

trait RelPk@{ rel_name|pascal }@ {
    fn primary(&self) -> Option<rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Primary>;
}
impl RelPk@{ rel_name|pascal }@ for _@{ pascal_name }@ {
    fn primary(&self) -> Option<rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Primary> {
        Some(@{ rel.get_local_cols(rel_name, def)|fmt_join_with_paren("self._{raw_name}(){null_question}", ", ") }@.into())
    }
}
impl RelPk@{ rel_name|pascal }@ for _@{ pascal_name }@Updater {
    fn primary(&self) -> Option<rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Primary> {
        Some(@{ rel.get_local_cols(rel_name, def)|fmt_join_with_paren("self._{raw_name}(){null_question}", ", ") }@.into())
    }
}
@%- if !config.force_disable_cache %@
impl RelPk@{ rel_name|pascal }@ for _@{ pascal_name }@Cache {
    fn primary(&self) -> Option<rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Primary> {
        Some(@{ rel.get_local_cols(rel_name, def)|fmt_join_with_paren("self._{raw_name}(){null_question}", ", ") }@.into())
    }
}
@%- endif %@
@%- endfor %@
@%- for (model, rel_name, rel) in def.relations_belonging_outer_db(false) %@
struct RelCol@{ rel_name|pascal }@;
impl RelCol@{ rel_name|pascal }@ {
    fn cols() -> &'static str {
        r#"@{ rel.get_local_cols(rel_name, def)|fmt_join_with_paren("{col_esc}", ", ") }@"#
    }
    fn cols_with_idx(idx: usize) -> String {
        format!(r#"@{ rel.get_local_cols(rel_name, def)|fmt_join_with_paren("_t{}.{col_esc}", ", ") }@"#, @{ rel.get_local_cols(rel_name, def)|fmt_join("idx", ", ") }@)
    }
}

trait RelPk@{ rel_name|pascal }@ {
    fn primary(&self) -> Option<rel_@{ rel.db()|snake }@_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Primary>;
}
impl RelPk@{ rel_name|pascal }@ for _@{ pascal_name }@ {
    fn primary(&self) -> Option<rel_@{ rel.db()|snake }@_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Primary> {
        Some(@{ rel.get_local_cols(rel_name, def)|fmt_join_with_paren("self._{raw_name}(){null_question}", ", ") }@.into())
    }
}
impl RelPk@{ rel_name|pascal }@ for _@{ pascal_name }@Updater {
    fn primary(&self) -> Option<rel_@{ rel.db()|snake }@_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Primary> {
        Some(@{ rel.get_local_cols(rel_name, def)|fmt_join_with_paren("self._{raw_name}(){null_question}", ", ") }@.into())
    }
}
@%- if !config.force_disable_cache %@
impl RelPk@{ rel_name|pascal }@ for _@{ pascal_name }@Cache {
    fn primary(&self) -> Option<rel_@{ rel.db()|snake }@_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Primary> {
        Some(@{ rel.get_local_cols(rel_name, def)|fmt_join_with_paren("self._{raw_name}(){null_question}", ", ") }@.into())
    }
}
@%- endif %@
@%- endfor %@
@%- for (model, rel_name, rel) in def.relations_one(false) %@
struct RelCol@{ rel_name|pascal }@;
impl RelCol@{ rel_name|pascal }@ {
    fn cols() -> &'static str {
        r#"@{ rel.get_foreign_cols(def)|fmt_join_foreign("{col_esc}", ", ") }@"#
    }
    fn cols_with_paren() -> &'static str {
        r#"@{ rel.get_foreign_cols(def)|fmt_join_foreign_with_paren("{col_esc}", ", ") }@"#
    }
    fn set_op_none(op: &mut rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::OpData) {
        @{- rel.get_foreign_cols(def)|fmt_join_foreign("
        op.{var} = Op::None;", "") }@
    }
}

trait RelFil@{ rel_name|pascal }@ where Self: Sized {
    fn filter(&self) -> rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Filter_;
    fn in_filter(list: &[Self]) -> rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Filter_;
}
impl RelFil@{ rel_name|pascal }@ for _@{ pascal_name }@ {
    fn filter(&self) -> rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Filter_ {
        use rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@ as rel;
        let pk: Primary = self.into();
        rel::Filter_::new_and()
        @{- rel.get_foreign_cols(def)|fmt_join_foreign("
            .and(rel::Filter_::Eq(rel::ColOne_::{var}(pk.{index}.inner().into())))", "") }@
    }
    fn in_filter(list: &[Self]) -> rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Filter_ {
        use rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@ as rel;
        @%- if rel.get_foreign_cols(def).len() == 1 %@
        let mut vec = Vec::new();
        for row in list {
            let pk: Primary = row.into();
            vec.push(pk.0.inner().into());
        }
        @{ rel.get_foreign_cols(def)|fmt_join_foreign("rel::Filter_::In(rel::ColMany_::{var}(vec))", "") }@
        @%- else %@
        let mut filter = rel::Filter_::new_or();
        for row in list {
            let pk: Primary = row.into();
            filter = filter.or(rel::Filter_::new_and()
            @{- rel.get_foreign_cols(def)|fmt_join_foreign("
                .and(rel::Filter_::Eq(rel::ColOne_::{var}(pk.{index}.inner().into())))", "") }@);
        }
        filter
        @%- endif %@
    }
}
impl RelFil@{ rel_name|pascal }@ for _@{ pascal_name }@Updater {
    fn filter(&self) -> rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Filter_ {
        use rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@ as rel;
        let pk: Primary = self.into();
        rel::Filter_::new_and()
        @{- rel.get_foreign_cols(def)|fmt_join_foreign("
            .and(rel::Filter_::Eq(rel::ColOne_::{var}(pk.{index}.inner().into())))", "") }@
    }
    fn in_filter(list: &[Self]) -> rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Filter_ {
        use rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@ as rel;
        @%- if rel.get_foreign_cols(def).len() == 1 %@
        let mut vec = Vec::new();
        for row in list {
            let pk: Primary = row.into();
            vec.push(pk.0.inner().into());
        }
        @{ rel.get_foreign_cols(def)|fmt_join_foreign("rel::Filter_::In(rel::ColMany_::{var}(vec))", "") }@
        @%- else %@
        let mut filter = rel::Filter_::new_or();
        for row in list {
            let pk: Primary = row.into();
            filter = filter.or(rel::Filter_::new_and()
            @{- rel.get_foreign_cols(def)|fmt_join_foreign("
                .and(rel::Filter_::Eq(rel::ColOne_::{var}(pk.{index}.inner().into())))", "") }@);
        }
        filter
        @%- endif %@
    }
}
impl RelFil@{ rel_name|pascal }@ for &ForInsert {
    fn filter(&self) -> rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Filter_ {
        use rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@ as rel;
        let pk: Primary = (&self._data).into();
        rel::Filter_::new_and()
        @{- rel.get_foreign_cols(def)|fmt_join_foreign("
            .and(rel::Filter_::Eq(rel::ColOne_::{var}(pk.{index}.inner().into())))", "") }@
    }
    fn in_filter(list: &[Self]) -> rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Filter_ {
        use rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@ as rel;
        @%- if rel.get_foreign_cols(def).len() == 1 %@
        let mut vec = Vec::new();
        for row in list {
            let pk: Primary = (&row._data).into();
            vec.push(pk.0.inner().into());
        }
        @{ rel.get_foreign_cols(def)|fmt_join_foreign("rel::Filter_::In(rel::ColMany_::{var}(vec))", "") }@
        @%- else %@
        let mut filter = rel::Filter_::new_or();
        for row in list {
            let pk: Primary = (&row._data).into();
            filter = filter.or(rel::Filter_::new_and()
            @{- rel.get_foreign_cols(def)|fmt_join_foreign("
                .and(rel::Filter_::Eq(rel::ColOne_::{var}(pk.{index}.inner().into())))", "") }@);
        }
        filter
        @%- endif %@
    }
}
@%- if !config.force_disable_cache %@
impl RelFil@{ rel_name|pascal }@ for CacheWrapper {
    fn filter(&self) -> rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Filter_ {
        use rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@ as rel;
        let pk: Primary = self.into();
        rel::Filter_::new_and()
        @{- rel.get_foreign_cols(def)|fmt_join_foreign("
            .and(rel::Filter_::Eq(rel::ColOne_::{var}(pk.{index}.inner().into())))", "") }@
    }
    fn in_filter(list: &[Self]) -> rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Filter_ {
        use rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@ as rel;
        @%- if rel.get_foreign_cols(def).len() == 1 %@
        let mut vec = Vec::new();
        for row in list {
            let pk: Primary = row.into();
            vec.push(pk.0.inner().into());
        }
        @{ rel.get_foreign_cols(def)|fmt_join_foreign("rel::Filter_::In(rel::ColMany_::{var}(vec))", "") }@
        @%- else %@
        let mut filter = rel::Filter_::new_or();
        for row in list {
            let pk: Primary = row.into();
            filter = filter.or(rel::Filter_::new_and()
            @{- rel.get_foreign_cols(def)|fmt_join_foreign("
                .and(rel::Filter_::Eq(rel::ColOne_::{var}(pk.{index}.inner().into())))", "") }@);
        }
        filter
        @%- endif %@
    }
}
impl RelFil@{ rel_name|pascal }@ for _@{ pascal_name }@Cache {
    fn filter(&self) -> rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Filter_ {
        use rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@ as rel;
        let pk: Primary = self.into();
        rel::Filter_::new_and()
        @{- rel.get_foreign_cols(def)|fmt_join_foreign("
            .and(rel::Filter_::Eq(rel::ColOne_::{var}(pk.{index}.inner().into())))", "") }@
    }
    fn in_filter(list: &[Self]) -> rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Filter_ {
        use rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@ as rel;
        @%- if rel.get_foreign_cols(def).len() == 1 %@
        let mut vec = Vec::new();
        for row in list {
            let pk: Primary = row.into();
            vec.push(pk.0.inner().into());
        }
        @{ rel.get_foreign_cols(def)|fmt_join_foreign("rel::Filter_::In(rel::ColMany_::{var}(vec))", "") }@
        @%- else %@
        let mut filter = rel::Filter_::new_or();
        for row in list {
            let pk: Primary = row.into();
            filter = filter.or(rel::Filter_::new_and()
            @{- rel.get_foreign_cols(def)|fmt_join_foreign("
                .and(rel::Filter_::Eq(rel::ColOne_::{var}(pk.{index}.inner().into())))", "") }@);
        }
        filter
        @%- endif %@
    }
}
@%- endif %@
pub(crate) trait RelFk@{ rel_name|pascal }@ {
    fn get_fk(&self) -> Option<Primary>;
    fn set_fk(&mut self, pk: InnerPrimary);
}
impl RelFk@{ rel_name|pascal }@ for rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Data {
    fn get_fk(&self) -> Option<Primary> {
        Some(@{ rel.get_foreign_cols(def)|fmt_join_foreign_with_paren("self.{raw_name}{null_question}{clone}", ", ") }@.into())
    }
    fn set_fk(&mut self, pk: InnerPrimary) {
        @{- rel.get_foreign_cols(def)|fmt_join_foreign_not_null_or_null("
        self.{raw_name} = pk.{index}{raw_to_inner};", "
        self.{raw_name} = Some(pk.{index}{raw_to_inner});", "") }@
    }
}
@%- if !config.force_disable_cache %@
impl RelFk@{ rel_name|pascal }@ for rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::CacheData {
    fn get_fk(&self) -> Option<Primary> {
        Some(@{ rel.get_foreign_cols(def)|fmt_join_foreign_with_paren("self.{raw_name}{null_question}{clone}", ", ") }@.into())
    }
    fn set_fk(&mut self, pk: InnerPrimary) {
        @{- rel.get_foreign_cols(def)|fmt_join_foreign_not_null_or_null("
        self.{raw_name} = pk.{index}{raw_to_inner};", "
        self.{raw_name} = Some(pk.{index}{raw_to_inner});", "") }@
    }
}
@%- endif %@
impl RelFk@{ rel_name|pascal }@ for rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::ForInsert {
    fn get_fk(&self) -> Option<Primary> {
        Some(@{ rel.get_foreign_cols(def)|fmt_join_foreign_with_paren("self._data.{raw_name}{null_question}{clone}", ", ") }@.into())
    }
    fn set_fk(&mut self, pk: InnerPrimary) {
        @{- rel.get_foreign_cols(def)|fmt_join_foreign_not_null_or_null("
        self._data.{raw_name} = pk.{index}{raw_to_inner};", "
        self._data.{raw_name} = Some(pk.{index}{raw_to_inner});", "") }@
    }
}
@%- endfor %@
@%- for (model, rel_name, rel) in def.relations_many(false) %@
struct RelCol@{ rel_name|pascal }@;
impl RelCol@{ rel_name|pascal }@ {
    fn cols() -> &'static str {
        r#"@{ rel.get_foreign_cols(def)|fmt_join_foreign("{col_esc}", ", ") }@"#
    }
    fn cols_with_paren() -> &'static str {
        r#"@{ rel.get_foreign_cols(def)|fmt_join_foreign_with_paren("{col_esc}", ", ") }@"#
    }
    fn set_op_none(op: &mut rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::OpData) {
        @{- rel.get_foreign_cols(def)|fmt_join_foreign("
        op.{var} = Op::None;", "") }@
    }
}

trait RelFil@{ rel_name|pascal }@ where Self: Sized {
    fn filter(&self) -> rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Filter_;
    fn in_filter(list: &[Self]) -> rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Filter_;
}
impl RelFil@{ rel_name|pascal }@ for _@{ pascal_name }@ {
    fn filter(&self) -> rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Filter_ {
        use rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@ as rel;
        let pk: Primary = self.into();
        rel::Filter_::new_and()
        @{- rel.get_foreign_cols(def)|fmt_join_foreign("
            .and(rel::Filter_::Eq(rel::ColOne_::{var}(pk.{index}.inner().into())))", "") }@
    }
    fn in_filter(list: &[Self]) -> rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Filter_ {
        use rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@ as rel;
        @%- if rel.get_foreign_cols(def).len() == 1 %@
        let mut vec = Vec::new();
        for row in list {
            let pk: Primary = row.into();
            vec.push(pk.0.inner().into());
        }
        @{ rel.get_foreign_cols(def)|fmt_join_foreign("rel::Filter_::In(rel::ColMany_::{var}(vec))", "") }@
        @%- else %@
        let mut filter = rel::Filter_::new_or();
        for row in list {
            let pk: Primary = row.into();
            filter = filter.or(rel::Filter_::new_and()
            @{- rel.get_foreign_cols(def)|fmt_join_foreign("
                .and(rel::Filter_::Eq(rel::ColOne_::{var}(pk.{index}.inner().into())))", "") }@);
        }
        filter
        @%- endif %@
    }
}
impl RelFil@{ rel_name|pascal }@ for _@{ pascal_name }@Updater {
    fn filter(&self) -> rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Filter_ {
        use rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@ as rel;
        let pk: Primary = self.into();
        rel::Filter_::new_and()
        @{- rel.get_foreign_cols(def)|fmt_join_foreign("
            .and(rel::Filter_::Eq(rel::ColOne_::{var}(pk.{index}.inner().into())))", "") }@
    }
    fn in_filter(list: &[Self]) -> rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Filter_ {
        use rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@ as rel;
        @%- if rel.get_foreign_cols(def).len() == 1 %@
        let mut vec = Vec::new();
        for row in list {
            let pk: Primary = row.into();
            vec.push(pk.0.inner().into());
        }
        @{ rel.get_foreign_cols(def)|fmt_join_foreign("rel::Filter_::In(rel::ColMany_::{var}(vec))", "") }@
        @%- else %@
        let mut filter = rel::Filter_::new_or();
        for row in list {
            let pk: Primary = row.into();
            filter = filter.or(rel::Filter_::new_and()
            @{- rel.get_foreign_cols(def)|fmt_join_foreign("
                .and(rel::Filter_::Eq(rel::ColOne_::{var}(pk.{index}.inner().into())))", "") }@);
        }
        filter
        @%- endif %@
    }
}
impl RelFil@{ rel_name|pascal }@ for &ForInsert {
    fn filter(&self) -> rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Filter_ {
        use rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@ as rel;
        let pk: Primary = (&self._data).into();
        rel::Filter_::new_and()
        @{- rel.get_foreign_cols(def)|fmt_join_foreign("
            .and(rel::Filter_::Eq(rel::ColOne_::{var}(pk.{index}.inner().into())))", "") }@
    }
    fn in_filter(list: &[Self]) -> rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Filter_ {
        use rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@ as rel;
        @%- if rel.get_foreign_cols(def).len() == 1 %@
        let mut vec = Vec::new();
        for row in list {
            let pk: Primary = (&row._data).into();
            vec.push(pk.0.inner().into());
        }
        @{ rel.get_foreign_cols(def)|fmt_join_foreign("rel::Filter_::In(rel::ColMany_::{var}(vec))", "") }@
        @%- else %@
        let mut filter = rel::Filter_::new_or();
        for row in list {
            let pk: Primary = (&row._data).into();
            filter = filter.or(rel::Filter_::new_and()
            @{- rel.get_foreign_cols(def)|fmt_join_foreign("
                .and(rel::Filter_::Eq(rel::ColOne_::{var}(pk.{index}.inner().into())))", "") }@);
        }
        filter
        @%- endif %@
    }
}
@%- if !config.force_disable_cache %@
impl RelFil@{ rel_name|pascal }@ for CacheWrapper {
    fn filter(&self) -> rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Filter_ {
        use rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@ as rel;
        let pk: Primary = self.into();
        rel::Filter_::new_and()
        @{- rel.get_foreign_cols(def)|fmt_join_foreign("
            .and(rel::Filter_::Eq(rel::ColOne_::{var}(pk.{index}.inner().into())))", "") }@
    }
    fn in_filter(list: &[Self]) -> rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Filter_ {
        use rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@ as rel;
        @%- if rel.get_foreign_cols(def).len() == 1 %@
        let mut vec = Vec::new();
        for row in list {
            let pk: Primary = row.into();
            vec.push(pk.0.inner().into());
        }
        @{ rel.get_foreign_cols(def)|fmt_join_foreign("rel::Filter_::In(rel::ColMany_::{var}(vec))", "") }@
        @%- else %@
        let mut filter = rel::Filter_::new_or();
        for row in list {
            let pk: Primary = row.into();
            filter = filter.or(rel::Filter_::new_and()
            @{- rel.get_foreign_cols(def)|fmt_join_foreign("
                .and(rel::Filter_::Eq(rel::ColOne_::{var}(pk.{index}.inner().into())))", "") }@);
        }
        filter
        @%- endif %@
    }
}
impl RelFil@{ rel_name|pascal }@ for _@{ pascal_name }@Cache {
    fn filter(&self) -> rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Filter_ {
        use rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@ as rel;
        let pk: Primary = self.into();
        rel::Filter_::new_and()
        @{- rel.get_foreign_cols(def)|fmt_join_foreign("
            .and(rel::Filter_::Eq(rel::ColOne_::{var}(pk.{index}.inner().into())))", "") }@
    }
    fn in_filter(list: &[Self]) -> rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Filter_ {
        use rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@ as rel;
        @%- if rel.get_foreign_cols(def).len() == 1 %@
        let mut vec = Vec::new();
        for row in list {
            let pk: Primary = row.into();
            vec.push(pk.0.inner().into());
        }
        @{ rel.get_foreign_cols(def)|fmt_join_foreign("rel::Filter_::In(rel::ColMany_::{var}(vec))", "") }@
        @%- else %@
        let mut filter = rel::Filter_::new_or();
        for row in list {
            let pk: Primary = row.into();
            filter = filter.or(rel::Filter_::new_and()
            @{- rel.get_foreign_cols(def)|fmt_join_foreign("
                .and(rel::Filter_::Eq(rel::ColOne_::{var}(pk.{index}.inner().into())))", "") }@);
        }
        filter
        @%- endif %@
    }
}
@%- endif %@
pub(crate) trait RelFk@{ rel_name|pascal }@ {
    fn get_fk(&self) -> Option<Primary>;
    fn set_fk(&mut self, pk: InnerPrimary);
}
impl RelFk@{ rel_name|pascal }@ for rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Data {
    fn get_fk(&self) -> Option<Primary> {
        Some(@{ rel.get_foreign_cols(def)|fmt_join_foreign_with_paren("self.{raw_name}{null_question}{clone}", ", ") }@.into())
    }
    fn set_fk(&mut self, pk: InnerPrimary) {
        @{- rel.get_foreign_cols(def)|fmt_join_foreign_not_null_or_null("
        self.{raw_name} = pk.{index}{raw_to_inner};", "
        self.{raw_name} = Some(pk.{index}{raw_to_inner});", "") }@
    }
}
@%- if !config.force_disable_cache %@
impl RelFk@{ rel_name|pascal }@ for rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::CacheData {
    fn get_fk(&self) -> Option<Primary> {
        Some(@{ rel.get_foreign_cols(def)|fmt_join_foreign_with_paren("self.{raw_name}{null_question}{clone}", ", ") }@.into())
    }
    fn set_fk(&mut self, pk: InnerPrimary) {
        @{- rel.get_foreign_cols(def)|fmt_join_foreign_not_null_or_null("
        self.{raw_name} = pk.{index}{raw_to_inner};", "
        self.{raw_name} = Some(pk.{index}{raw_to_inner});", "") }@
    }
}
@%- endif %@
impl RelFk@{ rel_name|pascal }@ for rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::ForInsert {
    fn get_fk(&self) -> Option<Primary> {
        Some(@{ rel.get_foreign_cols(def)|fmt_join_foreign_with_paren("self._data.{raw_name}{null_question}{clone}", ", ") }@.into())
    }
    fn set_fk(&mut self, pk: InnerPrimary) {
        @{- rel.get_foreign_cols(def)|fmt_join_foreign_not_null_or_null("
        self._data.{raw_name} = pk.{index}{raw_to_inner};", "
        self._data.{raw_name} = Some(pk.{index}{raw_to_inner});", "") }@
    }
}
@%- endfor %@
#@
@%- for (model, rel_name, rel) in def.relations_one(false) %@
struct RelCol@{ rel_name|pascal }@;
impl RelCol@{ rel_name|pascal }@ {
    fn set_op_none(op: &mut rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::OpData) {
        @{- rel.get_foreign_cols(def)|fmt_join_foreign("
        op.{var} = Op::None;", "") }@
    }
}
@%- endfor %@
@%- for (model, rel_name, rel) in def.relations_many(false) %@
struct RelCol@{ rel_name|pascal }@;
impl RelCol@{ rel_name|pascal }@ {
    fn set_op_none(op: &mut rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::OpData) {
        @{- rel.get_foreign_cols(def)|fmt_join_foreign("
        op.{var} = Op::None;", "") }@
    }
}
@%- endfor %@

@% for (name, column_def) in def.id() -%@
impl std::ops::Deref for @{ id_name }@ {
    type Target = @{ column_def.get_deref_type(false) }@;
    fn deref(&self) -> &@{ column_def.get_deref_type(false) }@ {
        &self.0
    }
}

impl @{ id_name }@ {
    pub fn inner(&self) -> @{ column_def.get_inner_type(false, false) }@ {
        self.0@{ column_def.clone_str() }@
    }
@%- if !def.disable_update() %@
    pub fn updater(&self) -> _@{ pascal_name }@Updater {
        _@{ pascal_name }@Updater {
            _data: Data {
                @{ name }@: self.inner(),
                ..Data::default()
            },
            _update: Data::default(),
            _filter_flag: Default::default(),
            _is_new: false,
            _do_delete: false,
            _upsert: false,
            _is_loaded: false,
            _op: OpData::default(),
@{- def.relations_one(false)|fmt_rel_join("\n            {rel_name}: None,", "") }@
@{- def.relations_many(false)|fmt_rel_join("\n            {rel_name}: None,", "") }@
@{- def.relations_belonging(false)|fmt_rel_join("\n            {rel_name}: None,", "") }@
@{- def.relations_belonging_outer_db(false)|fmt_rel_outer_db_join("\n            {rel_name}: None,", "") }@
        }
    }
@%- endif %@
}

impl From<@{ column_def.get_inner_type(false, false) }@> for @{ id_name }@ {
    fn from(id: @{ column_def.get_inner_type(false, false) }@) -> Self {
        Self(id)
    }
}
impl From<&@{ column_def.get_inner_type(false, false) }@> for @{ id_name }@ {
    fn from(id: &@{ column_def.get_inner_type(false, false) }@) -> Self {
        Self(id.clone())
    }
}
impl From<@{ id_name }@> for @{ column_def.get_inner_type(false, false) }@ {
    fn from(id: @{ id_name }@) -> Self {
        id.0
    }
}
@%- if column_def.get_inner_type(true, false) != column_def.get_inner_type(false, false)%@
impl From<@{ column_def.get_inner_type(true, false) }@> for @{ id_name }@ {
    fn from(id: @{ column_def.get_inner_type(true, false) }@) -> Self {
        Self(id.into())
    }
}
impl From<&@{ column_def.get_inner_type(true, false) }@> for @{ id_name }@ {
    fn from(id: &@{ column_def.get_inner_type(true, false) }@) -> Self {
        Self(id.clone().into())
    }
}
@%- if column_def.get_inner_type(true, false) == "String" %@
impl From<&str> for @{ id_name }@ {
    fn from(id: &str) -> Self {
        Self(id.to_string().into())
    }
}
@%- endif %@
impl From<@{ id_name }@> for @{ column_def.get_inner_type(true, false) }@ {
    fn from(id: @{ id_name }@) -> Self {
        id.0.as_ref().clone()
    }
}
impl From<@{ def.primaries()|fmt_join_with_paren("{raw_inner}", ", ") }@> for Primary {
    fn from(id: @{ def.primaries()|fmt_join_with_paren("{raw_inner}", ", ") }@) -> Self {
        @% if def.primaries().len() == 1 %@Self(id.into())@% else %@Self(@{ def.primaries()|fmt_join("id.{index}.into()", ", ") }@)@% endif %@
    }
}
@%- endif %@
impl From<&@{ id_name }@> for @{ id_name }@ {
    fn from(id: &@{ id_name }@) -> Self {
        Self(id.inner())
    }
}
@%- endfor %@
impl From<&InnerPrimary> for @{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@ {
    fn from(id: &InnerPrimary) -> Self {
        @{ def.primaries()|fmt_join_with_paren("id.{index}{clone}.into()", ", ") }@
    }
}
impl From<@{ def.primaries()|fmt_join_with_paren("{outer_ref}", ", ") }@> for Primary {
    fn from(id: @{ def.primaries()|fmt_join_with_paren("{outer_ref}", ", ") }@) -> Self {
        @% if def.primaries().len() == 1 -%@
        Self(id.to_owned().into())
        @%- else -%@
        Self(@{ def.primaries()|fmt_join("id.{index}.to_owned().into()", ", ") }@)
        @%- endif %@
    }
}
impl From<&Primary> for Primary {
    fn from(id: &Primary) -> Self {
        id.clone()
    }
}
impl From<&Primary> for InnerPrimary {
    fn from(id: &Primary) -> Self {
        Self(@{ def.primaries()|fmt_join("id.{index}{clone}.into()", ", ") }@)
    }
}
impl From<&InnerPrimary> for Primary {
    fn from(id: &InnerPrimary) -> Self {
        Self(@{ def.primaries()|fmt_join("id.{index}{clone}.into()", ", ") }@)
    }
}
@%- if def.primaries()|fmt_join_with_paren("{outer_ref}", ", ") != def.primaries()|fmt_join_with_paren("{inner}", ", ") %@
impl From<@{ def.primaries()|fmt_join_with_paren("{inner}", ", ") }@> for Primary {
    fn from(id: @{ def.primaries()|fmt_join_with_paren("{inner}", ", ") }@) -> Self {
        @% if def.primaries().len() == 1 %@Self(id.into())@% else %@Self(@{ def.primaries()|fmt_join("id.{index}.into()", ", ") }@)@% endif %@
    }
}
@%- endif %@
@%- if def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") != def.primaries()|fmt_join_with_paren("{inner}", ", ") %@
impl From<@{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@> for Primary {
    fn from(id: @{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@) -> Self {
        @% if def.primaries().len() == 1 %@Self(id)@% else %@Self(@{ def.primaries()|fmt_join("id.{index}", ", ") }@)@% endif %@
    }
}
@%- endif %@
@%- if def.primaries().len() > 1 %@
impl From<&@{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@> for Primary {
    fn from(id: &@{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@) -> Self {
        Self(@{ def.primaries()|fmt_join("id.{index}{clone}", ", ") }@)
    }
}
@%- endif %@
impl From<&_@{ pascal_name }@> for Primary {
    fn from(obj: &_@{ pascal_name }@) -> Self {
        Self(@{ def.primaries()|fmt_join("obj._inner.{var}{clone}.into()", ", ") }@)
    }
}
impl From<&_@{ pascal_name }@> for InnerPrimary {
    fn from(obj: &_@{ pascal_name }@) -> Self {
        Self(@{ def.primaries()|fmt_join("obj._inner.{var}{clone}", ", ") }@)
    }
}

impl From<&Data> for Primary {
    fn from(obj: &Data) -> Self {
        Self(@{ def.primaries()|fmt_join("obj.{var}{clone}.into()", ", ") }@)
    }
}
impl From<&Data> for InnerPrimary {
    fn from(obj: &Data) -> Self {
        Self(@{ def.primaries()|fmt_join("obj.{var}{clone}", ", ") }@)
    }
}
@%- if !config.force_disable_cache %@

impl From<&CacheData> for InnerPrimary {
    fn from(obj: &CacheData) -> Self {
        Self(@{ def.primaries()|fmt_join("obj.{var}{clone}", ", ") }@)
    }
}

impl From<&_@{ pascal_name }@Cache> for Primary {
    fn from(obj: &_@{ pascal_name }@Cache) -> Self {
        Self(@{ def.primaries()|fmt_join("obj._wrapper._inner.{var}{clone}.into()", ", ") }@)
    }
}
impl From<&_@{ pascal_name }@Cache> for InnerPrimary {
    fn from(obj: &_@{ pascal_name }@Cache) -> Self {
        Self(@{ def.primaries()|fmt_join("obj._wrapper._inner.{var}{clone}", ", ") }@)
    }
}

impl From<&Arc<CacheWrapper>> for InnerPrimary {
    fn from(obj: &Arc<CacheWrapper>) -> Self {
        Self(@{ def.primaries()|fmt_join("obj._inner.{var}{clone}", ", ") }@)
    }
}
impl From<&CacheWrapper> for Primary {
    fn from(obj: &CacheWrapper) -> Self {
        Self(@{ def.primaries()|fmt_join("obj._inner.{var}{clone}.into()", ", ") }@)
    }
}
@%- endif %@

impl From<&_@{ pascal_name }@Updater> for Primary {
    fn from(obj: &_@{ pascal_name }@Updater) -> Self {
        Self(@{ def.primaries()|fmt_join("obj._data.{var}{clone}.into()", ", ") }@)
    }
}

impl From<&_@{ pascal_name }@Updater> for InnerPrimary {
    fn from(obj: &_@{ pascal_name }@Updater) -> Self {
        Self(@{ def.primaries()|fmt_join("obj._data.{var}{clone}", ", ") }@)
    }
}

impl From<&mut _@{ pascal_name }@Updater> for InnerPrimary {
    fn from(obj: &mut _@{ pascal_name }@Updater) -> Self {
        Self(@{ def.primaries()|fmt_join("obj._data.{var}{clone}", ", ") }@)
    }
}

impl fmt::Display for InnerPrimary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "@{ def.primaries()|fmt_join("{col}={disp}", ", ") }@"@{ def.primaries()|fmt_join(", self.{index}", "") }@)
    }
}

impl _@{ pascal_name }@Getter for _@{ pascal_name }@ {
    @{- def.all_fields()|fmt_join("
    fn _{raw_name}(&self) -> {outer} {
        {convert_outer_prefix}self._inner.{var}{clone_for_outer}{convert_outer}
    }", "") }@
    @{- def.relations_one_and_belonging(false)|fmt_rel_join("
    fn _{raw_rel_name}(&self) -> Result<Option<&rel_{class_mod}::{class}>> {
        Ok(self.{rel_name}.as_ref().context(\"{raw_rel_name} is not loaded\")?.as_ref().map(|b| &**b))
    }", "") }@
    @{- def.relations_many(false)|fmt_rel_join("
    fn _{raw_rel_name}(&self) -> Result<&Vec<rel_{class_mod}::{class}>> {
        self.{rel_name}.as_ref().context(\"{raw_rel_name} is not loaded\")
    }", "") }@
    @{- def.relations_belonging_outer_db(false)|fmt_rel_outer_db_join("
    fn _{raw_rel_name}(&self) -> Result<Option<&rel_{class_mod}::{class}>> {
        Ok(self.{rel_name}.as_ref().context(\"{raw_rel_name} is not loaded\")?.as_ref().map(|b| &**b))
    }", "") }@
}

@%- for parent in def.parents() %@
impl crate::models::@{ parent.group_name|snake|ident }@::@{ parent.name|snake|ident }@::_@{ parent.name|pascal }@Getter for _@{ pascal_name }@ {
    @{- parent.primaries()|fmt_join("
    fn _{raw_name}(&self) -> &{inner} {
        &self._inner.{var}
    }", "") }@
    @{- parent.non_primaries()|fmt_join("
    fn _{raw_name}(&self) -> {outer} {
        {convert_outer_prefix}self._inner.{var}{clone_for_outer}{convert_outer}
    }", "") }@
    @{- parent.relations_one_and_belonging(false)|fmt_rel_join("
    fn _{raw_rel_name}(&self) -> Result<Option<&rel_{class_mod}::{class}>> {
        Ok(self.{rel_name}.as_ref().context(\"{raw_rel_name} is not loaded\")?.as_ref().map(|b| &**b))
    }", "") }@
    @{- parent.relations_many(false)|fmt_rel_join("
    fn _{raw_rel_name}(&self) -> Result<&Vec<rel_{class_mod}::{class}>> {
        Ok(self.{rel_name}.as_ref().context(\"{raw_rel_name} is not loaded\")?.as_ref())
    }", "") }@
}
@%- endfor %@
@%- if !config.force_disable_cache %@

static CACHE_WRAPPER_AVG: AtomicUsize = AtomicUsize::new(0);
static CACHE_WRAPPER_AVG_NUM: AtomicUsize = AtomicUsize::new(0);

impl CacheVal for CacheWrapper {
    fn _size(&self) -> u32 {
        let mut size = calc_mem_size(std::mem::size_of::<Self>());
        @{- def.cache_cols_not_null_sized()|fmt_join("
        size += self._inner.{var}._size();", "") }@
        @{- def.cache_cols_null_sized()|fmt_join("
        size += self._inner.{var}.as_ref().map(|v| v._size()).unwrap_or(0);", "") }@
        @{- def.relations_one_cache(false)|fmt_rel_join("
        size += self.{rel_name}.as_ref().map(|l| l.as_ref().map(|v| v._size() as usize).unwrap_or(0)).unwrap_or(0);", "") }@
        @{- def.relations_many_cache(false)|fmt_rel_join("
        size += self.{rel_name}.as_ref().map(|l| l.iter().fold(0, |i, v| i + v._size() as usize)).unwrap_or(0);", "") }@
        size.try_into().unwrap_or(u32::MAX)
    }
    fn _type_id(&self) -> u64 {
        Self::__type_id()
    }
    fn __type_id() -> u64 {
        CACHE_TYPE_ID
    }
    fn _shard_id(&self) -> ShardId {
        self._shard_id
    }
    fn _time(&self) -> MSec {
        self._time
    }
    fn _estimate() -> usize {
        CACHE_WRAPPER_AVG.load(Ordering::Relaxed)
    }
    fn _encode(&self) -> Result<Vec<u8>> {
        let bytes = senax_encoder::pack(self)?;
        let vec = encode_all(bytes.as_ref(), 1)?;
        let num = CACHE_WRAPPER_AVG_NUM.load(Ordering::Relaxed);
        let ave = (CACHE_WRAPPER_AVG.load(Ordering::Relaxed) * num + vec.len()) / num.saturating_add(1);
        CACHE_WRAPPER_AVG_NUM.store(num.saturating_add(1), Ordering::Relaxed);
        CACHE_WRAPPER_AVG.store(ave, Ordering::Relaxed);
        Ok(vec)
    }
    fn _decode(v: &[u8]) -> Result<Self> {
        let mut bytes = Bytes::from(decode_all(v)?.to_vec());
        Ok(senax_encoder::unpack(&mut bytes)?)
    }
}

impl CacheWrapper {
    @{- def.cache_cols()|fmt_join("
    fn _{raw_name}(&self) -> {outer} {
        {convert_outer_prefix}self._inner.{var}{clone_for_outer}{convert_outer}
    }", "") }@
    @{- def.relations_one_cache(false)|fmt_rel_join("
    fn _{raw_rel_name}(&self) -> Result<Option<&Arc<rel_{class_mod}::CacheWrapper>>> {
        self.{rel_name}.as_ref().context(\"{raw_rel_name} is not loaded\").map(|v| v.as_ref())
    }", "") }@
    @{- def.relations_many_cache(false)|fmt_rel_join("
    fn _{raw_rel_name}(&self) -> Result<&Vec<Arc<rel_{class_mod}::CacheWrapper>>> {
        self.{rel_name}.as_ref().context(\"{raw_rel_name} is not loaded\")
    }", "") }@
}

impl _@{ pascal_name }@Cache {
    @{- def.cache_cols()|fmt_join("
{label}{comment}    pub fn _{raw_name}(&self) -> {outer} {
        self._wrapper._{raw_name}()
    }", "") }@
    @{- def.relations_one_cache(false)|fmt_rel_join("
{label}{comment}    pub fn _{raw_rel_name}(&self) -> Result<Option<rel_{class_mod}::{class}Cache>> {
        if let Some(v) = &self.{rel_name} {
            Ok(v.as_ref().map(|v| (**v).clone()))
        } else {
            Ok(self._wrapper._{raw_rel_name}()?.map(|v| (v.clone(), Default::default()).into()))
        }
    }", "") }@
    @{- def.relations_one_uncached(false)|fmt_rel_join("
{label}{comment}    pub fn _{raw_rel_name}(&self) -> Result<Option<rel_{class_mod}::{class}>> {
        Ok(self.{rel_name}.as_ref().context(\"{raw_rel_name} is not loaded\")?.as_ref().map(|v| (**v).clone()))
    }", "") }@
    @{- def.relations_many_cache(false)|fmt_rel_join("
{label}{comment}    pub fn _{raw_rel_name}(&self) -> Result<Vec<rel_{class_mod}::{class}Cache>> {
        if let Some(v) = &self.{rel_name} {
            Ok(v.to_vec())
        } else {
            Ok(self._wrapper._{raw_rel_name}()?.iter().map(|v| (v.clone(), Default::default()).into()).collect())
        }
    }", "") }@
    @{- def.relations_many_uncached(false)|fmt_rel_join("
{label}{comment}    pub fn _{raw_rel_name}(&self) -> Result<Vec<rel_{class_mod}::{class}>> {
        Ok(self.{rel_name}.as_ref().context(\"{raw_rel_name} is not loaded\")?.to_vec())
    }", "") }@
    @{- def.relations_belonging_cache(false)|fmt_rel_join("
{label}{comment}    pub fn _{raw_rel_name}(&self) -> Result<Option<rel_{class_mod}::{class}Cache>> {
        Ok(self.{rel_name}.as_ref().context(\"{raw_rel_name} is not loaded\")?.as_ref().map(|b| *b.clone()))
    }", "") }@
    @{- def.relations_belonging_uncached(false)|fmt_rel_join("
{label}{comment}    pub fn _{raw_rel_name}(&self) -> Result<Option<rel_{class_mod}::{class}>> {
        Ok(self.{rel_name}.as_ref().context(\"{raw_rel_name} is not loaded\")?.as_ref().map(|b| *b.clone()))
    }", "") }@
    @{- def.relations_belonging_outer_db(false)|fmt_rel_outer_db_join("
{label}{comment}    pub fn _{raw_rel_name}(&self) -> Result<Option<rel_{class_mod}::{class}>> {
        Ok(self.{rel_name}.as_ref().context(\"{raw_rel_name} is not loaded\")?.as_ref().map(|b| *b.clone()))
    }", "") }@
}
@%- endif %@

impl Updater for _@{ pascal_name }@Updater {
    fn is_new(&self) -> bool {
        self._is_new
    }
    fn has_been_deleted(&self) -> bool {
        @{ def.soft_delete_tpl("false","self._data.deleted_at.is_some()","self._data.deleted != 0")}@
    }
    fn mark_for_delete(&mut self) {
        self._do_delete = true;
    }
    fn unmark_for_delete(&mut self) {
        self._do_delete = false;
    }
    fn will_be_deleted(&self) -> bool {
        self._do_delete
    }
    fn mark_for_upsert(&mut self) {
        self._upsert = true;
    }
    fn is_updated(&self) -> bool {
        false
        @%- if !def.disable_update() %@
        @{- def.non_primaries_except_read_only()|fmt_join("
        || self._op.{var} != Op::None && self._op.{var} != Op::Skip", "") }@
        @%- endif %@
    }
    fn overwrite_except_skip(&mut self, updater: Self) {
        self.overwrite_with(updater, false)
    }
    fn overwrite_only_set(&mut self, updater: Self) {
        self.overwrite_with(updater, true)
    }
    fn overwrite_with(&mut self, updater: Self, set_only: bool) {
        @%- if !def.disable_update() %@
        @{- def.for_cmp()|fmt_join("
        if (if set_only { updater._op.{var} == Op::Set } else { updater._op.{var} != Op::Skip }) && self._data.{var} != updater._data.{var} {
            self._op.{var} = Op::Set;
            self._data.{var} = updater._data.{var}.clone();
            self._update.{var} = updater._data.{var};
        }", "") }@
        @%- endif %@
    }
}

#[allow(non_snake_case)]
impl _@{ pascal_name }@Updater {
@{- def.all_fields()|fmt_join("
{label}{comment}    pub fn _{raw_name}(&self) -> {outer} {
        {convert_outer_prefix}self._data.{var}{clone_for_outer}{convert_outer}
    }", "") }@
@{- def.primaries()|fmt_join("
{label}{comment}    pub fn mut_{raw_name}(&self) -> Accessor{accessor_with_type} {
        Accessor{accessor} {
            val: &self._data.{var},
            _phantom: Default::default(),
        }
    }", "") }@
@{- def.non_primaries_except_read_only()|fmt_join("
{label}{comment}    pub fn mut_{raw_name}(&mut self) -> Accessor{accessor_with_type} {
        Accessor{accessor} {
            op: &mut self._op.{var},
            val: &mut self._data.{var},
            update: &mut self._update.{var},
            _phantom: Default::default(),
        }
    }", "") }@
@{- def.relations_one(false)|fmt_rel_join("
{label}{comment}    pub fn mut_{raw_rel_name}(&mut self) -> AccessorHasOne<'_, rel_{class_mod}::{class}Updater> {
        AccessorHasOne {
            name: \"{raw_rel_name}\",
            val: &mut self.{rel_name},
        }
    }", "") }@
@{- def.relations_many(false)|fmt_rel_join("
{label}{comment}    pub fn mut_{raw_rel_name}(&mut self) -> AccessorHasMany<'_, rel_{class_mod}::{class}Updater> {
        AccessorHasMany {
            name: \"{raw_rel_name}\",
            val: &mut self.{rel_name},
        }
    }", "") }@
@{- def.relations_belonging(false)|fmt_rel_join("
    fn _{raw_rel_name}(&self) -> Result<Option<&rel_{class_mod}::{class}>> {
        Ok(self.{rel_name}.as_ref().context(\"{raw_rel_name} is not loaded\")?.as_ref().map(|b| &**b))
    }", "") }@
@{- def.relations_belonging_outer_db(false)|fmt_rel_outer_db_join("
    fn _{raw_rel_name}(&self) -> Result<Option<&rel_{class_mod}::{class}>> {
        Ok(self.{rel_name}.as_ref().context(\"{raw_rel_name} is not loaded\")?.as_ref().map(|b| &**b))
    }", "") }@
}
#[async_trait::async_trait]
impl crate::misc::UpdaterForInner for _@{ pascal_name }@Updater {
    fn __validate(&self) -> Result<()> {
        self._data.validate()?;
@{- def.relations_one_and_many(false)|fmt_rel_join("
        if let Some(v) = self.{rel_name}.as_ref() {
            for v in v.iter() {
                v.__validate()?;
            }
        }", "") }@
        Ok(())
    }
    #[allow(clippy::unnecessary_cast)]
    #[allow(clippy::only_used_in_recursion)]
    #[allow(clippy::needless_if)]
    async fn __set_default_value(&mut self, conn: &mut DbConn) -> Result<()>
    {
        if self.is_new() {
            @{- def.auto_seq()|fmt_join("
            if self._data.{var} == 0 {
                self._data.{var} = conn.sequence(1).await? as {inner};
            }", "") }@
            @{- def.auto_uuid()|fmt_join("
            if self._data.{var}.is_nil() {
                if let Some(uuid_node) = crate::UUID_NODE.get() {
                    self._data.{var} = uuid::Uuid::now_v6(uuid_node);
                } else {
                    self._data.{var} = uuid::Uuid::now_v7();
                }
            }", "") }@
            @%- if def.created_at_conf().is_some() %@
            if self._op.@{ ConfigDef::created_at()|ident }@ == Op::None {
                self._data.@{ ConfigDef::created_at()|ident }@ = @{(def.created_at_conf().unwrap() == Timestampable::RealTime)|if_then_else_ref("SystemTime::now()","conn.time()")}@.into();
            }
            @%- endif %@
            @%- if def.updated_at_conf().is_some() %@
            if self._op.@{ ConfigDef::updated_at()|ident }@ == Op::None {
                self._data.@{ ConfigDef::updated_at()|ident }@ = @{(def.updated_at_conf().unwrap() == Timestampable::RealTime)|if_then_else_ref("SystemTime::now()","conn.time()")}@.into();
            }
            @%- endif %@
            @%- if def.versioned %@
            self._data.@{ version_col }@ = 1;
            @%- endif %@
            @{ def.inheritance_set() }@
        }
        @%- if def.updated_at_conf().is_some() %@
        if (self.is_updated() || self.will_be_deleted()) && self._op.@{ ConfigDef::updated_at()|ident }@ == Op::None {
            self.mut_@{ ConfigDef::updated_at() }@().set(@{(def.updated_at_conf().unwrap() == Timestampable::RealTime)|if_then_else_ref("SystemTime::now()","conn.time()")}@.into());
        }
        @%- endif %@
@{- def.relations_one_and_many(false)|fmt_rel_join("
        if let Some(v) = self.{rel_name}.as_mut() {
            for v in v.iter_mut() {
                RelCol{rel_name_pascal}::set_op_none(&mut v._op);
                v.__set_default_value(conn).await?;
            }
        }", "") }@
        Ok(())
    }

    #[allow(clippy::only_used_in_recursion)]
    fn __set_overwrite_extra_value(&mut self, conn: &mut DbConn)
    {
        if self.will_be_deleted() {
            @{- def.soft_delete_tpl2("
            panic!(\"DELETE is not supported.\");","
            self.mut_deleted_at().set(Some({val}.into()));","
            self.mut_deleted().set(true);","
            let deleted = cmp::max(1, SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as {u32});
            self.mut_deleted().set(deleted);")}@
        }
        @%- if def.versioned %@
        if !self.is_new() {  // Object obtained with a row lock from the database
            use senax_common::cache::CycleCounter as _;
            let version = self._@{ ConfigDef::version() }@().cycle_add(1);
            self._data.@{ ConfigDef::version() }@ = version;
            self._update.@{ ConfigDef::version() }@ = version;
            self._op.@{ ConfigDef::version() }@ = Op::Set;
        }
        @%- endif %@
        @{- def.relations_one_and_many(false)|fmt_rel_join("
        if let Some(v) = self.{rel_name}.as_mut() {
            for v in v.iter_mut() {
                v.__set_overwrite_extra_value(conn);
            }
        }", "") }@
    }
}

impl fmt::Display for _@{ pascal_name }@Updater {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_new() {
            write!(f, "{{INSERT: {}}}", &self._data)?;
        } else {
            write!(f, "{{UPDATE: {{")?;
            @%- if !def.disable_update() %@
            @{- def.primaries()|fmt_join("
            Accessor{accessor_with_sep_type}::_write_insert(f, \"{comma}\", \"{raw_name}\", &self._data.{var})?;", "") }@
            @{- def.all_except_secret_and_primary()|fmt_join("
            Accessor{accessor_with_sep_type}::_write_update(f, \"{comma}\", \"{raw_name}\", self._op.{var}, &self._update.{var})?;", "") }@
            @%- endif %@
            write!(f, "}}}}")?;
        }
        Ok(())
    }
}

impl From<(Data, BTreeMap<&'static str, bool>)> for _@{ pascal_name }@ {
    fn from(v: (Data, BTreeMap<&'static str, bool>)) -> Self {
        Self {
            _inner: v.0,
            _filter_flag: v.1,
@{- def.relations_one_and_belonging(false)|fmt_rel_join("\n            {rel_name}: None,", "") }@
@{- def.relations_many(false)|fmt_rel_join("\n            {rel_name}: None,", "") }@
@{- def.relations_belonging_outer_db(false)|fmt_rel_outer_db_join("\n            {rel_name}: None,", "") }@
        }
    }
}
impl From<(Data, BTreeMap<&'static str, bool>)> for _@{ pascal_name }@Updater {
    fn from(v: (Data, BTreeMap<&'static str, bool>)) -> Self {
        Self {
            _data: v.0,
            _update: Data::default(),
            _filter_flag: v.1,
            _is_new: false,
            _do_delete: false,
            _upsert: false,
            _is_loaded: true,
            _op: OpData::default(),
@{- def.relations_one(false)|fmt_rel_join("\n            {rel_name}: None,", "") }@
@{- def.relations_many(false)|fmt_rel_join("\n            {rel_name}: None,", "") }@
@{- def.relations_belonging(false)|fmt_rel_join("\n            {rel_name}: None,", "") }@
@{- def.relations_belonging_outer_db(false)|fmt_rel_outer_db_join("\n            {rel_name}: None,", "") }@
        }
    }
}
@%- if !config.force_disable_cache %@

impl From<(Arc<CacheWrapper>, BTreeMap<&'static str, bool>)> for _@{ pascal_name }@Cache {
    fn from(v: (Arc<CacheWrapper>, BTreeMap<&'static str, bool>)) -> Self {
        Self {
            _wrapper: v.0,
            _filter_flag: v.1,
@{- def.relations_one_cache(false)|fmt_rel_join("\n            {rel_name}: None,", "") }@
@{- def.relations_one_uncached(false)|fmt_rel_join("\n            {rel_name}: None,", "") }@
@{- def.relations_many_cache(false)|fmt_rel_join("\n            {rel_name}: None,", "") }@
@{- def.relations_many_uncached(false)|fmt_rel_join("\n            {rel_name}: None,", "") }@
@{- def.relations_belonging_cache(false)|fmt_rel_join("\n            {rel_name}: None,", "") }@
@{- def.relations_belonging_uncached(false)|fmt_rel_join("\n            {rel_name}: None,", "") }@
@{- def.relations_belonging_outer_db(false)|fmt_rel_outer_db_join("\n            {rel_name}: None,", "") }@
        }
    }
}

impl CacheWrapper {
    pub fn from_inner(inner: CacheData, shard_id: ShardId, time: MSec) -> Self {
        Self {
            _inner: inner,
            _shard_id: shard_id,
            _time: time,
@{- def.relations_one_cache(false)|fmt_rel_join("\n            {rel_name}: None,", "") }@
@{- def.relations_many_cache(false)|fmt_rel_join("\n            {rel_name}: None,", "") }@
        }
    }
    pub fn from_data(data: Data, shard_id: ShardId, time: MSec) -> Self {
        Self {
            _inner: CacheData {
@{- def.cache_cols()|fmt_join("\n                {var}: data.{var},", "") }@
            },
            _shard_id: shard_id,
            _time: time,
@{- def.relations_one_cache(false)|fmt_rel_join("\n            {rel_name}: None,", "") }@
@{- def.relations_many_cache(false)|fmt_rel_join("\n            {rel_name}: None,", "") }@
        }
    }
}
@%- endif %@

impl Serialize for _@{ pascal_name }@ {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        #[allow(unused_mut)]
        let mut len = @{ def.serializable().len() }@;
        @{- def.relations_one_and_belonging(false)|fmt_rel_join("
        if self.{rel_name}.is_some() {
            len += 1;
        }", "") }@
        @{- def.relations_many(false)|fmt_rel_join("
        if self.{rel_name}.is_some() {
            len += 1;
        }", "") }@
        @{- def.relations_belonging_outer_db(false)|fmt_rel_outer_db_join("
        if self.{rel_name}.is_some() {
            len += 1;
        }", "") }@
        let mut state = serializer.serialize_struct("@{ pascal_name }@", len)?;
        @{- def.serializable()|fmt_join("
        state.serialize_field(\"{var}\", &(self._inner.{var}{convert_serialize}))?;", "") }@
        @{- def.relations_one_and_belonging(false)|fmt_rel_join("
        if let Some(v) = &self.{rel_name} {
            state.serialize_field(\"{raw_rel_name}\", v)?;
        }", "") }@
        @{- def.relations_many(false)|fmt_rel_join("
        if let Some(v) = &self.{rel_name} {
            state.serialize_field(\"{raw_rel_name}\", v)?;
        }", "") }@
        @{- def.relations_belonging_outer_db(false)|fmt_rel_outer_db_join("
        if let Some(v) = &self.{rel_name} {
            state.serialize_field(\"{raw_rel_name}\", v)?;
        }", "") }@
        state.end()
    }
}
@%- if !config.force_disable_cache %@

impl Serialize for _@{ pascal_name }@Cache {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = @{ def.serializable_cache().len() + def.relations_one_cache(false).len() + def.relations_belonging_cache(false).len() + def.relations_many_cache(false).len() }@;
        let mut state = serializer.serialize_struct("@{ pascal_name }@", len)?;
        @{- def.serializable_cache()|fmt_join("
        state.serialize_field(\"{var}\", &(self._wrapper._inner.{var}{convert_serialize}))?;", "") }@
        @{- def.relations_one_cache(false)|fmt_rel_join("
        if let Ok(v) = &self._{raw_rel_name}() {
            state.serialize_field(\"{raw_ident_rel_name}\", v)?;
        }", "") }@
        @{- def.relations_many_cache(false)|fmt_rel_join("
        if let Ok(v) = &self._{raw_rel_name}() {
            state.serialize_field(\"{raw_ident_rel_name}\", v)?;
        }", "") }@
        @{- def.relations_belonging_cache(false)|fmt_rel_join("
        if self.{rel_name}.is_some() {
            state.serialize_field(\"{raw_ident_rel_name}\", &self.{rel_name})?;
        }", "") }@
        state.end()
    }
}
@%- endif %@

impl _@{ pascal_name }@Factory {
    #[allow(clippy::needless_update)]
    pub fn create(self) -> _@{ pascal_name }@Updater {
        _@{ pascal_name }@Updater {
            _data: Data {
@{ def.for_factory()|fmt_join("                {var}: self.{var}{convert_factory},", "\n") }@
                ..Data::default()
            },
            _update: Data::default(),
            _filter_flag: Default::default(),
            _is_new: true,
            _do_delete: false,
            _upsert: false,
            _is_loaded: true,
            _op: OpData::default(),
@{- def.relations_one(false)|fmt_rel_join("\n            {rel_name}: self.{rel_name}.map(|v| vec![v.create()]),", "") }@
@{- def.relations_many(false)|fmt_rel_join("\n            {rel_name}: self.{rel_name}.map(|v| v.into_iter().map(|v| v.create()).collect()),", "") }@
@{- def.relations_belonging(false)|fmt_rel_join("\n            {rel_name}: None,", "") }@
@{- def.relations_belonging_outer_db(false)|fmt_rel_outer_db_join("\n            {rel_name}: None,", "") }@
        }
    }
}

impl From<_@{ pascal_name }@Updater> for _@{ pascal_name }@ {
    fn from(from: _@{ pascal_name }@Updater) -> Self {
        let mut to: _@{ pascal_name }@ = (from._data, from._filter_flag).into();
@{- def.relations_one(false)|fmt_rel_join("
        to.{rel_name} = from.{rel_name}.map(|v| v.into_iter().filter(|v| !v.will_be_deleted()).next_back().map(|v| Box::new(v.into())));", "") }@
@{- def.relations_many(false)|fmt_rel_join("
        to.{rel_name} = from.{rel_name}.map(|v| v.into_iter().map(|v| v.into()).collect());", "") }@
@{- def.relations_belonging(false)|fmt_rel_join("
        to.{rel_name} = from.{rel_name};", "") }@
@{- def.relations_belonging_outer_db(false)|fmt_rel_outer_db_join("
        to.{rel_name} = from.{rel_name};", "") }@
        to
    }
}
impl From<Box<_@{ pascal_name }@Updater>> for Box<_@{ pascal_name }@> {
    fn from(from: Box<_@{ pascal_name }@Updater>) -> Self {
        let mut to: _@{ pascal_name }@ = (from._data, from._filter_flag).into();
@{- def.relations_one(false)|fmt_rel_join("
        to.{rel_name} = from.{rel_name}.map(|v| v.into_iter().filter(|v| !v.will_be_deleted()).next_back().map(|v| Box::new(v.into())));", "") }@
@{- def.relations_many(false)|fmt_rel_join("
        to.{rel_name} = from.{rel_name}.map(|v| v.into_iter().map(|v| v.into()).collect());", "") }@
@{- def.relations_belonging(false)|fmt_rel_join("
        to.{rel_name} = from.{rel_name};", "") }@
@{- def.relations_belonging_outer_db(false)|fmt_rel_outer_db_join("
        to.{rel_name} = from.{rel_name};", "") }@
        Box::new(to)
    }
}
@{-"\n"}@