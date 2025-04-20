use crate::repository::Repository;
use futures::future::BoxFuture;
use once_cell::sync::Lazy;
use std::sync::Arc;
use tokio::sync::RwLock;

pub type Handler<T> =
    Box<dyn Fn(Arc<dyn Repository>, T) -> BoxFuture<'static, anyhow::Result<T>> + Send + Sync>;

struct DomainEvent<T>(Arc<RwLock<Vec<Handler<T>>>>);
impl<T> DomainEvent<T> {
    pub async fn publish(&self, repo: Arc<dyn Repository>, mut event: T) -> anyhow::Result<T> {
        for f in self.0.read().await.iter() {
            event = f(repo.clone(), event).await?;
        }
        Ok(event)
    }
    pub async fn subscribe(&self, f: Handler<T>) {
        self.0.write().await.push(f);
    }
}
#[allow(unused_macros)]
macro_rules! event {
    ( $t:ty, $i:ident ) => {
        static $i: Lazy<DomainEvent<$t>> =
            Lazy::new(|| DomainEvent(Arc::new(RwLock::new(Vec::new()))));
        impl $t {
            pub async fn publish(self, repo: Arc<dyn Repository>) -> anyhow::Result<Self> {
                $i.publish(repo.clone(), self).await
            }
            pub async fn subscribe(f: Handler<$t>) {
                $i.subscribe(f).await
            }
        }
    };
}
#[allow(unused_macros)]
macro_rules! event_with_inner_handler {
    ( $t:ty, $i:ident ) => {
        static $i: Lazy<DomainEvent<$t>> =
            Lazy::new(|| DomainEvent(Arc::new(RwLock::new(Vec::new()))));
        impl $t {
            pub async fn publish(self, repo: Arc<dyn Repository>) -> anyhow::Result<Self> {
                let event = self.pre_handle(repo.clone()).await?;
                let event = $i.publish(repo.clone(), event).await?;
                event.post_handle(repo).await
            }
            pub async fn subscribe(f: Handler<$t>) {
                $i.subscribe(f).await
            }
        }
    };
}

// The following is sample code.
#[derive(Debug)]
pub struct UserRegistered {
    pub user_id: u64,
    pub name: String,
}
impl UserRegistered {
    // Events in the domain are written here because there is no initializer
    async fn pre_handle(self, _repo: Arc<dyn Repository>) -> anyhow::Result<Self> {
        Ok(self)
    }
    async fn post_handle(self, _repo: Arc<dyn Repository>) -> anyhow::Result<Self> {
        Ok(self)
    }
}
event_with_inner_handler!(UserRegistered, USER_REGISTERED);

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test() -> anyhow::Result<()> {
        use futures::future::FutureExt;
        UserRegistered::subscribe(Box::new(move |_repo, event| {
            async move {
                println!("{:?}", event);
                Ok(event)
            }
            .boxed()
        }))
        .await;
        let repo = Arc::new(crate::repository::MockRepository::new());
        UserRegistered {
            user_id: 1,
            name: "John Doe".to_string(),
        }
        .publish(repo.clone())
        .await?;
        Ok(())
    }
}
@{-"\n"}@