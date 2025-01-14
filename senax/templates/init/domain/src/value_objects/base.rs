@% if def.enum_values.is_some() && def.is_integer() -%@
@% let values = def.enum_values.as_ref().unwrap() -%@
#[derive(async_graphql::Enum, serde_repr::Serialize_repr, serde_repr::Deserialize_repr, Hash, PartialEq, Eq, Clone, Copy, Debug, Default, strum::Display, strum::EnumMessage, strum::EnumString, strum::IntoStaticStr, strum::FromRepr, schemars::JsonSchema)]
#[repr(@{ def.get_inner_type(true, true) }@)]
#[allow(non_camel_case_types)]
#[allow(clippy::upper_case_acronyms)]
#[graphql(name="Vo@{ pascal_name }@")]
#[derive(utoipa::ToSchema)]
#[schema(as = Vo@{ pascal_name }@)]
pub enum @{ pascal_name }@ {
@% for row in values -%@@{ row.label|label4 }@@{ row.comment|comment4 }@@{ row.label|strum_message4 }@@{ row.comment|strum_detailed4 }@    @% if loop.first %@#[default]@% endif %@@{ row.name }@@{ row.value_str() }@,
@% endfor -%@
}

impl @{ pascal_name }@ {
    pub fn inner(&self) -> @{ def.get_inner_type(true, true) }@ {
        *self as @{ def.get_inner_type(true, true) }@
    }
}
impl From<@{ def.get_inner_type(true, true) }@> for @{ pascal_name }@ {
    fn from(val: @{ def.get_inner_type(true, true) }@) -> Self {
        if let Some(val) = Self::from_repr(val) {
            val
        } else {
            panic!("{} is a value outside the range of @{ pascal_name }@.", val)
        }
    }
}
impl From<@{ pascal_name }@> for @{ def.get_inner_type(true, true) }@ {
    fn from(val: @{ pascal_name }@) -> Self {
        val.inner()
    }
}

@% else if def.enum_values.is_some() -%@
@% let values = def.enum_values.as_ref().unwrap() -%@
#[derive(async_graphql::Enum, serde::Serialize, serde::Deserialize, Hash, PartialEq, Eq, Clone, Copy, Debug, Default, strum::Display, strum::EnumMessage, strum::EnumString, strum::IntoStaticStr, schemars::JsonSchema)]
#[allow(non_camel_case_types)]
#[allow(clippy::upper_case_acronyms)]
#[graphql(name="Vo@{ pascal_name }@")]
#[derive(utoipa::ToSchema)]
#[schema(as = Vo@{ pascal_name }@)]
pub enum @{ pascal_name }@ {
@% for row in values -%@@{ row.label|label4 }@@{ row.comment|comment4 }@@{ row.label|strum_message4 }@@{ row.comment|strum_detailed4 }@    @% if loop.first %@#[default]@% endif %@@{ row.name }@,
@% endfor -%@
}
impl @{ pascal_name }@ {
    pub fn as_static_str(&self) -> &'static str {
        Into::<&'static str>::into(self)
    }
}

@% else -%@
#[derive(serde::Deserialize, serde::Serialize, Default,@% if def.is_hashable() %@ Hash,@% endif %@ PartialEq, PartialOrd,@% if def.is_equivalence() %@ Eq, Ord,@% endif %@ Clone,@% if def.is_copyable() %@ Copy,@% endif %@ derive_more::Display, derive_more::From, derive_more::Into, Debug, schemars::JsonSchema)]
#[serde(transparent)]
#[derive(utoipa::ToSchema)]
#[schema(as = Vo@{ pascal_name }@)]
pub struct @{ pascal_name }@(@{ def.get_inner_type(false, true) }@);
async_graphql::scalar!(@{ pascal_name }@, "Vo@{ pascal_name }@");
impl @{ pascal_name }@ {
    #[allow(clippy::clone_on_copy)]
    pub fn inner(&self) -> @{ def.get_inner_type(false, true) }@ {
        self.0.clone()
    }
}
@%- if def.get_inner_type(false, true) != def.get_inner_type(true, true) %@
impl From<@{ def.get_inner_type(true, true) }@> for @{ pascal_name }@ {
    fn from(v: @{ def.get_inner_type(true, true) }@) -> Self {
        Self(v.into())
    }
}
impl std::ops::Deref for @{ pascal_name }@ {
    type Target = @{ def.get_inner_type(true, true) }@;
    fn deref(&self) -> &@{ def.get_inner_type(true, true) }@ {
        self.0.as_ref()
    }
}
@%- else %@
impl std::ops::Deref for @{ pascal_name }@ {
    type Target = @{ def.get_inner_type(false, true) }@;
    fn deref(&self) -> &@{ def.get_inner_type(false, true) }@ {
        &self.0
    }
}
@%- endif %@
@%- if def.get_inner_type(true, true) == "String" %@
impl From<&str> for @{ pascal_name }@ {
    fn from(v: &str) -> Self {
        Self(v.to_string().into())
    }
}
@%- endif %@
@%- endif %@
@{-"\n"}@