#[allow(unused_imports)]
use domain::models::FromRawValue as _;
#[allow(unused_imports)]
use domain::models::@{ db|snake|to_var_name }@::@{ rel_mod }@ as _domain_;
#[allow(unused_imports)]
use domain::value_objects;
#[allow(unused_imports)]
use senax_common::types::blob::BlobToApi as _;

@{ def.label|label0 -}@
#[derive(async_graphql::SimpleObject, serde::Serialize)]
#[graphql(name = "@{ graphql_name }@")]
#[derive(utoipa::ToSchema)]
#[schema(as = @{ graphql_name }@)]
pub struct ResObj@{ rel_name|pascal }@ {
    #[graphql(name = "_id")]
    #[schema(value_type = String)]
    pub _id: async_graphql::ID,
@%- if camel_case %@
@{- def.for_api_response()|fmt_join("
{label_wo_hash}{res_api_schema_type}    pub {var}: {res_api_type},", "") }@
@{- def.relations_one_for_api_response()|fmt_rel_join("
{label_wo_hash}    pub {rel_name}: Option<_{raw_rel_name}::ResObj{rel_name_pascal}>,", "") }@
@{- def.relations_many_for_api_response()|fmt_rel_join("
{label_wo_hash}    pub {rel_name}: Vec<_{raw_rel_name}::ResObj{rel_name_pascal}>,", "") }@
@{- def.relations_belonging_for_api_response()|fmt_rel_join("
    #[graphql(name = \"_{raw_rel_name}_id\")]
    #[schema(value_type = Option<String>)]
    pub _{raw_rel_name}_id: Option<async_graphql::ID>,
{label_wo_hash}    pub {rel_name}: Option<_{raw_rel_name}::ResObj{rel_name_pascal}>,", "") }@
@%- else %@
@{- def.for_api_response()|fmt_join("
{label_wo_hash}{res_api_schema_type}    #[graphql(name = \"{raw_var}\")]
    pub {var}: {res_api_type},", "") }@
@{- def.relations_one_for_api_response()|fmt_rel_join("
{label_wo_hash}    #[graphql(name = \"{raw_rel_name}\")]
    pub {rel_name}: Option<_{raw_rel_name}::ResObj{rel_name_pascal}>,", "") }@
@{- def.relations_many_for_api_response()|fmt_rel_join("
{label_wo_hash}    #[graphql(name = \"{raw_rel_name}\")]
    pub {rel_name}: Vec<_{raw_rel_name}::ResObj{rel_name_pascal}>,", "") }@
@{- def.relations_belonging_for_api_response()|fmt_rel_join("
    #[graphql(name = \"_{raw_rel_name}_id\")]
    #[schema(value_type = Option<String>)]
    pub _{raw_rel_name}_id: Option<async_graphql::ID>,
{label_wo_hash}    #[graphql(name = \"{raw_rel_name}\")]
    pub {rel_name}: Option<_{raw_rel_name}::ResObj{rel_name_pascal}>,", "") }@
@%- endif %@
}

impl From<&dyn _domain_::@{ pascal_name }@> for ResObj@{ rel_name|pascal }@ {
    fn from(v: &dyn _domain_::@{ pascal_name }@) -> Self {
        Self {
            _id: v.into(),
            @{- def.for_api_response()|fmt_join("
            {var}: v.{var}(){to_res_api_type},", "") }@
            @{- def.relations_one_for_api_response()|fmt_rel_join("
            {rel_name}: v.{rel_name}().unwrap_or_default().map(|v| v.into()),", "") }@
            @{- def.relations_many_for_api_response()|fmt_rel_join("
            {rel_name}: v.{rel_name}().map(|l| l.map(|v| v.into()).collect()).unwrap_or_default(),", "") }@
            @{- def.relations_belonging_for_api_response()|fmt_rel_join("
            _{raw_rel_name}_id: v._{raw_rel_name}_id().map(|v| v.into()),
            {rel_name}: v.{rel_name}().unwrap_or_default().map(|v| v.into()),", "") }@
        }
    }
}

impl From<&dyn _domain_::@{ pascal_name }@Cache> for ResObj@{ rel_name|pascal }@ {
    fn from(v: &dyn _domain_::@{ pascal_name }@Cache) -> Self {
        Self {
            _id: v.into(),
            @{- def.for_api_response()|fmt_join("
            {var}: v.{var}(){to_res_api_type},", "") }@
            @{- def.relations_one_for_api_response()|fmt_rel_join("
            {rel_name}: v.{rel_name}().unwrap_or_default().map(|v| (&*v).into()),", "") }@
            @{- def.relations_many_for_api_response()|fmt_rel_join("
            {rel_name}: v.{rel_name}().map(|l| l.iter().map(|v| (&**v).into()).collect()).unwrap_or_default(),", "") }@
            @{- def.relations_belonging_for_api_response()|fmt_rel_join("
            _{raw_rel_name}_id: v._{raw_rel_name}_id().map(|v| v.into()),
            {rel_name}: v.{rel_name}().unwrap_or_default().map(|v| (&*v).into()),", "") }@
        }
    }
}

#[allow(unused_mut)]
#[allow(clippy::needless_update)]
pub fn joiner(_look_ahead: async_graphql::Lookahead<'_>) -> Option<Box<_domain_::Joiner_>> {
    if !_look_ahead.exists() {
        return None;
    }
    let joiner = _domain_::Joiner_ {
        @%- if camel_case %@
        @{- def.relations_one_for_api_response()|fmt_rel_join("
        {rel_name}: _{raw_rel_name}::joiner(_look_ahead.field(\"{rel_name_camel}\")),", "") }@
        @{- def.relations_many_for_api_response()|fmt_rel_join("
        {rel_name}: _{raw_rel_name}::joiner(_look_ahead.field(\"{rel_name_camel}\")),", "") }@
        @{- def.relations_belonging_for_api_response()|fmt_rel_join("
        {rel_name}: _{raw_rel_name}::joiner(_look_ahead.field(\"{rel_name_camel}\")),", "") }@
        @%- else %@
        @{- def.relations_one_for_api_response()|fmt_rel_join("
        {rel_name}: _{raw_rel_name}::joiner(_look_ahead.field(\"{raw_rel_name}\")),", "") }@
        @{- def.relations_many_for_api_response()|fmt_rel_join("
        {rel_name}: _{raw_rel_name}::joiner(_look_ahead.field(\"{raw_rel_name}\")),", "") }@
        @{- def.relations_belonging_for_api_response()|fmt_rel_join("
        {rel_name}: _{raw_rel_name}::joiner(_look_ahead.field(\"{raw_rel_name}\")),", "") }@
        @%- endif %@
        ..Default::default()
    };
    Some(Box::new(joiner))
}

#[allow(unused_mut)]
#[allow(dead_code)]
#[allow(clippy::needless_update)]
pub fn reader_joiner() -> Option<Box<_domain_::Joiner_>> {
    let joiner = _domain_::Joiner_ {
        @{- def.relations_one_for_api_response()|fmt_rel_join("
        {rel_name}: _{raw_rel_name}::reader_joiner(),", "") }@
        @{- def.relations_many_for_api_response()|fmt_rel_join("
        {rel_name}: _{raw_rel_name}::reader_joiner(),", "") }@
        @{- def.relations_belonging_for_api_response()|fmt_rel_join("
        {rel_name}: _{raw_rel_name}::reader_joiner(),", "") }@
        ..Default::default()
    };
    Some(Box::new(joiner))
}
