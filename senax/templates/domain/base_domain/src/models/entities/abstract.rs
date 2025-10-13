#[allow(unused_imports)]
use crate as domain;
#[allow(unused_imports)]
use crate::value_objects;
#[allow(unused_imports)]
use crate::models::@{ db|snake|to_var_name }@ as _model_;

pub mod consts {
@{- def.all_fields()|fmt_join("{api_validate_const}", "") }@
}

@% for (name, column_def) in def.num_enums(true) -%@
@% let values = column_def.enum_values.as_ref().unwrap() -%@
#[derive(async_graphql::Enum, serde_repr::Serialize_repr, serde_repr::Deserialize_repr, Hash, PartialEq, Eq, Clone, Copy, Debug, Default, strum::Display, strum::EnumMessage, strum::EnumString, strum::IntoStaticStr, strum::FromRepr, schemars::JsonSchema)]
#[repr(@{ column_def.get_inner_type(true, true) }@)]
#[allow(non_camel_case_types)]
#[allow(clippy::upper_case_acronyms)]
#[graphql(name="@{ db|pascal }@@{ group_name|pascal }@@{ mod_name|pascal }@@{ name|pascal }@")]
#[derive(utoipa::ToSchema)]
#[schema(as = @{ db|pascal }@@{ group_name|pascal }@@{ mod_name|pascal }@@{ name|pascal }@)]
pub enum @{ name|to_pascal_name }@ {
@% for row in values -%@@{ row.label|label4 }@@{ row.comment|comment4 }@@{ row.label|strum_message4 }@@{ row.comment|strum_detailed4 }@    @% if loop.first %@#[default]@% endif %@@{ row.name|to_var_name }@@{ row.value_str() }@,
@% endfor -%@
}
impl @{ name|to_pascal_name }@ {
    pub fn inner(&self) -> @{ column_def.get_inner_type(true, true) }@ {
        *self as @{ column_def.get_inner_type(true, true) }@
    }
@%- for row in values %@
    pub fn is_@{ row.name }@(&self) -> bool {
        self == &Self::@{ row.name|to_var_name }@
    }
@%- endfor %@
}
impl From<@{ column_def.get_inner_type(true, true) }@> for @{ name|to_pascal_name }@ {
    fn from(val: @{ column_def.get_inner_type(true, true) }@) -> Self {
        if let Some(val) = Self::from_repr(val) {
            val
        } else {
            panic!("{} is a value outside the range of @{ name|pascal }@.", val)
        }
    }
}
impl From<@{ name|to_pascal_name }@> for @{ column_def.get_inner_type(true, true) }@ {
    fn from(val: @{ name|to_pascal_name }@) -> Self {
        val.inner()
    }
}

@% endfor -%@
@% for (name, column_def) in def.str_enums(true) -%@
@% let values = column_def.enum_values.as_ref().unwrap() -%@
#[derive(async_graphql::Enum, serde::Deserialize, serde::Serialize, Hash, PartialEq, Eq, Clone, Copy, Debug, Default, strum::Display, strum::EnumMessage, strum::EnumString, strum::IntoStaticStr, schemars::JsonSchema)]
#[allow(non_camel_case_types)]
#[allow(clippy::upper_case_acronyms)]
#[graphql(name="@{ db|pascal }@@{ group_name|pascal }@@{ mod_name|pascal }@@{ name|pascal }@")]
#[derive(utoipa::ToSchema)]
#[schema(as = @{ db|pascal }@@{ group_name|pascal }@@{ mod_name|pascal }@@{ name|pascal }@)]
pub enum @{ name|to_pascal_name }@ {
@% for row in values -%@@{ row.label|label4 }@@{ row.comment|comment4 }@@{ row.label|strum_message4 }@@{ row.comment|strum_detailed4 }@    @% if loop.first %@#[default]@% endif %@@{ row.name|to_var_name }@,
@% endfor -%@
}
impl @{ name|to_pascal_name }@ {
    pub fn as_static_str(&self) -> &'static str {
        Into::<&'static str>::into(self)
    }
@%- for row in values %@
    pub fn is_@{ row.name }@(&self) -> bool {
        self == &Self::@{ row.name|to_var_name }@
    }
@%- endfor %@
}
@% endfor -%@

pub trait @{ pascal_name }@Common: std::fmt::Debug@% if !def.parents().is_empty() %@@% for parent in def.parents() %@ + super::super::@{ parent.group_name|to_var_name }@::@{ parent.name|to_var_name }@::@{ parent.name|pascal }@Common@% endfor %@@% endif %@ {
@{- def.primaries()|fmt_join("
{label}{comment}    fn _{raw_var}(&self) -> {inner};", "") }@
@{- def.only_version()|fmt_join("
{label}{comment}    fn {var}(&self) -> {outer};", "") }@
@{- def.cache_cols_except_primaries_and_invisibles()|fmt_join("
{label}{comment}    fn {var}(&self) -> {domain_outer};", "") }@
}

@{ def.label|label0 -}@
@{ def.comment|comment0 -}@
pub trait @{ pascal_name }@Cache: @{ pascal_name }@Common@% if !def.parents().is_empty() %@@% for parent in def.parents() %@ + super::super::@{ parent.group_name|to_var_name }@::@{ parent.name|to_var_name }@::@{ parent.name|pascal }@Cache@% endfor %@@% endif %@ {
@{- def.relations_one_cache(true)|fmt_rel_join("
{label}{comment}    fn {rel_name}(&self) -> anyhow::Result<Option<Box<dyn _model_::{class_mod_var}::{class}Cache>>>;", "") }@
@{- def.relations_many_cache(true)|fmt_rel_join("
{label}{comment}    fn {rel_name}(&self) -> anyhow::Result<Vec<Box<dyn _model_::{class_mod_var}::{class}Cache>>>;", "") }@
@{- def.relations_belonging_cache(true)|fmt_rel_join("
    fn _{raw_rel_name}_id(&self) -> Option<_model_::{class_mod_var}::{class}Primary> {
        Some({local_keys}.into())
    }", "") }@
@{- def.relations_belonging_cache(true)|fmt_rel_join("
{label}{comment}    fn {rel_name}(&self) -> anyhow::Result<Option<Box<dyn _model_::{class_mod_var}::{class}Cache>>>;", "") }@
}

@{ def.label|label0 -}@
@{ def.comment|comment0 -}@
pub trait @{ pascal_name }@: @{ pascal_name }@Common@% if !def.parents().is_empty() %@@% for parent in def.parents() %@ + super::super::@{ parent.group_name|to_var_name }@::@{ parent.name|to_var_name }@::@{ parent.name|pascal }@@% endfor %@@% endif %@ {
@{- def.non_cache_cols_except_primaries_and_invisibles()|fmt_join("
{label}{comment}    fn {var}(&self) -> {domain_outer};", "") }@
@{- def.relations_belonging(true)|fmt_rel_join("
    fn _{raw_rel_name}_id(&self) -> Option<_model_::{class_mod_var}::{class}Primary> {
        Some({local_keys}.into())
    }", "") }@
@{- def.relations_one_and_belonging(true)|fmt_rel_join("
{label}{comment}    fn {rel_name}(&self) -> anyhow::Result<Option<&dyn _model_::{class_mod_var}::{class}>>;", "") }@
@{- def.relations_many(true)|fmt_rel_join("
{label}{comment}    fn {rel_name}(&self) -> anyhow::Result<Box<dyn Iterator<Item = &dyn _model_::{class_mod_var}::{class}> + '_>>;", "") }@
}

@{ def.label|label0 -}@
@{ def.comment|comment0 -}@
pub trait @{ pascal_name }@Updater: @{ pascal_name }@Common + crate::models::MarkForDelete@% if !def.parents().is_empty() %@@% for parent in def.parents() %@ + super::super::@{ parent.group_name|to_var_name }@::@{ parent.name|to_var_name }@::@{ parent.name|pascal }@Updater@% endfor %@@% endif %@ {
@{- def.non_cache_cols_except_primaries_and_invisibles()|fmt_join("
{label}{comment}    fn {var}(&self) -> {domain_outer};", "") }@
@{- def.non_primaries_except_invisible_and_read_only(true)|fmt_join("
{label}{comment}    fn set_{raw_var}(&mut self, v: {domain_factory});", "") }@
@{- def.relations_one(true)|fmt_rel_join("
{label}{comment}    fn {rel_name}(&mut self) -> anyhow::Result<Option<&mut dyn _model_::{class_mod_var}::{class}Updater>>;
{label}{comment}    fn set_{raw_rel_name}(&mut self, v: Box<dyn _model_::{class_mod_var}::{class}Updater>);", "") }@
@{- def.relations_many(true)|fmt_rel_join("
{label}{comment}    fn {rel_name}(&mut self) -> anyhow::Result<Box<dyn domain::models::UpdateIterator<dyn _model_::{class_mod_var}::{class}Updater> + '_>>;
{label}{comment}    fn take_{raw_rel_name}(&mut self) -> Option<Vec<Box<dyn _model_::{class_mod_var}::{class}Updater>>>;
{label}{comment}    fn replace_{raw_rel_name}(&mut self, list: Vec<Box<dyn _model_::{class_mod_var}::{class}Updater>>);
{label}{comment}    fn push_{raw_rel_name}(&mut self, v: Box<dyn _model_::{class_mod_var}::{class}Updater>);", "") }@
}
@{-"\n"}@