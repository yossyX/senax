use async_trait::async_trait;
use domain::repository::@{ db|snake|ident }@::@{ base_group_name|snake|ident }@::_super::@{ group_name|snake|ident }@::@{ mod_name|ident }@::{@{ pascal_name }@QueryService, @{ pascal_name }@Repository};
pub use super::_base::_@{ mod_name }@::@{ pascal_name }@RepositoryImpl;

#[async_trait]
impl @{ pascal_name }@Repository for @{ pascal_name }@RepositoryImpl {}

#[async_trait]
impl @{ pascal_name }@QueryService for @{ pascal_name }@RepositoryImpl {}
@{-"\n"}@