use super::_base::_@{ mod_name }@;
use crate::DbConn;
use anyhow::Result;
use senax_common::ShardId;

@% for (enum_name, column_def) in def.num_enums(false) -%@
pub use super::_base::_@{ mod_name }@::_@{ enum_name|pascal }@;
@% endfor -%@
@% for (enum_name, column_def) in def.str_enums(false) -%@
pub use super::_base::_@{ mod_name }@::_@{ enum_name|pascal }@;
@% endfor -%@
#[rustfmt::skip]
pub use super::_base::_@{ mod_name }@::{
    _@{ pascal_name }@,@% if !config.force_disable_cache %@ _@{ pascal_name }@Cache,@% endif %@ _@{ pascal_name }@Factory, _@{ pascal_name }@Updater,
    @% for id in def.id() %@@{ id_name }@, @% endfor %@_@{ pascal_name }@Info, _@{ pascal_name }@Getter,
};
@%- if config.exclude_from_domain %@
// pub use super::_base::_@{ mod_name }@::{filter, order};
@%- else %@
// pub use domain::repository::@{ db|snake|to_var_name }@::@{ group_name|to_var_name }@::@{ mod_name|to_var_name }@::{filter, order};
@%- endif %@
@%- if config.exclude_from_domain %@
// pub use super::_base::_@{ mod_name }@::{join, Joiner_};
@%- else %@
// pub use domain::repository::@{ db|snake|to_var_name }@::@{ group_name|to_var_name }@::@{ mod_name|to_var_name }@::{join, Joiner_};
@%- endif %@
@%- if def.act_as_job_queue() %@
pub use super::_base::_@{ mod_name }@::QUEUE_NOTIFIER;
@%- endif %@

impl crate::models::ModelTr<Self, _@{ mod_name }@::CacheOp> for _@{ pascal_name }@ {
    async fn __before_delete(_conn: &mut DbConn, _list: &[Self]) -> Result<()> {
        // Not called unless the use_on_delete_fn flag is true.
        Ok(())
    }
    async fn __after_delete(_list: &[Self]) {
        // Not called unless the use_on_delete_fn flag is true.
    }
    async fn __receive_update_notice(msg: &_@{ mod_name }@::CacheOp) {
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
@{-"\n"}@