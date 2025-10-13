use async_graphql::Object;
use utoipa_actix_web::scope;

#[allow(unused_imports)]
use crate::_base::auto_api::{Role, RoleGuard};

// Do not modify this line. (GqlMod)

pub struct GqlQuery@{ db_route|pascal }@;
#[Object]
impl GqlQuery@{ db_route|pascal }@ {
    // This function can be removed.
    async fn _dummy(&self) -> bool {
        false
    }
    // Do not modify this line. (GqlQuery)
}

pub struct GqlMutation@{ db_route|pascal }@;
#[Object]
impl GqlMutation@{ db_route|pascal }@ {
    // This function can be removed.
    async fn _dummy(&self) -> bool {
        false
    }
    // Do not modify this line. (GqlMutation)
}

pub fn route_config(cfg: &mut utoipa_actix_web::service_config::ServiceConfig) {
    // Do not modify this line. (ApiRouteConfig)
}

pub fn gen_json_schema(dir: &std::path::Path) -> anyhow::Result<()> {
    // Do not modify this line. (JsonSchema)
    Ok(())
}
@{-"\n"}@