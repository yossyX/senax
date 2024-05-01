use async_trait::async_trait;
use domain::models::@{ db|snake|to_var_name }@::@{ group_name|to_var_name }@::@{ mod_name|to_var_name }@::{@{ pascal_name }@Query, @{ pascal_name }@Repository, @{ pascal_name }@Updater};

#[allow(unused_imports)]
use crate::misc::Updater as _;
#[allow(unused_imports)]
use crate::models::@{ group_name|to_var_name }@::@{ mod_name|to_var_name }@::*;
#[allow(unused_imports)]
use crate::DbConn;

pub use super::_base::_@{ mod_name }@::@{ pascal_name }@RepositoryImpl;

impl @{ pascal_name }@Updater for _@{ pascal_name }@Updater {}
@%- for parent in def.parents() %@
impl domain::models::@{ db|snake|to_var_name }@::@{ parent.group_name|to_var_name }@::@{ parent.name|to_var_name }@::@{ parent.name|pascal }@Updater for _@{ pascal_name }@Updater {}
@%- endfor %@

#[async_trait]
impl @{ pascal_name }@Repository for @{ pascal_name }@RepositoryImpl {}

#[async_trait]
impl @{ pascal_name }@Query for @{ pascal_name }@RepositoryImpl {}
@{-"\n"}@