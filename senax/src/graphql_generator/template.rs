use askama::Template;
use std::{collections::BTreeSet, sync::Arc};

use crate::{model_generator::template::filters, schema::ModelDef};

#[derive(Template)]
#[template(
    source = r###"use async_graphql::Object;

use crate::graphql::{Role, RoleGuard};

// Do not modify this line. (GqiMod:)

pub struct GqiQuery@{ db|pascal }@;
#[Object]
impl GqiQuery@{ db|pascal }@ {
    // Do not modify this line. (GqiQuery)
}

pub struct GqiMutation@{ db|pascal }@;
#[Object]
impl GqiMutation@{ db|pascal }@ {
    // Do not modify this line. (GqiMutation)
}
@{-"\n"}@"###,
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
pub mod @{ name|to_var_name }@;
@%- endfor %@
// Do not modify this line. (GqiMod:@{ all }@)"###,
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
    async fn @{ name|to_var_name }@(&self) -> @{ name|to_var_name }@::GqiQuery@{ db|pascal }@@{ name|pascal }@ {
        @{ name|to_var_name }@::GqiQuery@{ db|pascal }@@{ name|pascal }@
    }
    @%- endfor %@
    // Do not modify this line. (GqiQuery)"###,
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
    @%- for name in add_groups %@
    @%- if !camel_case %@
    #[graphql(name = "@{ name }@", guard = "RoleGuard::new(Role::Admin).or(RoleGuard::new(Role::User))")]
    @%- else %@
    #[graphql(guard = "RoleGuard::new(Role::Admin).or(RoleGuard::new(Role::User))")]
    @%- endif %@
    async fn @{ name|to_var_name }@(&self) -> @{ name|to_var_name }@::GqiMutation@{ db|pascal }@@{ name|pascal }@ {
        @{ name|to_var_name }@::GqiMutation@{ db|pascal }@@{ name|pascal }@
    }
    @%- endfor %@
    // Do not modify this line. (GqiMutation)"###,
    ext = "txt",
    escape = "none"
)]
pub struct DbMutationTemplate<'a> {
    pub db: &'a str,
    pub add_groups: &'a BTreeSet<String>,
    pub camel_case: bool,
}

#[derive(Template)]
#[template(
    source = r###"use async_graphql::Object;

// Do not modify this line. (GqiMod:)

pub struct GqiQuery@{ db|pascal }@@{ group|pascal }@;
#[Object]
impl GqiQuery@{ db|pascal }@@{ group|pascal }@ {
    // Do not modify this line. (GqiQuery)
}

pub struct GqiMutation@{ db|pascal }@@{ group|pascal }@;
#[Object]
impl GqiMutation@{ db|pascal }@@{ group|pascal }@ {
    // Do not modify this line. (GqiMutation)
}
@{-"\n"}@"###,
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
pub mod @{ name|to_var_name }@;
@%- endfor %@
// Do not modify this line. (GqiMod:@{ all }@)"###,
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
    async fn @{ name|to_var_name }@(&self) -> @{ name|to_var_name }@::Gqi@{ mode }@@{ db|pascal }@@{ group|pascal }@@{ name|pascal }@ {
        @{ name|to_var_name }@::Gqi@{ mode }@@{ db|pascal }@@{ group|pascal }@@{ name|pascal }@
    }
    @%- endfor %@
    // Do not modify this line. (Gqi@{ mode }@)"###,
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
#[template(path = "graphql/model.rs", escape = "none")]
pub struct ModelTemplate<'a> {
    pub db: &'a str,
    pub group: &'a str,
    pub mod_name: &'a str,
    pub name: &'a str,
    pub pascal_name: &'a str,
    pub id_name: &'a str,
    pub def: &'a Arc<ModelDef>,
}

#[derive(Template)]
#[template(path = "graphql/_model.rs", escape = "none")]
pub struct BaseModelTemplate<'a> {
    pub db: &'a str,
    pub group: &'a str,
    pub mod_name: &'a str,
    pub pascal_name: &'a str,
    pub def: &'a Arc<ModelDef>,
    pub camel_case: bool,
}

#[derive(Template)]
#[template(path = "graphql/_relation.rs", escape = "none")]
pub struct RelationTemplate<'a> {
    pub db: &'a str,
    pub group: &'a str,
    pub mod_name: &'a str,
    pub rel_name: &'a str,
    pub rel_id: &'a str,
    pub pascal_name: &'a str,
    pub class_mod: &'a str,
    pub def: &'a Arc<ModelDef>,
    pub camel_case: bool,
}

#[derive(Template)]
#[template(path = "graphql/_reference.rs", escape = "none")]
pub struct ReferenceTemplate<'a> {
    pub db: &'a str,
    pub group: &'a str,
    pub mod_name: &'a str,
    pub rel_name: &'a str,
    pub pascal_name: &'a str,
    pub class_mod: &'a str,
    pub def: &'a Arc<ModelDef>,
    pub camel_case: bool,
}
