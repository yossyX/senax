use askama::Template;
use std::{collections::BTreeSet, sync::Arc};

use crate::{filters, schema::ModelDef};

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
#[template(path = "api/db.rs", escape = "none")]
pub struct DbTemplate<'a> {
    pub db: &'a str,
    pub db_route: &'a str,
}

#[derive(Template)]
#[template(
    source = r###"
@%- for name in add_groups %@
pub use _@{ server|snake }@_@{ db_route|snake }@_@{ name|snake }@::api as @{ name|snake|to_var_name }@;
@%- endfor %@
// Do not modify this line. (GqlMod:@{ all }@)"###,
    ext = "txt",
    escape = "none"
)]
pub struct DbModTemplate<'a> {
    pub server: &'a str,
    pub db_route: &'a str,
    pub all: String,
    pub add_groups: &'a BTreeSet<String>,
}

#[derive(Template)]
#[template(
    source = r###"
    @%- for name in add_groups %@
    db_@{ db|snake }@_@{ name|snake }@::init();
    @%- endfor %@
    // Do not modify this line. (GqlInit)"###,
    ext = "txt",
    escape = "none"
)]
pub struct DbInitTemplate<'a> {
    pub db: &'a str,
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
impl GqlQuery@{ db|pascal }@@{ group|pascal }@ {
    // Do not modify this line. (GqlQuery)
}

pub struct GqlMutation@{ db|pascal }@@{ group|pascal }@;
#[Object]
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
    pub curly_end: &'a str,
    pub pascal_name: String,
    pub graphql_name: &'a str,
    pub id_name: &'a str,
    pub def: &'a Arc<ModelDef>,
    pub gql_fields: String,
    pub api_def: &'a ApiModelDef,
}
