#[allow(unused_imports)]
use crate as domain;
#[allow(unused_imports)]
use crate::models::@{ db|snake|to_var_name }@ as _model_;
#[allow(unused_imports)]
use crate::value_objects;
@%- for (name, rel_def) in def.belongs_to_outer_db() %@
#[allow(unused_imports)]
pub use crate::models::@{ rel_def.db()|snake|to_var_name }@ as _@{ rel_def.db()|snake }@_model_;
@%- endfor %@

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
@%- for id in def.id() %@
#[rustfmt::skip]
pub use super::_base::_@{ mod_name }@::@{ id_name }@;
@%- endfor %@
#[rustfmt::skip]
pub use super::_base::_@{ mod_name }@::{@{ pascal_name }@, @{ pascal_name }@Cache, @{ pascal_name }@Common};
#[rustfmt::skip]
pub use super::_base::_@{ mod_name }@::{self, @{ pascal_name }@Primary, @{ pascal_name }@UpdaterBase};
#[cfg(any(feature = "mock", test))]
#[rustfmt::skip]
pub use super::_base::_@{ mod_name }@::@{ pascal_name }@Entity;

pub const MODEL_ID: u64 = @{ model_id }@;

impl dyn @{ pascal_name }@Common {}
pub trait @{ pascal_name }@Updater: @{ pascal_name }@UpdaterBase {}
@%- for parent in def.parents() %@
#[cfg(any(feature = "mock", test))]
impl super::super::@{ parent.group_name|to_var_name }@::@{ parent.name|to_var_name }@::@{ parent.name|pascal }@Updater for @{ pascal_name }@Entity {}
@%- endfor %@

#[cfg(any(feature = "mock", test))]
impl @{ pascal_name }@Updater for @{ pascal_name }@Entity {}
@{-"\n"}@