use actix_web::{web, HttpResponse, Result};
use async_graphql::http::GraphiQLSource;
use async_graphql::{async_trait, EmptySubscription, Error, ErrorExtensions, Object, Schema};
use async_graphql_actix_web::{GraphQLRequest, GraphQLResponse};
use validator::ValidationErrors;

pub use db_session::session::session::{_SessionStore, SESSION_ROLE};
pub use senax_actix_session::Session;

use crate::auth::Role;

// Do not modify this line. (GqlDbMod)

pub type QuerySchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub enum GqiError {
    #[error("Could not find resource")]
    NotFound,

    #[error("ValidationError")]
    ValidationError(ValidationErrors),

    #[error("ServerError")]
    ServerError(String),
}

impl ErrorExtensions for GqiError {
    fn extend(&self) -> Error {
        Error::new(format!("{}", self)).extend_with(|_err, e| match self {
            GqiError::NotFound => e.set("code", "NOT_FOUND"),
            GqiError::ValidationError(reason) => e.set(
                "validation",
                async_graphql::Value::from_json(serde_json::to_value(reason.errors()).unwrap())
                    .unwrap(),
            ),
            GqiError::ServerError(reason) => {
                log::warn!("{}", reason);
            }
        })
    }
}

struct RoleGuard {
    role: Role,
}

impl RoleGuard {
    #[allow(dead_code)]
    fn new(role: Role) -> Self {
        Self { role }
    }
}

#[async_trait::async_trait]
impl async_graphql::Guard for RoleGuard {
    async fn check(&self, ctx: &async_graphql::Context<'_>) -> async_graphql::Result<()> {
        let session = ctx.data_unchecked::<Session<_SessionStore>>();
        let role = session.get_from_login::<Role>(SESSION_ROLE)?;
        if role == Some(self.role) || role.is_none() && self.role == Role::Guest {
            Ok(())
        } else {
            Err("Forbidden".into())
        }
    }
}

pub struct QueryRoot;
#[Object]
impl QueryRoot {
    // Remove the dummy function when the implementation is complete.
    async fn dummy(&self) -> bool {
        false
    }
}

pub struct MutationRoot;
#[Object]
impl MutationRoot {
    // Delete the line below once login implementation is complete.
    #[cfg(debug_assertions)]
    async fn login(
        &self,
        ctx: &async_graphql::Context<'_>, // ctx must be next to self
        #[graphql(validator(max_length = 255))] name: String,
        #[graphql(secret, validator(max_length = 255))] pw: String,
    ) -> async_graphql::Result<bool> {
        let session = ctx.data_unchecked::<Session<_SessionStore>>();
        session
            .update(|s| {
                // This is a sample implementation
                if name == "admin" {
                    s.insert_to_login(SESSION_ROLE, Role::Admin)?;
                } else {
                    s.insert_to_login(SESSION_ROLE, Role::User)?;
                }
                s.insert_to_login("name", name.clone())?;
                Ok(())
            })
            .await?;
        Ok(true)
    }
    async fn logout(&self, ctx: &async_graphql::Context<'_>) -> async_graphql::Result<bool> {
        let session = ctx.data_unchecked::<Session<_SessionStore>>();
        session
            .update(|s| {
                s.clear_login_data();
                Ok(())
            })
            .await?;
        Ok(true)
    }
}

pub async fn index(
    session: Session<_SessionStore>,
    schema: web::Data<QuerySchema>,
    req: GraphQLRequest,
) -> GraphQLResponse {
    schema.execute(req.into_inner().data(session)).await.into()
}

pub async fn index_graphiql() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(GraphiQLSource::build().endpoint("/gql").finish()))
}
@{-"\n"}@