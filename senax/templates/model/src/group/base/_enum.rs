use schemars::JsonSchema;
use serde_repr::{Deserialize_repr, Serialize_repr};
use strum::{EnumMessage, EnumString, IntoStaticStr};

use crate::misc::BindValue;

@{ def.title|comment0 -}@
@{ def.comment|comment0 -}@
#[derive(Serialize_repr, Deserialize_repr, Hash, Eq, PartialEq, Clone, Copy, Debug, strum::Display, EnumMessage, EnumString, IntoStaticStr, JsonSchema)]
#[repr(u8)]
#[allow(non_camel_case_types)]
pub enum @{ pascal_name }@ {
@% for row in def.enum_values -%@@{ row.title|comment4 }@@{ row.comment|comment4 }@@{ row.title|strum_message4 }@@{ row.comment|strum_detailed4 }@    @{ row.name }@ = @{ row.value }@,
@% endfor -%@
}
impl @{ pascal_name }@ {
    pub fn get(&self) -> u8 {
        *self as u8
    }
}
impl From<u8> for @{ pascal_name }@ {
    fn from(val: u8) -> Self {
        match val {
@% for row in def.enum_values %@            @{ row.value }@ => @{ pascal_name }@::@{ row.name }@,
@% endfor %@            _ => panic!("{} is a value outside the range of @{ pascal_name }@.", val),
        }
    }
}
impl From<@{ pascal_name }@> for u8 {
    fn from(val: @{ pascal_name }@) -> Self {
        val.get()
    }
}
impl From<@{ pascal_name }@> for BindValue {
    fn from(val: @{ pascal_name }@) -> Self {
        Self::Enum(Some(val.get()))
    }
}
impl From<Option<@{ pascal_name }@>> for BindValue {
    fn from(val: Option<@{ pascal_name }@>) -> Self {
        Self::Enum(val.map(|t| t.get()))
    }
}
@{-"\n"}@