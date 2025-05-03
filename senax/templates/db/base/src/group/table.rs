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