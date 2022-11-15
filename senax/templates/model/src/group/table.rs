@% for (enum_name, column_def) in def.enums() -%@
pub use super::base::_@{ mod_name }@::_@{ enum_name|pascal }@;
@% endfor -%@
@% for (enum_name, column_def) in def.db_enums() -%@
pub use super::base::_@{ mod_name }@::_@{ enum_name|pascal }@;
@% endfor -%@
#[rustfmt::skip]
pub use super::base::_@{ mod_name }@::{
    self, _@{ name|pascal }@, _@{ name|pascal }@Cache, _@{ name|pascal }@Factory, _@{ name|pascal }@ForUpdate,@% for id in def.id() %@ @{ id_name }@, @{ id_name }@Tr,@% endfor %@ _@{ name|pascal }@Info, _@{ name|pascal }@Rel, _@{ name|pascal }@Tr,
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
@{-"\n"}@