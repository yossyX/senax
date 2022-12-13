use actix_web::cookie::time::Duration;
use anyhow::Result;
use derive_more::Display;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::session_key::SessionKey;

#[derive(Default)]
pub struct SessionData {
    pub(crate) data: Vec<u8>,
    pub(crate) ttl: Duration,
    pub(crate) version: u32,
}

impl SessionData {
    pub fn new(data: &[u8], eol: u64, version: u32) -> SessionData {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        SessionData {
            data: data.to_vec(),
            ttl: Duration::new((eol as i64) - now, 0),
            version,
        }
    }
    pub fn is_empty_data(&self) -> bool {
        self.data.is_empty()
    }
    pub fn data(&self) -> &[u8] {
        &self.data
    }
    pub fn ttl(&self) -> i64 {
        self.ttl.whole_seconds()
    }
    pub fn ttl_as_duration(&self) -> Duration {
        self.ttl
    }
    pub fn eol(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        (now + self.ttl.whole_seconds()) as u64
    }
    pub fn version(&self) -> u32 {
        self.version
    }
}

#[async_trait::async_trait(?Send)]
pub trait SessionStore {
    async fn load(&self, session_key: &SessionKey) -> Result<Option<SessionData>>;
    async fn reload(&self, session_key: &SessionKey) -> Result<Option<SessionData>>;

    async fn save(&self, data: SessionData) -> Result<SessionKey, SaveError>;

    async fn update(
        &self,
        session_key: &SessionKey,
        data: SessionData,
    ) -> Result<SessionKey, SaveError>;

    async fn update_ttl(&self, session_key: &SessionKey, data: &SessionData) -> Result<()>;

    async fn delete(&self, session_key: &SessionKey) -> Result<()>;

    async fn gc(&self, start_key: &SessionKey) -> Result<()>;
}

#[derive(Debug, Display)]
pub enum SaveError {
    Retryable,
    Other(anyhow::Error),
}

impl std::error::Error for SaveError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Retryable => None,
            Self::Other(err) => Some(err.as_ref()),
        }
    }
}
