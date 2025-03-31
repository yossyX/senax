use super::_base::_@{ mod_name }@;
use crate::DbConn;
use anyhow::Result;
use senax_common::ShardId;

@% for (enum_name, column_def) in def.num_enums(false) -%@
pub@{ visibility }@ use super::_base::_@{ mod_name }@::_@{ enum_name|pascal }@;
@% endfor -%@
@% for (enum_name, column_def) in def.str_enums(false) -%@
pub@{ visibility }@ use super::_base::_@{ mod_name }@::_@{ enum_name|pascal }@;
@% endfor -%@
#[rustfmt::skip]
pub@{ visibility }@ use super::_base::_@{ mod_name }@::{
    _@{ pascal_name }@,@% if !config.force_disable_cache %@ _@{ pascal_name }@Cache,@% endif %@ _@{ pascal_name }@Factory, _@{ pascal_name }@Updater,
    @% for id in def.id() %@@{ id_name }@, @{ id_name }@Fetcher, @% endfor %@_@{ pascal_name }@Info, _@{ pascal_name }@Joiner, _@{ pascal_name }@Getter, UnionBuilder as _@{ pascal_name }@UnionBuilder,
};
@%- if config.excluded_from_domain %@
pub@{ visibility }@ use super::_base::_@{ mod_name }@::{filter, order};
@%- else %@
pub@{ visibility }@ use domain::models::@{ db|snake|to_var_name }@::@{ group_name|to_var_name }@::@{ mod_name|to_var_name }@::{filter, order};
@%- endif %@
@%- if config.excluded_from_domain %@
pub@{ visibility }@ use super::_base::_@{ mod_name }@::{join, Joiner_};
@%- else %@
pub@{ visibility }@ use domain::models::@{ db|snake|to_var_name }@::@{ group_name|to_var_name }@::@{ mod_name|to_var_name }@::{join, Joiner_};
@%- endif %@
@%- if def.act_as_job_queue() %@
pub use super::_base::_@{ mod_name }@::QUEUE_NOTIFIER;
@%- endif %@
@%- if def.act_as_session() %@

// Session Keys
pub const SESSION_ROLE: &str = "role";
@%- endif %@

impl _@{ pascal_name }@ {
    pub(crate) async fn _before_delete(_conn: &mut DbConn, _list: &[Self]) -> Result<()> {
        // Not called unless the use_on_delete_fn flag is true.
        Ok(())
    }
    pub(crate) async fn _after_delete(_list: &[Self]) {
        // Not called unless the use_on_delete_fn flag is true.
    }
    pub(crate) async fn _receive_update_notice(msg: &_@{ mod_name }@::CacheOp) {
        // Since the cache update lock is being acquired, it must be processed in the shortest possible time.
    }
}

impl _@{ pascal_name }@Factory {
    /// used by seeder
    pub async fn _shard_id(&self) -> ShardId {
        0
    }
}

impl std::fmt::Display for _@{ pascal_name }@ {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}
@%- if !config.force_disable_cache %@

impl std::fmt::Display for _@{ pascal_name }@Cache {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}
@%- endif %@
@%- if def.act_as_session() %@

use senax_common::session::{
    interface::{SaveError, SessionData, SessionStore},
    SessionKey,
};
const EOL_SHIFT: usize = 3;

use arc_swap::ArcSwapOption;
use crossbeam::queue::SegQueue;
use fxhash::FxHashMap;
use once_cell::sync::Lazy;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct SaveData {
    is_new: bool,
    key: SessionKey,
    data: SessionData,
    result: std::sync::Mutex<Option<Result<SessionKey, SaveError>>>,
}

async fn save_data(
    shard_id: usize,
    mut update_map: FxHashMap<_@{ pascal_name }@Id, Vec<Arc<SaveData>>>,
    new_list: Vec<Arc<SaveData>>,
) -> Result<()> {
    let update_ids: Vec<&_@{ pascal_name }@Id> = update_map.keys().collect();
    let mut conn = DbConn::_new(shard_id as ShardId);
    conn.begin().await?;
    let list = _@{ pascal_name }@::query()
        .filter(filter!(key IN update_ids))
        .skip_locked()
        .select_for_update(&mut conn)
        .await?;
    let mut updater_map = _@{ pascal_name }@::updater_list_to_map(list);
    let mut save_list = Vec::new();
    let mut update_list = Vec::new();
    for (id, mut l) in update_map.into_iter() {
        if let Some(mut updater) = updater_map.remove(&id.into()) {
            let save_data = l.pop().unwrap();
            if save_data.data.version() == updater._@{ ConfigDef::version() }@() {
                updater.mut_data().set(save_data.data.compressed_data());
                updater
                    .mut_eol()
                    .set((save_data.data.eol() >> EOL_SHIFT) as u32);
                let mut data = save_data.data.clone();
                data.set_version(updater._@{ ConfigDef::version() }@().wrapping_add(1));
                save_list.push(updater);
                update_list.push(save_data);
                for d in l.iter_mut() {
                    let mut result = d.result.lock().unwrap();
                    *result = Some(Err(SaveError::RetryableWithData(data.clone())));
                }
            } else {
                let data = updater._data();
                let eol = updater._eol();
                let version = updater._@{ ConfigDef::version() }@();
                let data = SessionData::new(data, (eol as u64) << EOL_SHIFT, version);
                save_data
                    .result
                    .lock()
                    .unwrap()
                    .replace(Err(SaveError::RetryableWithData(data.clone())));
                for d in l.iter_mut() {
                    d.result
                        .lock()
                        .unwrap()
                        .replace(Err(SaveError::RetryableWithData(data.clone())));
                }
            }
        } else {
            for d in l.iter_mut() {
                d.result.lock().unwrap().replace(Err(SaveError::Retryable));
            }
        }
    }
    for save_data in &new_list {
        let key: String = (&save_data.key).into();
        let session = _@{ pascal_name }@Factory {
            key: key.into(),
            data: save_data.data.compressed_data().into(),
            eol: (save_data.data.eol() >> EOL_SHIFT) as u32,
        }
        .create();
        save_list.push(session);
    }
    if save_list.is_empty() {
        conn.rollback().await?;
        return Ok(());
    }
    _@{ pascal_name }@::bulk_overwrite(&mut conn, save_list).await?;
    conn.commit().await?;
    for data in new_list {
        data.result.lock().unwrap().replace(Ok(data.key.clone()));
    }
    for data in update_list {
        data.result.lock().unwrap().replace(Ok(data.key.clone()));
    }
    Ok(())
}

fn calc_shard_id(key: &SessionKey) -> usize {
    let shard_num = DbConn::shard_num();
    if shard_num == 1 {
        0
    } else {
        (key.hash() as usize) % DbConn::shard_num()
    }
}

pub struct _@{ pascal_name }@Store;
#[async_trait::async_trait]
impl SessionStore for _@{ pascal_name }@Store {
    async fn load(&self, session_key: &SessionKey) -> Result<Option<SessionData>> {
        let conn = DbConn::_new(calc_shard_id(session_key) as ShardId);
        let id: String = session_key.into();
        let session = _@{ pascal_name }@::find_optional_from_cache(&conn, id)
            .await?
            .map(|s| SessionData::new(s._data(), (s._eol() as u64) << EOL_SHIFT, s._@{ ConfigDef::version() }@()));
        Ok(session)
    }

    async fn reload(&self, session_key: &SessionKey) -> Result<Option<SessionData>> {
        let mut conn = DbConn::_new(calc_shard_id(session_key) as ShardId);
        let id: String = session_key.into();
        let session = _@{ pascal_name }@::find_optional(&mut conn, id, None)
            .await?
            .map(|s| SessionData::new(s._data(), (s._eol() as u64) << EOL_SHIFT, s._@{ ConfigDef::version() }@()));
        Ok(session)
    }

    async fn save(
        &self,
        session_key: Option<SessionKey>,
        data: SessionData,
    ) -> Result<SessionKey, SaveError> {
        const VSHARDING: usize = 1;
        use core::sync::atomic::{AtomicUsize, Ordering};
        static VSHARD: AtomicUsize = AtomicUsize::new(0);
        static SAVE_DATA_SYNC: Lazy<Vec<Mutex<()>>> = Lazy::new(|| {
            (0..(DbConn::shard_num() << VSHARDING))
                .map(|_| Mutex::new(()))
                .collect()
        });
        static SAVE_DATA_QUEUE: Lazy<Vec<SegQueue<Arc<SaveData>>>> = Lazy::new(|| {
            (0..(DbConn::shard_num() << VSHARDING))
                .map(|_| SegQueue::new())
                .collect()
        });

        let buf = Arc::new(SaveData {
            is_new: session_key.is_none(),
            key: session_key.unwrap_or_default(),
            data,
            result: std::sync::Mutex::new(None),
        });
        let mut vshard_id = calc_shard_id(&buf.key) << VSHARDING;
        vshard_id += VSHARD.fetch_add(1, Ordering::Relaxed) & ((1 << VSHARDING) - 1);
        SAVE_DATA_QUEUE[vshard_id].push(Arc::clone(&buf));
        let _lock = SAVE_DATA_SYNC[vshard_id].lock().await;
        let result = buf.result.lock().unwrap().take();
        if let Some(result) = result {
            return result;
        }
        let mut update_map: FxHashMap<_@{ pascal_name }@Id, Vec<Arc<SaveData>>> = FxHashMap::default();
        let mut new_list: Vec<Arc<SaveData>> = Vec::new();
        let mut contains = false;
        while let Some(x) = SAVE_DATA_QUEUE[vshard_id].pop() {
            contains = contains || Arc::ptr_eq(&buf, &x);
            if x.is_new {
                new_list.push(x);
            } else {
                let key: String = (&x.key).into();
                update_map
                    .entry(key.into())
                    .or_default()
                    .push(x);
            }
        }
        let key: String = (&buf.key).into();
        if !contains {
            if buf.is_new {
                new_list.push(buf.clone());
            } else {
                let key: String = (&buf.key).into();
                update_map
                    .entry(key.into())
                    .or_default()
                    .push(buf.clone());
            }
        }
        save_data(vshard_id >> VSHARDING, update_map, new_list)
            .await
            .map_err(SaveError::Other)?;
        let mut result = buf.result.lock().unwrap();
        result.take().unwrap_or(Err(SaveError::Retryable))
    }

    async fn update_ttl(&self, session_key: &SessionKey, data: &SessionData) -> Result<()> {
        let mut conn = DbConn::_new(calc_shard_id(session_key) as ShardId);
        let s_key: String = session_key.into();
        let id: _@{ pascal_name }@Id = s_key.into();
        let mut session = id.updater();
        session.mut_eol().set((data.eol() >> EOL_SHIFT) as u32);
        session.mut_@{ ConfigDef::updated_at() }@().skip_and_empty();
        _@{ pascal_name }@::update_delayed(&mut conn, session).await
    }

    async fn delete(&self, session_key: &SessionKey) -> Result<()> {
        let mut conn = DbConn::_new(calc_shard_id(session_key) as ShardId);
        let s_key: String = session_key.into();
        let id: _@{ pascal_name }@Id = s_key.into();
        let mut session = id.updater();
        session.mut_eol().set(0);
        _@{ pascal_name }@::update_delayed(&mut conn, session).await
    }

    async fn gc(&self, start_key: &SessionKey) -> Result<()> {
        for shard_id in DbConn::shard_num_range() {
            let key = start_key.clone();
            tokio::spawn(async move {
                if let Err(err) = gc(shard_id, key).await {
                    log::error!("{}", err);
                }
            });
        }
        Ok(())
    }
}

async fn gc(shard_id: ShardId, start_key: SessionKey) -> Result<()> {
    let mut conn = DbConn::_new(shard_id);
    conn.begin_without_transaction().await?;
    let s_key: String = start_key.into();
    let eol = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        >> EOL_SHIFT) as u32;
    let mut filter = filter!((key < s_key) AND (eol < eol));
    const LIMIT: usize = 1000;
    loop {
        if _@{ pascal_name }@::query()
            .filter(filter.clone())
            .limit(LIMIT)
            .force_delete(&mut conn)
            .await?
            < LIMIT as u64
        {
            break;
        }
    }
    conn.end_of_without_transaction().await?;
    Ok(())
}
@%- endif %@
@{-"\n"}@