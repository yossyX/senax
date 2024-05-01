use rand::{distributions::Alphanumeric, Rng as _};
use std::convert::TryFrom;
use std::time::{SystemTime, UNIX_EPOCH};

const KEY_LENGTH: usize = 64;
const FULL_KEY_LENGTH: usize = 80;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionKey(String);

impl TryFrom<String> for SessionKey {
    type Error = anyhow::Error;

    fn try_from(val: String) -> Result<Self, Self::Error> {
        anyhow::ensure!(
            val.len() == FULL_KEY_LENGTH,
            "Session ID length is invalid."
        );
        Ok(SessionKey(val))
    }
}

impl From<SessionKey> for String {
    fn from(key: SessionKey) -> Self {
        key.0
    }
}

impl From<&SessionKey> for String {
    fn from(key: &SessionKey) -> Self {
        key.0.clone()
    }
}

impl Default for SessionKey {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SessionKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl SessionKey {
    pub fn new() -> SessionKey {
        Self::generate(SystemTime::now())
    }
    pub fn generate_past(ttl: std::time::Duration) -> SessionKey {
        Self::generate(SystemTime::now().checked_sub(ttl).unwrap())
    }
    fn generate(time: SystemTime) -> SessionKey {
        let mut rng = rand::thread_rng();
        let value = std::iter::repeat(())
            .map(|()| rng.sample(Alphanumeric))
            .take(KEY_LENGTH)
            .collect::<Vec<_>>();
        let time = (time.duration_since(UNIX_EPOCH).unwrap().as_nanos() >> 2) as u64;
        SessionKey(format!(
            "{:016X}{}",
            time,
            String::from_utf8(value).unwrap()
        ))
    }
    pub fn hash(&self) -> u32 {
        crc32fast::hash(self.0.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let key = SessionKey::new();
        let str: String = key.into();
        assert_eq!(str.len(), FULL_KEY_LENGTH);
    }
}
