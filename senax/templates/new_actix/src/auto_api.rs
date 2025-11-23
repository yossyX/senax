#[allow(unused_imports)]
use actix_web::cookie::Cookie;
use actix_web::{HttpRequest, HttpResponse, Result, web};
use async_graphql::http::GraphiQLSource;
use async_graphql::{EmptySubscription, Object, Schema};
use async_graphql_actix_web::{GraphQLRequest, GraphQLResponse};
use domain::repository::Repository;
#[allow(unused_imports)]
use utoipa_actix_web::scope;
@%- if session %@

#[allow(unused_imports)]
pub use db_session::repositories::session::session::{_SessionStore, SESSION_ROLE};
#[allow(unused_imports)]
pub use senax_actix_session::Session;
@%- endif %@

use crate::_base::auth::AuthInfo;
use crate::_base::context::Ctx;
use crate::_base::db::RepositoryImpl;

// Do not modify this line. (ApiDbMod)

pub type QuerySchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

pub const LIMIT_COMPLEXITY: usize = 1000;

pub struct QueryRoot;
#[Object]
#[allow(non_snake_case)]
impl QueryRoot {
    #[graphql(name = "_dummy")] // async_graphql::Object cannot be empty.
    async fn _dummy(&self) -> bool {
        false
    }
}

pub struct MutationRoot;
#[Object]
#[allow(non_snake_case)]
impl MutationRoot {
    #[graphql(name = "_dummy")]
    async fn _dummy(&self) -> bool {
        false
    }
    #[cfg(debug_assertions)]
    #[graphql(complexity = LIMIT_COMPLEXITY)]
    async fn login(
        &self,
        gql_ctx: &async_graphql::Context<'_>, // Context must be next to self
        #[graphql(validator(max_length = 255))] username: String,
        role: crate::_base::auth::Role,
    ) -> async_graphql::Result<String> {
        let exp = chrono::Utc::now()
            .checked_add_signed(chrono::Duration::hours(24))
            .expect("valid timestamp")
            .timestamp() as usize;
        let auth = AuthInfo(
            crate::auth::AuthInfoInner {
                username,
                role,
                exp,
            }
            .into(),
        );

        let jwt = jsonwebtoken::encode(
            &jsonwebtoken::Header::default(),
            &auth,
            &jsonwebtoken::EncodingKey::from_secret(crate::auth::SECRET.get().unwrap().as_bytes()),
        )?;

        use base64::{engine::general_purpose::URL_SAFE, Engine as _};
        let v = serde_json::to_string(&auth)?;
        let v = URL_SAFE.encode(v);
        let cookie = Cookie::build("jwt", &v)
            .http_only(true)
            // .secure(true)
            .same_site(actix_web::cookie::SameSite::Strict)
            .finish();
        gql_ctx.insert_http_header("Set-Cookie", cookie.to_string());
        Ok(jwt)
    }
}

pub fn route_config(cfg: &mut utoipa_actix_web::service_config::ServiceConfig) {
    // Do not modify this line. (ApiRouteConfig)
}

pub async fn graphql(
    schema: web::Data<QuerySchema>,
    req: GraphQLRequest,
    http_req: HttpRequest,
) -> GraphQLResponse {
    let result: anyhow::Result<async_graphql::Response> = async move {
        let auth = AuthInfo::retrieve(&http_req).unwrap_or_default();
        let ctx = Ctx::get(&http_req);
        let repo = RepositoryImpl::new_with_ctx(&ctx);
        repo.begin().await?;
        let request = req.into_inner().data(repo.clone()).data(ctx).data(auth);
        let res = schema.execute(request).await;
        if res.errors.is_empty() {
            repo.commit().await?;
        } else {
            repo.rollback().await?;
        }
        Ok(res)
    }
    .await;
    match result {
        Ok(res) => res.into(),
        Err(e) => {
            log::error!("{}", e);
            let e = async_graphql::ServerError::new("Server Error", None);
            let res = async_graphql::Response::from_errors(vec![e]);
            res.into()
        }
    }
}

pub async fn graphiql() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(GraphiQLSource::build().endpoint("/gql").finish()
        .replace("graphiql/graphiql.", "graphiql@3.9.0/graphiql.")))
}

#[allow(unused_variables)]
pub fn gen_json_schema(dir: &std::path::Path) -> anyhow::Result<()> {
    // Do not modify this line. (ApiJsonSchema)
    Ok(())
}
@{-"\n"}@