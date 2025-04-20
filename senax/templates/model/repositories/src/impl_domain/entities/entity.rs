use async_trait::async_trait;
use domain::repository::@{ db|snake|to_var_name }@::@{ group_name|to_var_name }@::@{ mod_name|to_var_name }@::{@{ pascal_name }@QueryService, @{ pascal_name }@Repository};
pub use super::_base::_@{ mod_name }@::@{ pascal_name }@RepositoryImpl;

#[async_trait]
impl @{ pascal_name }@Repository for @{ pascal_name }@RepositoryImpl {}

#[async_trait]
impl @{ pascal_name }@QueryService for @{ pascal_name }@RepositoryImpl {}
@{-"\n"}@