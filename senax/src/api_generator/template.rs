use askama::Template;
use compact_str::CompactString;
use std::sync::Arc;

use crate::{filters, schema::ModelDef};

use super::schema::{ApiDbDef, ApiModelDef};

#[derive(Template)]
#[template(
    source = r###"
    impl QueryRoot {
    #[graphql(name = "@{ db_route }@")]
    async fn @{ db_route|to_var_name }@(&self) -> @{ db_route|snake|to_var_name }@::GqlQuery@{ db_route|pascal }@ {
        @{ db_route|snake|to_var_name }@::GqlQuery@{ db_route|pascal }@
    }"###,
    ext = "txt",
    escape = "none"
)]
pub struct QueryRootTemplate<'a> {
    pub db_route: &'a str,
}

#[derive(Template)]
#[template(
    source = r###"
    impl MutationRoot {
    #[graphql(name = "@{ db_route }@")]
    async fn @{ db_route|to_var_name }@(&self) -> @{ db_route|snake|to_var_name }@::GqlMutation@{ db_route|pascal }@ {
        @{ db_route|snake|to_var_name }@::GqlMutation@{ db_route|pascal }@
    }"###,
    ext = "txt",
    escape = "none"
)]
pub struct MutationRootTemplate<'a> {
    pub db_route: &'a str,
}

#[derive(Template)]
#[template(
    source = r###"#![allow(clippy::module_inception)]
use async_graphql::Object;
use utoipa_actix_web::scope;

// Do not modify this line. (GqlMod)

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

#[allow(dead_code)]
#[derive(Template)]
#[template(path = "api/model.rs", escape = "none")]
pub struct ModelTemplate<'a> {
    pub server_name: &'a str,
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
    pub version_col: CompactString,
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
    pub server_name: &'a str,
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
#[template(path = "api/config.yml", escape = "none")]
pub struct DbConfigTemplate;

#[allow(dead_code)]
#[derive(Template)]
#[template(path = "api/model.tsx", escape = "none")]
pub struct ModelTsTemplate<'a> {
    pub path: String,
    pub model_route: &'a str,
    pub curly_begin: String,
    pub curly_end: String,
    pub pascal_name: String,
    pub graphql_name: &'a str,
    pub id_name: &'a str,
    pub def: &'a Arc<ModelDef>,
    pub gql_fields: String,
    pub api_def: &'a ApiModelDef,
}
