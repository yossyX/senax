use async_graphql::Object;
use utoipa_actix_web::scope;

#[allow(unused_imports)]
use crate::_base::auto_api::{Role, RoleGuard};

// Do not modify this line. (GqlMod:)

pub fn init() {
    // Do not modify this line. (DbInit)
}

pub struct GqlQuery@{ db_route|pascal }@;
#[Object]
impl GqlQuery@{ db_route|pascal }@ {
    // Do not modify this line. (GqlQuery)
}

pub struct GqlMutation@{ db_route|pascal }@;
#[Object]
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
@{-"\n"}@