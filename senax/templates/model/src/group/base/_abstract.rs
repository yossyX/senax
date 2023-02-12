// This code is auto-generated and will always be overwritten.

use schemars::JsonSchema;
use serde_json::Value;
use serde_repr::{Deserialize_repr, Serialize_repr};
use sqlx::Row;
use std::hash::Hash;
use std::vec::Vec;
use strum::{EnumMessage, EnumString, IntoStaticStr};

use crate::misc::BindValue;

@% for mod_name in def.relation_mods() -%@
use crate::@{ mod_name[0] }@::@{ mod_name[1] }@::_@{ mod_name[1] }@ as rel_@{ mod_name[0] }@_@{ mod_name[1] }@;
@% endfor %@
@% for (name, column_def) in def.enums() -%@
@% if column_def.enum_values.is_some() -%@
@% let values = column_def.enum_values.as_ref().unwrap() -%@
#[derive(async_graphql::Enum, Serialize_repr, Deserialize_repr, Hash, Eq, PartialEq, Clone, Copy, Debug, strum::Display, EnumMessage, EnumString, IntoStaticStr, JsonSchema)]
#[repr(u8)]
#[allow(non_camel_case_types)]
#[graphql(name="@{ group_name|to_pascal_name }@@{ mod_name|to_pascal_name }@@{ name|to_pascal_name }@")]
pub enum _@{ name|to_pascal_name }@ {
@% for row in values -%@@{ row.title|comment4 }@@{ row.comment|comment4 }@@{ row.title|strum_message4 }@@{ row.comment|strum_detailed4 }@    @{ row.name }@ = @{ row.value }@,
@% endfor -%@
}
impl _@{ name|to_pascal_name }@ {
    pub fn get(&self) -> u8 {
        *self as u8
    }
}
impl From<u8> for _@{ name|to_pascal_name }@ {
    fn from(val: u8) -> Self {
        match val {
@% for row in values %@            @{ row.value }@ => _@{ name|to_pascal_name }@::@{ row.name }@,
@% endfor %@            _ => panic!("{} is a value outside the range of _@{ name|to_pascal_name }@.", val),
        }
    }
}
impl From<_@{ name|to_pascal_name }@> for u8 {
    fn from(val: _@{ name|to_pascal_name }@) -> Self {
        val.get()
    }
}
impl From<_@{ name|to_pascal_name }@> for BindValue {
    fn from(val: _@{ name|to_pascal_name }@) -> Self {
        Self::Enum(Some(val.get()))
    }
}
impl From<Option<_@{ name|to_pascal_name }@>> for BindValue {
    fn from(val: Option<_@{ name|to_pascal_name }@>) -> Self {
        Self::Enum(val.map(|t| t.get()))
    }
}

@% endif -%@
@% endfor -%@
@% for (name, column_def) in def.db_enums() -%@
@% if column_def.db_enum_values.is_some() -%@
@% let values = column_def.db_enum_values.as_ref().unwrap() -%@
#[derive(async_graphql::Enum, Serialize, Deserialize, Hash, Eq, PartialEq, Clone, Copy, Debug, strum::Display, EnumMessage, EnumString, IntoStaticStr)]
#[allow(non_camel_case_types)]
#[graphql(name="@{ group_name|to_pascal_name }@@{ mod_name|to_pascal_name }@@{ name|to_pascal_name }@")]
pub enum _@{ name|to_pascal_name }@ {
@% for row in values -%@@{ row.title|comment4 }@@{ row.comment|comment4 }@@{ row.title|strum_message4 }@@{ row.comment|strum_detailed4 }@    @{ row.name }@,
@% endfor -%@
}

@% endif -%@
@% endfor -%@
@{ def.title|comment0 -}@
@{ def.comment|comment0 -}@
pub trait _@{ pascal_name }@Tr {
@{ def.primaries()|fmt_join("{title}{comment}    fn {var}(&self) -> &{inner};
", "") -}@
@{ def.non_primaries()|fmt_join("{title}{comment}    fn {var}(&self) -> {outer};
", "") -}@
@{ def.relations_one_except_cache()|fmt_rel_join("{title}{comment}    fn {alias}(&self) -> Option<&rel_{class_mod}::{class}>;
", "") -}@
@{ def.relations_one_only_cache()|fmt_rel_join("{title}{comment}    fn {alias}(&self) -> Option<&rel_{class_mod}::{class}Cache>;
", "") -}@
@{ def.relations_many()|fmt_rel_join("{title}{comment}    fn {alias}(&self) -> &Vec<rel_{class_mod}::{class}>;
", "") -}@
}
@{-"\n"}@