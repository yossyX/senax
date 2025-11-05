#[allow(unused_imports)]
use actix_web::cookie::Cookie;
use actix_web::HttpRequest;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use jsonwebtoken::{DecodingKey, Validation};
use once_cell::sync::{Lazy, OnceCell};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::context::Ctx;

pub static INNER_KEY: Lazy<String> = Lazy::new(|| {
    format!(
        "{}{}{}{}{}",
        obfstr::obfstr!("@{ Secret::secret_key(1) }@"),
        @{ Secret::secret_no(1) }@u64,
        obfstr::obfstr!("@{ Secret::secret_key(2) }@"),
        @{ Secret::secret_no(2) }@u64,
        obfstr::obfstr!("@{ Secret::secret_key(3) }@")
    )
});
pub static SECRET: OnceCell<String> = OnceCell::new();

#[derive(
    async_graphql::Enum,
    Debug,
    PartialEq,
    Eq,
    Copy,
    Clone,
    Default,
    Deserialize,
    Serialize,
    derive_more::Display,
)]
#[graphql(name = "_Role")]
#[derive(utoipa::ToSchema)]
#[schema(as = _Role)]
#[allow(non_camel_case_types)]
pub enum Role {
    // Do not modify below this line. (RoleStart)
    // Do not modify above this line. (RoleEnd)
}
#[allow(non_snake_case)]
impl Role {
    // Do not modify below this line. (ImplRoleStart)
    // Do not modify above this line. (ImplRoleEnd)
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuthInfoInner {
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub role: Role,
    #[serde(default)]
    pub exp: usize,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuthInfo(pub Arc<AuthInfoInner>);
impl std::ops::Deref for AuthInfo {
    type Target = AuthInfoInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl AuthInfo {
    pub fn retrieve(req: &HttpRequest) -> Option<AuthInfo> {
        use actix_web::HttpMessage;
        let extensions = req.extensions();
        extensions.get::<AuthInfo>().cloned()
    }
    pub fn username(&self) -> &str {
        &self.username
    }
    #[allow(dead_code)]
    pub fn role(&self) -> Option<Role> {
        // if self.role == Role::_None {
        //     return None;
        // }
        Some(self.role)
    }
    #[allow(dead_code)]
    pub fn has_role(&self, roles: &[Role]) -> Option<bool> {
        if roles.is_empty() {
            return Some(true);
        }
        let role = self.role()?;
        Some(roles.iter().any(|v| *v == role))
    }
}

#[cfg(debug_assertions)]
fn get_cookie_string_from_header(req: &HttpRequest) -> Option<String> {
    let cookie_header = req.headers().get("cookie");
    if let Some(v) = cookie_header {
        let cookie_string = v.to_str().unwrap();
        return Some(String::from(cookie_string));
    }
    None
}
#[cfg(debug_assertions)]
fn get_cookie_value(key: &str, cookie_string: String) -> Option<String> {
    let kv: Vec<&str> = cookie_string.split(';').collect();
    for c in kv {
        match Cookie::parse(c) {
            Ok(kv) => {
                if key == kv.name() {
                    return Some(String::from(kv.value()));
                }
            }
            Err(e) => {
                println!("cookie parse error. -> {}", e);
            }
        }
    }
    None
}
pub fn retrieve_auth(http_req: &HttpRequest) -> Option<AuthInfo> {
    #[cfg(debug_assertions)]
    {
        let cookie_string = get_cookie_string_from_header(http_req);
        if let Some(s) = cookie_string {
            if let Some(v) = get_cookie_value("jwt", s) {
                use base64::{engine::general_purpose::URL_SAFE, Engine as _};
                if let Ok(v) = URL_SAFE.decode(&v) {
                    if let Ok(v) = String::from_utf8(v) {
                        return Some(serde_json::from_str(&v).unwrap());
                    }
                }
            }
        }
    }
    let ctx = Ctx::get(http_req);
    if let Some(auth_header) = http_req.headers().get("Authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                let token_data = jsonwebtoken::decode::<AuthInfo>(
                    token,
                    &DecodingKey::from_secret(SECRET.get().unwrap().as_bytes()),
                    &Validation::default(),
                );
                match token_data {
                    Ok(token_data) => {
                        return Some(token_data.claims);
                    }
                    Err(e) => {
                        warn!(ctx = ctx.ctx_no(); "Illegal JWT Token: {}", e);
                    }
                }
            } else {
                warn!(ctx = ctx.ctx_no(); "Not Bearer Authorization");
            }
        }
    } else {
        warn!(ctx = ctx.ctx_no(); "Authorization Not Found");
    }
    None
}

#[cfg(test)]
#[allow(dead_code)]
pub fn create_jwt(username: String, role: Role) -> String {
    let exp = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::hours(24))
        .expect("valid timestamp")
        .timestamp() as usize;

    jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &AuthInfo(
            AuthInfoInner {
                username,
                role,
                exp,
            }
            .into(),
        ),
        &jsonwebtoken::EncodingKey::from_secret(SECRET.get().unwrap().as_bytes()),
    )
    .unwrap()
}

// Appropriate when FIPS 140-2 is not required
#[allow(dead_code)]
pub async fn hash_password(password: String) -> anyhow::Result<String> {
    Ok(actix_web::web::block(move || {
        let salt = SaltString::generate(&mut OsRng);
        argon2()
            .hash_password(password.as_bytes(), &salt)
            .expect("password hash error")
            .to_string()
    })
    .await?)
}

#[allow(dead_code)]
pub async fn check_password(password: String, pw_hash: String) -> anyhow::Result<bool> {
    Ok(actix_web::web::block(move || {
        let parsed_hash = PasswordHash::new(&pw_hash).expect("check password error");
        argon2()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok()
    })
    .await?)
}

fn argon2() -> Argon2<'static> {
    Argon2::new_with_secret(
        SECRET.get().unwrap().as_bytes(),
        argon2::Algorithm::default(),
        argon2::Version::default(),
        argon2::Params::new(65536, 3, 4, None).expect("argon2 error"),
    )
    .expect("argon2 error")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hash_password() -> anyhow::Result<()> {
        let _ = SECRET.set("n7Ut0RNihZe5Ys3yYUNnMBHSMKBbs2sYSWJZHcF7".to_string());
        let password = "hunter42";
        let pw_hash = hash_password(password.to_string()).await?;
        assert!(check_password(password.to_string(), pw_hash).await?);
        Ok(())
    }
}
@{-"\n"}@