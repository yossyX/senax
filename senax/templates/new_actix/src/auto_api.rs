#[allow(unused_imports)]
use actix_web::cookie::Cookie;
use actix_web::{web, HttpRequest, HttpResponse, Result};
use anyhow::{ensure, Context};
use async_graphql::http::GraphiQLSource;
use async_graphql::{async_trait, EmptySubscription, Error, ErrorExtensions, Object, Schema};
use async_graphql_actix_web::{GraphQLRequest, GraphQLResponse};
use domain::models::Repositories;
use regex::Regex;
use std::collections::BTreeMap;
use validator::ValidationErrors;

pub use db_session::models::session::session::{_SessionStore, SESSION_ROLE};
pub use senax_actix_session::Session;

use crate::auth::{AuthInfo, Role};
use crate::context::Ctx;
use crate::db::RepositoriesImpl;

// Do not modify this line. (ApiDbMod)

pub type QuerySchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

pub const LIMIT_COMPLEXITY: usize = 1000;

#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub enum GqlError {
    #[error("Not Found")]
    NotFound,

    #[error("Forbidden")]
    Forbidden,

    #[error("Validation Error")]
    ValidationError(ValidationErrors),

    #[error("Validation Error")]
    ValidationErrorList(BTreeMap<usize, ValidationErrors>),

    #[error("Bad Request")]
    BadRequest,

    #[error("Conflict")]
    Conflict,

    #[error("Internal Server Error")]
    ServerError,
}

impl GqlError {
    pub fn server_error(
        gql_ctx: &async_graphql::Context<'_>,
        reason: anyhow::Error,
    ) -> async_graphql::Error {
        let ctx = gql_ctx.data::<Ctx>().unwrap();
        if let Some(e) = reason.downcast_ref::<senax_common::err::RowNotFound>() {
            warn!(target: "server::row_not_found", ctx = ctx.ctx_no(), table = e.table; "{}", e.id);
            GqlError::BadRequest.extend()
        } else if let Some(e) = reason.downcast_ref::<GqlError>() {
            e.extend()
        } else if let Some(e) = reason.downcast_ref::<sqlx::Error>() {
            use sqlx::error::ErrorKind;
            match e {
                sqlx::Error::Database(e) => {
                    match e.kind() {
                        ErrorKind::UniqueViolation => {
                            warn!(target: "server::bad_request", ctx = ctx.ctx_no(); "{}", reason);
                            GqlError::Conflict.extend()
                        }
                        ErrorKind::Other => {
                            error!(target: "server::internal_error", ctx = ctx.ctx_no(); "{}", reason);
                            GqlError::ServerError.extend()
                        }
                        _ => {
                            warn!(target: "server::bad_request", ctx = ctx.ctx_no(); "{}", reason);
                            GqlError::BadRequest.extend()
                        },
                    }
                }
                sqlx::Error::RowNotFound => {
                    log::warn!(ctx = ctx.ctx_no(); "{}", reason);
                    GqlError::BadRequest.extend()
                }
                _ => {
                    log::error!(ctx = ctx.ctx_no(); "{}", reason);
                    GqlError::ServerError.extend()
                }
            }
        } else {
            log::error!(ctx = ctx.ctx_no(); "{}", reason);
            GqlError::ServerError.extend()
        }
    }
}

impl ErrorExtensions for GqlError {
    fn extend(&self) -> Error {
        Error::new(format!("{}", self)).extend_with(|_err, e| match self {
            GqlError::NotFound => e.set("code", "NOT_FOUND"),
            GqlError::Forbidden => e.set("code", "FORBIDDEN"),
            GqlError::ValidationError(reason) => e.set(
                "validation",
                async_graphql::Value::from_json(serde_json::to_value(reason.errors()).unwrap())
                    .unwrap(),
            ),
            GqlError::ValidationErrorList(reason) => {
                let errors: BTreeMap<_, _> = reason.iter().map(|(k, v)| (k, v.errors())).collect();
                e.set(
                    "validation",
                    async_graphql::Value::from_json(serde_json::to_value(errors).unwrap()).unwrap(),
                )
            }
            GqlError::BadRequest => e.set("code", "BAD_REQUEST"),
            GqlError::Conflict => e.set("code", "CONFLICT"),
            GqlError::ServerError => e.set("code", "SERVER_ERROR"),
        })
    }
}

#[allow(dead_code)]
struct RoleGuard(Role);

#[async_trait::async_trait]
impl async_graphql::Guard for RoleGuard {
    async fn check(&self, _gql_ctx: &async_graphql::Context<'_>) -> async_graphql::Result<()> {
        let auth: &AuthInfo = _gql_ctx.data()?;
        if auth.role == self.0 {
            return Ok(());
        }
        Err("Forbidden".into())
    }
}

#[allow(dead_code)]
struct NoGuard;

#[async_trait::async_trait]
impl async_graphql::Guard for NoGuard {
    async fn check(&self, _gql_ctx: &async_graphql::Context<'_>) -> async_graphql::Result<()> {
        Ok(())
    }
}

pub struct QueryRoot;
#[Object]
impl QueryRoot {
}

pub struct MutationRoot;
#[Object]
impl MutationRoot {
    #[cfg(debug_assertions)]
    #[graphql(complexity = LIMIT_COMPLEXITY)]
    async fn login(
        &self,
        gql_ctx: &async_graphql::Context<'_>, // Context must be next to self
        #[graphql(validator(max_length = 255))] username: String,
        role: Role,
    ) -> async_graphql::Result<String> {
        let exp = chrono::Utc::now()
            .checked_add_signed(chrono::Duration::hours(24))
            .expect("valid timestamp")
            .timestamp() as usize;
        let auth = AuthInfo {
            username,
            role,
            exp,
        };

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

pub async fn graphql(
    schema: web::Data<QuerySchema>,
    req: GraphQLRequest,
    http_req: HttpRequest,
) -> GraphQLResponse {
    let result: anyhow::Result<async_graphql::Response> = async move {
        let auth = AuthInfo::retrieve(&http_req).unwrap_or_default();
        let ctx = Ctx::get(&http_req);
        let repo = RepositoriesImpl::new_with_ctx(&ctx);
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
        .body(GraphiQLSource::build().endpoint("/gql").finish()))
}

#[allow(unused_variables)]
pub fn gen_json_schema(dir: &std::path::Path) -> anyhow::Result<()> {
    // Do not modify this line. (ApiJsonSchema)
    Ok(())
}

#[allow(dead_code)]
pub fn write_json_schema(file_path: &std::path::Path, schema: String) -> anyhow::Result<()> {
    ensure!(file_path.exists(), "File not found: {:?}", file_path);
    let contents = std::fs::read_to_string(file_path)
        .with_context(|| format!("File cannot be read: {}", file_path.display()))?;
    let re = Regex::new(r"(?s)// Do not modify below this line. \(JsonSchemaStart\).+// Do not modify up to this line. \(JsonSchemaEnd\)").unwrap();
    ensure!(
        re.is_match(&contents),
        "File contents are invalid.: {:?}",
        file_path
    );
    let tpl = format!("// Do not modify below this line. (JsonSchemaStart)\nexport const JsonSchema = {};\n// Do not modify up to this line. (JsonSchemaEnd)", schema);
    println!("{}", file_path.display());
    std::fs::write(file_path, &*re.replace(&contents, tpl))?;
    Ok(())
}

#[cfg(feature = "v8")]
thread_local!(static V8_ISOLATE: std::cell::RefCell<v8::OwnedIsolate> = std::cell::RefCell::new(v8::Isolate::new(v8::CreateParams::default())));

#[cfg(feature = "v8")]
#[allow(dead_code)]
pub async fn js_update(
    script: &'static str,
    list: Vec<String>,
    value: serde_json::Value,
    auth: &AuthInfo,
) -> anyhow::Result<Vec<String>> {
    let auth = auth.clone();
    actix_web::web::block(move || {
        V8_ISOLATE.with(|isolate| {
            let mut isolate = isolate.borrow_mut();
            let handle_scope = &mut v8::HandleScope::new(&mut *isolate);
            let context = v8::Context::new(handle_scope);
            let scope = &mut v8::ContextScope::new(handle_scope, context);
            let tc_scope = &mut v8::TryCatch::new(scope);

            let code = v8::String::new(tc_scope, script).unwrap();
            let script = match v8::Script::compile(tc_scope, code, None) {
                Some(script) => script,
                None => {
                    let exception = tc_scope.exception().unwrap();
                    let result = exception.to_string(tc_scope).unwrap();
                    anyhow::bail!("error::{}", result.to_rust_string_lossy(tc_scope));
                }
            };
            script.run(tc_scope);
            if let Some(exception) = tc_scope.exception() {
                let result = exception.to_string(tc_scope).unwrap();
                anyhow::bail!("error::{}", result.to_rust_string_lossy(tc_scope));
            }
            let mut result = Vec::new();
            for req in list {
                let code = v8::String::new(
                    tc_scope,
                    &format!(
                        "JSON.stringify(update({}, {}, {}));",
                        req,
                        serde_json::to_string(&value)?,
                        serde_json::to_string(&auth)?
                    ),
                )
                .unwrap();
                let script = match v8::Script::compile(tc_scope, code, None) {
                    Some(script) => script,
                    None => {
                        let exception = tc_scope.exception().unwrap();
                        let result = exception.to_string(tc_scope).unwrap();
                        anyhow::bail!("error::{}", result.to_rust_string_lossy(tc_scope));
                    }
                };
                if let Some(ret) = script.run(tc_scope) {
                    let ret = ret.to_string(tc_scope).unwrap();
                    let ret = ret.to_rust_string_lossy(tc_scope);
                    result.push(ret);
                } else if let Some(exception) = tc_scope.exception() {
                    let result = exception.to_string(tc_scope).unwrap();
                    anyhow::bail!("error::{}", result.to_rust_string_lossy(tc_scope));
                }
            }
            Ok(result)
        })
    })
    .await?
}

#[cfg(feature = "rquickjs")]
#[allow(dead_code)]
pub async fn js_update(
    script: &'static str,
    list: Vec<String>,
    value: serde_json::Value,
    auth: &AuthInfo,
) -> anyhow::Result<Vec<String>> {
    let auth = auth.clone();
    actix_web::web::block(move || {
        use rquickjs::{Context, Error::Exception, Runtime};
        let rt = Runtime::new()?;
        let ctx = Context::full(&rt)?;
        ctx.enable_big_num_ext(true);
        ctx.with(|ctx| {
            match ctx.eval::<(), _>(script) {
                Ok(_) => {}
                Err(Exception) => anyhow::bail!("js_update error::{:?}", ctx.catch()),
                Err(e) => anyhow::bail!("js_update error::{:?}", e),
            }
            let mut result = Vec::new();
            for req in list {
                let code = format!(
                    "JSON.stringify(update({}, {}, {}));",
                    req,
                    serde_json::to_string(&value)?,
                    serde_json::to_string(&auth)?
                );
                match ctx.eval::<String, _>(code) {
                    Ok(ret) => {
                        result.push(ret);
                    }
                    Err(Exception) => anyhow::bail!("js_update error::{:?}", ctx.catch()),
                    Err(e) => anyhow::bail!("js_update error::{:?}", e),
                }
            }
            Ok(result)
        })
    })
    .await?
}
@{-"\n"}@