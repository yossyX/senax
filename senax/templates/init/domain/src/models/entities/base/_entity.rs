// This code is auto-generated and will always be overwritten.
use async_trait::async_trait;

#[allow(unused_imports)]
use crate as domain;
use crate::models::Check_;
#[allow(unused_imports)]
use crate::models::{self, ToGeoPoint as _, ToPoint as _};
#[allow(unused_imports)]
use crate::value_objects;

use super::super::@{ mod_name|to_var_name }@ as _self;
use super::super::@{ mod_name|to_var_name }@::@{ pascal_name }@Updater as _Updater;
#[allow(unused_imports)]
use crate::models::@{ db|snake|to_var_name }@ as _model_;

pub mod consts {
@{- def.all_fields()|fmt_join("{api_validate_const}", "") }@
}

@% for (name, column_def) in def.id() -%@
#[derive(serde::Deserialize, serde::Serialize, Hash, PartialEq, Eq, PartialOrd, Ord, Clone,@% if column_def.is_copyable() %@ Copy,@% endif %@ derive_more::Display, Debug, Default)]
#[serde(transparent)]
@%- if !column_def.is_displayable() %@
#[display(fmt = "{:?}", _0)]
@%- endif %@
pub struct @{ id_name }@(@{ column_def.get_inner_type(false, false) }@);
async_graphql::scalar!(@{ id_name }@, "@{ db|pascal }@@{ group_name|pascal }@@{ id_name }@");

impl @{ id_name }@ {
    pub fn inner(&self) -> @{ column_def.get_inner_type(false, false) }@ {
        self.0@{ column_def.clone_str() }@
    }
}
impl std::ops::Deref for @{ id_name }@ {
    type Target = @{ column_def.get_deref_type(false) }@;
    fn deref(&self) -> &@{ column_def.get_deref_type(false) }@ {
        &self.0
    }
}
impl From<@{ column_def.get_inner_type(false, false) }@> for @{ id_name }@ {
    fn from(id: @{ column_def.get_inner_type(false, false) }@) -> Self {
        Self(id)
    }
}
@%- if column_def.get_inner_type(false, true) != column_def.get_inner_type(true, true) %@
impl From<@{ column_def.get_inner_type(true, true) }@> for @{ id_name }@ {
    fn from(id: @{ column_def.get_inner_type(true, true) }@) -> Self {
        Self(id.into())
    }
}
@%- endif %@
@%- if column_def.get_inner_type(true, true) == "String" %@
impl From<&str> for @{ id_name }@ {
    fn from(id: &str) -> Self {
        Self(id.to_string().into())
    }
}
@%- endif %@
impl From<@{ id_name }@> for @{ column_def.get_inner_type(false, false) }@ {
    fn from(id: @{ id_name }@) -> Self {
        id.0
    }
}
impl From<&@{ id_name }@> for @{ id_name }@ {
    fn from(id: &@{ id_name }@) -> Self {
        Self(id.inner())
    }
}
@%- endfor %@

#[derive(Clone)]
pub struct @{ pascal_name }@Primary(@{ def.primaries()|fmt_join("pub {domain_outer_owned}", ", ") }@);

#[allow(clippy::clone_on_copy)]
impl From<@{ def.primaries()|fmt_join_with_paren("{outer_ref}", ", ") }@> for @{ pascal_name }@Primary {
    fn from(id: @{ def.primaries()|fmt_join_with_paren("{outer_ref}", ", ") }@) -> Self {
        @% if def.primaries().len() == 1 -%@
        Self(id.to_owned().into())
        @%- else -%@
        Self(@{ def.primaries()|fmt_join("id.{index}.to_owned().into()", ", ") }@)
        @%- endif %@
    }
}
@%- if def.primaries()|fmt_join_with_paren("{outer_ref}", ", ") != def.primaries()|fmt_join_with_paren("{inner}", ", ") %@
#[allow(clippy::useless_conversion)]
impl From<@{ def.primaries()|fmt_join_with_paren("{inner}", ", ") }@> for @{ pascal_name }@Primary {
    fn from(id: @{ def.primaries()|fmt_join_with_paren("{inner}", ", ") }@) -> Self {
        @% if def.primaries().len() == 1 %@Self(id.into())@% else %@Self(@{ def.primaries()|fmt_join("id.{index}.into()", ", ") }@)@% endif %@
    }
}
@%- endif %@
@%- if def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") != def.primaries()|fmt_join_with_paren("{inner}", ", ") %@
impl From<@{ def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") }@> for @{ pascal_name }@Primary {
    fn from(id: @{ def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") }@) -> Self {
        @% if def.primaries().len() == 1 %@Self(id)@% else %@Self(@{ def.primaries()|fmt_join("id.{index}", ", ") }@)@% endif %@
    }
}
@%- endif %@
@% if def.primaries().len() > 1 -%@
impl From<&@{ def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") }@> for @{ pascal_name }@Primary {
    fn from(id: &@{ def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") }@) -> Self {
        Self(@{ def.primaries()|fmt_join("id.{index}{clone}", ", ") }@)
    }
}
@% endif -%@
impl From<@{ pascal_name }@Primary> for @{ def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") }@ {
    fn from(id: @{ pascal_name }@Primary) -> Self {
        @{ def.primaries()|fmt_join_with_paren("id.{index}", ", ") }@
    }
}
#[allow(clippy::useless_conversion)]
impl TryFrom<&async_graphql::ID> for @{ pascal_name }@Primary {
    type Error = anyhow::Error;
    fn try_from(v: &async_graphql::ID) -> Result<Self, Self::Error> {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
        let bytes = URL_SAFE_NO_PAD.decode(v.as_str())?;
        let (model_id, id): (u64, @{ def.primaries()|fmt_join_with_paren("{inner}", ", ") }@) = ciborium::from_reader(bytes.as_slice())?;
        anyhow::ensure!(
            model_id == _self::MODEL_ID,
            "{} is not an ID of the @{ pascal_name }@ model.",
            v.as_str()
        );
        Ok(@% if def.primaries().len() == 1 %@Self(id.into())@% else %@Self(@{ def.primaries()|fmt_join("id.{index}.into()", ", ") }@)@% endif %@)
    }
}

fn to_graphql_id(id: @{ def.primaries()|fmt_join_with_paren("{inner}", ", ") }@) -> async_graphql::ID {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    let v: (u64, @{ def.primaries()|fmt_join_with_paren("{inner}", ", ") }@) = (_self::MODEL_ID, id);
    let mut buf = Vec::new();
    ciborium::into_writer(&v, &mut buf).unwrap();
    URL_SAFE_NO_PAD.encode(buf).into()
}
#[allow(clippy::useless_conversion)]
impl From<&dyn @{ pascal_name }@> for async_graphql::ID {
    fn from(obj: &dyn @{ pascal_name }@) -> Self {
        to_graphql_id(@{ def.primaries()|fmt_join_with_paren("obj.{var}().to_owned().into()", ", ") }@)
    }
}
#[allow(clippy::useless_conversion)]
impl From<&dyn @{ pascal_name }@Cache> for async_graphql::ID {
    fn from(obj: &dyn @{ pascal_name }@Cache) -> Self {
        to_graphql_id(@{ def.primaries()|fmt_join_with_paren("obj.{var}().to_owned().into()", ", ") }@)
    }
}
#[allow(clippy::useless_conversion)]
impl From<&dyn _Updater> for async_graphql::ID {
    fn from(obj: &dyn _Updater) -> Self {
        to_graphql_id(@{ def.primaries()|fmt_join_with_paren("obj.{var}().to_owned().into()", ", ") }@)
    }
}
#[allow(clippy::useless_conversion)]
impl From<@{ pascal_name }@Primary> for async_graphql::ID {
    fn from(id: @{ pascal_name }@Primary) -> Self {
        to_graphql_id(@{ def.primaries()|fmt_join_with_paren("id.{index}.into()", ", ") }@)
    }
}
#[allow(clippy::useless_conversion)]
impl From<&@{ pascal_name }@Primary> for async_graphql::ID {
    fn from(id: &@{ pascal_name }@Primary) -> Self {
        to_graphql_id(@{ def.primaries()|fmt_join_with_paren("id.{index}{clone}.into()", ", ") }@)
    }
}

@% for (name, column_def) in def.num_enums(true) -%@
@% let values = column_def.enum_values.as_ref().unwrap() -%@
#[derive(async_graphql::Enum, serde_repr::Serialize_repr, serde_repr::Deserialize_repr, Hash, PartialEq, Eq, Clone, Copy, Debug, Default, strum::Display, strum::EnumMessage, strum::EnumString, strum::IntoStaticStr, strum::FromRepr, schemars::JsonSchema)]
#[repr(@{ column_def.get_inner_type(true, true) }@)]
#[allow(non_camel_case_types)]
#[allow(clippy::upper_case_acronyms)]
#[graphql(name="@{ db|pascal }@@{ group_name|pascal }@@{ mod_name|pascal }@@{ name|pascal }@")]
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
#[derive(async_graphql::Enum, serde::Serialize, serde::Deserialize, Hash, PartialEq, Eq, Clone, Copy, Debug, Default, strum::Display, strum::EnumMessage, strum::EnumString, strum::IntoStaticStr, schemars::JsonSchema)]
#[allow(non_camel_case_types)]
#[allow(clippy::upper_case_acronyms)]
#[graphql(name="@{ db|pascal }@@{ group_name|pascal }@@{ mod_name|pascal }@@{ name|pascal }@")]
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

pub trait @{ pascal_name }@Common: std::fmt::Debug@% for parent in def.parent() %@ + super::super::super::@{ parent.group_name|to_var_name }@::@{ parent.name|to_var_name }@::@{ parent.name|pascal }@Common@% endfor %@ + 'static {
@{- def.primaries()|fmt_join("
{label}{comment}    fn {var}(&self) -> {outer};", "") }@
@{- def.only_version()|fmt_join("
{label}{comment}    fn {var}(&self) -> {outer};", "") }@
@{- def.cache_cols_wo_primaries_and_read_only()|fmt_join("
{label}{comment}    fn {var}(&self) -> {domain_outer};", "") }@
}

@{ def.label|label0 -}@
@{ def.comment|comment0 -}@
pub trait @{ pascal_name }@Cache: @{ pascal_name }@Common + dyn_clone::DynClone + Send + Sync@% for parent in def.parent() %@ + super::super::super::@{ parent.group_name|to_var_name }@::@{ parent.name|to_var_name }@::@{ parent.name|pascal }@Cache@% endfor %@ + 'static {
@{- def.relations_one_cache(true)|fmt_rel_join("
{label}{comment}    fn {rel_name}(&self) -> Option<Box<dyn _model_::{class_mod_var}::{class}Cache>>;", "") }@
@{- def.relations_one_uncached(true)|fmt_rel_join("
{label}{comment}    fn {rel_name}(&self) -> Option<Box<dyn _model_::{class_mod_var}::{class}>>;", "") }@
@{- def.relations_many_cache(true)|fmt_rel_join("
{label}{comment}    fn {rel_name}(&self) -> Vec<Box<dyn _model_::{class_mod_var}::{class}Cache>>;", "") }@
@{- def.relations_many_uncached(true)|fmt_rel_join("
{label}{comment}    fn {rel_name}(&self) -> Vec<Box<dyn _model_::{class_mod_var}::{class}>>;", "") }@
@{- def.relations_belonging(true)|fmt_rel_join("
    fn _{raw_rel_name}_id(&self) -> Option<_model_::{class_mod_var}::{class}Primary> {
        Some({local_keys}.into())
    }", "") }@
@{- def.relations_belonging_cache(true)|fmt_rel_join("
{label}{comment}    fn {rel_name}(&self) -> Option<Box<dyn _model_::{class_mod_var}::{class}Cache>>;", "") }@
@{- def.relations_belonging_uncached(true)|fmt_rel_join("
{label}{comment}    fn {rel_name}(&self) -> Option<Box<dyn _model_::{class_mod_var}::{class}>>;", "") }@
}

@{ def.label|label0 -}@
@{ def.comment|comment0 -}@
pub trait @{ pascal_name }@: @{ pascal_name }@Common + Send + Sync@% for parent in def.parent() %@ + super::super::super::@{ parent.group_name|to_var_name }@::@{ parent.name|to_var_name }@::@{ parent.name|pascal }@@% endfor %@ + 'static {
@{- def.non_cache_cols_wo_primaries_and_read_only()|fmt_join("
{label}{comment}    fn {var}(&self) -> {domain_outer};", "") }@
@{- def.relations_belonging(true)|fmt_rel_join("
    fn _{raw_rel_name}_id(&self) -> Option<_model_::{class_mod_var}::{class}Primary> {
        Some({local_keys}.into())
    }", "") }@
@{- def.relations_one_and_belonging(true)|fmt_rel_join("
{label}{comment}    fn {rel_name}(&self) -> Option<&dyn _model_::{class_mod_var}::{class}>;", "") }@
@{- def.relations_many(true)|fmt_rel_join("
{label}{comment}    fn {rel_name}(&self) -> Box<dyn Iterator<Item = &dyn _model_::{class_mod_var}::{class}> + '_>;", "") }@
}

@{ def.label|label0 -}@
pub trait @{ pascal_name }@UpdaterBase: downcast_rs::Downcast + Send + Sync + @{ pascal_name }@Common + crate::models::MarkForDelete@% for parent in def.parent() %@ + super::super::super::@{ parent.group_name|to_var_name }@::@{ parent.name|to_var_name }@::@{ parent.name|pascal }@Updater@% endfor %@ + 'static {
@{- def.non_cache_cols_wo_primaries_and_read_only()|fmt_join("
{label}{comment}    fn {var}(&self) -> {domain_outer};", "") }@
@{- def.non_primaries_wo_read_only(true)|fmt_join("
{label}{comment}    fn set_{raw_var}(&mut self, v: {domain_outer_owned});", "") }@
@{- def.relations_one(true)|fmt_rel_join("
{label}{comment}    fn {rel_name}(&mut self) -> Option<&mut dyn _model_::{class_mod_var}::{class}Updater>;
{label}{comment}    fn set_{raw_rel_name}(&mut self, v: Box<dyn _model_::{class_mod_var}::{class}Updater>);", "") }@
@{- def.relations_many(true)|fmt_rel_join("
{label}{comment}    fn {rel_name}(&mut self) -> Box<dyn domain::models::UpdateIterator<dyn _model_::{class_mod_var}::{class}Updater> + '_>;
{label}{comment}    fn take_{raw_rel_name}(&mut self) -> Option<Vec<Box<dyn _model_::{class_mod_var}::{class}Updater>>>;
{label}{comment}    fn replace_{raw_rel_name}(&mut self, list: Vec<Box<dyn _model_::{class_mod_var}::{class}Updater>>);
{label}{comment}    fn push_{raw_rel_name}(&mut self, v: Box<dyn _model_::{class_mod_var}::{class}Updater>);", "") }@
}
downcast_rs::impl_downcast!(_Updater);

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct @{ pascal_name }@Factory {
@{- def.non_auto_primary_for_factory()|fmt_join("
    pub {var}: {domain_outer_owned},", "") }@
}

impl @{ pascal_name }@Factory {
    pub fn from(value: serde_json::Value) -> anyhow::Result<Self> {
        Ok(serde_json::from_value(value)?)
    }
    pub fn create(self, repo: &dyn domain::models::Repositories) -> Box<dyn _Updater> {
        let repo = repo.@{ db|snake }@_repository().@{ group_name|to_var_name }@().@{ mod_name|to_var_name }@();
        repo.convert_factory(self)
    }
}

#[cfg(any(feature = "mock", test))]
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct @{ pascal_name }@Entity {
@{- def.primaries()|fmt_join("
    pub {var}: {domain_outer_owned},", "") }@
@{- def.non_primaries_wo_read_only(false)|fmt_join("
    pub {var}: {domain_outer_owned},", "") }@
@{- def.relations_one_and_belonging(false)|fmt_rel_join("
    pub {rel_name}: Option<Box<_model_::{class_mod_var}::{class}Entity>>,", "") }@
@{- def.relations_many(false)|fmt_rel_join("
    pub {rel_name}: Vec<Box<_model_::{class_mod_var}::{class}Entity>>,", "") }@
    #[serde(skip)]
    pub _delete: bool,
}

@%- for parent in def.parents() %@

#[cfg(any(feature = "mock", test))]
#[allow(clippy::useless_conversion)]
impl super::super::super::@{ parent.group_name|to_var_name }@::@{ parent.name|to_var_name }@::@{ parent.name|pascal }@Common for @{ pascal_name }@Entity {
@{- parent.primaries()|fmt_join("
    fn _{raw_var}(&self) -> {inner} {
        self.{var}.0{clone}
    }", "") }@
@{- parent.only_version()|fmt_join("
    fn {var}(&self) -> {outer} {
        1
    }", "") }@
@{- parent.cache_cols_wo_primaries_and_read_only()|fmt_join("
    fn {var}(&self) -> {domain_outer} {
        {convert_domain_outer_prefix}self.{var}{clone_for_outer}{convert_domain_outer}
    }", "") }@
}
#[cfg(any(feature = "mock", test))]
impl super::super::super::@{ parent.group_name|to_var_name }@::@{ parent.name|to_var_name }@::@{ parent.name|pascal }@Cache for @{ pascal_name }@Entity {
@{- parent.relations_one_cache(true)|fmt_rel_join("
    fn {rel_name}(&self) -> Option<Box<dyn _model_::{class_mod_var}::{class}Cache>> {
        self.{rel_name}.as_ref().map(|v| Box::<dyn _model_::{class_mod_var}::{class}Cache>::from(v.clone()))
    }", "") }@
@{- parent.relations_many_cache(true)|fmt_rel_join("
    fn {rel_name}(&self) -> Vec<Box<dyn _model_::{class_mod_var}::{class}Cache>> {
        self.{rel_name}.iter().map(|v| Box::<dyn _model_::{class_mod_var}::{class}Cache>::from(v.clone())).collect()
    }", "") }@
@{- parent.relations_belonging_cache(true)|fmt_rel_join("
    fn {rel_name}(&self) -> Option<Box<dyn _model_::{class_mod_var}::{class}Cache>> {
        self.{rel_name}.as_ref().map(|v| Box::<dyn _model_::{class_mod_var}::{class}Cache>::from(v.clone()))
    }", "") }@
}
#[cfg(any(feature = "mock", test))]
#[allow(clippy::useless_conversion)]
impl super::super::super::@{ parent.group_name|to_var_name }@::@{ parent.name|to_var_name }@::@{ parent.name|pascal }@ for @{ pascal_name }@Entity {
@{- parent.non_cache_cols_wo_primaries_and_read_only()|fmt_join("
    fn {var}(&self) -> {domain_outer} {
        {convert_domain_outer_prefix}self.{var}{clone_for_outer}{convert_domain_outer}
    }", "") }@
@{- parent.relations_one_and_belonging(true)|fmt_rel_join("
    fn {rel_name}(&self) -> Option<&dyn _model_::{class_mod_var}::{class}> {
        self.{rel_name}.as_ref().map(|v| v.as_ref() as &dyn _model_::{class_mod_var}::{class})
    }", "") }@
@{- parent.relations_many(true)|fmt_rel_join("
    fn {rel_name}(&self) -> Box<dyn Iterator<Item = &dyn _model_::{class_mod_var}::{class}> + '_> {
        Box::new(self.{rel_name}.iter().map(|v| v.as_ref() as &dyn _model_::{class_mod_var}::{class}))
    }", "") }@
}
#[cfg(any(feature = "mock", test))]
#[allow(clippy::useless_conversion)]
impl super::super::super::@{ parent.group_name|to_var_name }@::@{ parent.name|to_var_name }@::@{ parent.name|pascal }@UpdaterBase for @{ pascal_name }@Entity {
@{- parent.non_cache_cols_wo_primaries_and_read_only()|fmt_join("
    fn {var}(&self) -> {domain_outer} {
        {convert_domain_outer_prefix}self.{var}{clone_for_outer}{convert_domain_outer}
    }", "") }@
@{- parent.non_primaries_wo_read_only(true)|fmt_join("
    fn set_{raw_var}(&mut self, v: {domain_outer_owned}) {
        self.{var} = v
    }", "") }@
@{- parent.relations_one(true)|fmt_rel_join("
    fn {rel_name}(&mut self) -> Option<&mut dyn _model_::{class_mod_var}::{class}Updater> {
        self.{rel_name}.as_mut().map(|v| v.as_mut() as &mut dyn _model_::{class_mod_var}::{class}Updater)
    }
    fn set_{raw_rel_name}(&mut self, v: Box<dyn _model_::{class_mod_var}::{class}Updater>) {
        self.{rel_name} = if let Ok(v) = v.downcast::<_model_::{class_mod_var}::{class}Entity>() {
            Some(v)
        } else {
            panic!(\"Only {class}Entity is accepted.\");
        };
    }", "") }@
@{- parent.relations_many(true)|fmt_rel_join("
    fn {rel_name}(&mut self) -> Box<dyn domain::models::UpdateIterator<dyn _model_::{class_mod_var}::{class}Updater> + '_> {
        struct V<'a, T>(&'a mut Vec<Box<T>>);
        impl<T: _model_::{class_mod_var}::{class}Updater> domain::models::UpdateIterator<dyn _model_::{class_mod_var}::{class}Updater> for V<'_, T> {
            fn iter_mut(&mut self) -> Box<dyn Iterator<Item = &mut (dyn _model_::{class_mod_var}::{class}Updater + 'static)> + '_> {
                Box::new(self.0.iter_mut().map(|v| v.as_mut() as &mut dyn _model_::{class_mod_var}::{class}Updater))
            }
        }
        Box::new(V(&mut self.{rel_name}))
    }
    fn take_{raw_rel_name}(&mut self) -> Option<Vec<Box<dyn _model_::{class_mod_var}::{class}Updater>>> {
        Some(self.{rel_name}.drain(..).map(|v| v as Box<dyn _model_::{class_mod_var}::{class}Updater>).collect())
    }
    fn replace_{raw_rel_name}(&mut self, list: Vec<Box<dyn _model_::{class_mod_var}::{class}Updater>>) {
        self.{rel_name}.clear();
        for row in list {
            self.push_{raw_rel_name}(row);
        }
    }
    fn push_{raw_rel_name}(&mut self, v: Box<dyn _model_::{class_mod_var}::{class}Updater>) {
        if let Ok(v) = v.downcast::<_model_::{class_mod_var}::{class}Entity>() {
            self.{rel_name}.push(v)
        } else {
            panic!(\"Only {class}Entity is accepted.\");
        }
    }", "") }@
}
@%- endfor %@

#[cfg(any(feature = "mock", test))]
#[allow(clippy::useless_conversion)]
impl @{ pascal_name }@Common for @{ pascal_name }@Entity {
@{- def.primaries()|fmt_join("
    fn {var}(&self) -> {outer} {
        {convert_domain_outer_prefix}self.{var}{clone_for_outer}{convert_domain_outer}
    }", "") }@
@{- def.only_version()|fmt_join("
    fn {var}(&self) -> {outer} {
        1
    }", "") }@
@{- def.cache_cols_wo_primaries_and_read_only()|fmt_join("
    fn {var}(&self) -> {domain_outer} {
        {convert_domain_outer_prefix}self.{var}{clone_for_outer}{convert_domain_outer}
    }", "") }@
}

#[cfg(any(feature = "mock", test))]
impl @{ pascal_name }@Cache for @{ pascal_name }@Entity {
@{- def.relations_one_cache(true)|fmt_rel_join("
    fn {rel_name}(&self) -> Option<Box<dyn _model_::{class_mod_var}::{class}Cache>> {
        self.{rel_name}.as_ref().map(|v| Box::<dyn _model_::{class_mod_var}::{class}Cache>::from(v.clone()))
    }", "") }@
@{- def.relations_one_uncached(true)|fmt_rel_join("
    fn {rel_name}(&self) -> Option<Box<dyn _model_::{class_mod_var}::{class}>> {
        self.{rel_name}.as_ref().map(|v| Box::<dyn _model_::{class_mod_var}::{class}>::from(v.clone()))
    }", "") }@
@{- def.relations_many_cache(true)|fmt_rel_join("
    fn {rel_name}(&self) -> Vec<Box<dyn _model_::{class_mod_var}::{class}Cache>> {
        self.{rel_name}.iter().map(|v| Box::<dyn _model_::{class_mod_var}::{class}Cache>::from(v.clone())).collect()
    }", "") }@
@{- def.relations_many_uncached(true)|fmt_rel_join("
    fn {rel_name}(&self) -> Vec<Box<dyn _model_::{class_mod_var}::{class}>> {
        self.{rel_name}.iter().map(|v| Box::<dyn _model_::{class_mod_var}::{class}>::from(v.clone())).collect()
    }", "") }@
@{- def.relations_belonging_cache(true)|fmt_rel_join("
    fn {rel_name}(&self) -> Option<Box<dyn _model_::{class_mod_var}::{class}Cache>> {
        self.{rel_name}.as_ref().map(|v| Box::<dyn _model_::{class_mod_var}::{class}Cache>::from(v.clone()))
    }", "") }@
@{- def.relations_belonging_uncached(true)|fmt_rel_join("
    fn {rel_name}(&self) -> Option<Box<dyn _model_::{class_mod_var}::{class}>> {
        self.{rel_name}.as_ref().map(|v| Box::<dyn _model_::{class_mod_var}::{class}>::from(v.clone()))
    }", "") }@
}

#[cfg(any(feature = "mock", test))]
#[allow(clippy::useless_conversion)]
impl @{ pascal_name }@ for @{ pascal_name }@Entity {
@{- def.non_cache_cols_wo_primaries_and_read_only()|fmt_join("
    fn {var}(&self) -> {domain_outer} {
        {convert_domain_outer_prefix}self.{var}{clone_for_outer}{convert_domain_outer}
    }", "") }@
@{- def.relations_one_and_belonging(true)|fmt_rel_join("
    fn {rel_name}(&self) -> Option<&dyn _model_::{class_mod_var}::{class}> {
        self.{rel_name}.as_ref().map(|v| v.as_ref() as &dyn _model_::{class_mod_var}::{class})
    }", "") }@
@{- def.relations_many(true)|fmt_rel_join("
    fn {rel_name}(&self) -> Box<dyn Iterator<Item = &dyn _model_::{class_mod_var}::{class}> + '_> {
        Box::new(self.{rel_name}.iter().map(|v| v.as_ref() as &dyn _model_::{class_mod_var}::{class}))
    }", "") }@
}

#[cfg(any(feature = "mock", test))]
#[allow(clippy::useless_conversion)]
impl @{ pascal_name }@UpdaterBase for @{ pascal_name }@Entity {
@{- def.non_cache_cols_wo_primaries_and_read_only()|fmt_join("
    fn {var}(&self) -> {domain_outer} {
        {convert_domain_outer_prefix}self.{var}{clone_for_outer}{convert_domain_outer}
    }", "") }@
@{- def.non_primaries_wo_read_only(true)|fmt_join("
    fn set_{raw_var}(&mut self, v: {domain_outer_owned}) {
        self.{var} = v
    }", "") }@
@{- def.relations_one(true)|fmt_rel_join("
    fn {rel_name}(&mut self) -> Option<&mut dyn _model_::{class_mod_var}::{class}Updater> {
        self.{rel_name}.as_mut().map(|v| v.as_mut() as &mut dyn _model_::{class_mod_var}::{class}Updater)
    }
    fn set_{raw_rel_name}(&mut self, v: Box<dyn _model_::{class_mod_var}::{class}Updater>) {
        self.{rel_name} = if let Ok(v) = v.downcast::<_model_::{class_mod_var}::{class}Entity>() {
            Some(v)
        } else {
            panic!(\"Only {class}Entity is accepted.\");
        };
    }", "") }@
@{- def.relations_many(true)|fmt_rel_join("
    fn {rel_name}(&mut self) -> Box<dyn domain::models::UpdateIterator<dyn _model_::{class_mod_var}::{class}Updater> + '_> {
        struct V<'a, T>(&'a mut Vec<Box<T>>);
        impl<T: _model_::{class_mod_var}::{class}Updater> domain::models::UpdateIterator<dyn _model_::{class_mod_var}::{class}Updater> for V<'_, T> {
            fn iter_mut(&mut self) -> Box<dyn Iterator<Item = &mut (dyn _model_::{class_mod_var}::{class}Updater + 'static)> + '_> {
                Box::new(self.0.iter_mut().map(|v| v.as_mut() as &mut dyn _model_::{class_mod_var}::{class}Updater))
            }
        }
        Box::new(V(&mut self.{rel_name}))
    }
    fn take_{raw_rel_name}(&mut self) -> Option<Vec<Box<dyn _model_::{class_mod_var}::{class}Updater>>> {
        Some(self.{rel_name}.drain(..).map(|v| v as Box<dyn _model_::{class_mod_var}::{class}Updater>).collect())
    }
    fn replace_{raw_rel_name}(&mut self, list: Vec<Box<dyn _model_::{class_mod_var}::{class}Updater>>) {
        self.{rel_name}.clear();
        for row in list {
            self.push_{raw_rel_name}(row);
        }
    }
    fn push_{raw_rel_name}(&mut self, v: Box<dyn _model_::{class_mod_var}::{class}Updater>) {
        if let Ok(v) = v.downcast::<_model_::{class_mod_var}::{class}Entity>() {
            self.{rel_name}.push(v)
        } else {
            panic!(\"Only {class}Entity is accepted.\");
        }
    }", "") }@
}
#[cfg(any(feature = "mock", test))]
impl crate::models::MarkForDelete for @{ pascal_name }@Entity {
    fn mark_for_delete(&mut self) {
        self._delete = true;
    }
    fn unmark_for_delete(&mut self) {
        self._delete = false;
    }
}

#[derive(Debug, Clone, Default)]
pub struct Joiner_ {
@{- def.relations()|fmt_rel_join("
    pub {rel_name}: Option<Box<_model_::{class_mod_var}::Joiner_>>,", "") }@
}
impl Joiner_ {
    #[allow(clippy::nonminimal_bool)]
    pub fn has_some(&self) -> bool {
        false
        @{- def.relations()|fmt_rel_join("
            || self.{rel_name}.is_some()", "") }@
    }
    #[allow(unused_variables)]
    pub fn merge(lhs: Option<Box<Self>>, rhs: Option<Box<Self>>) -> Option<Box<Self>> {
        if let Some(lhs) = lhs {
            if let Some(rhs) = rhs {
                Some(Box::new(Joiner_{
                    @{- def.relations()|fmt_rel_join("
                    {rel_name}: _model_::{class_mod_var}::Joiner_::merge(lhs.{rel_name}, rhs.{rel_name}),", "") }@
                }))
            } else {
                Some(lhs)
            }
        } else {
            rhs
        }
    }
}
@%- let fetch_macro_name = "{}_{}_{}"|format(db, group_name, model_name) %@
@% let model_path = "$crate::models::{}::{}::{}"|format(db|snake|to_var_name, group_name|to_var_name, mod_name|to_var_name) -%@
@% let base_path = "$crate::models::{}::{}::_base::_{}"|format(db|snake|to_var_name, group_name|to_var_name, mod_name) -%@
#[macro_export]
macro_rules! _join_@{ fetch_macro_name }@ {
@{- def.relations()|fmt_rel_join("
    ({rel_name}) => ($crate::models::--1--::{group_var}::{mod_var}::join!({}));
    ({rel_name}: $p:tt) => ($crate::models::--1--::{group_var}::{mod_var}::join!($p));", "")|replace1(db|snake|to_var_name) }@
    () => ();
}
pub use _join_@{ fetch_macro_name }@ as _join;
#[macro_export]
macro_rules! join_@{ fetch_macro_name }@ {
    ({$($i:ident $(: $p:tt)?),*}) => (Some(Box::new(@{ model_path }@::Joiner_ {
        $($i: @{ base_path }@::_join!($i $(: $p)?),)*
        ..Default::default()
    })));
}
pub use join_@{ fetch_macro_name }@ as join;

#[allow(unused_imports)]
use @{ pascal_name }@RepositoryFindForUpdateBuilder as _RepositoryFindForUpdateBuilder;

#[async_trait]
pub trait @{ pascal_name }@RepositoryFindForUpdateBuilder: Send + Sync {
    async fn query(self: Box<Self>) -> anyhow::Result<Box<dyn _Updater>>;
    fn visibility_filter(self: Box<Self>, filter: Filter_) -> Box<dyn _RepositoryFindForUpdateBuilder>;
    @%- if def.is_soft_delete() %@
    fn with_trashed(self: Box<Self>, mode: bool) -> Box<dyn _RepositoryFindForUpdateBuilder>;
    @%- endif %@
    fn join(self: Box<Self>, joiner: Option<Box<Joiner_>>) -> Box<dyn _RepositoryFindForUpdateBuilder>;
}

#[async_trait]
pub trait _@{ pascal_name }@Repository: Send + Sync {
@%- if !def.disable_update() %@
    fn find_for_update(&self, id: @{ def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") }@) -> Box<dyn @{ pascal_name }@RepositoryFindForUpdateBuilder>;
@%- endif %@
    fn convert_factory(&self, factory: @{ pascal_name }@Factory) -> Box<dyn _Updater>;
    #[deprecated(note = "This method should not be used outside the domain.")]
    async fn save(&self, obj: Box<dyn _Updater>) -> anyhow::Result<Option<Box<dyn @{ pascal_name }@>>>;
    @%- if !def.disable_update() %@
    #[deprecated(note = "This method should not be used outside the domain.")]
    async fn import(&self, list: Vec<Box<dyn _Updater>>, option: Option<crate::models::ImportOption>) -> anyhow::Result<()>;
    @%- endif %@
    @%- if def.use_insert_delayed() %@
    #[deprecated(note = "This method should not be used outside the domain.")]
    async fn insert_delayed(&self, obj: Box<dyn _Updater>) -> anyhow::Result<()>;
    @%- endif %@
@%- if !def.disable_update() %@
    #[deprecated(note = "This method should not be used outside the domain.")]
    async fn delete(&self, obj: Box<dyn _Updater>) -> anyhow::Result<()>;
    @%- if def.primaries().len() == 1 %@
    #[deprecated(note = "This method should not be used outside the domain.")]
    async fn delete_by_ids(&self, ids: &[@{ def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") }@]) -> anyhow::Result<u64>;
    @%- endif %@
    #[deprecated(note = "This method should not be used outside the domain.")]
    async fn delete_all(&self) -> anyhow::Result<()>;
@%- endif %@
@%- if def.act_as_job_queue() %@
    async fn fetch(&self, limit: usize) -> anyhow::Result<Vec<Box<dyn _Updater>>>;
@%- endif %@
@%- for (selector, selector_def) in def.selectors %@
    fn @{ selector|to_var_name }@(&self) -> Box<dyn @{ pascal_name }@Repository@{ selector|pascal }@Builder>;
@%- endfor %@
}
@%- for (selector, selector_def) in def.selectors %@

#[allow(unused_imports)]
use @{ pascal_name }@Repository@{ selector|pascal }@Builder as _Repository@{ selector|pascal }@Builder;

#[async_trait]
pub trait @{ pascal_name }@Repository@{ selector|pascal }@Builder: Send + Sync {
    async fn query(self: Box<Self>) -> anyhow::Result<Vec<Box<dyn _Updater>>>;
    async fn count(self: Box<Self>) -> anyhow::Result<i64>;
    fn query_filter(self: Box<Self>, filter: @{ pascal_name }@Query@{ selector|pascal }@Filter) -> Box<dyn _Repository@{ selector|pascal }@Builder>;
    fn query_filter_by_json(self: Box<Self>, filter: serde_json::Value) -> anyhow::Result<Box<dyn _Repository@{ selector|pascal }@Builder>> {
        Ok(self.query_filter(serde_json::from_value(filter)?))
    }
    fn visibility_filter(self: Box<Self>, filter: Filter_) -> Box<dyn _Repository@{ selector|pascal }@Builder>;
    @%- if def.is_soft_delete() %@
    fn with_trashed(self: Box<Self>, mode: bool) -> Box<dyn _Repository@{ selector|pascal }@Builder>;
    @%- endif %@
    fn join(self: Box<Self>, joiner: Option<Box<Joiner_>>) -> Box<dyn _Repository@{ selector|pascal }@Builder>;
}
@%- endfor %@
@%- for (selector, selector_def) in def.selectors %@

#[allow(unused_imports)]
use @{ pascal_name }@Query@{ selector|pascal }@Builder as _Query@{ selector|pascal }@Builder;
#[allow(unused_imports)]
use validator::Validate as _;
@%- for filter_map in selector_def.nested_filters(selector, def) %@

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Debug, Clone, Default, validator::Validate, async_graphql::InputObject)]
#[serde(deny_unknown_fields)]
#[allow(non_camel_case_types)]
#[graphql(name = "@{ db|pascal }@@{ group_name|pascal }@@{ pascal_name }@Query@{ selector|pascal }@@{ filter_map.pascal_name }@Filter")]
pub struct @{ pascal_name }@Query@{ selector|pascal }@@{ filter_map.pascal_name }@Filter {
    @%- for (filter, filter_def) in filter_map.filters %@
    #[graphql(name = "@{ filter }@")]
    @%- if !filter_def.required %@
    #[serde(default, skip_serializing_if = "Option::is_none")]
    @%- endif %@
    @%- if filter_def.has_default() %@
    #[validate(custom = "crate::models::reject_empty")]
    @%- endif %@
    pub @{ filter|to_var_name }@: @{ filter_def.type_str(filter, pascal_name, selector, filter_map.pascal_name) }@,
    @%- endfor %@
    #[graphql(name = "_and")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub _and: Option<Vec<@{ pascal_name }@Query@{ selector|pascal }@@{ filter_map.pascal_name }@Filter>>,
    #[graphql(name = "_or")]
    #[validate]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub _or: Option<Vec<@{ pascal_name }@Query@{ selector|pascal }@@{ filter_map.pascal_name }@Filter>>,
}
@%- for (name, type_name) in filter_map.ranges(pascal_name, selector, filter_map.pascal_name) %@

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Debug, Clone, Default, async_graphql::InputObject)]
#[serde(deny_unknown_fields)]
#[allow(non_camel_case_types)]
#[graphql(name = "@{ db|pascal }@@{ group_name|pascal }@@{ pascal_name }@Query@{ selector|pascal }@Range@{ filter_map.pascal_name }@_@{ name|pascal }@")]
pub struct @{ pascal_name }@Query@{ selector|pascal }@Range@{ filter_map.pascal_name }@_@{ name|pascal }@ {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eq: Option<@{ type_name }@>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lt: Option<@{ type_name }@>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lte: Option<@{ type_name }@>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gt: Option<@{ type_name }@>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gte: Option<@{ type_name }@>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_null: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_not_null: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_null_or_lt: Option<@{ type_name }@>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_null_or_lte: Option<@{ type_name }@>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_null_or_gt: Option<@{ type_name }@>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_null_or_gte: Option<@{ type_name }@>,
}
@%- endfor %@
@%- for (name, fields) in filter_map.range_tuples() %@

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Debug, Clone, Default, async_graphql::InputObject)]
#[serde(deny_unknown_fields)]
#[allow(non_camel_case_types)]
#[graphql(name = "@{ db|pascal }@@{ group_name|pascal }@@{ pascal_name }@Query@{ selector|pascal }@RangeValues@{ filter_map.pascal_name }@_@{ name|pascal }@")]
pub struct @{ pascal_name }@Query@{ selector|pascal }@RangeValues@{ filter_map.pascal_name }@_@{ name|pascal }@ {
    @%- for (field, _type) in fields.clone() %@
    #[graphql(name = "@{ field }@")]
    pub @{ field|to_var_name }@: @{ _type }@,
    @%- endfor %@
}
impl @{ pascal_name }@Query@{ selector|pascal }@@{ filter_map.pascal_name }@RangeValues@{ filter_map.pascal_name }@_@{ name|pascal }@ {
    pub fn values(&self) -> (@% for (field, _type) in fields.clone() %@@{ _type }@, @% endfor %@) {
        (@% for (field, _type) in fields.clone() %@self.@{ field|to_var_name }@, @% endfor %@)
    }
}
@%- endfor %@
@%- for (name, type_name) in filter_map.identities(pascal_name, selector, filter_map.pascal_name) %@

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Debug, Clone, Default, async_graphql::InputObject)]
#[serde(deny_unknown_fields)]
#[allow(non_camel_case_types)]
#[graphql(name = "@{ db|pascal }@@{ group_name|pascal }@@{ pascal_name }@Query@{ selector|pascal }@Identity@{ filter_map.pascal_name }@_@{ name|pascal }@")]
pub struct @{ pascal_name }@Query@{ selector|pascal }@Identity@{ filter_map.pascal_name }@_@{ name|pascal }@ {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eq: Option<@{ type_name }@>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#in: Option<Vec<@{ type_name }@>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_null: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_not_null: Option<bool>,
}
@%- endfor %@
@%- for (name, fields) in filter_map.identity_tuples() %@

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Debug, Clone, Default, async_graphql::InputObject)]
#[serde(deny_unknown_fields)]
#[allow(non_camel_case_types)]
#[graphql(name = "@{ db|pascal }@@{ group_name|pascal }@@{ pascal_name }@Query@{ selector|pascal }@IdentityValues@{ filter_map.pascal_name }@_@{ name|pascal }@")]
pub struct @{ pascal_name }@Query@{ selector|pascal }@IdentityValues@{ filter_map.pascal_name }@_@{ name|pascal }@ {
    @%- for (field, _type) in fields.clone() %@
    #[graphql(name = "@{ field }@")]
    pub @{ field|to_var_name }@: @{ _type }@,
    @%- endfor %@
}
impl @{ pascal_name }@Query@{ selector|pascal }@IdentityValues@{ filter_map.pascal_name }@_@{ name|pascal }@ {
    pub fn values(&self) -> (@% for (field, _type) in fields.clone() %@@{ _type }@, @% endfor %@) {
        (@% for (field, _type) in fields.clone() %@self.@{ field|to_var_name }@, @% endfor %@)
    }
}
@%- endfor %@
@%- endfor %@

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Eq, Debug, Clone, Copy, Default, async_graphql::Enum)]
#[serde(deny_unknown_fields)]
#[graphql(name = "@{ db|pascal }@@{ group_name|pascal }@@{ pascal_name }@Query@{ selector|pascal }@Order")]
pub enum @{ pascal_name }@Query@{ selector|pascal }@Order {
    #[default]
    @%- for (order, _) in selector_def.orders %@
    @{ order|pascal }@,
    @%- endfor %@
}

#[allow(unused_parens)]
impl @{ pascal_name }@Query@{ selector|pascal }@Order {
    #[allow(clippy::borrowed_box)]
    pub fn to_cursor<T: @{ pascal_name }@Common + ?Sized>(&self, obj: &Box<T>) -> anyhow::Result<String> {
        match self {
            @%- for (order, order_def) in selector_def.orders %@
            @{ pascal_name }@Query@{ selector|pascal }@Order::@{ order|pascal }@ => {
                use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
                let v = @{ order_def.field_tuple(def) }@;
                let mut buf = Vec::new();
                ciborium::into_writer(&v, &mut buf)?;
                Ok(URL_SAFE_NO_PAD.encode(buf))
            }
            @%- endfor %@
        }
    }
}

#[allow(unused_parens)]
#[derive(Debug, Clone)]
pub enum @{ pascal_name }@Query@{ selector|pascal }@Cursor {
    @%- for (order, order_def) in selector_def.orders %@
    @{ order|pascal }@(models::Cursor<@{ order_def.type_str(def) }@>),
    @%- endfor %@
}
#[allow(unused_parens)]
impl @{ pascal_name }@Query@{ selector|pascal }@Cursor {
    @%- for (order, order_def) in selector_def.orders %@
    pub fn @{ order }@_from_str(v: &str) -> anyhow::Result<@{ order_def.type_str(def) }@> {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
        Ok(ciborium::from_reader(URL_SAFE_NO_PAD.decode(v)?.as_slice())?)
    }
    @%- endfor %@
}

#[async_trait]
pub trait @{ pascal_name }@Query@{ selector|pascal }@Builder: Send + Sync {
    async fn query(self: Box<Self>) -> anyhow::Result<Vec<Box<dyn @{ pascal_name }@@% if def.use_cache() %@Cache@% endif %@>>>;
    async fn count(self: Box<Self>) -> anyhow::Result<i64>;
    fn query_filter(self: Box<Self>, filter: @{ pascal_name }@Query@{ selector|pascal }@Filter) -> Box<dyn _Query@{ selector|pascal }@Builder>;
    fn query_filter_by_json(self: Box<Self>, filter: serde_json::Value) -> anyhow::Result<Box<dyn _Query@{ selector|pascal }@Builder>> {
        Ok(self.query_filter(serde_json::from_value(filter)?))
    }
    fn visibility_filter(self: Box<Self>, filter: Filter_) -> Box<dyn _Query@{ selector|pascal }@Builder>;
    fn cursor(self: Box<Self>, cursor: @{ pascal_name }@Query@{ selector|pascal }@Cursor) -> Box<dyn _Query@{ selector|pascal }@Builder>;
    fn order_by(self: Box<Self>, order: @{ pascal_name }@Query@{ selector|pascal }@Order) -> Box<dyn _Query@{ selector|pascal }@Builder>;
    fn reverse(self: Box<Self>, mode: bool) -> Box<dyn _Query@{ selector|pascal }@Builder>;
    fn limit(self: Box<Self>, limit: usize) -> Box<dyn _Query@{ selector|pascal }@Builder>;
    fn offset(self: Box<Self>, offset: usize) -> Box<dyn _Query@{ selector|pascal }@Builder>;
    @%- if def.is_soft_delete() %@
    fn with_trashed(self: Box<Self>, mode: bool) -> Box<dyn _Query@{ selector|pascal }@Builder>;
    @%- endif %@
    fn join(self: Box<Self>, joiner: Option<Box<Joiner_>>) -> Box<dyn _Query@{ selector|pascal }@Builder>;
}
@%- endfor %@

#[allow(unused_imports)]
use @{ pascal_name }@QueryFindBuilder as _QueryFindBuilder;

#[async_trait]
pub trait @{ pascal_name }@QueryFindBuilder: Send + Sync {
    async fn query(self: Box<Self>) -> anyhow::Result<Option<Box<dyn @{ pascal_name }@@% if def.use_cache() %@Cache@% endif %@>>>;
    fn visibility_filter(self: Box<Self>, filter: Filter_) -> Box<dyn _QueryFindBuilder>;
    @%- if def.is_soft_delete() %@
    fn with_trashed(self: Box<Self>, mode: bool) -> Box<dyn _QueryFindBuilder>;
    @%- endif %@
    fn join(self: Box<Self>, joiner: Option<Box<Joiner_>>) -> Box<dyn _QueryFindBuilder>;
}

#[allow(unused_imports)]
use @{ pascal_name }@QueryFindDirectlyBuilder as _QueryFindDirectlyBuilder;

#[async_trait]
pub trait @{ pascal_name }@QueryFindDirectlyBuilder: Send + Sync {
    async fn query(self: Box<Self>) -> anyhow::Result<Option<Box<dyn @{ pascal_name }@>>>;
    fn visibility_filter(self: Box<Self>, filter: Filter_) -> Box<dyn _QueryFindDirectlyBuilder>;
    @%- if def.is_soft_delete() %@
    fn with_trashed(self: Box<Self>, mode: bool) -> Box<dyn _QueryFindDirectlyBuilder>;
    @%- endif %@
    fn join(self: Box<Self>, joiner: Option<Box<Joiner_>>) -> Box<dyn _QueryFindDirectlyBuilder>;
}

#[async_trait]
pub trait _@{ pascal_name }@Query: Send + Sync {
    @%- if def.use_all_row_cache() && !def.use_filtered_row_cache() %@
    async fn all(&self) -> anyhow::Result<Box<dyn crate::models::EntityIterator<dyn @{ pascal_name }@Cache>>>;
    @%- endif %@
    @%- for (selector, selector_def) in def.selectors %@
    fn @{ selector|to_var_name }@(&self) -> Box<dyn @{ pascal_name }@Query@{ selector|pascal }@Builder>;
    @%- endfor %@
    @%- if def.use_cache() %@
    fn find(&self, id: @{ def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") }@) -> Box<dyn @{ pascal_name }@QueryFindBuilder>;
    @%- else %@
    fn find(&self, id: @{ def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") }@) -> Box<dyn @{ pascal_name }@QueryFindDirectlyBuilder>;
    @%- endif %@
    fn find_directly(&self, id: @{ def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") }@) -> Box<dyn @{ pascal_name }@QueryFindDirectlyBuilder>;
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug)]
pub enum Col_ {
@{ def.all_fields()|fmt_join("    {var},", "\n") }@
}
#[allow(unreachable_patterns)]
impl Col_ {
    #[allow(clippy::match_single_binding)]
    pub fn check_null<T: @{ pascal_name }@Common + ?Sized>(&self, _obj: &T) -> bool {
        match self {
            @{- def.primaries()|fmt_join("
            Col_::{var} => {filter_check_null},", "") }@
            @{- def.cache_cols_wo_primaries_and_read_only()|fmt_join("
            Col_::{var} => {filter_check_null},", "") }@
            _ => unimplemented!(),
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug)]
pub enum ColOne_ {
@{ def.all_fields_without_json()|fmt_join("    {var}({filter_type}),", "\n") }@
@%- for (index_name, index) in def.multi_index() %@
    @{ index.join_fields(def, "{name}", "_") }@(@{ index.join_fields(def, "{type}", ", ") }@),
@%- endfor %@
}
#[allow(unreachable_patterns)]
impl ColOne_ {
    #[allow(clippy::match_single_binding)]
    pub fn check_eq<T: @{ pascal_name }@Common + ?Sized>(&self, _obj: &T) -> bool {
        match self {
            @{- def.equivalence_cache_fields_without_json()|fmt_join("
            ColOne_::{var}(c) => _obj.{var}(){filter_check_eq},", "") }@
            @%- for (index_name, index) in def.multi_index() %@
            ColOne_::@{ index.join_fields(def, "{name}", "_") }@(@{ index.join_fields(def, "c{index}", ", ") }@) => @{ index.join_fields(def, "(_obj.{var}(){filter_check_eq})", " && ") }@,
            @%- endfor %@
            _ => unimplemented!(),
        }
    }
    #[allow(clippy::match_single_binding)]
    pub fn check_cmp<T: @{ pascal_name }@Common + ?Sized>(&self, _obj: &T, order: std::cmp::Ordering, eq: bool) -> Result<bool, bool> {
        let o = match self {
            @{- def.comparable_cache_fields_without_json()|fmt_join("
            ColOne_::{var}(c) => _obj.{var}(){filter_check_cmp},", "") }@
            @%- for (index_name, index) in def.multi_index() %@
            ColOne_::@{ index.join_fields(def, "{name}", "_") }@(@{ index.join_fields(def, "c{index}", ", ") }@) => @{ index.join_fields(def, "(_obj.{var}(){filter_check_cmp})", ".then") }@,
            @%- endfor %@
            _ => unimplemented!(),
        };
        Ok(o == order || eq && o == std::cmp::Ordering::Equal)
    }
    #[allow(clippy::match_single_binding)]
    pub fn check_like<T: @{ pascal_name }@Common + ?Sized>(&self, _obj: &T) -> bool {
        #[allow(unused_imports)]
        use crate::models::Like as _;
        match self {
            @{- def.string_cache_fields()|fmt_join("
            ColOne_::{var}(c) => _obj.{var}(){filter_like},", "") }@
            _ => unimplemented!(),
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Hash, serde::Serialize)]
pub enum ColKey_ {
    @{- def.unique_key()|fmt_index_col("
    {var}({filter_type}),", "") }@
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug)]
pub enum ColMany_ {
@{ def.all_fields_without_json()|fmt_join("    {var}(Vec<{filter_type}>),", "\n") }@
@%- for (index_name, index) in def.multi_index() %@
    @{ index.join_fields(def, "{name}", "_") }@(Vec<(@{ index.join_fields(def, "{type}", ", ") }@)>),
@%- endfor %@
}
#[allow(unreachable_patterns)]
impl ColMany_ {
    #[allow(clippy::match_single_binding)]
    pub fn check_in<T: @{ pascal_name }@Common + ?Sized>(&self, _obj: &T) -> bool {
        match self {
            @{- def.equivalence_cache_fields_without_json()|fmt_join("
            ColMany_::{var}(list) => list.iter().any(|c| _obj.{var}(){filter_check_eq}),", "") }@
            @%- for (index_name, index) in def.multi_index() %@
            ColMany_::@{ index.join_fields(def, "{name}", "_") }@(list) => list.iter().any(|(@{ index.join_fields(def, "c{index}", ", ") }@)| @{ index.join_fields(def, "(_obj.{var}(){filter_check_eq})", " && ") }@),
            @%- endfor %@
            _ => unimplemented!(),
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug)]
pub enum ColJson_ {
@{- def.all_fields_only_json()|fmt_join("
    {var}(serde_json::Value),", "") }@
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug)]
pub enum ColJsonArray_ {
@{- def.all_fields_only_json()|fmt_join("
    {var}(Vec<serde_json::Value>),", "") }@
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug)]
pub enum ColGeo_ {
@{- def.all_fields_only_geo()|fmt_join("
    {var}(serde_json::Value, u32),", "") }@
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug)]
pub enum ColGeoDistance_ {
@{- def.all_fields_only_geo()|fmt_join("
    {var}(serde_json::Value, f64, u32),", "") }@
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug)]
pub enum ColRel_ {
@{- def.relations_one_and_belonging(false)|fmt_rel_join("
    {rel_name}(Option<Box<_model_::{base_class_mod_var}::Filter_>>),", "") }@
@{- def.relations_many(false)|fmt_rel_join("
    {rel_name}(Option<Box<_model_::{base_class_mod_var}::Filter_>>),", "") }@
}
impl ColRel_ {
    #[allow(unreachable_patterns)]
    #[allow(clippy::needless_update)]
    #[allow(clippy::match_single_binding)]
    fn joiner(&self) -> Option<Box<Joiner_>> {
        match self {
            @{- def.relations_one_and_belonging(false)|fmt_rel_join("
            ColRel_::{rel_name}(c) => Some(Box::new(Joiner_{
                {rel_name}: Some(c.as_ref().and_then(|c| c.joiner()).unwrap_or_default()),
                ..Default::default()
            })),", "") }@
            @{- def.relations_many(false)|fmt_rel_join("
            ColRel_::{rel_name}(c) => Some(Box::new(Joiner_{
                {rel_name}: Some(c.as_ref().and_then(|c| c.joiner()).unwrap_or_default()),
                ..Default::default()
            })),", "") }@
            _ => unreachable!()
        }
    }
}
impl Check_<dyn @{ pascal_name }@Cache> for ColRel_ {
    #[allow(unreachable_patterns)]
    #[allow(clippy::match_single_binding)]
    fn check(&self, _obj: &dyn @{ pascal_name }@Cache) -> bool {
        match self {
            @{- def.relations_one_and_belonging(false)|fmt_rel_join("
            ColRel_::{rel_name}(None) => _obj.{rel_name}().is_some(),
            ColRel_::{rel_name}(Some(f)) => _obj.{rel_name}().map(|v| f.check(&*v)).unwrap_or_default(),", "") }@
            @{- def.relations_many(false)|fmt_rel_join("
            ColRel_::{rel_name}(None) => !_obj.{rel_name}().is_empty(),
            ColRel_::{rel_name}(Some(f)) => _obj.{rel_name}().iter().any(|v| f.check(v.as_ref())),", "") }@
            _ => unreachable!()
        }
    }
}
impl Check_<dyn @{ pascal_name }@> for ColRel_ {
    #[allow(unreachable_patterns)]
    #[allow(clippy::match_single_binding)]
    fn check(&self, _obj: &dyn @{ pascal_name }@) -> bool {
        match self {
            @{- def.relations_one_and_belonging(false)|fmt_rel_join("
            ColRel_::{rel_name}(None) => _obj.{rel_name}().is_some(),
            ColRel_::{rel_name}(Some(f)) => _obj.{rel_name}().map(|v| f.check(v)).unwrap_or_default(),", "") }@
            @{- def.relations_many(false)|fmt_rel_join("
            ColRel_::{rel_name}(None) => _obj.{rel_name}().next().is_some(),
            ColRel_::{rel_name}(Some(f)) => _obj.{rel_name}().any(|v| f.check(v)),", "") }@
            _ => unreachable!()
        }
    }
}

#[derive(Clone, Debug)]
pub enum Filter_ {
    WithTrashed,
    OnlyTrashed,
    Match(Vec<Col_>, String),
    MatchBoolean(Vec<Col_>, String),
    MatchExpansion(Vec<Col_>, String),
    IsNull(Col_),
    IsNotNull(Col_),
    Eq(ColOne_),
    EqKey(ColKey_),
    NotEq(ColOne_),
    Gt(ColOne_),
    Gte(ColOne_),
    Lt(ColOne_),
    Lte(ColOne_),
    Like(ColOne_),
    AllBits(ColMany_),
    AnyBits(ColOne_),
    In(ColMany_),
    NotIn(ColMany_),
    MemberOf(ColJson_, Option<String>),
    Contains(ColJsonArray_, Option<String>),
    Overlaps(ColJsonArray_, Option<String>),
    JsonIn(ColJsonArray_, String),
    JsonContainsPath(ColJson_, String),
    JsonEq(ColJson_, String),
    JsonLt(ColJson_, String),
    JsonLte(ColJson_, String),
    JsonGt(ColJson_, String),
    JsonGte(ColJson_, String),
    Within(ColGeo_),
    Intersects(ColGeo_),
    Crosses(ColGeo_),
    DWithin(ColGeoDistance_),
    Not(Box<Filter_>),
    And(Vec<Filter_>),
    Or(Vec<Filter_>),
    Exists(ColRel_),
    NotExists(ColRel_),
    EqAny(ColRel_),
    NotAll(ColRel_),
    Raw(String),
    RawWithParam(String, Vec<String>),
    Boolean(bool),
}
impl Filter_ {
    pub fn new_and() -> Filter_ {
        Filter_::And(vec![])
    }
    pub fn new_or() -> Filter_ {
        Filter_::Or(vec![])
    }
    pub fn and(mut self, filter: Filter_) -> Filter_ {
        match self {
            Filter_::And(ref mut v) => {
                v.push(filter);
                self
            },
            _ => Filter_::And(vec![self, filter]),
        }
    }
    pub fn or(mut self, filter: Filter_) -> Filter_ {
        match self {
            Filter_::Or(ref mut v) => {
                v.push(filter);
                self
            },
            Filter_::And(ref v) if v.is_empty() => {
                Filter_::Or(vec![filter])
            },
            _ => Filter_::Or(vec![self, filter]),
        }
    }
    pub fn when<F>(self, condition: bool, f: F) -> Self
    where
        F: FnOnce(Self) -> Self,
    {
        if condition {
            f(self)
        } else {
            self
        }
    }
    pub fn if_let_some<T, F>(self, value: &Option<T>, f: F) -> Self
    where
        F: FnOnce(Self, &T) -> Self,
    {
        if let Some(v) = value {
            f(self, v)
        } else {
            self
        }
    }
    pub fn joiner(&self) -> Option<Box<Joiner_>> {
        match self {
            Filter_::And(list) => list.iter().fold(None, |acc, c| Joiner_::merge(acc, c.joiner())),
            Filter_::Or(list) => list.iter().fold(None, |acc, c| Joiner_::merge(acc, c.joiner())),
            Filter_::Exists(c) => c.joiner(),
            Filter_::NotExists(c) => c.joiner(),
            Filter_::EqAny(c) => c.joiner(),
            Filter_::NotAll(c) => c.joiner(),
            _ => None
        }
    }
}
impl Check_<dyn @{ pascal_name }@Cache> for Filter_ {
    fn check(&self, obj: &dyn @{ pascal_name }@Cache) -> bool {
        match self {
            Filter_::IsNull(c) => c.check_null(obj),
            Filter_::IsNotNull(c) => !c.check_null(obj),
            Filter_::Eq(c) => c.check_eq(obj),
            Filter_::NotEq(c) => !c.check_eq(obj),
            Filter_::Gt(c) => c.check_cmp(obj, std::cmp::Ordering::Greater, false).unwrap_or_else(|x| x),
            Filter_::Gte(c) => c.check_cmp(obj, std::cmp::Ordering::Greater, true).unwrap_or_else(|x| x),
            Filter_::Lt(c) => c.check_cmp(obj, std::cmp::Ordering::Less, false).unwrap_or_else(|x| x),
            Filter_::Lte(c) => c.check_cmp(obj, std::cmp::Ordering::Less, true).unwrap_or_else(|x| x),
            Filter_::Like(c) => c.check_like(obj),
            Filter_::In(c) => c.check_in(obj),
            Filter_::NotIn(c) => !c.check_in(obj),
            Filter_::Not(c) => !c.check(obj),
            Filter_::And(list) => list.iter().all(|c| c.check(obj)),
            Filter_::Or(list) => list.iter().any(|c| c.check(obj)),
            Filter_::Exists(c) => c.check(obj),
            Filter_::NotExists(c) => !c.check(obj),
            Filter_::EqAny(c) => c.check(obj),
            Filter_::NotAll(c) => !c.check(obj),
            Filter_::Boolean(c) => *c,
            _ => unimplemented!(),
        }
    }
}
impl Check_<dyn @{ pascal_name }@> for Filter_ {
    fn check(&self, obj: &dyn @{ pascal_name }@) -> bool {
        match self {
            Filter_::IsNull(c) => c.check_null(obj),
            Filter_::IsNotNull(c) => !c.check_null(obj),
            Filter_::Eq(c) => c.check_eq(obj),
            Filter_::NotEq(c) => !c.check_eq(obj),
            Filter_::Gt(c) => c.check_cmp(obj, std::cmp::Ordering::Greater, false).unwrap_or_else(|x| x),
            Filter_::Gte(c) => c.check_cmp(obj, std::cmp::Ordering::Greater, true).unwrap_or_else(|x| x),
            Filter_::Lt(c) => c.check_cmp(obj, std::cmp::Ordering::Less, false).unwrap_or_else(|x| x),
            Filter_::Lte(c) => c.check_cmp(obj, std::cmp::Ordering::Less, true).unwrap_or_else(|x| x),
            Filter_::Like(c) => c.check_like(obj),
            Filter_::In(c) => c.check_in(obj),
            Filter_::NotIn(c) => !c.check_in(obj),
            Filter_::Not(c) => !c.check(obj),
            Filter_::And(list) => list.iter().all(|c| c.check(obj)),
            Filter_::Or(list) => list.iter().any(|c| c.check(obj)),
            Filter_::Exists(c) => c.check(obj),
            Filter_::NotExists(c) => !c.check(obj),
            Filter_::EqAny(c) => c.check(obj),
            Filter_::NotAll(c) => !c.check(obj),
            Filter_::Boolean(c) => *c,
            _ => unimplemented!(),
        }
    }
}

@% let filter_macro_name = "filter_{}_{}"|format(group_name, model_name) -%@
@% let model_path = "$crate::models::{}::{}::_base::_{}"|format(db|snake|to_var_name, group_name|to_var_name, mod_name) -%@
@% if config.export_db_layer %@#[macro_export]@% else %@#[allow(unused_macros)]@% endif %@
macro_rules! @{ filter_macro_name }@_null {
@%- for (col_name, column_def) in def.nullable() %@
    (@{ col_name }@) => (@{ model_path }@::Col_::@{ col_name|to_var_name }@);
@%- endfor %@
    () => (); // For empty case
}
pub use @{ filter_macro_name }@_null as filter_null;

@% if config.export_db_layer %@#[macro_export]@% else %@#[allow(unused_macros)]@% endif %@
macro_rules! @{ filter_macro_name }@_text {
@%- for (col_name, column_def) in def.text() %@
    (@{ col_name }@) => (@{ model_path }@::Col_::@{ col_name|to_var_name }@);
@%- endfor %@
    () => (); // For empty case
}
pub use @{ filter_macro_name }@_text as filter_text;

@% if config.export_db_layer %@#[macro_export]@% else %@#[allow(unused_macros)]@% endif %@
macro_rules! @{ filter_macro_name }@_one {
@%- for (col_name, column_def) in def.all_fields_without_json() %@
    (@{ col_name }@ $e:expr) => (@{ model_path }@::ColOne_::@{ col_name|to_var_name }@($e.clone().try_into()?));
@%- endfor %@
}
pub use @{ filter_macro_name }@_one as filter_one;

@% if config.export_db_layer %@#[macro_export]@% else %@#[allow(unused_macros)]@% endif %@
macro_rules! @{ filter_macro_name }@_many {
@%- for (col_name, column_def) in def.all_fields_without_json() %@
    (@{ col_name }@ [$($e:expr),*]) => (@{ model_path }@::ColMany_::@{ col_name|to_var_name }@(vec![ $( $e.clone().try_into()? ),* ]));
    (@{ col_name }@ $e:expr) => (@{ model_path }@::ColMany_::@{ col_name|to_var_name }@($e.into_iter().map(|v| v.clone().try_into()).collect::<Result<Vec<_>, _>>()?));
@%- endfor %@
}
pub use @{ filter_macro_name }@_many as filter_many;

@% if config.export_db_layer %@#[macro_export]@% else %@#[allow(unused_macros)]@% endif %@
macro_rules! @{ filter_macro_name }@_json {
@%- for (col_name, column_def) in def.all_fields_only_json() %@
    (@{ col_name }@ $e:expr) => (@{ model_path }@::ColJson_::@{ col_name|to_var_name }@($e.clone().try_into()?));
@%- endfor %@
    () => ();
}
pub use @{ filter_macro_name }@_json as filter_json;

@% if config.export_db_layer %@#[macro_export]@% else %@#[allow(unused_macros)]@% endif %@
macro_rules! @{ filter_macro_name }@_json_array {
@%- for (col_name, column_def) in def.all_fields_only_json() %@
    (@{ col_name }@ $e:expr) => (@{ model_path }@::ColJsonArray_::@{ col_name|to_var_name }@($e.iter().map(|v| v.clone().try_into()).collect::<Result<Vec<_>, _>>()?));
@%- endfor %@
    () => ();
}
pub use @{ filter_macro_name }@_json_array as filter_json_array;

@% if config.export_db_layer %@#[macro_export]@% else %@#[allow(unused_macros)]@% endif %@
macro_rules! @{ filter_macro_name }@_geo {
@%- for (col_name, column_def) in def.all_fields_only_geo() %@
    (@{ col_name }@ $e:expr, $s:expr) => (@{ model_path }@::ColGeo_::@{ col_name|to_var_name }@($e.clone().try_into()?, $s));
    (@{ col_name }@ $e:expr) => (@{ model_path }@::ColGeo_::@{ col_name|to_var_name }@($e.clone().try_into()?, @{ column_def.srid() }@));
@%- endfor %@
    () => ();
}
pub use @{ filter_macro_name }@_geo as filter_geo;

@% if config.export_db_layer %@#[macro_export]@% else %@#[allow(unused_macros)]@% endif %@
macro_rules! @{ filter_macro_name }@_geo_distance {
@%- for (col_name, column_def) in def.all_fields_only_geo() %@
    (@{ col_name }@ $e:expr, $d:expr, $s:expr) => (@{ model_path }@::ColGeoDistance_::@{ col_name|to_var_name }@($e.clone().try_into()?, $d, $s));
    (@{ col_name }@ $e:expr, $d:expr) => (@{ model_path }@::ColGeoDistance_::@{ col_name|to_var_name }@($e.clone().try_into()?, $d, @{ column_def.srid() }@));
@%- endfor %@
    () => ();
}
pub use @{ filter_macro_name }@_geo_distance as filter_geo_distance;

@% if config.export_db_layer %@#[macro_export]@% else %@#[allow(unused_macros)]@% endif %@
macro_rules! @{ filter_macro_name }@_rel {
@%- for (model_def, col_name, rel_def) in def.relations_one_and_belonging(false) %@
    (@{ col_name }@) => (@{ model_path }@::ColRel_::@{ col_name|to_var_name }@(None));
    (@{ col_name }@ $t:tt) => (@{ model_path }@::ColRel_::@{ col_name|to_var_name }@(Some(Box::new($crate::models::@{ db|snake|to_var_name }@::@{ rel_def.get_group_name()|snake|to_var_name }@::_base::_@{ rel_def.get_mod_name() }@::filter!($t)))));
@%- endfor %@
@%- for (model_def, col_name, rel_def) in def.relations_many(false) %@
    (@{ col_name }@) => (@{ model_path }@::ColRel_::@{ col_name|to_var_name }@(None));
    (@{ col_name }@ $t:tt) => (@{ model_path }@::ColRel_::@{ col_name|to_var_name }@(Some(Box::new($crate::models::@{ db|snake|to_var_name }@::@{ rel_def.get_group_name()|snake|to_var_name }@::_base::_@{ rel_def.get_mod_name() }@::filter!($t)))));
@%- endfor %@
    () => ();
}
pub use @{ filter_macro_name }@_rel as filter_rel;

@% if config.export_db_layer %@#[macro_export]@% else %@#[allow(unused_macros)]@% endif %@
macro_rules! @{ filter_macro_name }@ {
    () => (@{ model_path }@::Filter_::new_and());
@%- for (index_name, index) in def.multi_index() %@
    ((@{ index.join_fields(def, "{name}", ", ") }@) = (@{ index.join_fields(def, "$e{index}:expr", ", ") }@)) => (@{ model_path }@::Filter_::Eq(@{ model_path }@::ColOne_::@{ index.join_fields(def, "{name}", "_") }@(@{ index.join_fields(def, "$e{index}.clone().try_into()?", ", ") }@)));
    ((@{ index.join_fields(def, "{name}", ", ") }@) > (@{ index.join_fields(def, "$e{index}:expr", ", ") }@)) => (@{ model_path }@::Filter_::Gt(@{ model_path }@::ColOne_::@{ index.join_fields(def, "{name}", "_") }@(@{ index.join_fields(def, "$e{index}.clone().try_into()?", ", ") }@)));
    ((@{ index.join_fields(def, "{name}", ", ") }@) >= (@{ index.join_fields(def, "$e{index}:expr", ", ") }@)) => (@{ model_path }@::Filter_::Gte(@{ model_path }@::ColOne_::@{ index.join_fields(def, "{name}", "_") }@(@{ index.join_fields(def, "$e{index}.clone().try_into()?", ", ") }@)));
    ((@{ index.join_fields(def, "{name}", ", ") }@) < (@{ index.join_fields(def, "$e{index}:expr", ", ") }@)) => (@{ model_path }@::Filter_::Lt(@{ model_path }@::ColOne_::@{ index.join_fields(def, "{name}", "_") }@(@{ index.join_fields(def, "$e{index}.clone().try_into()?", ", ") }@)));
    ((@{ index.join_fields(def, "{name}", ", ") }@) <= (@{ index.join_fields(def, "$e{index}:expr", ", ") }@)) => (@{ model_path }@::Filter_::Lte(@{ model_path }@::ColOne_::@{ index.join_fields(def, "{name}", "_") }@(@{ index.join_fields(def, "$e{index}.clone().try_into()?", ", ") }@)));
    ((@{ index.join_fields(def, "{name}", ", ") }@) = $e:expr) => (@{ model_path }@::Filter_::Eq(@{ model_path }@::ColOne_::@{ index.join_fields(def, "{name}", "_") }@(@{ index.join_fields(def, "$e.{index}.clone().try_into()?", ", ") }@)));
    ((@{ index.join_fields(def, "{name}", ", ") }@) IN $e:expr) => (@{ model_path }@::Filter_::In(@{ model_path }@::ColMany_::@{ index.join_fields(def, "{name}", "_") }@($e.into_iter().map(|v| (@{ index.join_fields(def, "v.{index}.clone()", ", ") }@).try_into()).collect::<Result<_, _>>()?)));
    ((@{ index.join_fields(def, "{name}", ", ") }@) NOT IN $e:expr) => (@{ model_path }@::Filter_::NotIn(@{ model_path }@::ColMany_::@{ index.join_fields(def, "{name}", "_") }@($e.into_iter().map(|v| (@{ index.join_fields(def, "v.{index}.clone()", ", ") }@).try_into()).collect::<Result<_, _>>()?)));
    ((@{ index.join_fields(def, "{name}", ", ") }@) > $e:expr) => (@{ model_path }@::Filter_::Gt(@{ model_path }@::ColOne_::@{ index.join_fields(def, "{name}", "_") }@(@{ index.join_fields(def, "$e.{index}.clone().try_into()?", ", ") }@)));
    ((@{ index.join_fields(def, "{name}", ", ") }@) >= $e:expr) => (@{ model_path }@::Filter_::Gte(@{ model_path }@::ColOne_::@{ index.join_fields(def, "{name}", "_") }@(@{ index.join_fields(def, "$e.{index}.clone().try_into()?", ", ") }@)));
    ((@{ index.join_fields(def, "{name}", ", ") }@) < $e:expr) => (@{ model_path }@::Filter_::Lt(@{ model_path }@::ColOne_::@{ index.join_fields(def, "{name}", "_") }@(@{ index.join_fields(def, "$e.{index}.clone().try_into()?", ", ") }@)));
    ((@{ index.join_fields(def, "{name}", ", ") }@) <= $e:expr) => (@{ model_path }@::Filter_::Lte(@{ model_path }@::ColOne_::@{ index.join_fields(def, "{name}", "_") }@(@{ index.join_fields(def, "$e.{index}.clone().try_into()?", ", ") }@)));
@%- endfor %@
    (($($t:tt)*)) => (@{ model_path }@::filter!($($t)*));
    (NOT $t:tt) => (@{ model_path }@::Filter_::Not(Box::new(@{ model_path }@::filter!($t))));
    (WITH_TRASHED) => (@{ model_path }@::Filter_::WithTrashed);
    (ONLY_TRASHED) => (@{ model_path }@::Filter_::OnlyTrashed);
    (BOOLEAN $e:expr) => (@{ model_path }@::Filter_::Boolean($e));
    (RAW $e:expr) => (@{ model_path }@::Filter_::Raw($e.to_string()));
    (RAW $e:expr , [$($p:expr),*] ) => (@{ model_path }@::Filter_::RawWithParam($e.to_string(), vec![ $( $p.to_string() ),* ]));
    (RAW $e:expr , $p:expr ) => (@{ model_path }@::Filter_::RawWithParam($e.to_string(), $p.iter().map(|v| v.to_string()).collect()));
    (MATCH ( $($i:ident),+ ) AGAINST ($e:expr) IN BOOLEAN MODE) => (@{ model_path }@::Filter_::MatchBoolean(vec![ $( @{ model_path }@::filter_text!($i) ),* ], $e.to_string()));
    (MATCH ( $($i:ident),+ ) AGAINST ($e:expr) WITH QUERY EXPANSION) => (@{ model_path }@::Filter_::MatchExpansion(vec![ $( @{ model_path }@::filter_text!($i) ),* ], $e.to_string()));
    (MATCH ( $($i:ident),+ ) AGAINST ($e:expr)) => (@{ model_path }@::Filter_::Match(vec![ $( @{ model_path }@::filter_text!($i) ),* ], $e.to_string()));
    ($i:ident EXISTS) => (@{ model_path }@::Filter_::Exists(@{ model_path }@::filter_rel!($i)));
    ($i:ident EXISTS $t:tt) => (@{ model_path }@::Filter_::Exists(@{ model_path }@::filter_rel!($i $t)));
    ($i:ident NOT EXISTS) => (@{ model_path }@::Filter_::NotExists(@{ model_path }@::filter_rel!($i)));
    ($i:ident NOT EXISTS $t:tt) => (@{ model_path }@::Filter_::NotExists(@{ model_path }@::filter_rel!($i $t)));
    ($i:ident = ANY $t:tt) => (@{ model_path }@::Filter_::EqAny(@{ model_path }@::filter_rel!($i $t)));
    ($i:ident NOT ALL $t:tt) => (@{ model_path }@::Filter_::NotAll(@{ model_path }@::filter_rel!($i $t)));
    ($i:ident IS NULL) => (@{ model_path }@::Filter_::IsNull(@{ model_path }@::filter_null!($i)));
    ($i:ident IS NOT NULL) => (@{ model_path }@::Filter_::IsNotNull(@{ model_path }@::filter_null!($i)));
    ($i:ident = $e:expr) => (@{ model_path }@::Filter_::Eq(@{ model_path }@::filter_one!($i $e)));
    ($i:ident != $e:expr) => (@{ model_path }@::Filter_::NotEq(@{ model_path }@::filter_one!($i $e)));
    ($i:ident > $e:expr) => (@{ model_path }@::Filter_::Gt(@{ model_path }@::filter_one!($i $e)));
    ($i:ident >= $e:expr) => (@{ model_path }@::Filter_::Gte(@{ model_path }@::filter_one!($i $e)));
    ($i:ident < $e:expr) => (@{ model_path }@::Filter_::Lt(@{ model_path }@::filter_one!($i $e)));
    ($i:ident <= $e:expr) => (@{ model_path }@::Filter_::Lte(@{ model_path }@::filter_one!($i $e)));
    ($i:ident LIKE $e:expr) => (@{ model_path }@::Filter_::Like(@{ model_path }@::filter_one!($i $e)));
    ($i:ident ALL_BITS $e:expr) => (@{ model_path }@::Filter_::AllBits(@{ model_path }@::filter_many!($i [$e, $e])));
    ($i:ident ANY_BITS $e:expr) => (@{ model_path }@::Filter_::AnyBits(@{ model_path }@::filter_one!($i $e)));
    ($i:ident BETWEEN ($e1:expr, $e2:expr)) => (@{ model_path }@::filter!(($i >= $e1) AND ($i <= $e2)));
    ($i:ident RIGHT_OPEN ($e1:expr, $e2:expr)) => (@{ model_path }@::filter!(($i >= $e1) AND ($i < $e2)));
    ($i:ident IN ( $($e:expr),* )) => (@{ model_path }@::Filter_::In(@{ model_path }@::filter_many!($i [ $( $e ),* ])));
    ($i:ident IN $e:expr) => (@{ model_path }@::Filter_::In(@{ model_path }@::filter_many!($i $e)));
    ($i:ident NOT IN ( $($e:expr),* )) => (@{ model_path }@::Filter_::NotIn(@{ model_path }@::filter_many!($i [ $( $e ),* ])));
    ($i:ident NOT IN $e:expr) => (@{ model_path }@::Filter_::NotIn(@{ model_path }@::filter_many!($i $e)));
    ($i:ident HAS $e:expr) => (@{ model_path }@::Filter_::MemberOf(@{ model_path }@::filter_json!($i $e), None));
    ($i:ident -> ($p:expr) HAS $e:expr) => (@{ model_path }@::Filter_::MemberOf(@{ model_path }@::filter_json!($i $e), Some($p.to_string())));
    ($i:ident CONTAINS [ $($e:expr),* ]) => (@{ model_path }@::Filter_::Contains(@{ model_path }@::filter_json_array!($i vec![ $( $e ),* ]), None));
    ($i:ident CONTAINS $e:expr) => (@{ model_path }@::Filter_::Contains(@{ model_path }@::filter_json_array!($i $e), None));
    ($i:ident -> ($p:expr) CONTAINS [ $($e:expr),* ]) => (@{ model_path }@::Filter_::Contains(@{ model_path }@::filter_json_array!($i vec![ $( $e ),* ]), Some($p.to_string())));
    ($i:ident -> ($p:expr) CONTAINS $e:expr) => (@{ model_path }@::Filter_::Contains(@{ model_path }@::filter_json_array!($i $e), Some($p.to_string())));
    ($i:ident OVERLAPS [ $($e:expr),* ]) => (@{ model_path }@::Filter_::Overlaps(@{ model_path }@::filter_json_array!($i vec![ $( $e ),* ]), None));
    ($i:ident OVERLAPS $e:expr) => (@{ model_path }@::Filter_::Overlaps(@{ model_path }@::filter_json_array!($i $e), None));
    ($i:ident -> ($p:expr) OVERLAPS [ $($e:expr),* ]) => (@{ model_path }@::Filter_::Overlaps(@{ model_path }@::filter_json_array!($i vec![ $( $e ),* ]), Some($p.to_string())));
    ($i:ident -> ($p:expr) OVERLAPS $e:expr) => (@{ model_path }@::Filter_::Overlaps(@{ model_path }@::filter_json_array!($i $e), Some($p.to_string())));
    ($i:ident -> ($p:expr) IN [ $($e:expr),* ]) => (@{ model_path }@::Filter_::JsonIn(@{ model_path }@::filter_json_array!($i vec![ $( $e ),* ]), Some($p.to_string())));
    ($i:ident -> ($p:expr) IN $e:expr) => (@{ model_path }@::Filter_::JsonIn(@{ model_path }@::filter_json_array!($i $e), Some($p.to_string())));
    ($i:ident JSON_CONTAINS_PATH ($p:expr)) => (@{ model_path }@::Filter_::JsonContainsPath(@{ model_path }@::filter_json!($i 0), $p.to_string()));
    ($i:ident -> ($p:expr) = $e:expr) => (@{ model_path }@::Filter_::JsonEq(@{ model_path }@::filter_json!($i $e), $p.to_string()));
    ($i:ident -> ($p:expr) < $e:expr) => (@{ model_path }@::Filter_::JsonLt(@{ model_path }@::filter_json!($i $e), $p.to_string()));
    ($i:ident -> ($p:expr) <= $e:expr) => (@{ model_path }@::Filter_::JsonLte(@{ model_path }@::filter_json!($i $e), $p.to_string()));
    ($i:ident -> ($p:expr) > $e:expr) => (@{ model_path }@::Filter_::JsonGt(@{ model_path }@::filter_json!($i $e), $p.to_string()));
    ($i:ident -> ($p:expr) >= $e:expr) => (@{ model_path }@::Filter_::JsonGte(@{ model_path }@::filter_json!($i $e), $p.to_string()));
    ($i:ident WITHIN_WITH_SRID $e:expr, $s:expr) => (@{ model_path }@::Filter_::Within(@{ model_path }@::filter_geo!($i $e, $s)));
    ($i:ident WITHIN $e:expr) => (@{ model_path }@::Filter_::Within(@{ model_path }@::filter_geo!($i $e)));
    ($i:ident INTERSECTS_WITH_SRID $e:expr, $s:expr) => (@{ model_path }@::Filter_::Intersects(@{ model_path }@::filter_geo!($i $e, $s)));
    ($i:ident INTERSECTS $e:expr) => (@{ model_path }@::Filter_::Intersects(@{ model_path }@::filter_geo!($i $e)));
    ($i:ident CROSSES_WITH_SRID $e:expr, $s:expr) => (@{ model_path }@::Filter_::Crosses(@{ model_path }@::filter_geo!($i $e, $s)));
    ($i:ident CROSSES $e:expr) => (@{ model_path }@::Filter_::Crosses(@{ model_path }@::filter_geo!($i $e)));
    ($i:ident D_WITHIN_WITH_SRID $e:expr, $d:expr, $s:expr) => (@{ model_path }@::Filter_::DWithin(@{ model_path }@::filter_geo_distance!($i $e, $d, $s)));
    ($i:ident D_WITHIN $e:expr, $d:expr) => (@{ model_path }@::Filter_::DWithin(@{ model_path }@::filter_geo_distance!($i $e, $d)));
    ($t1:tt AND $($t2:tt)AND+) => (@{ model_path }@::Filter_::And(vec![ @{ model_path }@::filter!($t1), $( @{ model_path }@::filter!($t2) ),* ]));
    ($t1:tt OR $($t2:tt)OR+) => (@{ model_path }@::Filter_::Or(vec![ @{ model_path }@::filter!($t1), $( @{ model_path }@::filter!($t2) ),* ]));
}
pub use @{ filter_macro_name }@ as filter;

#[derive(Clone, Debug)]
pub enum Order_ {
    Asc(Col_),
    Desc(Col_),
    IsNullAsc(Col_),
    IsNullDesc(Col_),
}

@% let order_macro_name = "order_{}_{}"|format(group_name, model_name) -%@
@% if config.export_db_layer %@#[macro_export]@% else %@#[allow(unused_macros)]@% endif %@
macro_rules! @{ order_macro_name }@_col {
@%- for (col_name, column_def) in def.all_fields() %@
    (@{ col_name }@) => (@{ model_path }@::Col_::@{ col_name|to_var_name }@);
@%- endfor %@
}
pub use @{ order_macro_name }@_col as order_by_col;

@% if config.export_db_layer %@#[macro_export]@% else %@#[allow(unused_macros)]@% endif %@
macro_rules! @{ order_macro_name }@_one {
    ($i:ident) => (@{ model_path }@::Order_::Asc(@{ model_path }@::order_by_col!($i)));
    ($i:ident ASC) => (@{ model_path }@::Order_::Asc(@{ model_path }@::order_by_col!($i)));
    ($i:ident DESC) => (@{ model_path }@::Order_::Desc(@{ model_path }@::order_by_col!($i)));
    ($i:ident IS NULL ASC) => (@{ model_path }@::Order_::IsNullAsc(@{ model_path }@::order_by_col!($i)));
    ($i:ident IS NULL DESC) => (@{ model_path }@::Order_::IsNullDesc(@{ model_path }@::order_by_col!($i)));
}
pub use @{ order_macro_name }@_one as order_by_one;

@% if config.export_db_layer %@#[macro_export]@% else %@#[allow(unused_macros)]@% endif %@
macro_rules! @{ order_macro_name }@ {
    ($($($i:ident)+),+) => (vec![$( @{ model_path }@::order_by_one!($($i)+)),+]);
}
pub use @{ order_macro_name }@ as order;


#[cfg(any(feature = "mock", test))]
#[derive(derive_new::new, Clone, Default)]
pub struct Emu@{ pascal_name }@Repository(pub std::sync::Arc<std::sync::Mutex<std::collections::BTreeMap<@{- def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") }@, @{ pascal_name }@Entity>>>);

#[cfg(any(feature = "mock", test))]
impl Emu@{ pascal_name }@Repository {
    pub fn _load(&self, data: &Vec<@{ pascal_name }@Entity>) {
        let mut map = self.0.lock().unwrap();
        for v in data {
            map.insert(@{- def.primaries()|fmt_join_with_paren("v.{var}{clone}", ", ") }@, v.clone());
        }
    }
}
#[cfg(any(feature = "mock", test))]
#[async_trait]
impl _@{ pascal_name }@Repository for Emu@{ pascal_name }@Repository {
    @%- if !def.disable_update() %@
    fn find_for_update(&self, id: @{ def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") }@) -> Box<dyn @{ pascal_name }@RepositoryFindForUpdateBuilder> {
        struct V(Option<@{ pascal_name }@Entity>, Option<Filter_>);
        #[async_trait]
        impl @{ pascal_name }@RepositoryFindForUpdateBuilder for V {
            async fn query(self: Box<Self>) -> anyhow::Result<Box<dyn _Updater>> {
                use anyhow::Context;
                let filter = self.1;
                self.0.filter(|v| filter.map(|f| f.check(v as &dyn @{ pascal_name }@)).unwrap_or(true))
                    .map(|v| Box::new(v) as Box<dyn _Updater>)
                    .with_context(|| "Not Found")
            }
            fn visibility_filter(mut self: Box<Self>, filter: Filter_) -> Box<dyn _RepositoryFindForUpdateBuilder> { self.1 = Some(filter); self }
            @%- if def.is_soft_delete() %@
            fn with_trashed(self: Box<Self>, _mode: bool) -> Box<dyn _RepositoryFindForUpdateBuilder> { self }
            @%- endif %@
            fn join(self: Box<Self>, _join: Option<Box<Joiner_>>) -> Box<dyn _RepositoryFindForUpdateBuilder> { self }
        }
        let map = self.0.lock().unwrap();
        Box::new(V(map.get(&id).cloned(), None))
    }
    @%- endif %@
    fn convert_factory(&self, factory: @{ pascal_name }@Factory) -> Box<dyn _Updater> {
        Box::new(@{ pascal_name }@Entity {
@{- def.non_auto_primary_for_factory()|fmt_join("
            {var}: factory.{var},", "") }@
            ..Default::default()
        })
    }
    #[allow(unused_mut)]
    async fn save(&self, obj: Box<dyn _Updater>) -> anyhow::Result<Option<Box<dyn @{ pascal_name }@>>> {
        let mut obj = if let Ok(obj) = obj.downcast::<@{ pascal_name }@Entity>() {
            obj
        } else {
            panic!("Only @{ pascal_name }@Entity is accepted.");
        };
        if obj._delete {
            @%- if !def.disable_update() %@
            #[allow(deprecated)]
            self.delete(obj).await?;
            @%- endif %@
            Ok(None)
        } else {
            let mut map = self.0.lock().unwrap();
            @%- for (name, column_def) in def.auto_inc_or_seq() %@
            if obj.@{ name|to_var_name }@ == 0.into() {
                obj.@{ name|to_var_name }@ = (map.iter().map(|(_k, v)| @{ column_def.get_inner_type(true, false) }@::from(v.@{ name|to_var_name }@)).max().unwrap_or_default() + 1).into();
            }
            @%- endfor %@
            @%- for (name, column_def) in def.auto_uuid() %@
            if obj.@{ name|to_var_name }@.is_nil() {
                obj.@{ name|to_var_name }@ = uuid::Uuid::new_v4().into();
            }
            @%- endfor %@
            map.insert(@{- def.primaries()|fmt_join_with_paren("obj.{var}{clone}", ", ") }@, *obj.clone());
            Ok(Some(obj as Box<dyn @{ pascal_name }@>))
        }
    }
    @%- if !def.disable_update() %@
    #[allow(unused_mut)]
    async fn import(&self, list: Vec<Box<dyn _Updater>>, _option: Option<crate::models::ImportOption>) -> anyhow::Result<()> {
        for obj in list {
            let mut obj = if let Ok(obj) = obj.downcast::<@{ pascal_name }@Entity>() {
                obj
            } else {
                panic!("Only @{ pascal_name }@Entity is accepted.");
            };
            if obj._delete {
                @%- if !def.disable_update() %@
                #[allow(deprecated)]
                self.delete(obj).await?;
                @%- endif %@
            } else {
                let mut map = self.0.lock().unwrap();
                @%- for (name, column_def) in def.auto_inc_or_seq() %@
                if obj.@{ name|to_var_name }@ == 0.into() {
                    obj.@{ name|to_var_name }@ = (map.iter().map(|(_k, v)| @{ column_def.get_inner_type(true, false) }@::from(v.@{ name|to_var_name }@)).max().unwrap_or_default() + 1).into();
                }
                @%- endfor %@
                @%- for (name, column_def) in def.auto_uuid() %@
                if obj.@{ name|to_var_name }@.is_nil() {
                    obj.@{ name|to_var_name }@ = uuid::Uuid::new_v4().into();
                }
                @%- endfor %@
                map.insert(@{- def.primaries()|fmt_join_with_paren("obj.{var}{clone}", ", ") }@, *obj.clone());
            }
        }
        Ok(())
    }
    @%- endif %@
    @%- if def.use_insert_delayed() %@
    #[allow(unused_mut)]
    async fn insert_delayed(&self, obj: Box<dyn _Updater>) -> anyhow::Result<()> {
        let mut obj = if let Ok(obj) = obj.downcast::<@{ pascal_name }@Entity>() {
            obj
        } else {
            panic!("Only @{ pascal_name }@Entity is accepted.");
        };
        let mut map = self.0.lock().unwrap();
        @%- for (name, column_def) in def.auto_inc_or_seq() %@
        if obj.@{ name|to_var_name }@ == 0.into() {
            obj.@{ name|to_var_name }@ = (map.iter().map(|(_k, v)| @{ column_def.get_inner_type(true, false) }@::from(v.@{ name|to_var_name }@)).max().unwrap_or_default() + 1).into();
        }
        @%- endfor %@
        @%- for (name, column_def) in def.auto_uuid() %@
        if obj.@{ name|to_var_name }@.is_empty() {
            obj.@{ name|to_var_name }@ = uuid::Uuid::new_v4().to_string().into();
        }
        @%- endfor %@
        map.insert(@{- def.primaries()|fmt_join_with_paren("obj.{var}{clone}", ", ") }@, *obj.clone());
        Ok(())
    }
    @%- endif %@
    @%- if !def.disable_update() %@
    async fn delete(&self, obj: Box<dyn _Updater>) -> anyhow::Result<()> {
        let mut map = self.0.lock().unwrap();
        map.remove(&@{- def.primaries()|fmt_join_with_paren("obj.{var}(){clone}", ", ") }@);
        Ok(())
    }
    @%- if def.primaries().len() == 1 %@
    async fn delete_by_ids(&self, ids: &[@{ def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") }@]) -> anyhow::Result<u64> {
        let mut count = 0;
        let mut map = self.0.lock().unwrap();
        for id in ids {
            if map.remove(id).is_some() {
                count += 1;
            }
        }
        Ok(count)
    }
    @%- endif %@
    async fn delete_all(&self) -> anyhow::Result<()> {
        let mut map = self.0.lock().unwrap();
        map.clear();
        Ok(())
    }
    @%- endif %@
    @%- if def.act_as_job_queue() %@
    async fn fetch(&self, limit: usize) -> anyhow::Result<Vec<Box<dyn _Updater>>> {
        let map = self.0.lock().unwrap();
        Ok(map.iter().take(limit).map(|(_, v)| Box::new(v.clone()) as Box<dyn _Updater>).collect())
    }
    @%- endif %@
    @%- for (selector, selector_def) in def.selectors %@
    fn @{ selector|to_var_name }@(&self) -> Box<dyn @{ pascal_name }@Repository@{ selector|pascal }@Builder> {
        #[derive(Default)]
        struct V {
            _list: Vec<@{ pascal_name }@Entity>,
            query_filter: Option<@{ pascal_name }@Query@{ selector|pascal }@Filter>,
            visibility_filter: Option<Filter_>,
            @%- if def.is_soft_delete() %@
            with_trashed: bool,
            @%- endif %@
        }
        #[async_trait]
        impl @{ pascal_name }@Repository@{ selector|pascal }@Builder for V {
            async fn query(self: Box<Self>) -> anyhow::Result<Vec<Box<dyn _Updater>>> {
                let list: Vec<_> = self._list.into_iter()
                    .filter(|v| {
                        if let Some(filter) = &self.query_filter {
                            if !_filter_@{ selector }@(v, filter) {
                                return false;
                            }
                        }
                        if let Some(filter) = &self.visibility_filter {
                            if !filter.check(v as &dyn @{ pascal_name }@) {
                                return false;
                            }
                        }
                        @{ def.soft_delete_tpl2("true","self.with_trashed || v.deleted_at.is_none()","self.with_trashed || !v.deleted","self.with_trashed || v.deleted == 0")}@
                    })
                    .map(|v| Box::new(v) as Box<dyn _Updater>).collect();
                Ok(list)
            }
            async fn count(self: Box<Self>) -> anyhow::Result<i64> {
                let list: Vec<_> = self._list.into_iter()
                    .filter(|v| {
                        if let Some(filter) = &self.query_filter {
                            if !_filter_@{ selector }@(v, filter) {
                                return false;
                            }
                        }
                        if let Some(filter) = &self.visibility_filter {
                            if !filter.check(v as &dyn @{ pascal_name }@) {
                                return false;
                            }
                        }
                        @{ def.soft_delete_tpl2("true","self.with_trashed || v.deleted_at.is_none()","self.with_trashed || !v.deleted","self.with_trashed || v.deleted == 0")}@
                    })
                    .map(|v| Box::new(v) as Box<dyn @{ pascal_name }@@% if def.use_cache() %@Cache@% endif %@>).collect();
                Ok(list.len() as i64)
            }
            fn query_filter(mut self: Box<Self>, filter: @{ pascal_name }@Query@{ selector|pascal }@Filter) -> Box<dyn _Repository@{ selector|pascal }@Builder> { self.query_filter = Some(filter); self }
            fn visibility_filter(mut self: Box<Self>, filter: Filter_) -> Box<dyn _Repository@{ selector|pascal }@Builder> { self.visibility_filter = Some(filter); self }
            @%- if def.is_soft_delete() %@
            fn with_trashed(mut self: Box<Self>, mode: bool) -> Box<dyn _Repository@{ selector|pascal }@Builder> { self.with_trashed = mode; self  }
            @%- endif %@
            fn join(self: Box<Self>, _join: Option<Box<Joiner_>>) -> Box<dyn _Repository@{ selector|pascal }@Builder> { self }
        }
        Box::new(V{_list: self.0.lock().unwrap().values().map(|v| v.clone()).collect(), ..Default::default()})
    }
    @%- endfor %@
}
@%- for (selector, selector_def) in def.selectors %@
@%- for filter_map in selector_def.nested_filters(selector, def) %@
#[cfg(any(feature = "mock", test))]
#[allow(unused_variables)]
#[allow(unused_imports)]
fn _filter@{ filter_map.suffix }@(v: &super::super::super::@{ filter_map.model_group()|snake|to_var_name }@::@{ filter_map.model_name()|snake|to_var_name }@::@{ filter_map.model_name()|pascal }@Entity, filter: &@{ pascal_name }@Query@{ selector|pascal }@@{ filter_map.pascal_name }@Filter) -> bool {
    use super::super::super::@{ filter_map.model_group()|snake|to_var_name }@::@{ filter_map.model_name()|snake|to_var_name }@::*;
    @%- for (filter, filter_def) in filter_map.filters %@
    @{- filter_def.emu_str(filter, filter_map.model) }@
    @%- endfor %@
    if let Some(_and) = &filter._and {
        if !_and.iter().all(|f| _filter@{ filter_map.suffix }@(v, f)) {
            return false;
        }
    }
    if let Some(_or) = &filter._or {
        if !_or.iter().any(|f| _filter@{ filter_map.suffix }@(v, f)) {
            return false;
        }
    }
    true
}
@%- endfor %@
@%- endfor %@

#[cfg(any(feature = "mock", test))]
#[async_trait]
impl _@{ pascal_name }@Query for Emu@{ pascal_name }@Repository {
    @%- if def.use_all_row_cache() && !def.use_filtered_row_cache() %@
    async fn all(&self) -> anyhow::Result<Box<dyn crate::models::EntityIterator<dyn @{ pascal_name }@Cache>>> {
        struct V(std::collections::BTreeMap<@{- def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") }@, @{ pascal_name }@Entity>);
        impl crate::models::EntityIterator<dyn @{ pascal_name }@Cache> for V {
            fn iter(&self) -> Box<dyn Iterator<Item = &(dyn @{ pascal_name }@Cache + 'static)> + '_> {
                Box::new(self.0.iter().map(|(_, v)| v as &dyn @{ pascal_name }@Cache))
            }
            fn into_iter(self) -> Box<dyn Iterator<Item = Box<dyn @{ pascal_name }@Cache>>> {
                Box::new(self.0.into_iter().map(|(_, v)| Box::new(v) as Box<dyn @{ pascal_name }@Cache>))
            }
        }
        Ok(Box::new(V(self.0.lock().unwrap().clone())))
    }
    @%- endif %@
    @%- for (selector, selector_def) in def.selectors %@
    fn @{ selector|to_var_name }@(&self) -> Box<dyn @{ pascal_name }@Query@{ selector|pascal }@Builder> {
        #[derive(Default)]
        struct V {
            _list: Vec<@{ pascal_name }@Entity>,
            query_filter: Option<@{ pascal_name }@Query@{ selector|pascal }@Filter>,
            visibility_filter: Option<Filter_>,
            cursor: Option<@{ pascal_name }@Query@{ selector|pascal }@Cursor>,
            order: Option<@{ pascal_name }@Query@{ selector|pascal }@Order>,
            reverse: bool,
            limit: usize,
            offset: usize,
            @%- if def.is_soft_delete() %@
            with_trashed: bool,
            @%- endif %@
        }
        fn _cursor(v: &@{ pascal_name }@Entity, cursor: &@{ pascal_name }@Query@{ selector|pascal }@Cursor) -> bool {
            match cursor {
                @%- for (cursor, cursor_def) in selector_def.orders %@
                @{ pascal_name }@Query@{ selector|pascal }@Cursor::@{ cursor|pascal }@(c) => {
                    match c {
                        @{- cursor_def.emu_str(def) }@
                    }
                }
                @%- endfor %@
            }
            true
        }
        #[async_trait]
        impl @{ pascal_name }@Query@{ selector|pascal }@Builder for V {
            async fn query(self: Box<Self>) -> anyhow::Result<Vec<Box<dyn @{ pascal_name }@@% if def.use_cache() %@Cache@% endif %@>>> {
                let mut list: Vec<_> = self._list.into_iter()
                    .filter(|v| {
                        if let Some(filter) = &self.query_filter {
                            if !_filter_@{ selector }@(v, filter) {
                                return false;
                            }
                        }
                        if let Some(filter) = &self.visibility_filter {
                            if !filter.check(v as &dyn @{ pascal_name }@) {
                                return false;
                            }
                        }
                        if let Some(cursor) = &self.cursor {
                            if !_cursor(v, cursor) {
                                return false;
                            }
                        }
                        @{ def.soft_delete_tpl2("true","self.with_trashed || v.deleted_at.is_none()","self.with_trashed || !v.deleted","self.with_trashed || v.deleted == 0")}@
                    })
                    .map(|v| Box::new(v) as Box<dyn @{ pascal_name }@@% if def.use_cache() %@Cache@% endif %@>).collect();
                list.sort_by(|a, b| {
                    match self.order.unwrap_or_default() {
                        @%- for (order, fields) in selector_def.orders %@
                        @{ pascal_name }@Query@{ selector|pascal }@Order::@{ order|pascal }@ => @{ selector_def.emu_order(order) }@,
                        @%- endfor %@
                    }
                });
                if self.reverse {
                    list.reverse();
                }
                if self.offset > 0 {
                    list = list.split_off(std::cmp::min(list.len(), self.offset));
                }
                if self.limit > 0 {
                    list.truncate(self.limit);
                }
                Ok(list)
            }
            async fn count(self: Box<Self>) -> anyhow::Result<i64> {
                let list: Vec<_> = self._list.into_iter()
                    .filter(|v| {
                        if let Some(filter) = &self.query_filter {
                            if !_filter_@{ selector }@(v, filter) {
                                return false;
                            }
                        }
                        if let Some(filter) = &self.visibility_filter {
                            if !filter.check(v as &dyn @{ pascal_name }@) {
                                return false;
                            }
                        }
                        if let Some(cursor) = &self.cursor {
                            if !_cursor(v, cursor) {
                                return false;
                            }
                        }
                        @{ def.soft_delete_tpl2("true","self.with_trashed || v.deleted_at.is_none()","self.with_trashed || !v.deleted","self.with_trashed || v.deleted == 0")}@
                    })
                    .map(|v| Box::new(v) as Box<dyn @{ pascal_name }@@% if def.use_cache() %@Cache@% endif %@>).collect();
                Ok(list.len() as i64)
            }
            fn query_filter(mut self: Box<Self>, filter: @{ pascal_name }@Query@{ selector|pascal }@Filter) -> Box<dyn _Query@{ selector|pascal }@Builder> { self.query_filter = Some(filter); self }
            fn visibility_filter(mut self: Box<Self>, filter: Filter_) -> Box<dyn _Query@{ selector|pascal }@Builder> { self.visibility_filter = Some(filter); self }
            fn cursor(mut self: Box<Self>, cursor: @{ pascal_name }@Query@{ selector|pascal }@Cursor) -> Box<dyn _Query@{ selector|pascal }@Builder> { self.cursor = Some(cursor); self }
            fn order_by(mut self: Box<Self>, order: @{ pascal_name }@Query@{ selector|pascal }@Order) -> Box<dyn _Query@{ selector|pascal }@Builder> { self.order = Some(order); self  }
            fn reverse(mut self: Box<Self>, mode: bool) -> Box<dyn _Query@{ selector|pascal }@Builder> { self.reverse = mode; self  }
            fn limit(mut self: Box<Self>, limit: usize) -> Box<dyn _Query@{ selector|pascal }@Builder> { self.limit = limit; self  }
            fn offset(mut self: Box<Self>, offset: usize) -> Box<dyn _Query@{ selector|pascal }@Builder> { self.offset = offset; self  }
            @%- if def.is_soft_delete() %@
            fn with_trashed(mut self: Box<Self>, mode: bool) -> Box<dyn _Query@{ selector|pascal }@Builder> { self.with_trashed = mode; self  }
            @%- endif %@
            fn join(self: Box<Self>, _join: Option<Box<Joiner_>>) -> Box<dyn _Query@{ selector|pascal }@Builder> { self }
        }
        Box::new(V{_list: self.0.lock().unwrap().values().map(|v| v.clone()).collect(), ..Default::default()})
    }
    @%- endfor %@
    @%- if def.use_cache() %@
    fn find(&self, id: @{ def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") }@) -> Box<dyn @{ pascal_name }@QueryFindBuilder> {
        struct V(Option<@{ pascal_name }@Entity>, bool, Option<Filter_>);
        #[async_trait]
        impl @{ pascal_name }@QueryFindBuilder for V {
            async fn query(self: Box<Self>) -> anyhow::Result<Option<Box<dyn @{ pascal_name }@@% if def.use_cache() %@Cache@% endif %@>>> {
                let filter = self.2;
                Ok(self.0.filter(|v| filter.map(|f| f.check(v as &dyn @{ pascal_name }@)).unwrap_or(true))@{- def.soft_delete_tpl2("",".filter(|v| self.1 || v.deleted_at.is_none())",".filter(|v| self.1 || !v.deleted)",".filter(|v| self.1 || v.deleted == 0)")}@.map(|v| Box::new(v) as Box<dyn @{ pascal_name }@@% if def.use_cache() %@Cache@% endif %@>))
            }
            fn visibility_filter(mut self: Box<Self>, filter: Filter_) -> Box<dyn _QueryFindBuilder> { self.2 = Some(filter); self }
            @%- if def.is_soft_delete() %@
            fn with_trashed(mut self: Box<Self>, mode: bool) -> Box<dyn _QueryFindBuilder> { self.1 = mode; self }
            @%- endif %@
            fn join(self: Box<Self>, _join: Option<Box<Joiner_>>) -> Box<dyn _QueryFindBuilder> { self }
        }
        let map = self.0.lock().unwrap();
        Box::new(V(map.get(&id).cloned(), false, None))
    }
    @%- else %@
    fn find(&self, id: @{ def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") }@) -> Box<dyn @{ pascal_name }@QueryFindDirectlyBuilder> {
        self.find_directly(id)
    }
    @%- endif %@
    fn find_directly(&self, id: @{ def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") }@) -> Box<dyn @{ pascal_name }@QueryFindDirectlyBuilder> {
        struct V(Option<@{ pascal_name }@Entity>, Option<Filter_>);
        #[async_trait]
        impl @{ pascal_name }@QueryFindDirectlyBuilder for V {
            async fn query(self: Box<Self>) -> anyhow::Result<Option<Box<dyn @{ pascal_name }@>>> {
                let filter = self.1;
                Ok(self.0.filter(|v| filter.map(|f| f.check(v as &dyn @{ pascal_name }@)).unwrap_or(true)).map(|v| Box::new(v) as Box<dyn @{ pascal_name }@>))
            }
            fn visibility_filter(mut self: Box<Self>, filter: Filter_) -> Box<dyn _QueryFindDirectlyBuilder> { self.1 = Some(filter); self }
            @%- if def.is_soft_delete() %@
            fn with_trashed(self: Box<Self>, _mode: bool) -> Box<dyn _QueryFindDirectlyBuilder> { self }
            @%- endif %@
            fn join(self: Box<Self>, _join: Option<Box<Joiner_>>) -> Box<dyn _QueryFindDirectlyBuilder> { self }
        }
        let map = self.0.lock().unwrap();
        Box::new(V(map.get(&id).cloned(), None))
    }
}
@{-"\n"}@