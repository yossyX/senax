#[allow(unused_imports)]
use crate as domain;
#[allow(unused_imports)]
use crate::models::{self, ToGeoPoint as _, ToPoint as _};
#[allow(unused_imports)]
use crate::value_objects;

#[allow(unused_imports)]
use crate::models::@{ db|snake|ident }@ as _model_;
@%- for (name, rel_def) in def.belongs_to_outer_db() %@
pub use crate::models::@{ rel_def.db()|snake|ident }@ as _@{ rel_def.db()|snake }@_model_;
@%- endfor %@

pub const MODEL_ID: u64 = @{ model_id }@;

pub mod consts {
@{- def.all_fields()|fmt_join("{api_validate_const}", "") }@
}

@% for (name, column_def) in def.id() -%@
#[derive(serde::Deserialize, serde::Serialize, Hash, PartialEq, Eq, PartialOrd, Ord, Clone,@% if column_def.is_copyable() %@ Copy,@% endif %@ derive_more::Display, Debug, Default)]
#[serde(transparent)]
@%- if !column_def.is_displayable() %@
#[display("{:?}", _0)]
@%- endif %@
#[derive(utoipa::ToSchema)]
#[schema(as = @{ config.layer_name(db, group_name) }@@{ id_name }@)]
pub struct @{ id_name }@(@{ column_def.get_inner_type(false, false) }@);
async_graphql::scalar!(@{ id_name }@, "@{ config.layer_name(db, group_name) }@@{ id_name }@");

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

impl<@{ def.primaries()|fmt_join("T{index}: Into<{domain_outer_owned}>", ", ") }@> From<@{ def.primaries()|fmt_join_with_paren("T{index}", ", ") }@> for @{ pascal_name }@Primary {
    fn from(id: @{ def.primaries()|fmt_join_with_paren("T{index}", ", ") }@) -> Self {
        @% if def.primaries().len() == 1 %@Self(id.into())@% else %@Self(@{ def.primaries()|fmt_join("id.{index}.into()", ", ") }@)@% endif %@
    }
}

impl @{ pascal_name }@Primary {
    pub fn into(self) -> @{ def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") }@ {
        @{ def.primaries()|fmt_join_with_paren("self.{index}", ", ") }@
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
            model_id == MODEL_ID,
            "{} is not an ID of the @{ pascal_name }@ model.",
            v.as_str()
        );
        Ok(@% if def.primaries().len() == 1 %@Self(id.into())@% else %@Self(@{ def.primaries()|fmt_join("id.{index}.into()", ", ") }@)@% endif %@)
    }
}

fn to_graphql_id(id: @{ def.primaries()|fmt_join_with_paren("{inner}", ", ") }@) -> async_graphql::ID {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    let v: (u64, @{ def.primaries()|fmt_join_with_paren("{inner}", ", ") }@) = (MODEL_ID, id);
    let mut buf = Vec::new();
    ciborium::into_writer(&v, &mut buf).unwrap();
    URL_SAFE_NO_PAD.encode(buf).into()
}
#[allow(clippy::useless_conversion)]
impl From<&dyn @{ pascal_name }@> for async_graphql::ID {
    fn from(obj: &dyn @{ pascal_name }@) -> Self {
        to_graphql_id(@{ def.primaries()|fmt_join_with_paren("obj.{ident}().to_owned().into()", ", ") }@)
    }
}
#[allow(clippy::useless_conversion)]
impl From<&dyn @{ pascal_name }@Updater> for async_graphql::ID {
    fn from(obj: &dyn @{ pascal_name }@Updater) -> Self {
        to_graphql_id(@{ def.primaries()|fmt_join_with_paren("obj.{ident}().to_owned().into()", ", ") }@)
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
#[derive(async_graphql::Enum, serde::Serialize, serde::Deserialize, Hash, PartialEq, Eq, Clone, Copy, Debug, Default, strum::Display, strum::EnumMessage, strum::EnumString, strum::IntoStaticStr, schemars::JsonSchema)]
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

pub trait @{ pascal_name }@: std::fmt::Debug + crate::models::FilterFlag + dyn_clone::DynClone + Send + Sync@% for parent in def.parent() %@ + super::super::@{ parent.group_name|ident }@::@{ parent.name|ident }@::@{ parent.name|pascal }@@% endfor %@ + 'static {
@{- def.primaries()|fmt_join("
{label}{comment}    fn {ident}(&self) -> {outer};", "") }@
@{- def.only_version()|fmt_join("
{label}{comment}    fn {ident}(&self) -> {outer};", "") }@
@{- def.cols_except_primaries_and_invisibles()|fmt_join("
{label}{comment}    fn {ident}(&self) -> {domain_outer};", "") }@
@{- def.relations_belonging(true)|fmt_rel_join("
    fn _{raw_rel_name}_id(&self) -> Option<_model_::{class_mod_path}::{class}Primary> {
        Some({local_keys}.into())
    }", "") }@
@{- def.relations_belonging_outer_db(true)|fmt_rel_outer_db_join("
    fn _{raw_rel_name}_id(&self) -> Option<_{db_snake}_model_::{class_mod_path}::{class}Primary> {
        Some({local_keys}.into())
    }", "") }@
@{- def.relations_one_and_belonging(true)|fmt_rel_join("
{label}{comment}    fn {rel_name}(&self) -> anyhow::Result<Option<&dyn _model_::{class_mod_path}::{class}>>;", "") }@
@{- def.relations_many(true)|fmt_rel_join("
{label}{comment}    fn {rel_name}(&self) -> anyhow::Result<Box<dyn Iterator<Item = &dyn _model_::{class_mod_path}::{class}> + '_>>;", "") }@
@{- def.relations_belonging_outer_db(true)|fmt_rel_outer_db_join("
{label}{comment}    fn {rel_name}(&self) -> anyhow::Result<Option<&dyn _{db_snake}_model_::{class_mod_path}::{class}>>;", "") }@
}

dyn_clone::clone_trait_object!(@{ pascal_name }@);

@{ def.label|label0 -}@
pub trait @{ pascal_name }@Updater: std::any::Any + Send + Sync + @{ pascal_name }@ + crate::models::MarkForDelete@% for parent in def.parent() %@ + super::super::@{ parent.group_name|ident }@::@{ parent.name|ident }@::@{ parent.name|pascal }@Updater@% endfor %@ + 'static {
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

#[cfg(any(feature = "mock", test))]
#[allow(unused_imports)]
use crate::models::ToRawValue as _;

#[cfg(any(feature = "mock", test))]
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct @{ pascal_name }@Entity {
@{- def.primaries()|fmt_join("
    pub {ident}: {domain_outer_owned},", "") }@
@{- def.non_primaries_except_invisibles(false)|fmt_join("
    pub {ident}: {domain_outer_owned},", "") }@
@{- def.relations_one_and_belonging(false)|fmt_rel_join("
    pub {rel_name}: Option<Box<_model_::{class_mod_path}::{class}Entity>>,", "") }@
@{- def.relations_many(false)|fmt_rel_join("
    pub {rel_name}: Vec<Box<_model_::{class_mod_path}::{class}Entity>>,", "") }@
@{- def.relations_belonging_outer_db(false)|fmt_rel_outer_db_join("
    pub {rel_name}: Option<Box<_{db_snake}_model_::{class_mod_path}::{class}Entity>>,", "") }@
    #[serde(skip)]
    pub _delete: bool,
    #[serde(skip)]
    pub _filter_flag: std::collections::BTreeMap<&'static str, bool>,
}

@%- for parent in def.parents() %@

#[cfg(any(feature = "mock", test))]
#[allow(clippy::useless_conversion)]
impl super::super::@{ parent.group_name|ident }@::@{ parent.name|ident }@::@{ parent.name|pascal }@ for @{ pascal_name }@Entity {
@{- parent.primaries()|fmt_join("
    fn _{raw_name}(&self) -> {inner} {
        self.{ident}.0{clone}
    }", "") }@
@{- parent.only_version()|fmt_join("
    fn {ident}(&self) -> {outer} {
        1
    }", "") }@
@{- parent.cols_except_primaries_and_invisibles()|fmt_join("
    fn {ident}(&self) -> {domain_outer} {
        {convert_domain_outer_prefix}self.{ident}{clone_for_outer}{convert_domain_outer}
    }", "") }@
@{- parent.relations_one_and_belonging(true)|fmt_rel_join("
    fn {rel_name}(&self) -> anyhow::Result<Option<&dyn _model_::{class_mod_path}::{class}>> {
        Ok(self.{rel_name}.as_ref().map(|v| v.as_ref() as &dyn _model_::{class_mod_path}::{class}))
    }", "") }@
@{- parent.relations_many(true)|fmt_rel_join("
    fn {rel_name}(&self) -> anyhow::Result<Box<dyn Iterator<Item = &dyn _model_::{class_mod_path}::{class}> + '_>> {
        Ok(Box::new(self.{rel_name}.iter().map(|v| v.as_ref() as &dyn _model_::{class_mod_path}::{class})))
    }", "") }@
}
#[cfg(any(feature = "mock", test))]
#[allow(clippy::useless_conversion)]
impl super::super::@{ parent.group_name|ident }@::@{ parent.name|ident }@::@{ parent.name|pascal }@Updater for @{ pascal_name }@Entity {
@{- parent.non_primaries_except_invisible_and_read_only(true)|fmt_join("
    fn set_{raw_name}(&mut self, v: {domain_factory}) {
        self.{ident} = v{convert_domain_factory}
    }", "") }@
@{- parent.relations_one(true)|fmt_rel_join("
    fn {rel_name}(&mut self) -> anyhow::Result<Option<&mut dyn _model_::{class_mod_path}::{class}Updater>> {
        Ok(self.{rel_name}.as_mut().map(|v| v.as_mut() as &mut dyn _model_::{class_mod_path}::{class}Updater))
    }
    fn set_{raw_rel_name}(&mut self, v: Box<dyn _model_::{class_mod_path}::{class}Updater>) {
        self.{rel_name} = if let Ok(v) = (v as Box<dyn std::any::Any>).downcast::<_model_::{class_mod_path}::{class}Entity>() {
            Some(v)
        } else {
            panic!(\"Only {class}Entity is accepted.\");
        };
    }", "") }@
@{- parent.relations_many(true)|fmt_rel_join("
    fn {rel_name}(&mut self) -> anyhow::Result<Box<dyn domain::models::UpdateIterator<dyn _model_::{class_mod_path}::{class}Updater> + '_>> {
        struct V<'a, T>(&'a mut Vec<Box<T>>);
        impl<T: _model_::{class_mod_path}::{class}Updater> domain::models::UpdateIterator<dyn _model_::{class_mod_path}::{class}Updater> for V<'_, T> {
            fn iter_mut(&mut self) -> Box<dyn Iterator<Item = &mut (dyn _model_::{class_mod_path}::{class}Updater + 'static)> + '_> {
                Box::new(self.0.iter_mut().map(|v| v.as_mut() as &mut dyn _model_::{class_mod_path}::{class}Updater))
            }
        }
        Ok(Box::new(V(&mut self.{rel_name})))
    }
    fn take_{raw_rel_name}(&mut self) -> Option<Vec<Box<dyn _model_::{class_mod_path}::{class}Updater>>> {
        Some(self.{rel_name}.drain(..).map(|v| v as Box<dyn _model_::{class_mod_path}::{class}Updater>).collect())
    }
    fn replace_{raw_rel_name}(&mut self, list: Vec<Box<dyn _model_::{class_mod_path}::{class}Updater>>) {
        self.{rel_name}.clear();
        for row in list {
            self.push_{raw_rel_name}(row);
        }
    }
    fn push_{raw_rel_name}(&mut self, v: Box<dyn _model_::{class_mod_path}::{class}Updater>) {
        if let Ok(v) = (v as Box<dyn std::any::Any>).downcast::<_model_::{class_mod_path}::{class}Entity>() {
            self.{rel_name}.push(v)
        } else {
            panic!(\"Only {class}Entity is accepted.\");
        }
    }", "") }@
}
@%- endfor %@

#[cfg(any(feature = "mock", test))]
#[allow(clippy::useless_conversion)]
impl @{ pascal_name }@ for @{ pascal_name }@Entity {
@{- def.primaries()|fmt_join("
    fn {ident}(&self) -> {outer} {
        {convert_domain_outer_prefix}self.{ident}{clone_for_outer}{convert_domain_outer}
    }", "") }@
@{- def.only_version()|fmt_join("
    fn {ident}(&self) -> {outer} {
        1
    }", "") }@
@{- def.cols_except_primaries_and_invisibles()|fmt_join("
    fn {ident}(&self) -> {domain_outer} {
        {convert_domain_outer_prefix}self.{ident}{clone_for_outer}{convert_domain_outer}
    }", "") }@
@{- def.relations_one_and_belonging(true)|fmt_rel_join("
    fn {rel_name}(&self) -> anyhow::Result<Option<&dyn _model_::{class_mod_path}::{class}>> {
        Ok(self.{rel_name}.as_ref().map(|v| v.as_ref() as &dyn _model_::{class_mod_path}::{class}))
    }", "") }@
@{- def.relations_many(true)|fmt_rel_join("
    fn {rel_name}(&self) -> anyhow::Result<Box<dyn Iterator<Item = &dyn _model_::{class_mod_path}::{class}> + '_>> {
        Ok(Box::new(self.{rel_name}.iter().map(|v| v.as_ref() as &dyn _model_::{class_mod_path}::{class})))
    }", "") }@
@{- def.relations_belonging_outer_db(true)|fmt_rel_outer_db_join("
    fn {rel_name}(&self) -> anyhow::Result<Option<&dyn _{db_snake}_model_::{class_mod_path}::{class}>> {
        Ok(self.{rel_name}.as_ref().map(|v| v.as_ref() as &dyn _{db_snake}_model_::{class_mod_path}::{class}))
    }", "") }@
}
#[cfg(any(feature = "mock", test))]
impl crate::models::FilterFlag for @{ pascal_name }@Entity {
    fn get_flag(&self, name: &'static str) -> Option<bool> {
        self._filter_flag.get(name).copied()
    }
}

#[cfg(any(feature = "mock", test))]
#[allow(clippy::useless_conversion)]
impl @{ pascal_name }@Updater for @{ pascal_name }@Entity {
@{- def.non_primaries_except_invisible_and_read_only(true)|fmt_join("
    fn set_{raw_name}(&mut self, v: {domain_factory}) {
        self.{ident} = v{convert_domain_factory}
    }", "") }@
@{- def.relations_one(true)|fmt_rel_join("
    fn {rel_name}(&mut self) -> anyhow::Result<Option<&mut dyn _model_::{class_mod_path}::{class}Updater>> {
        Ok(self.{rel_name}.as_mut().map(|v| v.as_mut() as &mut dyn _model_::{class_mod_path}::{class}Updater))
    }
    fn set_{raw_rel_name}(&mut self, v: Box<dyn _model_::{class_mod_path}::{class}Updater>) {
        self.{rel_name} = if let Ok(v) = (v as Box<dyn std::any::Any>).downcast::<_model_::{class_mod_path}::{class}Entity>() {
            Some(v)
        } else {
            panic!(\"Only {class}Entity is accepted.\");
        };
    }", "") }@
@{- def.relations_many(true)|fmt_rel_join("
    fn {rel_name}(&mut self) -> anyhow::Result<Box<dyn domain::models::UpdateIterator<dyn _model_::{class_mod_path}::{class}Updater> + '_>> {
        struct V<'a, T>(&'a mut Vec<Box<T>>);
        impl<T: _model_::{class_mod_path}::{class}Updater> domain::models::UpdateIterator<dyn _model_::{class_mod_path}::{class}Updater> for V<'_, T> {
            fn iter_mut(&mut self) -> Box<dyn Iterator<Item = &mut (dyn _model_::{class_mod_path}::{class}Updater + 'static)> + '_> {
                Box::new(self.0.iter_mut().map(|v| v.as_mut() as &mut dyn _model_::{class_mod_path}::{class}Updater))
            }
        }
        Ok(Box::new(V(&mut self.{rel_name})))
    }
    fn take_{raw_rel_name}(&mut self) -> Option<Vec<Box<dyn _model_::{class_mod_path}::{class}Updater>>> {
        Some(self.{rel_name}.drain(..).map(|v| v as Box<dyn _model_::{class_mod_path}::{class}Updater>).collect())
    }
    fn replace_{raw_rel_name}(&mut self, list: Vec<Box<dyn _model_::{class_mod_path}::{class}Updater>>) {
        self.{rel_name}.clear();
        for row in list {
            self.push_{raw_rel_name}(row);
        }
    }
    fn push_{raw_rel_name}(&mut self, v: Box<dyn _model_::{class_mod_path}::{class}Updater>) {
        if let Ok(v) = (v as Box<dyn std::any::Any>).downcast::<_model_::{class_mod_path}::{class}Entity>() {
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
@{-"\n"}@
