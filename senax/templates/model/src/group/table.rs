@% for (enum_name, column_def) in def.enums() -%@
pub use super::base::_@{ mod_name }@::_@{ enum_name|pascal }@;
@% endfor -%@
@% for (enum_name, column_def) in def.db_enums() -%@
pub use super::base::_@{ mod_name }@::_@{ enum_name|pascal }@;
@% endfor -%@
#[rustfmt::skip]
pub use super::base::_@{ mod_name }@::{
    self, _@{ name|pascal }@, _@{ name|pascal }@Cache, _@{ name|pascal }@Factory, _@{ name|pascal }@ForUpdate,@% for id in def.id() %@ @{ id_name }@, @{ id_name }@Tr,@% endfor %@ _@{ name|pascal }@Info, _@{ name|pascal }@Rel, _@{ name|pascal }@Tr, _@{ name|pascal }@MutTr,
};

use crate::DbConn;
use anyhow::Result;
use senax_common::ShardId;

impl _@{ name|pascal }@ {
    pub(crate) async fn _before_delete(_conn: &mut DbConn, _list: &[Self]) -> Result<()> {
        // Not called unless the on_delete_fn flag is true.
        Ok(())
    }
    pub(crate) async fn _after_delete(_list: &[Self]) {
        // Not called unless the on_delete_fn flag is true.
    }
    pub(crate) async fn _receive_update_notice(msg: &super::base::_@{ mod_name }@::CacheOp) {}
}

impl _@{ name|pascal }@Factory {
    /// used by seeder
    pub async fn _shard_id(&self) -> ShardId {
        0
    }
}

impl std::fmt::Display for _@{ name|pascal }@ {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl std::fmt::Display for _@{ name|pascal }@Cache {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}
@%- if def.as_session() %@

use senax_actix_session::{
    interface::{SaveError, SessionData, SessionStore},
    SessionKey,
};
const EOL_SHIFT: usize = 3;
#[rustfmt::skip]
pub const SESSION_SECRET_KEY: &[u8] = &[
    @{ def.session_secret_key(1) }@,
    @{ def.session_secret_key(2) }@,
    @{ def.session_secret_key(3) }@,
    @{ def.session_secret_key(4) }@,
];
pub struct _@{ name|pascal }@Store;
#[async_trait::async_trait(?Send)]
impl SessionStore for _@{ name|pascal }@Store {
    async fn load(&self, session_key: &SessionKey) -> Result<Option<SessionData>> {
        let conn = DbConn::new();
        let id: String = session_key.into();
        let session = _@{ name|pascal }@::find_optional_from_cache(&conn, id)
            .await?
            .map(|s| SessionData::new(s.data(), (s.eol() as u64) << EOL_SHIFT, s._version()));
        Ok(session)
    }

    async fn reload(&self, session_key: &SessionKey) -> Result<Option<SessionData>> {
        let mut conn = DbConn::new();
        let id: String = session_key.into();
        let session = _@{ name|pascal }@::find_optional(&mut conn, id)
            .await?
            .map(|s| SessionData::new(s.data(), (s.eol() as u64) << EOL_SHIFT, s._version()));
        Ok(session)
    }

    async fn save(&self, data: SessionData) -> Result<SessionKey, SaveError> {
        let mut conn = DbConn::new();
        conn.begin_without_transaction().await.unwrap();
        let key = SessionKey::new();
        let s_key: String = (&key).into();
        let session = _@{ name|pascal }@Factory {
            key: s_key.into(),
            data: data.data().into(),
            eol: (data.eol() >> EOL_SHIFT) as u32,
        }
        .create(&conn);
        match _@{ name|pascal }@::save(&mut conn, session).await {
            Ok(_) => Ok(key),
            Err(e) => {
                if let Some(err) = e.downcast_ref::<sqlx::Error>() {
                    match err {
                        sqlx::Error::Database(..) => Err(SaveError::Retryable),
                        _ => Err(SaveError::Other(e)),
                    }
                } else {
                    Err(SaveError::Other(e))
                }
            }
        }
    }

    async fn update(
        &self,
        session_key: &SessionKey,
        data: SessionData,
    ) -> Result<SessionKey, SaveError> {
        let mut conn = DbConn::new();
        conn.begin_without_transaction().await.unwrap();
        let s_key: String = session_key.into();
        let id: _@{ name|pascal }@Id = s_key.into();
        let mut session = id.for_update(&conn);
        session.data().set(data.data().into());
        session.eol().set((data.eol() >> EOL_SHIFT) as u32);
        session._version().set(data.version());
        match _@{ name|pascal }@::save(&mut conn, session).await {
            Ok(_) => Ok(session_key.clone()),
            Err(e) => {
                if e.is::<senax_common::err::RowNotFound>() {
                    Err(SaveError::Retryable)
                } else {
                    Err(SaveError::Other(e))
                }
            }
        }
    }

    async fn update_ttl(&self, session_key: &SessionKey, data: &SessionData) -> Result<()> {
        let mut conn = DbConn::new();
        let s_key: String = session_key.into();
        let id: _@{ name|pascal }@Id = s_key.into();
        let mut session = id.for_update(&conn);
        session.eol().set((data.eol() >> EOL_SHIFT) as u32);
        session.updated_at().skip_update();
        _@{ name|pascal }@::update_delayed(&mut conn, session).await
    }

    async fn delete(&self, session_key: &SessionKey) -> Result<()> {
        let mut conn = DbConn::new();
        let s_key: String = session_key.into();
        let id: _@{ name|pascal }@Id = s_key.into();
        let mut session = id.for_update(&conn);
        session.eol().set(0);
        _@{ name|pascal }@::update_delayed(&mut conn, session).await
    }

    async fn gc(&self, start_key: &SessionKey) -> Result<()> {
        let mut conn = DbConn::new();
        conn.begin_without_transaction().await.unwrap();
        let s_key: String = start_key.into();
        let eol = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            >> EOL_SHIFT) as u32;
        let mut cond = _session::Cond::new_and();
        cond.add(_session::Cond::Lt(_session::ColOne::key(s_key.into())));
        cond.add(_session::Cond::Lt(_session::ColOne::eol(eol)));
        loop {
            _@{ name|pascal }@::query()
                .cond(cond.clone())
                .order_by(vec![_session::OrderBy::Asc(_session::Col::key)])
                .limit(1000)
                .force_delete(&mut conn)
                .await?;
            if _@{ name|pascal }@::query()
                .cond(cond.clone())
                .count(&mut conn)
                .await?
                == 0
            {
                return Ok(());
            }
        }
    }
}
@%- endif %@
@{-"\n"}@