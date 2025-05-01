use base_domain as domain;
use domain::models::@{ db|snake|to_var_name }@::@{ group_name|snake|to_var_name }@::@{ mod_name|to_var_name }@::@{ pascal_name }@Updater;

#[allow(unused_imports)]
use crate::models::@{ group_name|snake|to_var_name }@::@{ mod_name|to_var_name }@::*;

impl @{ pascal_name }@Updater for _@{ pascal_name }@Updater {}
@%- for parent in def.parents() %@
impl domain::models::@{ db|snake|to_var_name }@::@{ parent.group_name|to_var_name }@::@{ parent.name|to_var_name }@::@{ parent.name|pascal }@Updater for _@{ pascal_name }@Updater {}
@%- endfor %@
@{-"\n"}@