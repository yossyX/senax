// This code is auto-generated and will always be overwritten.

use schemars::JsonSchema;
use serde_json::Value;
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::hash::Hash;
use std::vec::Vec;
use strum::{EnumMessage, EnumString, FromRepr, IntoStaticStr};

use crate::misc::BindValue;

@% for mod_name in def.relation_mods() -%@
use crate::models::@{ mod_name[0]|to_var_name }@::_base::_@{ mod_name[1] }@ as rel_@{ mod_name[0] }@_@{ mod_name[1] }@;
@% endfor %@
@% for (name, column_def) in def.num_enums(false) -%@
@% let values = column_def.enum_values.as_ref().unwrap() -%@
#[derive(Serialize_repr, Deserialize_repr, sqlx::Type, Hash, PartialEq, Eq, Clone, Copy, Debug, Default, strum::Display, EnumMessage, EnumString, FromRepr, IntoStaticStr, JsonSchema)]
#[repr(@{ column_def.get_inner_type(true, true) }@)]
#[allow(non_camel_case_types)]
#[allow(clippy::upper_case_acronyms)]
pub@{ visibility }@ enum _@{ name|pascal }@ {
@% for row in values -%@@{ row.label|label4 }@@{ row.comment|comment4 }@@{ row.label|strum_message4 }@@{ row.comment|strum_detailed4 }@    @% if loop.first %@#[default]@% endif %@@{ row.name|to_var_name }@@{ row.value_str() }@,
@% endfor -%@
}
impl _@{ name|pascal }@ {
    pub fn inner(&self) -> @{ column_def.get_inner_type(true, true) }@ {
        *self as @{ column_def.get_inner_type(true, true) }@
    }
@%- for row in values %@
    pub fn is_@{ row.name }@(&self) -> bool {
        self == &Self::@{ row.name|to_var_name }@
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
@%- if !config.excluded_from_domain %@
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
#[derive(Serialize, Deserialize, Hash, PartialEq, Eq, Clone, Copy, Debug, Default, strum::Display, EnumMessage, EnumString, IntoStaticStr, JsonSchema)]
#[allow(non_camel_case_types)]
#[allow(clippy::upper_case_acronyms)]
pub@{ visibility }@ enum _@{ name|pascal }@ {
@% for row in values -%@@{ row.label|label4 }@@{ row.comment|comment4 }@@{ row.label|strum_message4 }@@{ row.comment|strum_detailed4 }@    @% if loop.first %@#[default]@% endif %@@{ row.name|to_var_name }@,
@% endfor -%@
}
impl _@{ name|pascal }@ {
    pub fn as_static_str(&self) -> &'static str {
        Into::<&'static str>::into(self)
    }
@%- for row in values %@
    pub fn is_@{ row.name }@(&self) -> bool {
        self == &Self::@{ row.name|to_var_name }@
    }
@%- endfor %@
}
@%- if !config.excluded_from_domain %@
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
pub@{ visibility }@ trait _@{ pascal_name }@Getter: Send + Sync {
@{- def.primaries()|fmt_join("
{label}{comment}    fn _{raw_var}(&self) -> &{inner};", "") -}@
@{- def.non_primaries()|fmt_join("
{label}{comment}    fn _{raw_var}(&self) -> {outer};", "") -}@
@{- def.relations_one_and_belonging(false)|fmt_rel_join("
{label}{comment}    fn _{raw_rel_name}(&self) -> Option<&rel_{class_mod}::{class}>;", "") -}@
@{- def.relations_many(false)|fmt_rel_join("
{label}{comment}    fn _{raw_rel_name}(&self) -> &Vec<rel_{class_mod}::{class}>;", "") -}@
}
@{-"\n"}@