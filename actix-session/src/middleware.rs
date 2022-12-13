use actix_utils::future::{ready, Ready};
use actix_web::{
    body::MessageBody,
    cookie::{Cookie, CookieJar, Key},
    dev::{forward_ready, ResponseHead, Service, ServiceRequest, ServiceResponse, Transform},
    http::header::{HeaderValue, SET_COOKIE},
    HttpResponse,
};
use std::{convert::TryInto, fmt, future::Future, pin::Pin, rc::Rc};

use crate::{
    config::{self, Configuration, CookieConfiguration, SessionMiddlewareBuilder},
    interface::SessionStore,
    session_key::SessionKey,
    Session, SessionStatus,
};

#[derive(Clone)]
pub struct SessionMiddleware<Store: SessionStore> {
    storage_backend: Rc<Store>,
    configuration: Rc<Configuration>,
}

impl<Store: SessionStore> SessionMiddleware<Store> {
    pub fn new(store: Store, key: &[u8]) -> Self {
        Self::builder(store, key).build()
    }

    pub fn builder(store: Store, key: &[u8]) -> SessionMiddlewareBuilder<Store> {
        let key = Key::from(key);
        SessionMiddlewareBuilder::new(store, config::default_configuration(key))
    }

    /// Execute session gc.
    /// ttl_sec should be the same as the session ttl.
    pub async fn gc(store: Store, ttl_sec: Option<i64>) {
        let ttl = match ttl_sec {
            Some(ttl) => std::time::Duration::from_secs(ttl as u64),
            None => std::time::Duration::from_secs(
                config::default_ttl().whole_seconds().try_into().unwrap(),
            ),
        };
        let start_key = SessionKey::generate_past(ttl);
        if let Err(e) = store.gc(&start_key).await {
            tracing::warn!("{}", e);
        }
    }

    pub(crate) fn from_parts(store: Store, configuration: Configuration) -> Self {
        Self {
            storage_backend: Rc::new(store),
            configuration: Rc::new(configuration),
        }
    }
}

impl<S, B, Store> Transform<S, ServiceRequest> for SessionMiddleware<Store>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error> + 'static,
    S::Future: 'static,
    B: MessageBody + 'static,
    Store: SessionStore + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = actix_web::Error;
    type Transform = InnerSessionMiddleware<S, Store>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(InnerSessionMiddleware {
            service: Rc::new(service),
            configuration: Rc::clone(&self.configuration),
            storage_backend: Rc::clone(&self.storage_backend),
        }))
    }
}

pub(crate) fn e500<E: fmt::Debug + fmt::Display + 'static>(err: E) -> actix_web::Error {
    actix_web::error::InternalError::from_response(
        err,
        HttpResponse::InternalServerError().finish(),
    )
    .into()
}

#[doc(hidden)]
#[non_exhaustive]
pub struct InnerSessionMiddleware<S, Store: SessionStore + 'static> {
    service: Rc<S>,
    configuration: Rc<Configuration>,
    storage_backend: Rc<Store>,
}

impl<S, B, Store> Service<ServiceRequest> for InnerSessionMiddleware<S, Store>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error> + 'static,
    S::Future: 'static,
    Store: SessionStore + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = actix_web::Error;
    #[allow(clippy::type_complexity)]
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    forward_ready!(service);

    fn call(&self, mut req: ServiceRequest) -> Self::Future {
        let service = Rc::clone(&self.service);
        let storage_backend = Rc::clone(&self.storage_backend);
        let configuration = Rc::clone(&self.configuration);

        Box::pin(async move {
            let session_key = extract_session_key(&req, &configuration.cookie);

            Session::set_session(&mut req, session_key, storage_backend, &configuration)
                .await
                .map_err(e500)?;

            let mut res = service.call(req).await?;
            let (status, session_key) = Session::<Store>::get_status(&mut res);
            match status {
                SessionStatus::Changed if session_key.is_some() => {
                    set_session_cookie(
                        res.response_mut().head_mut(),
                        session_key.unwrap(),
                        &configuration.cookie,
                    )
                    .map_err(e500)?;
                }
                SessionStatus::Purged => {
                    delete_session_cookie(res.response_mut().head_mut(), &configuration.cookie)
                        .map_err(e500)?;
                }
                _ => {}
            }
            Ok(res)
        })
    }
}

fn extract_session_key(req: &ServiceRequest, config: &CookieConfiguration) -> Option<SessionKey> {
    let cookies = req.cookies().ok()?;
    let session_cookie = cookies
        .iter()
        .find(|&cookie| cookie.name() == config.name)?;

    let mut jar = CookieJar::new();
    jar.add_original(session_cookie.clone());

    let verification_result = jar.private(&config.key).get(&config.name);

    if verification_result.is_none() {
        tracing::warn!("The session cookie failed to decrypt.");
    }

    match verification_result?.value().to_owned().try_into() {
        Ok(session_key) => Some(session_key),
        Err(err) => {
            tracing::warn!("{}", err);
            None
        }
    }
}

fn set_session_cookie(
    response: &mut ResponseHead,
    session_key: SessionKey,
    config: &CookieConfiguration,
) -> Result<(), anyhow::Error> {
    let value: String = session_key.into();
    let mut cookie = Cookie::new(config.name.clone(), value);

    cookie.set_secure(config.secure);
    cookie.set_http_only(config.http_only);
    cookie.set_same_site(config.same_site);
    cookie.set_path(config.path.clone());

    if let Some(max_age) = config.max_age {
        cookie.set_max_age(max_age);
    }

    if let Some(ref domain) = config.domain {
        cookie.set_domain(domain.clone());
    }

    let mut jar = CookieJar::new();
    jar.private_mut(&config.key).add(cookie);

    let cookie = jar.delta().next().unwrap();
    let val = HeaderValue::from_str(&cookie.encoded().to_string())?;
    response.headers_mut().append(SET_COOKIE, val);

    Ok(())
}

fn delete_session_cookie(
    response: &mut ResponseHead,
    config: &CookieConfiguration,
) -> Result<(), anyhow::Error> {
    let removal_cookie = Cookie::build(config.name.clone(), "")
        .path(config.path.clone())
        .secure(config.secure)
        .http_only(config.http_only)
        .same_site(config.same_site);

    let mut removal_cookie = if let Some(ref domain) = config.domain {
        removal_cookie.domain(domain)
    } else {
        removal_cookie
    }
    .finish();

    removal_cookie.make_removal();

    let val = HeaderValue::from_str(&removal_cookie.to_string())?;
    response.headers_mut().append(SET_COOKIE, val);

    Ok(())
}
