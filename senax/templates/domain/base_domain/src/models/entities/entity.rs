#[allow(unused_imports)]
use crate as domain;
#[allow(unused_imports)]
use crate::models::{self, ToGeoPoint as _, ToPoint as _};
#[allow(unused_imports)]
use crate::value_objects;

#[allow(unused_imports)]
use crate::models::@{ db|snake|to_var_name }@ as _model_;
@%- for (name, rel_def) in def.belongs_to_outer_db() %@
pub use crate::models::@{ rel_def.db()|to_var_name }@ as _@{ rel_def.db() }@_model_;
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
#[schema(as = @{ db|pascal }@@{ group_name|pascal }@@{ id_name }@)]
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
impl From<&dyn @{ pascal_name }@Updater> for async_graphql::ID {
    fn from(obj: &dyn @{ pascal_name }@Updater) -> Self {
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
#[derive(async_graphql::Enum, serde::Serialize, serde::Deserialize, Hash, PartialEq, Eq, Clone, Copy, Debug, Default, strum::Display, strum::EnumMessage, strum::EnumString, strum::IntoStaticStr, schemars::JsonSchema)]
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

pub trait @{ pascal_name }@Common: std::fmt::Debug@% for parent in def.parent() %@ + super::super::@{ parent.group_name|to_var_name }@::@{ parent.name|to_var_name }@::@{ parent.name|pascal }@Common@% endfor %@ + 'static {
@{- def.primaries()|fmt_join("
{label}{comment}    fn {var}(&self) -> {outer};", "") }@
@{- def.only_version()|fmt_join("
{label}{comment}    fn {var}(&self) -> {outer};", "") }@
@{- def.cache_cols_wo_primaries_and_invisibles()|fmt_join("
{label}{comment}    fn {var}(&self) -> {domain_outer};", "") }@
}

@{ def.label|label0 -}@
@{ def.comment|comment0 -}@
pub trait @{ pascal_name }@Cache: @{ pascal_name }@Common + dyn_clone::DynClone + Send + Sync@% for parent in def.parent() %@ + super::super::@{ parent.group_name|to_var_name }@::@{ parent.name|to_var_name }@::@{ parent.name|pascal }@Cache@% endfor %@ + 'static {
@{- def.relations_one_cache(true)|fmt_rel_join("
{label}{comment}    fn {rel_name}(&self) -> anyhow::Result<Option<Box<dyn _model_::{class_mod_var}::{class}Cache>>>;", "") }@
@{- def.relations_one_uncached(true)|fmt_rel_join("
{label}{comment}    fn {rel_name}(&self) -> anyhow::Result<Option<Box<dyn _model_::{class_mod_var}::{class}>>>;", "") }@
@{- def.relations_many_cache(true)|fmt_rel_join("
{label}{comment}    fn {rel_name}(&self) -> anyhow::Result<Vec<Box<dyn _model_::{class_mod_var}::{class}Cache>>>;", "") }@
@{- def.relations_many_uncached(true)|fmt_rel_join("
{label}{comment}    fn {rel_name}(&self) -> anyhow::Result<Vec<Box<dyn _model_::{class_mod_var}::{class}>>>;", "") }@
@{- def.relations_belonging(true)|fmt_rel_join("
    fn _{raw_rel_name}_id(&self) -> Option<_model_::{class_mod_var}::{class}Primary> {
        Some({local_keys}.into())
    }", "") }@
@{- def.relations_belonging_outer_db(true)|fmt_rel_outer_db_join("
    fn _{raw_rel_name}_id(&self) -> Option<_{raw_db}_model_::{class_mod_var}::{class}Primary> {
        Some({local_keys}.into())
    }", "") }@
@{- def.relations_belonging_cache(true)|fmt_rel_join("
{label}{comment}    fn {rel_name}(&self) -> anyhow::Result<Option<Box<dyn _model_::{class_mod_var}::{class}Cache>>>;", "") }@
@{- def.relations_belonging_uncached(true)|fmt_rel_join("
{label}{comment}    fn {rel_name}(&self) -> anyhow::Result<Option<Box<dyn _model_::{class_mod_var}::{class}>>>;", "") }@
@{- def.relations_belonging_outer_db(true)|fmt_rel_outer_db_join("
{label}{comment}    fn {rel_name}(&self) -> anyhow::Result<Option<Box<dyn _{raw_db}_model_::{class_mod_var}::{class}>>>;", "") }@
}

@{ def.label|label0 -}@
@{ def.comment|comment0 -}@
pub trait @{ pascal_name }@: @{ pascal_name }@Common + Send + Sync@% for parent in def.parent() %@ + super::super::@{ parent.group_name|to_var_name }@::@{ parent.name|to_var_name }@::@{ parent.name|pascal }@@% endfor %@ + 'static {
@{- def.non_cache_cols_wo_primaries_and_invisibles()|fmt_join("
{label}{comment}    fn {var}(&self) -> {domain_outer};", "") }@
@{- def.relations_belonging(true)|fmt_rel_join("
    fn _{raw_rel_name}_id(&self) -> Option<_model_::{class_mod_var}::{class}Primary> {
        Some({local_keys}.into())
    }", "") }@
@{- def.relations_belonging_outer_db(true)|fmt_rel_outer_db_join("
    fn _{raw_rel_name}_id(&self) -> Option<_{raw_db}_model_::{class_mod_var}::{class}Primary> {
        Some({local_keys}.into())
    }", "") }@
@{- def.relations_one_and_belonging(true)|fmt_rel_join("
{label}{comment}    fn {rel_name}(&self) -> anyhow::Result<Option<&dyn _model_::{class_mod_var}::{class}>>;", "") }@
@{- def.relations_many(true)|fmt_rel_join("
{label}{comment}    fn {rel_name}(&self) -> anyhow::Result<Box<dyn Iterator<Item = &dyn _model_::{class_mod_var}::{class}> + '_>>;", "") }@
@{- def.relations_belonging_outer_db(true)|fmt_rel_outer_db_join("
{label}{comment}    fn {rel_name}(&self) -> anyhow::Result<Option<&dyn _{raw_db}_model_::{class_mod_var}::{class}>>;", "") }@
}

@{ def.label|label0 -}@
pub trait @{ pascal_name }@Updater: downcast_rs::Downcast + Send + Sync + @{ pascal_name }@Common + crate::models::MarkForDelete@% for parent in def.parent() %@ + super::super::@{ parent.group_name|to_var_name }@::@{ parent.name|to_var_name }@::@{ parent.name|pascal }@Updater@% endfor %@ + 'static {
@{- def.non_cache_cols_wo_primaries_and_invisibles()|fmt_join("
{label}{comment}    fn {var}(&self) -> {domain_outer};", "") }@
@{- def.non_primaries_wo_invisible_and_read_only(true)|fmt_join("
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
downcast_rs::impl_downcast!(@{ pascal_name }@Updater);

#[cfg(any(feature = "mock", test))]
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct @{ pascal_name }@Entity {
@{- def.primaries()|fmt_join("
    pub {var}: {domain_outer_owned},", "") }@
@{- def.non_primaries_wo_invisibles(false)|fmt_join("
    pub {var}: {domain_outer_owned},", "") }@
@{- def.relations_one_and_belonging(false)|fmt_rel_join("
    pub {rel_name}: Option<Box<_model_::{class_mod_var}::{class}Entity>>,", "") }@
@{- def.relations_many(false)|fmt_rel_join("
    pub {rel_name}: Vec<Box<_model_::{class_mod_var}::{class}Entity>>,", "") }@
@{- def.relations_belonging_outer_db(false)|fmt_rel_outer_db_join("
    pub {rel_name}: Option<Box<_{raw_db}_model_::{class_mod_var}::{class}Entity>>,", "") }@
    #[serde(skip)]
    pub _delete: bool,
}

@%- for parent in def.parents() %@

#[cfg(any(feature = "mock", test))]
#[allow(clippy::useless_conversion)]
impl super::super::@{ parent.group_name|to_var_name }@::@{ parent.name|to_var_name }@::@{ parent.name|pascal }@Common for @{ pascal_name }@Entity {
@{- parent.primaries()|fmt_join("
    fn _{raw_var}(&self) -> {inner} {
        self.{var}.0{clone}
    }", "") }@
@{- parent.only_version()|fmt_join("
    fn {var}(&self) -> {outer} {
        1
    }", "") }@
@{- parent.cache_cols_wo_primaries_and_invisibles()|fmt_join("
    fn {var}(&self) -> {domain_outer} {
        {convert_domain_outer_prefix}self.{var}{clone_for_outer}{convert_domain_outer}
    }", "") }@
}
#[cfg(any(feature = "mock", test))]
impl super::super::@{ parent.group_name|to_var_name }@::@{ parent.name|to_var_name }@::@{ parent.name|pascal }@Cache for @{ pascal_name }@Entity {
@{- parent.relations_one_cache(true)|fmt_rel_join("
    fn {rel_name}(&self) -> anyhow::Result<Option<Box<dyn _model_::{class_mod_var}::{class}Cache>>> {
        Ok(self.{rel_name}.as_ref().map(|v| Box::<dyn _model_::{class_mod_var}::{class}Cache>::from(v.clone())))
    }", "") }@
@{- parent.relations_many_cache(true)|fmt_rel_join("
    fn {rel_name}(&self) -> anyhow::Result<Vec<Box<dyn _model_::{class_mod_var}::{class}Cache>>> {
        Ok(self.{rel_name}.iter().map(|v| Box::<dyn _model_::{class_mod_var}::{class}Cache>::from(v.clone())).collect())
    }", "") }@
@{- parent.relations_belonging_cache(true)|fmt_rel_join("
    fn {rel_name}(&self) -> anyhow::Result<Option<Box<dyn _model_::{class_mod_var}::{class}Cache>>> {
        Ok(self.{rel_name}.as_ref().map(|v| Box::<dyn _model_::{class_mod_var}::{class}Cache>::from(v.clone())))
    }", "") }@
}
#[cfg(any(feature = "mock", test))]
#[allow(clippy::useless_conversion)]
impl super::super::@{ parent.group_name|to_var_name }@::@{ parent.name|to_var_name }@::@{ parent.name|pascal }@ for @{ pascal_name }@Entity {
@{- parent.non_cache_cols_wo_primaries_and_invisibles()|fmt_join("
    fn {var}(&self) -> {domain_outer} {
        {convert_domain_outer_prefix}self.{var}{clone_for_outer}{convert_domain_outer}
    }", "") }@
@{- parent.relations_one_and_belonging(true)|fmt_rel_join("
    fn {rel_name}(&self) -> anyhow::Result<Option<&dyn _model_::{class_mod_var}::{class}>> {
        Ok(self.{rel_name}.as_ref().map(|v| v.as_ref() as &dyn _model_::{class_mod_var}::{class}))
    }", "") }@
@{- parent.relations_many(true)|fmt_rel_join("
    fn {rel_name}(&self) -> anyhow::Result<Box<dyn Iterator<Item = &dyn _model_::{class_mod_var}::{class}> + '_>> {
        Ok(Box::new(self.{rel_name}.iter().map(|v| v.as_ref() as &dyn _model_::{class_mod_var}::{class})))
    }", "") }@
}
#[cfg(any(feature = "mock", test))]
#[allow(clippy::useless_conversion)]
impl super::super::@{ parent.group_name|to_var_name }@::@{ parent.name|to_var_name }@::@{ parent.name|pascal }@Updater for @{ pascal_name }@Entity {
@{- parent.non_cache_cols_wo_primaries_and_invisibles()|fmt_join("
    fn {var}(&self) -> {domain_outer} {
        {convert_domain_outer_prefix}self.{var}{clone_for_outer}{convert_domain_outer}
    }", "") }@
@{- parent.non_primaries_wo_invisible_and_read_only(true)|fmt_join("
    fn set_{raw_var}(&mut self, v: {domain_factory}) {
        self.{var} = v{convert_domain_factory}
    }", "") }@
@{- parent.relations_one(true)|fmt_rel_join("
    fn {rel_name}(&mut self) -> anyhow::Result<Option<&mut dyn _model_::{class_mod_var}::{class}Updater>> {
        Ok(self.{rel_name}.as_mut().map(|v| v.as_mut() as &mut dyn _model_::{class_mod_var}::{class}Updater))
    }
    fn set_{raw_rel_name}(&mut self, v: Box<dyn _model_::{class_mod_var}::{class}Updater>) {
        self.{rel_name} = if let Ok(v) = v.downcast::<_model_::{class_mod_var}::{class}Entity>() {
            Some(v)
        } else {
            panic!(\"Only {class}Entity is accepted.\");
        };
    }", "") }@
@{- parent.relations_many(true)|fmt_rel_join("
    fn {rel_name}(&mut self) -> anyhow::Result<Box<dyn domain::models::UpdateIterator<dyn _model_::{class_mod_var}::{class}Updater> + '_>> {
        struct V<'a, T>(&'a mut Vec<Box<T>>);
        impl<T: _model_::{class_mod_var}::{class}Updater> domain::models::UpdateIterator<dyn _model_::{class_mod_var}::{class}Updater> for V<'_, T> {
            fn iter_mut(&mut self) -> Box<dyn Iterator<Item = &mut (dyn _model_::{class_mod_var}::{class}Updater + 'static)> + '_> {
                Box::new(self.0.iter_mut().map(|v| v.as_mut() as &mut dyn _model_::{class_mod_var}::{class}Updater))
            }
        }
        Ok(Box::new(V(&mut self.{rel_name})))
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
@{- def.cache_cols_wo_primaries_and_invisibles()|fmt_join("
    fn {var}(&self) -> {domain_outer} {
        {convert_domain_outer_prefix}self.{var}{clone_for_outer}{convert_domain_outer}
    }", "") }@
}

#[cfg(any(feature = "mock", test))]
impl @{ pascal_name }@Cache for @{ pascal_name }@Entity {
@{- def.relations_one_cache(true)|fmt_rel_join("
    fn {rel_name}(&self) -> anyhow::Result<Option<Box<dyn _model_::{class_mod_var}::{class}Cache>>> {
        Ok(self.{rel_name}.as_ref().map(|v| Box::<dyn _model_::{class_mod_var}::{class}Cache>::from(v.clone())))
    }", "") }@
@{- def.relations_one_uncached(true)|fmt_rel_join("
    fn {rel_name}(&self) -> anyhow::Result<Option<Box<dyn _model_::{class_mod_var}::{class}>>> {
        Ok(self.{rel_name}.as_ref().map(|v| Box::<dyn _model_::{class_mod_var}::{class}>::from(v.clone())))
    }", "") }@
@{- def.relations_many_cache(true)|fmt_rel_join("
    fn {rel_name}(&self) -> anyhow::Result<Vec<Box<dyn _model_::{class_mod_var}::{class}Cache>>> {
        Ok(self.{rel_name}.iter().map(|v| Box::<dyn _model_::{class_mod_var}::{class}Cache>::from(v.clone())).collect())
    }", "") }@
@{- def.relations_many_uncached(true)|fmt_rel_join("
    fn {rel_name}(&self) -> anyhow::Result<Vec<Box<dyn _model_::{class_mod_var}::{class}>>> {
        Ok(self.{rel_name}.iter().map(|v| Box::<dyn _model_::{class_mod_var}::{class}>::from(v.clone())).collect())
    }", "") }@
@{- def.relations_belonging_cache(true)|fmt_rel_join("
    fn {rel_name}(&self) -> anyhow::Result<Option<Box<dyn _model_::{class_mod_var}::{class}Cache>>> {
        Ok(self.{rel_name}.as_ref().map(|v| Box::<dyn _model_::{class_mod_var}::{class}Cache>::from(v.clone())))
    }", "") }@
@{- def.relations_belonging_uncached(true)|fmt_rel_join("
    fn {rel_name}(&self) -> anyhow::Result<Option<Box<dyn _model_::{class_mod_var}::{class}>>> {
        Ok(self.{rel_name}.as_ref().map(|v| Box::<dyn _model_::{class_mod_var}::{class}>::from(v.clone())))
    }", "") }@
@{- def.relations_belonging_outer_db(true)|fmt_rel_outer_db_join("
    fn {rel_name}(&self) -> anyhow::Result<Option<Box<dyn _{raw_db}_model_::{class_mod_var}::{class}>>> {
        Ok(self.{rel_name}.as_ref().map(|v| Box::<dyn _{raw_db}_model_::{class_mod_var}::{class}>::from(v.clone())))
    }", "") }@
}

#[cfg(any(feature = "mock", test))]
#[allow(clippy::useless_conversion)]
impl @{ pascal_name }@ for @{ pascal_name }@Entity {
@{- def.non_cache_cols_wo_primaries_and_invisibles()|fmt_join("
    fn {var}(&self) -> {domain_outer} {
        {convert_domain_outer_prefix}self.{var}{clone_for_outer}{convert_domain_outer}
    }", "") }@
@{- def.relations_one_and_belonging(true)|fmt_rel_join("
    fn {rel_name}(&self) -> anyhow::Result<Option<&dyn _model_::{class_mod_var}::{class}>> {
        Ok(self.{rel_name}.as_ref().map(|v| v.as_ref() as &dyn _model_::{class_mod_var}::{class}))
    }", "") }@
@{- def.relations_many(true)|fmt_rel_join("
    fn {rel_name}(&self) -> anyhow::Result<Box<dyn Iterator<Item = &dyn _model_::{class_mod_var}::{class}> + '_>> {
        Ok(Box::new(self.{rel_name}.iter().map(|v| v.as_ref() as &dyn _model_::{class_mod_var}::{class})))
    }", "") }@
@{- def.relations_belonging_outer_db(true)|fmt_rel_outer_db_join("
    fn {rel_name}(&self) -> anyhow::Result<Option<&dyn _{raw_db}_model_::{class_mod_var}::{class}>> {
        Ok(self.{rel_name}.as_ref().map(|v| v.as_ref() as &dyn _{raw_db}_model_::{class_mod_var}::{class}))
    }", "") }@
}

#[cfg(any(feature = "mock", test))]
#[allow(clippy::useless_conversion)]
impl @{ pascal_name }@Updater for @{ pascal_name }@Entity {
@{- def.non_cache_cols_wo_primaries_and_invisibles()|fmt_join("
    fn {var}(&self) -> {domain_outer} {
        {convert_domain_outer_prefix}self.{var}{clone_for_outer}{convert_domain_outer}
    }", "") }@
@{- def.non_primaries_wo_invisible_and_read_only(true)|fmt_join("
    fn set_{raw_var}(&mut self, v: {domain_factory}) {
        self.{var} = v{convert_domain_factory}
    }", "") }@
@{- def.relations_one(true)|fmt_rel_join("
    fn {rel_name}(&mut self) -> anyhow::Result<Option<&mut dyn _model_::{class_mod_var}::{class}Updater>> {
        Ok(self.{rel_name}.as_mut().map(|v| v.as_mut() as &mut dyn _model_::{class_mod_var}::{class}Updater))
    }
    fn set_{raw_rel_name}(&mut self, v: Box<dyn _model_::{class_mod_var}::{class}Updater>) {
        self.{rel_name} = if let Ok(v) = v.downcast::<_model_::{class_mod_var}::{class}Entity>() {
            Some(v)
        } else {
            panic!(\"Only {class}Entity is accepted.\");
        };
    }", "") }@
@{- def.relations_many(true)|fmt_rel_join("
    fn {rel_name}(&mut self) -> anyhow::Result<Box<dyn domain::models::UpdateIterator<dyn _model_::{class_mod_var}::{class}Updater> + '_>> {
        struct V<'a, T>(&'a mut Vec<Box<T>>);
        impl<T: _model_::{class_mod_var}::{class}Updater> domain::models::UpdateIterator<dyn _model_::{class_mod_var}::{class}Updater> for V<'_, T> {
            fn iter_mut(&mut self) -> Box<dyn Iterator<Item = &mut (dyn _model_::{class_mod_var}::{class}Updater + 'static)> + '_> {
                Box::new(self.0.iter_mut().map(|v| v.as_mut() as &mut dyn _model_::{class_mod_var}::{class}Updater))
            }
        }
        Ok(Box::new(V(&mut self.{rel_name})))
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
@{-"\n"}@
