#[rustfmt::skip]
pub use super::_base::_@{ mod_name }@::consts;
@%- for (enum_name, column_def) in def.num_enums(true) %@
#[rustfmt::skip]
pub use super::_base::_@{ mod_name }@::@{ enum_name|pascal }@;
@%- endfor %@
@%- for (enum_name, column_def) in def.str_enums(true) %@
#[rustfmt::skip]
pub use super::_base::_@{ mod_name }@::@{ enum_name|pascal }@;
@%- endfor %@
#[rustfmt::skip]
pub use super::_base::_@{ mod_name }@::{@{ pascal_name }@, @{ pascal_name }@Cache, @{ pascal_name }@Common, @{ pascal_name }@UpdaterBase};

impl dyn @{ pascal_name }@Common {}
pub trait @{ pascal_name }@Updater: @{ pascal_name }@UpdaterBase {}
@{-"\n"}@