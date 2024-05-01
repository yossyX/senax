#[allow(unused_imports)]
use domain::models::@{ db|snake|to_var_name }@::@{ rel_mod }@::{
    self as _domain_, @{ pascal_name }@Updater as _, @{ pascal_name }@UpdaterBase as _
};
#[allow(unused_imports)]
use domain::value_objects;
#[allow(unused_imports)]
use senax_common::types::blob::{ApiToBlob as _, BlobToApi as _};
#[allow(unused_imports)]
use std::collections::HashMap;
@%- if !no_read %@

@{ def.label|label0 -}@
#[derive(async_graphql::SimpleObject, serde::Serialize)]
#[graphql(name = "@{ graphql_name }@")]
pub struct ResObj@{ rel_name|pascal }@ {
    #[graphql(name = "_id")]
    pub _id: async_graphql::ID,
@%- if camel_case %@
@{- def.for_api_response_except(rel_id)|fmt_join("
{label_wo_hash}    pub {var}: {res_api_type},", "") }@
@{- def.relations_one_for_api_response()|fmt_rel_join("
{label_wo_hash}    pub {rel_name}: Option<_{raw_rel_name}::ResObj{rel_name_pascal}>,", "") }@
@{- def.relations_many_for_api_response()|fmt_rel_join("
{label_wo_hash}    pub {rel_name}: Vec<_{raw_rel_name}::ResObj{rel_name_pascal}>,", "") }@
@{- def.relations_belonging_for_api_response()|fmt_rel_join("
    #[graphql(name = \"_{raw_rel_name}_id\")]
    pub _{raw_rel_name}_id: Option<async_graphql::ID>,
{label_wo_hash}    pub {rel_name}: Option<_{raw_rel_name}::ResObj{rel_name_pascal}>,", "") }@
@%- else %@
@{- def.for_api_response_except(rel_id)|fmt_join("
{label_wo_hash}    #[graphql(name = \"{raw_var}\")]
    pub {var}: {res_api_type},", "") }@
@{- def.relations_one_for_api_response()|fmt_rel_join("
{label_wo_hash}    #[graphql(name = \"{raw_rel_name}\")]
    pub {rel_name}: Option<_{raw_rel_name}::ResObj{rel_name_pascal}>,", "") }@
@{- def.relations_many_for_api_response()|fmt_rel_join("
{label_wo_hash}    #[graphql(name = \"{raw_rel_name}\")]
    pub {rel_name}: Vec<_{raw_rel_name}::ResObj{rel_name_pascal}>,", "") }@
@{- def.relations_belonging_for_api_response()|fmt_rel_join("
    #[graphql(name = \"_{raw_rel_name}_id\")]
    pub _{raw_rel_name}_id: Option<async_graphql::ID>,
{label_wo_hash}    #[graphql(name = \"{raw_rel_name}\")]
    pub {rel_name}: Option<_{raw_rel_name}::ResObj{rel_name_pascal}>,", "") }@
@%- endif %@
}

impl From<&dyn _domain_::@{ pascal_name }@> for ResObj@{ rel_name|pascal }@ {
    fn from(v: &dyn _domain_::@{ pascal_name }@) -> Self {
        Self {
            _id: v.into(),
            @{- def.for_api_response_except(rel_id)|fmt_join("
            {var}: v.{var}(){to_res_api_type},", "") }@
            @{- def.relations_one_for_api_response()|fmt_rel_join("
            {rel_name}: v.{rel_name}().map(|v| v.into()),", "") }@
            @{- def.relations_many_for_api_response()|fmt_rel_join("
            {rel_name}: v.{rel_name}().map(|v| v.into()).collect(),", "") }@
            @{- def.relations_belonging_for_api_response()|fmt_rel_join("
            _{raw_rel_name}_id: v._{raw_rel_name}_id().map(|v| v.into()),
            {rel_name}: v.{rel_name}().map(|v| v.into()),", "") }@
        }
    }
}

impl From<&dyn _domain_::@{ pascal_name }@Cache> for ResObj@{ rel_name|pascal }@ {
    fn from(v: &dyn _domain_::@{ pascal_name }@Cache) -> Self {
        Self {
            _id: v.into(),
            @{- def.for_api_response_except(rel_id)|fmt_join("
            {var}: v.{var}(){to_res_api_type},", "") }@
            @{- def.relations_one_for_api_response()|fmt_rel_join("
            {rel_name}: v.{rel_name}().map(|v| (&*v).into()),", "") }@
            @{- def.relations_many_for_api_response()|fmt_rel_join("
            {rel_name}: v.{rel_name}().iter().map(|v| (&**v).into()).collect(),", "") }@
            @{- def.relations_belonging_for_api_response()|fmt_rel_join("
            _{raw_rel_name}_id: v._{raw_rel_name}_id().map(|v| v.into()),
            {rel_name}: v.{rel_name}().map(|v| (&*v).into()),", "") }@
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
@%- endif %@
@%- if !api_def.disable_mutation && !no_update %@

#[allow(unused_mut)]
#[allow(clippy::needless_update)]
pub fn updater_joiner() -> Option<Box<_domain_::Joiner_>> {
    let joiner = _domain_::Joiner_ {
        @{- def.relations_one_for_api_request()|fmt_rel_join("
        {rel_name}: _{raw_rel_name}::updater_joiner(),", "") }@
        @{- def.relations_many_for_api_request()|fmt_rel_join("
        {rel_name}: _{raw_rel_name}::updater_joiner(),", "") }@
        ..Default::default()
    };
    Some(Box::new(joiner))
}

use serde::{Deserialize, Serialize};
use crate::auth::AuthInfo;
use crate::db::RepositoriesImpl;

@{ def.label|label0 -}@
#[derive(Debug, async_graphql::InputObject, validator::Validate, Serialize, Deserialize, schemars::JsonSchema)]
#[graphql(name = "Req@{ graphql_name }@")]
pub struct ReqObj@{ rel_name|pascal }@ {
@%- if camel_case %@
@{- def.auto_primary()|fmt_join("
{label_wo_hash}{graphql_secret}{api_validate}{api_serde_default}    pub {var}: {req_api_option_type},", "") }@
@{- def.for_api_request_except(rel_id)|fmt_join("
{label_wo_hash}{graphql_secret}{api_validate}{api_serde_default}    pub {var}: {req_api_type},", "") }@
@{- def.relations_one_for_api_request()|fmt_rel_join("
{label_wo_hash}    pub {rel_name}: Option<_{raw_rel_name}::ReqObj{rel_name_pascal}>,", "") }@
@{- def.relations_many_for_api_request()|fmt_rel_join("
{label_wo_hash}    pub {rel_name}: Option<Vec<_{raw_rel_name}::ReqObj{rel_name_pascal}>>,", "") }@
@%- else %@
@{- def.auto_primary()|fmt_join("
{label_wo_hash}    #[graphql(name = \"{raw_var}\")]
{graphql_secret}{api_validate}{api_serde_default}    pub {var}: {req_api_option_type},", "") }@
@{- def.for_api_request_except(rel_id)|fmt_join("
{label_wo_hash}    #[graphql(name = \"{raw_var}\")]
{graphql_secret}{api_validate}{api_serde_default}    pub {var}: {req_api_type},", "") }@
@{- def.relations_one_for_api_request()|fmt_rel_join("
{label_wo_hash}    #[graphql(name = \"{raw_rel_name}\")]
    pub {rel_name}: Option<_{raw_rel_name}::ReqObj{rel_name_pascal}>,", "") }@
@{- def.relations_many_for_api_request()|fmt_rel_join("
{label_wo_hash}    #[graphql(name = \"{raw_rel_name}\")]
    pub {rel_name}: Option<Vec<_{raw_rel_name}::ReqObj{rel_name_pascal}>>,", "") }@
@%- endif %@
}

@{- def.fields_with_default()|fmt_join("
fn default_{raw_var}() -> {req_api_type} {
    {api_default}
}", "") }@

#[allow(clippy::useless_conversion)]
#[allow(clippy::redundant_closure_call)]
impl From<&mut dyn _domain_::@{ pascal_name }@Updater> for ReqObj@{ rel_name|pascal }@ {
    fn from(v: &mut dyn _domain_::@{ pascal_name }@Updater) -> Self {
        Self {
            @{- def.auto_primary()|fmt_join("
            {var}: Some(v.{var}(){to_req_api_type}),", "") }@
            @{- def.for_api_request_except(rel_id)|fmt_join("
            {var}: v.{var}(){to_req_api_type},", "") }@
            @{- def.relations_one_for_api_request()|fmt_rel_join("
            {rel_name}: (|| v.{rel_name}().map(|v| v.into()))(),", "") }@
            @{- def.relations_many_for_api_request()|fmt_rel_join("
            {rel_name}: (|| Some(v.{rel_name}().iter_mut().map(|v| v.into()).collect()))(),", "") }@
        }
    }
}

#[allow(clippy::let_and_return)]
#[allow(unused_mut)]
#[allow(unused_variables)]
pub fn create_entity(input: ReqObj@{ rel_name|pascal }@, repo: &RepositoriesImpl, auth: &AuthInfo) -> Box<dyn _domain_::@{ pascal_name }@Updater> {
    let mut obj = _domain_::@{ pascal_name }@Factory {
@{- def.non_auto_primary_for_factory()|fmt_join_with_foreign_default("
        {var}: {from_api_rel_type},", "", rel_id) }@
    }
    .create(repo);
    @{- def.relations_one_for_api_request()|fmt_rel_join("
    if let Some(input) = input.{rel_name} {
        obj.set_{raw_rel_name}(_{raw_rel_name}::create_entity(input, repo, auth));
    }", "") }@
    @{- def.relations_many_for_api_request()|fmt_rel_join("
    if let Some(data_list) = input.{rel_name} {
        obj.replace_{raw_rel_name}(_{raw_rel_name}::create_list(data_list, repo, auth));
    }", "") }@
    obj
}
@%- if has_many %@

pub fn create_list(
    data_list: Vec<ReqObj@{ rel_name|pascal }@>,
    repo: &RepositoriesImpl,
    auth: &AuthInfo,
) -> Vec<Box<dyn _domain_::@{ pascal_name }@Updater>> {
    data_list.into_iter().map(|v| create_entity(v, repo, auth)).collect()
}

pub fn update_list(
    list: Vec<Box<dyn _domain_::@{ pascal_name }@Updater>>,
    data_list: Vec<ReqObj@{ rel_name|pascal }@>,
    repo: &RepositoriesImpl,
    auth: &AuthInfo,
) -> anyhow::Result<Vec<Box<dyn _domain_::@{ pascal_name }@Updater>>> {
    let mut map: HashMap<_, _> = list
        .into_iter()
        .map(|mut v| { v.mark_for_delete(); v} )
        .map(|v| (v.@{ def.primary_except(rel_id)|to_var_name }@().inner(), v))
        .collect();
    let mut list = Vec::new();
    for row in data_list.into_iter() {
        @%- if def.primary_except_is_auto(rel_id) %@
        if let Some(id) = row.@{ def.primary_except(rel_id)|to_var_name }@ {
            if let Some(mut updater) = map.remove(&id) {
                updater.unmark_for_delete();
                update_updater(&mut *updater, row, repo, auth)?;
                list.push(updater);
            } else {
                anyhow::bail!("The @{ def.primary_except(rel_id) }@ of @{ rel_name }@ is invalid.");
            }
        } else {
            list.push(create_entity(row, repo, auth));
        }
        @%- else %@
        if let Some(mut updater) = map.remove(&row.@{ def.primary_except(rel_id)|to_var_name }@) {
            updater.unmark_for_delete();
            update_updater(&mut *updater, row, repo, auth)?;
            list.push(updater);
        } else {
            list.push(create_entity(row, repo, auth));
        }
        @%- endif %@
    }
    map.into_iter().for_each(|(_, v)| {
        list.push(v);
    });
    Ok(list)
}
@%- endif %@
@%- if !replace %@

#[allow(unused_variables)]
pub fn update_updater(
    updater: &mut dyn _domain_::@{ pascal_name }@Updater,
    input: ReqObj@{ rel_name|pascal }@,
    repo: &RepositoriesImpl,
    auth: &AuthInfo,
) -> anyhow::Result<()> {
@{- def.for_api_request_except_without_primary(rel_id)|fmt_join("
    updater.set_{raw_var}({from_api_type_for_update});", "") }@
@{- def.relations_one_for_api_request_with_replace_type(true)|fmt_rel_join("
    if let Some(input) = input.{rel_name} {
        updater.set_{raw_rel_name}(_{raw_rel_name}::create_entity(input, repo, auth));
    }", "") }@
@{- def.relations_one_for_api_request_with_replace_type(false)|fmt_rel_join("
    if let Some(input) = input.{rel_name} {
        if let Some(updater) = updater.{rel_name}() {
            _{raw_rel_name}::update_updater(updater, input, repo, auth)?;
        } else {
            updater.set_{raw_rel_name}(_{raw_rel_name}::create_entity(input, repo, auth));
        }
    }", "") }@
@{- def.relations_many_for_api_request()|fmt_rel_join("
    if let Some(data_list) = input.{rel_name} {
        let list = updater.take_{raw_rel_name}().unwrap();
        updater.replace_{raw_rel_name}(_{raw_rel_name}::update_list(list, data_list, repo, auth)?);        
    }", "") }@
    Ok(())
}
@%- endif %@
@%- endif %@