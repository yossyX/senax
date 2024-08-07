use anyhow::Result;
use derive_more::Display;
use std::time::{SystemTime, UNIX_EPOCH};
use time::Duration;
use zstd::{decode_all, stream::copy_encode};

use crate::session::SessionKey;

#[derive(Default, Clone)]
pub struct SessionData {
    pub(crate) data: Vec<u8>,
    pub(crate) ttl: Duration,
    pub(crate) version: u32,
}

impl SessionData {
    pub fn new(data: &[u8], eol: u64, version: u32) -> SessionData {
        let data = if data.len() > 1 && data[0] == 0 {
            decode_all(&data[1..]).unwrap_or_default()
        } else {
            data.to_vec()
        };
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        SessionData {
            data,
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
    pub fn compressed_data(&self) -> Vec<u8> {
        let mut enc = Vec::<u8>::with_capacity(self.data.len() + 100);
        enc.push(0);
        copy_encode(&*self.data, &mut enc, 1).unwrap();
        if enc.len() < self.data.len() {
            enc
        } else {
            self.data.clone()
        }
    }
    pub fn ttl(&self) -> i64 {
        self.ttl.whole_seconds()
    }
    pub fn ttl_as_duration(&self) -> Duration {
        self.ttl
    }
    pub fn set_ttl(&mut self, ttl: Duration) {
        self.ttl = ttl;
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
    pub fn set_version(&mut self, version: u32) {
        self.version = version;
    }
}

impl From<(Vec<u8>, Duration, u32)> for SessionData {
    fn from(v: (Vec<u8>, Duration, u32)) -> Self {
        Self {
            data: v.0,
            ttl: v.1,
            version: v.2,
        }
    }
}

#[async_trait::async_trait]
pub trait SessionStore {
    async fn load(&self, session_key: &SessionKey) -> Result<Option<SessionData>>;
    async fn reload(&self, session_key: &SessionKey) -> Result<Option<SessionData>>;
    async fn save(
        &self,
        session_key: Option<SessionKey>,
        data: SessionData,
    ) -> Result<SessionKey, SaveError>;
    async fn update_ttl(&self, session_key: &SessionKey, data: &SessionData) -> Result<()>;
    async fn delete(&self, session_key: &SessionKey) -> Result<()>;
    async fn gc(&self, start_key: &SessionKey) -> Result<()>;
}

#[derive(Display)]
#[display(fmt = "")]
pub enum SaveError {
    Retryable,
    RetryableWithData(SessionData),
    Other(anyhow::Error),
}
impl std::fmt::Debug for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SaveError").finish()
    }
}

impl std::error::Error for SaveError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Retryable => None,
            Self::RetryableWithData(_) => None,
            Self::Other(err) => Some(err.as_ref()),
        }
    }
}
