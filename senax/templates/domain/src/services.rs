use async_trait::async_trait;

// An example of uses of services 
#[cfg_attr(any(feature = "mock", test), mockall::automock)]
#[async_trait]
pub trait Notify {
    async fn register_user(&self, user_name: &str, email: &str);
}
@{-"\n"}@