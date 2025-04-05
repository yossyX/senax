use actix_utils::future::{Ready, ready};
use actix_web::{
    FromRequest, HttpMessage, HttpRequest,
    dev::{Extensions, Payload, ServiceRequest, ServiceResponse},
    error::Error,
};
use anyhow::{Context, Result, bail};
use senax_common::session::SessionKey;
use senax_common::session::interface::{SaveError, SessionData, SessionStore};
use serde::{Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, mem, sync::Arc, sync::Mutex};
use time::Duration;

use crate::{config::Configuration, middleware::e500};

const MAX_RETRY_COUNT: usize = 10;

#[derive(Clone)]
pub struct Session<Store: SessionStore + 'static>(Arc<Mutex<SessionInner<Store>>>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionStatus {
    Unchanged,
    Changed,
    Purged,
}

pub struct SessionInner<Store: SessionStore + 'static> {
    session_key: Option<SessionKey>,
    guest_zone: HashMap<String, Vec<u8>>,
    user_zone: HashMap<String, Vec<u8>>,
    debug_zone: HashMap<String, Vec<u8>>,
    update: bool,
    status: SessionStatus,
    state_ttl: Duration,
    version: u32,
    storage: Arc<Store>,
}

impl<Store: SessionStore + 'static> SessionInner<Store> {
    pub fn get_from_guest_zone<T: DeserializeOwned>(&mut self, key: &str) -> Result<Option<T>> {
        if let Some(val) = self.guest_zone.get(key) {
            Ok(Some(ciborium::from_reader(val.as_slice())?))
        } else {
            Ok(None)
        }
    }

    pub fn insert_to_guest_zone<T: Serialize>(
        &mut self,
        key: impl Into<String>,
        value: T,
    ) -> Result<()> {
        self.update = true;
        let mut buf = Vec::new();
        ciborium::into_writer(&value, &mut buf)?;
        self.guest_zone.insert(key.into(), buf);
        Ok(())
    }

    pub fn remove_from_guest_zone(&mut self, key: &str) {
        self.update = true;
        self.guest_zone.remove(key);
    }

    pub fn remove_from_guest_zone_as<T: DeserializeOwned>(
        &mut self,
        key: &str,
    ) -> Option<Result<T>> {
        self.update = true;
        self.guest_zone
            .remove(key)
            .map(|val| Ok(ciborium::from_reader(val.as_slice())?))
    }

    pub fn clear_guest_zone(&mut self) {
        self.update = true;
        self.guest_zone.clear();
    }

    pub fn get_from_user_zone<T: DeserializeOwned>(&mut self, key: &str) -> Result<Option<T>> {
        if let Some(val) = self.user_zone.get(key) {
            Ok(Some(ciborium::from_reader(val.as_slice())?))
        } else {
            Ok(None)
        }
    }

    pub fn insert_to_user_zone<T: Serialize>(
        &mut self,
        key: impl Into<String>,
        value: T,
    ) -> Result<()> {
        self.update = true;
        let mut buf = Vec::new();
        ciborium::into_writer(&value, &mut buf)?;
        self.user_zone.insert(key.into(), buf);
        Ok(())
    }

    pub fn remove_from_user_zone(&mut self, key: &str) {
        self.update = true;
        self.user_zone.remove(key);
    }

    pub fn remove_from_user_zone_as<T: DeserializeOwned>(
        &mut self,
        key: &str,
    ) -> Option<Result<T>> {
        self.update = true;
        self.user_zone
            .remove(key)
            .map(|val| Ok(ciborium::from_reader(val.as_slice())?))
    }

    pub fn clear_user_zone(&mut self) {
        self.update = true;
        self.user_zone.clear();
    }

    pub fn get_from_debug_zone<T: DeserializeOwned>(&mut self, key: &str) -> Result<Option<T>> {
        if cfg!(debug_assertions) {
            if let Some(val) = self.debug_zone.get(key) {
                Ok(Some(ciborium::from_reader(val.as_slice())?))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    pub fn insert_to_debug_zone<T: Serialize>(
        &mut self,
        key: impl Into<String>,
        value: T,
    ) -> Result<()> {
        if cfg!(debug_assertions) {
            self.update = true;
            let mut buf = Vec::new();
            ciborium::into_writer(&value, &mut buf)?;
            self.debug_zone.insert(key.into(), buf);
        }
        Ok(())
    }

    pub fn remove_from_debug_zone(&mut self, key: &str) {
        if cfg!(debug_assertions) {
            self.update = true;
            self.debug_zone.remove(key);
        }
    }

    pub fn remove_from_debug_zone_as<T: DeserializeOwned>(
        &mut self,
        key: &str,
    ) -> Option<Result<T>> {
        if cfg!(debug_assertions) {
            self.update = true;
            self.debug_zone
                .remove(key)
                .map(|val| Ok(ciborium::from_reader(val.as_slice())?))
        } else {
            None
        }
    }

    pub fn clear_debug_zone(&mut self) {
        if cfg!(debug_assertions) {
            self.update = true;
            self.debug_zone.clear();
        }
    }
}

impl<Store: SessionStore + 'static> Session<Store> {
    pub fn session_key(&self) -> Option<SessionKey> {
        self.0.lock().unwrap().session_key.as_ref().cloned()
    }

    pub fn csrf_token(&self) -> Option<String> {
        use std::fmt::Write;
        self.0.lock().unwrap().session_key.as_ref().map(|v| {
            Sha256::digest(String::from(v))
                .iter()
                .take(8)
                .fold(String::new(), |mut output, x| {
                    write!(output, "{:02X}", x).unwrap();
                    output
                })
        })
    }

    pub fn contains_in_guest_zone(&self, key: &str) -> bool {
        self.0.lock().unwrap().guest_zone.contains_key(key)
    }

    pub fn contains_in_user_zone(&self, key: &str) -> bool {
        self.0.lock().unwrap().user_zone.contains_key(key)
    }

    pub fn contains_in_debug_zone(&self, key: &str) -> bool {
        if cfg!(debug_assertions) {
            self.0.lock().unwrap().debug_zone.contains_key(key)
        } else {
            false
        }
    }

    pub fn get_from_guest_zone<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        if let Some(val) = self.0.lock().unwrap().guest_zone.get(key) {
            Ok(Some(ciborium::from_reader(val.as_slice())?))
        } else {
            Ok(None)
        }
    }

    pub fn get_from_user_zone<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        if let Some(val) = self.0.lock().unwrap().user_zone.get(key) {
            Ok(Some(ciborium::from_reader(val.as_slice())?))
        } else {
            Ok(None)
        }
    }

    pub fn get_from_debug_zone<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        if cfg!(debug_assertions) {
            if let Some(val) = self.0.lock().unwrap().debug_zone.get(key) {
                Ok(Some(ciborium::from_reader(val.as_slice())?))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    pub fn keys_of_guest_zone(&self) -> Vec<String> {
        self.0.lock().unwrap().guest_zone.keys().cloned().collect()
    }

    pub fn keys_of_user_zone(&self) -> Vec<String> {
        self.0.lock().unwrap().user_zone.keys().cloned().collect()
    }

    pub fn keys_of_debug_zone(&self) -> Vec<String> {
        self.0.lock().unwrap().debug_zone.keys().cloned().collect()
    }

    pub fn status(&self) -> SessionStatus {
        self.0.lock().unwrap().status
    }

    pub async fn update<F, R>(&self, f: F) -> Result<R>
    where
        F: Fn(&mut SessionInner<Store>) -> Result<R>,
    {
        let mut retry_count = 0;
        loop {
            let (f_result, session_data, key, storage) = {
                let mut inner = self.0.lock().unwrap();
                let f_result = f(&mut inner);
                if !inner.update {
                    return f_result;
                }
                inner.update = false;
                let list = vec![&inner.debug_zone, &inner.user_zone, &inner.guest_zone];
                let mut buf = Vec::new();
                ciborium::into_writer(&list, &mut buf)?;
                let session_data = SessionData::from((buf, inner.state_ttl, inner.version));
                let key = inner.session_key.as_ref().cloned();
                let storage = Arc::clone(&inner.storage);
                (f_result, session_data, key, storage)
            };
            match storage.save(key, session_data).await {
                Ok(key) => {
                    let mut inner = self.0.lock().unwrap();
                    if !matches!(&inner.session_key, Some(x) if x == &key) {
                        inner.status = SessionStatus::Changed;
                        inner.session_key = Some(key);
                    }
                }
                Err(SaveError::Retryable) => {
                    retry_count += 1;
                    if retry_count > MAX_RETRY_COUNT {
                        bail!("too many session update retry");
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                    self.reload().await?;
                    continue;
                }
                Err(SaveError::RetryableWithData(data)) => {
                    self.reload_from_data(data).await?;
                    continue;
                }
                Err(SaveError::Other(e)) => {
                    return Err(e);
                }
            }
            return f_result;
        }
    }

    pub async fn purge(&self) -> Result<()> {
        let key = self.0.lock().unwrap().session_key.as_ref().cloned();
        let storage = Arc::clone(&self.0.lock().unwrap().storage);
        if key.is_some() {
            storage.delete(key.as_ref().unwrap()).await?;
        }
        let mut inner = self.0.lock().unwrap();
        inner.status = SessionStatus::Purged;
        inner.session_key = None;
        inner.guest_zone.clear();
        inner.user_zone.clear();
        inner.debug_zone.clear();
        inner.version = 0;
        Ok(())
    }

    /// Update session key.
    /// The old session will not be deleted for the following reasons:
    /// * If there are simultaneous accesses, the access that comes after the renew will create a new session and cancel the previous cookie.
    /// * Consider cases where modified cookies cannot be received due to communication errors.
    ///
    /// Thus, since previous session data is not deleted, RENEW should not be used for logout.
    pub async fn renew<F, R>(&self, f: F) -> Result<R>
    where
        F: Fn(&mut SessionInner<Store>) -> Result<R>,
    {
        self.reload().await?;
        let mut retry_count = 0;
        loop {
            let (f_result, session_data, storage) = {
                let mut inner = self.0.lock().unwrap();
                let f_result = f(&mut inner);
                inner.update = false;
                let list = vec![&inner.debug_zone, &inner.user_zone, &inner.guest_zone];
                let mut buf = Vec::new();
                ciborium::into_writer(&list, &mut buf)?;
                let session_data = SessionData::from((buf, inner.state_ttl, inner.version));
                let storage = Arc::clone(&inner.storage);
                (f_result, session_data, storage)
            };
            match storage.save(None, session_data).await {
                Ok(key) => {
                    let mut inner = self.0.lock().unwrap();
                    inner.status = SessionStatus::Changed;
                    inner.session_key = Some(key);
                }
                Err(SaveError::Retryable) => {
                    retry_count += 1;
                    if retry_count > MAX_RETRY_COUNT {
                        bail!("too many session renew retry");
                    }
                    continue;
                }
                Err(SaveError::RetryableWithData(_)) => {
                    bail!("unreachable error");
                }
                Err(SaveError::Other(e)) => {
                    return Err(e);
                }
            }
            return f_result;
        }
    }

    pub(crate) async fn set_session(
        req: &mut ServiceRequest,
        session_key: Option<SessionKey>,
        storage: Arc<Store>,
        configuration: &Configuration,
    ) -> Result<()> {
        let (session_key, mut data) = if let Some(session_key) = session_key {
            let data = storage.load(&session_key).await?;
            if let Some(data) = data {
                if data.ttl_as_duration().is_positive() {
                    (Some(session_key), data)
                } else {
                    (None, SessionData::default())
                }
            } else {
                (None, SessionData::default())
            }
        } else {
            (None, SessionData::default())
        };
        let mut status = SessionStatus::Unchanged;
        let ttl = configuration.session.state_ttl.whole_seconds();
        if let Some(session_key) = &session_key {
            if (ttl - data.ttl()) > (ttl >> 6) {
                data.set_ttl(configuration.session.state_ttl);
                let _ = storage.update_ttl(session_key, &data).await.map_err(|e| {
                    log::warn!("{}", e);
                });
                if configuration.cookie.max_age.is_some() {
                    status = SessionStatus::Changed;
                }
            }
        }
        let mut list: Vec<HashMap<String, Vec<u8>>> = if data.is_empty_data() {
            Vec::new()
        } else {
            ciborium::from_reader(data.data())?
        };
        let inner = SessionInner::<Store> {
            session_key,
            guest_zone: list.pop().unwrap_or_default(),
            user_zone: list.pop().unwrap_or_default(),
            debug_zone: list.pop().unwrap_or_default(),
            update: false,
            status,
            state_ttl: configuration.session.state_ttl,
            version: data.version(),
            storage,
        };
        let inner = Arc::new(Mutex::new(inner));
        req.extensions_mut().insert(inner);
        Ok(())
    }

    pub async fn reset(&self) {
        let mut inner = self.0.lock().unwrap();
        inner.status = SessionStatus::Unchanged;
        inner.session_key = None;
        inner.guest_zone.clear();
        inner.user_zone.clear();
        inner.debug_zone.clear();
        inner.version = 0;
    }

    pub async fn load(&self, key: &str) -> Result<()> {
        let mut session_key = Some(key.to_string().try_into()?);
        let (data, mut list) = {
            let storage = Arc::clone(&self.0.lock().unwrap().storage);
            let data = {
                let data = storage.load(session_key.as_ref().unwrap()).await?;
                if let Some(data) = data {
                    data
                } else {
                    session_key = None;
                    SessionData::default()
                }
            };
            let list: Vec<HashMap<String, Vec<u8>>> = if data.is_empty_data() {
                Vec::new()
            } else {
                ciborium::from_reader(data.data())?
            };
            (data, list)
        };
        let mut inner = self.0.lock().unwrap();
        inner.session_key = session_key;
        inner.guest_zone = list.pop().unwrap_or_default();
        inner.user_zone = list.pop().unwrap_or_default();
        inner.debug_zone = list.pop().unwrap_or_default();
        inner.version = data.version();
        Ok(())
    }

    pub(crate) async fn reload(&self) -> Result<()> {
        let (data, mut list) = {
            let key = self.0.lock().unwrap().session_key.as_ref().cloned();
            let storage = Arc::clone(&self.0.lock().unwrap().storage);
            let data = if let Some(session_key) = key {
                let data = storage.reload(&session_key).await?;
                if let Some(data) = data {
                    data
                } else {
                    self.0.lock().unwrap().session_key = None;
                    return Ok(());
                }
            } else {
                return Ok(());
            };
            let list: Vec<HashMap<String, Vec<u8>>> = if data.is_empty_data() {
                Vec::new()
            } else {
                ciborium::from_reader(data.data())?
            };
            (data, list)
        };
        let mut inner = self.0.lock().unwrap();
        inner.guest_zone = list.pop().unwrap_or_default();
        inner.user_zone = list.pop().unwrap_or_default();
        inner.debug_zone = list.pop().unwrap_or_default();
        inner.version = data.version();
        Ok(())
    }

    pub(crate) async fn reload_from_data(&self, data: SessionData) -> Result<()> {
        let (data, mut list) = {
            let list: Vec<HashMap<String, Vec<u8>>> = if data.is_empty_data() {
                Vec::new()
            } else {
                ciborium::from_reader(data.data())?
            };
            (data, list)
        };
        let mut inner = self.0.lock().unwrap();
        inner.guest_zone = list.pop().unwrap_or_default();
        inner.user_zone = list.pop().unwrap_or_default();
        inner.debug_zone = list.pop().unwrap_or_default();
        inner.version = data.version();
        Ok(())
    }

    pub(crate) fn get_status<B>(
        res: &mut ServiceResponse<B>,
    ) -> (SessionStatus, Option<SessionKey>) {
        if let Some(s_impl) = res
            .request()
            .extensions()
            .get::<Arc<Mutex<SessionInner<Store>>>>()
        {
            let session_key = mem::take(&mut s_impl.lock().unwrap().session_key);
            (s_impl.lock().unwrap().status, session_key)
        } else {
            (SessionStatus::Unchanged, None)
        }
    }

    pub(crate) fn get_session(extensions: &mut Extensions) -> Result<Session<Store>> {
        let s_impl = extensions
            .get::<Arc<Mutex<SessionInner<Store>>>>()
            .with_context(|| "No session is set up.")?;
        Ok(Session(Arc::clone(s_impl)))
    }
}

impl<Store: SessionStore + 'static> std::fmt::Debug for Session<Store> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner = self.0.lock().unwrap();
        let list: Vec<HashMap<&String, ciborium::Value>> = vec![
            inner
                .guest_zone
                .iter()
                .map(|(k, v)| (k, ciborium::from_reader(v.as_slice()).unwrap()))
                .collect(),
            inner
                .user_zone
                .iter()
                .map(|(k, v)| (k, ciborium::from_reader(v.as_slice()).unwrap()))
                .collect(),
            inner
                .debug_zone
                .iter()
                .map(|(k, v)| (k, ciborium::from_reader(v.as_slice()).unwrap()))
                .collect(),
        ];
        f.debug_tuple("Session")
            .field(&inner.session_key)
            .field(&list)
            .finish()
    }
}

impl<Store: SessionStore + 'static> FromRequest for Session<Store> {
    type Error = Error;
    type Future = Ready<Result<Session<Store>, Error>>;

    #[inline]
    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        ready(Session::get_session(&mut req.extensions_mut()).map_err(e500))
    }
}
