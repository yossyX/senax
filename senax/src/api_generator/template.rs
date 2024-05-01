use askama::Template;
use std::{collections::BTreeSet, sync::Arc};

use crate::{model_generator::template::filters, schema::ModelDef};

use super::schema::ApiModelDef;

#[derive(Template)]
#[template(
    source = r###"
    impl QueryRoot {
    @%- if !camel_case %@
    #[graphql(name = "@{ db }@")]
    @%- endif %@
    async fn @{ db|to_var_name }@(&self) -> @{ db|snake|to_var_name }@::GqlQuery@{ db|pascal }@ {
        @{ db|snake|to_var_name }@::GqlQuery@{ db|pascal }@
    }"###,
    ext = "txt",
    escape = "none"
)]
pub struct QueryRootTemplate<'a> {
    pub db: &'a str,
    pub camel_case: bool,
}

#[derive(Template)]
#[template(
    source = r###"
    impl MutationRoot {
    @%- if !camel_case %@
    #[graphql(name = "@{ db }@")]
    @%- endif %@
    async fn @{ db|to_var_name }@(&self) -> @{ db|snake|to_var_name }@::GqlMutation@{ db|pascal }@ {
        @{ db|snake|to_var_name }@::GqlMutation@{ db|pascal }@
    }"###,
    ext = "txt",
    escape = "none"
)]
pub struct MutationRootTemplate<'a> {
    pub db: &'a str,
    pub camel_case: bool,
}

#[derive(Template)]
#[template(
    source = r###"use async_graphql::Object;

#[allow(unused_imports)]
use crate::auto_api::{Role, RoleGuard};

// Do not modify this line. (GqlMod:)

pub struct GqlQuery@{ db|pascal }@;
#[Object]
impl GqlQuery@{ db|pascal }@ {
    // Do not modify this line. (GqlQuery)
}

pub struct GqlMutation@{ db|pascal }@;
#[Object]
impl GqlMutation@{ db|pascal }@ {
    // Do not modify this line. (GqlMutation)
}

pub fn gen_json_schema(dir: &std::path::Path) -> anyhow::Result<()> {
    // Do not modify this line. (JsonSchema)
    Ok(())
}
"###,
    ext = "txt",
    escape = "none"
)]
pub struct DbTemplate<'a> {
    pub db: &'a str,
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
    async fn @{ name|to_var_name }@(&self) -> @{ name|snake|to_var_name }@::GqlQuery@{ db|pascal }@@{ name|pascal }@ {
        @{ name|snake|to_var_name }@::GqlQuery@{ db|pascal }@@{ name|pascal }@
    }
    @%- endfor %@
    // Do not modify this line. (GqlQuery)"###,
    ext = "txt",
    escape = "none"
)]
pub struct DbQueryTemplate<'a> {
    pub db: &'a str,
    pub add_groups: &'a BTreeSet<String>,
    pub camel_case: bool,
}

#[derive(Template)]
#[template(
    source = r###"
    @%- if !camel_case %@
    #[graphql(name = "@{ name }@")]
    @%- endif %@
    async fn @{ name|to_var_name }@(&self) -> @{ name|snake|to_var_name }@::GqlMutation@{ db|pascal }@@{ name|pascal }@ {
        @{ name|snake|to_var_name }@::GqlMutation@{ db|pascal }@@{ name|pascal }@
    }
    // Do not modify this line. (GqlMutation)"###,
    ext = "txt",
    escape = "none"
)]
pub struct DbMutationTemplate<'a> {
    pub db: &'a str,
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

#[derive(Template)]
#[template(path = "api/model.rs", escape = "none")]
pub struct ModelTemplate<'a> {
    pub db: &'a str,
    pub group: &'a str,
    pub mod_name: &'a str,
    pub name: &'a str,
    pub pascal_name: &'a str,
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
    pub graphql_name: &'a str,
    pub pascal_name: &'a str,
    pub def: &'a Arc<ModelDef>,
    pub camel_case: bool,
    pub api_def: &'a ApiModelDef,
    pub query_guard: String,
    pub create_guard: String,
    pub import_guard: String,
    pub update_guard: String,
    pub delete_guard: String,
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

#[derive(Template)]
#[template(path = "api/model.tsx", escape = "none")]
pub struct ModelTsTemplate<'a> {
    pub db: &'a str,
    pub db_case: String,
    pub group: &'a str,
    pub group_case: String,
    pub mod_name: &'a str,
    pub model_case: String,
    pub name: &'a str,
    pub pascal_name: &'a str,
    pub id_name: &'a str,
    pub def: &'a Arc<ModelDef>,
    pub gql_fields: String,
    pub api_def: &'a ApiModelDef,
}
