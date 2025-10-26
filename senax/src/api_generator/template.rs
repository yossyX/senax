use askama::Template;
use std::{collections::BTreeSet, sync::Arc};

use crate::{model_generator::template::filters, schema::ModelDef};

use super::schema::{ApiDbDef, ApiModelDef};

#[derive(Template)]
#[template(
    source = r###"
    impl QueryRoot {
    @%- if !camel_case %@
    #[graphql(name = "@{ db_route }@")]
    @%- endif %@
    async fn @{ db_route|to_var_name }@(&self) -> @{ db_route|snake|to_var_name }@::GqlQuery@{ db_route|pascal }@ {
        @{ db_route|snake|to_var_name }@::GqlQuery@{ db_route|pascal }@
    }"###,
    ext = "txt",
    escape = "none"
)]
pub struct QueryRootTemplate<'a> {
    pub db_route: &'a str,
    pub camel_case: bool,
}

#[derive(Template)]
#[template(
    source = r###"
    impl MutationRoot {
    @%- if !camel_case %@
    #[graphql(name = "@{ db_route }@")]
    @%- endif %@
    async fn @{ db_route|to_var_name }@(&self) -> @{ db_route|snake|to_var_name }@::GqlMutation@{ db_route|pascal }@ {
        @{ db_route|snake|to_var_name }@::GqlMutation@{ db_route|pascal }@
    }"###,
    ext = "txt",
    escape = "none"
)]
pub struct MutationRootTemplate<'a> {
    pub db_route: &'a str,
    pub camel_case: bool,
}

#[derive(Template)]
#[template(
    source = r###"use async_graphql::Object;
use utoipa_actix_web::scope;

#[allow(unused_imports)]
use crate::auto_api::{Role, RoleGuard};

// Do not modify this line. (GqlMod:)

pub struct GqlQuery@{ db_route|pascal }@;
#[Object]
#[allow(non_snake_case)]
impl GqlQuery@{ db_route|pascal }@ {
    // Do not modify this line. (GqlQuery)
}

pub struct GqlMutation@{ db_route|pascal }@;
#[Object]
#[allow(non_snake_case)]
impl GqlMutation@{ db_route|pascal }@ {
    // Do not modify this line. (GqlMutation)
}

pub fn route_config(cfg: &mut utoipa_actix_web::service_config::ServiceConfig) {
    // Do not modify this line. (ApiRouteConfig)
}

pub fn gen_json_schema(dir: &std::path::Path) -> anyhow::Result<()> {
    // Do not modify this line. (JsonSchema)
    Ok(())
}

#[macro_export]
macro_rules! gql_@{ db_route|snake }@_find {
    ( $f:ident $p:tt, $repo:expr, $auth:expr, $gql_ctx:expr ) => {
        match $f$p.await {
            Ok(obj) => {
                let obj = obj.ok_or_else(|| GqlError::NotFound.extend())?;
                Ok(ResObj::try_from_(&*obj, $auth, None)?)
            }
            Err(e) => {
                if $repo.@{ db|snake }@_query().should_retry(&e) {
                    $repo.@{ db|snake }@_query().reset_tx().await;
                    let obj = $f$p
                        .await
                        .map_err(|e| GqlError::server_error($gql_ctx, e))?;
                    let obj = obj.ok_or_else(|| GqlError::NotFound.extend())?;
                    Ok(ResObj::try_from_(&*obj, $auth, None)?)
                } else {
                    Err(GqlError::server_error($gql_ctx, e))
                }
            }
        }
    };
}

#[macro_export]
macro_rules! gql_@{ db_route|snake }@_selector {
    ( $f:ident $p:tt, $repo:expr, $gql_ctx:expr ) => {
        match $f$p.await {
            Ok(result) => Ok(result),
            Err(e) => {
                if $repo.@{ db|snake }@_query().should_retry(&e) {
                    $repo.@{ db|snake }@_query().reset_tx().await;
                    let result = $f$p
                        .await
                        .map_err(|e| GqlError::server_error($gql_ctx, e))?;
                    Ok(result)
                } else {
                    Err(GqlError::server_error($gql_ctx, e))
                }
            }
        }?
    };
}

#[macro_export]
macro_rules! api_@{ db_route|snake }@_selector {
    ( $f:ident $p:tt, $repo:expr ) => {
        match $f$p.await {
            Ok(result) => Ok(result),
            Err(e) => {
                if $repo.@{ db|snake }@_query().should_retry(&e) {
                    $repo.@{ db|snake }@_query().reset_tx().await;
                    let result = $f$p
                        .await
                        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;
                    Ok(result)
                } else {
                    Err(ApiError::InternalServerError(e.to_string()))
                }
            }
        }?
    };
}

#[macro_export]
macro_rules! gql_@{ db_route|snake }@_count {
    ( $f:ident $p:tt, $repo:expr, $gql_ctx:expr ) => {
        match $f$p.await {
            Ok(count) => Ok(count),
            Err(e) => {
                if $repo.@{ db|snake }@_query().should_retry(&e) {
                    $repo.@{ db|snake }@_query().reset_tx().await;
                    let count = $f$p
                        .await
                        .map_err(|e| GqlError::server_error($gql_ctx, e))?;
                    Ok(count)
                } else {
                    Err(GqlError::server_error($gql_ctx, e))
                }
            }
        }
    };
}
"###,
    ext = "txt",
    escape = "none"
)]
pub struct DbTemplate<'a> {
    pub db: &'a str,
    pub db_route: &'a str,
}

#[derive(Template)]
#[template(
    source = r###"
@%- for name in add_groups %@
pub mod @{ name|snake|to_var_name }@;
@%- endfor %@
// Do not modify this line. (GqlMod:@{ all }@)"###,
    ext = "txt",
    escape = "none"
)]
pub struct DbModTemplate<'a> {
    pub all: String,
    pub add_groups: &'a BTreeSet<String>,
}

#[derive(Template)]
#[template(
    source = r###"
    @%- for name in add_groups %@
    @%- if !camel_case %@
    #[graphql(name = "@{ name }@")]
    @%- endif %@
    async fn @{ name|to_var_name }@(&self) -> @{ name|snake|to_var_name }@::GqlQuery@{ db_route|pascal }@@{ name|pascal }@ {
        @{ name|snake|to_var_name }@::GqlQuery@{ db_route|pascal }@@{ name|pascal }@
    }
    @%- endfor %@
    // Do not modify this line. (GqlQuery)"###,
    ext = "txt",
    escape = "none"
)]
pub struct DbQueryTemplate<'a> {
    pub db_route: &'a str,
    pub add_groups: &'a BTreeSet<String>,
    pub camel_case: bool,
}

#[derive(Template)]
#[template(
    source = r###"
    @%- if !camel_case %@
    #[graphql(name = "@{ name }@")]
    @%- endif %@
    async fn @{ name|to_var_name }@(&self) -> @{ name|snake|to_var_name }@::GqlMutation@{ db_route|pascal }@@{ name|pascal }@ {
        @{ name|snake|to_var_name }@::GqlMutation@{ db_route|pascal }@@{ name|pascal }@
    }
    // Do not modify this line. (GqlMutation)"###,
    ext = "txt",
    escape = "none"
)]
pub struct DbMutationTemplate<'a> {
    pub db_route: &'a str,
    pub name: &'a str,
    pub camel_case: bool,
}

#[derive(Template)]
#[template(
    source = r###"
    @%- for name in add_groups %@
    @{ name|snake|to_var_name }@::gen_json_schema(&dir.join("@{ name }@"))?;
    @%- endfor %@
    // Do not modify this line. (JsonSchema)"###,
    ext = "txt",
    escape = "none"
)]
pub struct DbJsonSchemaTemplate<'a> {
    pub add_groups: &'a BTreeSet<String>,
}

#[derive(Template)]
#[template(
    source = r###"#![allow(clippy::module_inception)]
use async_graphql::Object;
use utoipa_actix_web::scope;

// Do not modify this line. (GqlMod:)

pub struct GqlQuery@{ db|pascal }@@{ group|pascal }@;
#[Object]
#[allow(non_snake_case)]
impl GqlQuery@{ db|pascal }@@{ group|pascal }@ {
    // Do not modify this line. (GqlQuery)
}

pub struct GqlMutation@{ db|pascal }@@{ group|pascal }@;
#[Object]
#[allow(non_snake_case)]
impl GqlMutation@{ db|pascal }@@{ group|pascal }@ {
    // Do not modify this line. (GqlMutation)
}

pub fn route_config(cfg: &mut utoipa_actix_web::service_config::ServiceConfig) {
    // Do not modify this line. (ApiRouteConfig)
}

pub fn gen_json_schema(dir: &std::path::Path) -> anyhow::Result<()> {
    // Do not modify this line. (JsonSchema)
    Ok(())
}
"###,
    ext = "txt",
    escape = "none"
)]
pub struct GroupTemplate<'a> {
    pub db: &'a str,
    pub group: &'a str,
}

#[derive(Template)]
#[template(
    source = r###"
@%- for name in add_models %@
pub mod @{ name|snake|to_var_name }@;
@%- endfor %@
// Do not modify this line. (GqlMod:@{ all }@)"###,
    ext = "txt",
    escape = "none"
)]
pub struct GroupModTemplate<'a> {
    pub all: String,
    pub add_models: &'a BTreeSet<String>,
}

#[derive(Template)]
#[template(
    source = r###"
    @%- for name in add_models %@
    @%- if !camel_case %@
    #[graphql(name = "@{ name }@")]
    @%- endif %@
    async fn @{ name|to_var_name }@(&self) -> @{ name|snake|to_var_name }@::Gql@{ mode }@@{ db|pascal }@@{ group|pascal }@@{ name|pascal }@ {
        @{ name|snake|to_var_name }@::Gql@{ mode }@@{ db|pascal }@@{ group|pascal }@@{ name|pascal }@
    }
    @%- endfor %@
    // Do not modify this line. (Gql@{ mode }@)"###,
    ext = "txt",
    escape = "none"
)]
pub struct GroupImplTemplate<'a> {
    pub mode: &'a str,
    pub db: &'a str,
    pub group: &'a str,
    pub add_models: &'a BTreeSet<String>,
    pub camel_case: bool,
}

#[derive(Template)]
#[template(
    source = r###"
    @%- for name in add_models %@
    @{ name|snake|to_var_name }@::gen_json_schema(dir)?;
    @%- endfor %@
    // Do not modify this line. (JsonSchema)"###,
    ext = "txt",
    escape = "none"
)]
pub struct GroupJsonSchemaTemplate<'a> {
    pub add_models: &'a BTreeSet<String>,
}

#[allow(dead_code)]
#[derive(Template)]
#[template(path = "api/model.rs", escape = "none")]
pub struct ModelTemplate<'a> {
    pub db: &'a str,
    pub db_route: &'a str,
    pub group: &'a str,
    pub group_route: &'a str,
    pub mod_name: &'a str,
    pub pascal_name: &'a str,
    pub graphql_name: &'a str,
    pub id_name: &'a str,
    pub def: &'a Arc<ModelDef>,
    pub camel_case: bool,
    pub api_def: &'a ApiModelDef,
}

#[derive(Template)]
#[template(path = "api/_model.rs", escape = "none")]
pub struct BaseModelTemplate<'a> {
    pub db: &'a str,
    pub group: &'a str,
    pub mod_name: &'a str,
    pub model_name: &'a str,
    pub pascal_name: &'a str,
    pub graphql_name: &'a str,
    pub config: &'a ApiDbDef,
    pub def: &'a Arc<ModelDef>,
    pub camel_case: bool,
    pub api_def: &'a ApiModelDef,
}

#[derive(Template)]
#[template(path = "api/_relation.rs", escape = "none")]
pub struct RelationTemplate<'a> {
    pub db: &'a str,
    pub graphql_name: &'a str,
    pub rel_name: &'a str,
    pub rel_id: &'a Vec<String>,
    pub pascal_name: &'a str,
    pub def: &'a Arc<ModelDef>,
    pub camel_case: bool,
    pub rel_mod: String,
    pub has_many: bool,
    pub no_read: bool,
    pub no_update: bool,
    pub replace: bool,
    pub api_def: &'a ApiModelDef,
}

#[derive(Template)]
#[template(path = "api/_reference.rs", escape = "none")]
pub struct ReferenceTemplate<'a> {
    pub db: &'a str,
    pub graphql_name: &'a str,
    pub rel_name: &'a str,
    pub pascal_name: &'a str,
    pub def: &'a Arc<ModelDef>,
    pub camel_case: bool,
    pub rel_mod: String,
}

#[derive(Template)]
#[template(path = "api/_config.yml", escape = "none")]
pub struct ConfigTemplate;

#[derive(Template)]
#[template(path = "api/config.yml", escape = "none")]
pub struct DbConfigTemplate;

#[allow(dead_code)]
#[derive(Template)]
#[template(path = "api/model.tsx", escape = "none")]
pub struct ModelTsTemplate<'a> {
    pub path: String,
    pub model_route: &'a str,
    pub curly_begin: String,
    pub curly_end: &'a str,
    pub pascal_name: String,
    pub graphql_name: &'a str,
    pub id_name: &'a str,
    pub def: &'a Arc<ModelDef>,
    pub gql_fields: String,
    pub api_def: &'a ApiModelDef,
}
