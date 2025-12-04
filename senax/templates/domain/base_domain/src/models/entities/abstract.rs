#[allow(unused_imports)]
use crate as domain;
#[allow(unused_imports)]
use crate::value_objects;
#[allow(unused_imports)]
use crate::models::@{ db|snake|ident }@ as _model_;

pub mod consts {
@{- def.all_fields()|fmt_join("{api_validate_const}", "") }@
}

@% for (name, column_def) in def.num_enums(true) -%@
@% let values = column_def.enum_values.as_ref().unwrap() -%@
#[derive(async_graphql::Enum, serde_repr::Serialize_repr, serde_repr::Deserialize_repr, Hash, PartialEq, Eq, Clone, Copy, Debug, Default, strum::Display, strum::EnumMessage, strum::EnumString, strum::IntoStaticStr, strum::FromRepr, schemars::JsonSchema)]
#[repr(@{ column_def.get_inner_type(true, true) }@)]
#[allow(non_camel_case_types)]
#[allow(clippy::upper_case_acronyms)]
#[graphql(name="@{ config.layer_name(db, group_name) }@@{ mod_name|pascal }@@{ name|pascal }@")]
#[derive(utoipa::ToSchema)]
#[schema(as = @{ config.layer_name(db, group_name) }@@{ mod_name|pascal }@@{ name|pascal }@)]
pub enum @{ name|to_pascal_name }@ {
@%- for row in values %@
    #[graphql(name="@{ row.name }@")]
@{ row.label|label4 }@@{ row.comment|comment4 }@@{ row.label|strum_message4 }@@{ row.comment|strum_detailed4 }@    @% if loop.first %@#[default]@% endif %@@{ row.name|ident }@@{ row.value_str() }@,
@%- endfor %@
}
#[allow(non_snake_case)]
impl @{ name|to_pascal_name }@ {
    pub fn inner(&self) -> @{ column_def.get_inner_type(true, true) }@ {
        *self as @{ column_def.get_inner_type(true, true) }@
    }
@%- for row in values %@
    pub fn is_@{ row.name }@(&self) -> bool {
        self == &Self::@{ row.name|ident }@
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
#[graphql(name="@{ config.layer_name(db, group_name) }@@{ mod_name|pascal }@@{ name|pascal }@")]
#[derive(utoipa::ToSchema)]
#[schema(as = @{ config.layer_name(db, group_name) }@@{ mod_name|pascal }@@{ name|pascal }@)]
pub enum @{ name|to_pascal_name }@ {
@%- for row in values %@
    #[graphql(name="@{ row.name }@")]
@{ row.label|label4 }@@{ row.comment|comment4 }@@{ row.label|strum_message4 }@@{ row.comment|strum_detailed4 }@    @% if loop.first %@#[default]@% endif %@@{ row.name|ident }@,
@%- endfor %@
}
#[allow(non_snake_case)]
impl @{ name|to_pascal_name }@ {
    pub fn as_static_str(&self) -> &'static str {
        Into::<&'static str>::into(self)
    }
@%- for row in values %@
    pub fn is_@{ row.name }@(&self) -> bool {
        self == &Self::@{ row.name|ident }@
    }
@%- endfor %@
}
@% endfor -%@

pub trait @{ pascal_name }@: std::fmt::Debug@% if !def.parents().is_empty() %@@% for parent in def.parents() %@ + super::super::@{ parent.group_name|ident }@::@{ parent.name|ident }@::@{ parent.name|pascal }@@% endfor %@@% endif %@ {
@{- def.primaries()|fmt_join("
{label}{comment}    fn _{raw_name}(&self) -> {inner};", "") }@
@{- def.only_version()|fmt_join("
{label}{comment}    fn {ident}(&self) -> {outer};", "") }@
@{- def.cols_except_primaries_and_invisibles()|fmt_join("
{label}{comment}    fn {ident}(&self) -> {domain_outer};", "") }@
@{- def.relations_belonging(true)|fmt_rel_join("
    fn _{raw_rel_name}_id(&self) -> Option<_model_::{class_mod_path}::{class}Primary> {
        Some({local_keys}.into())
    }", "") }@
@{- def.relations_one_and_belonging(true)|fmt_rel_join("
{label}{comment}    fn {rel_name}(&self) -> anyhow::Result<Option<&dyn _model_::{class_mod_path}::{class}>>;", "") }@
@{- def.relations_many(true)|fmt_rel_join("
{label}{comment}    fn {rel_name}(&self) -> anyhow::Result<Box<dyn Iterator<Item = &dyn _model_::{class_mod_path}::{class}> + '_>>;", "") }@
}

@{ def.label|label0 -}@
@{ def.comment|comment0 -}@
pub trait @{ pascal_name }@Updater: @{ pascal_name }@ + crate::models::MarkForDelete@% if !def.parents().is_empty() %@@% for parent in def.parents() %@ + super::super::@{ parent.group_name|ident }@::@{ parent.name|ident }@::@{ parent.name|pascal }@Updater@% endfor %@@% endif %@ {
@{- def.non_primaries_except_invisible_and_read_only(true)|fmt_join("
{label}{comment}    fn set_{raw_name}(&mut self, v: {domain_factory});", "") }@
@{- def.relations_one(true)|fmt_rel_join("
{label}{comment}    fn {rel_name}(&mut self) -> anyhow::Result<Option<&mut dyn _model_::{class_mod_path}::{class}Updater>>;
{label}{comment}    fn set_{raw_rel_name}(&mut self, v: Box<dyn _model_::{class_mod_path}::{class}Updater>);", "") }@
@{- def.relations_many(true)|fmt_rel_join("
{label}{comment}    fn {rel_name}(&mut self) -> anyhow::Result<Box<dyn domain::models::UpdateIterator<dyn _model_::{class_mod_path}::{class}Updater> + '_>>;
{label}{comment}    fn take_{raw_rel_name}(&mut self) -> Option<Vec<Box<dyn _model_::{class_mod_path}::{class}Updater>>>;
{label}{comment}    fn replace_{raw_rel_name}(&mut self, list: Vec<Box<dyn _model_::{class_mod_path}::{class}Updater>>);
{label}{comment}    fn push_{raw_rel_name}(&mut self, v: Box<dyn _model_::{class_mod_path}::{class}Updater>);", "") }@
}
@{-"\n"}@