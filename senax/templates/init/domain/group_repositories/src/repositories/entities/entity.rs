use async_trait::async_trait;
#[allow(unused_imports)]
use base_domain as domain;
#[allow(unused_imports)]
use base_domain::models::@{ db|snake|to_var_name }@ as _model_;
@%- for (name, rel_def) in def.belongs_to_outer_db() %@
#[allow(unused_imports)]
pub use base_domain::models::@{ rel_def.db()|to_var_name }@ as _@{ rel_def.db() }@_model_;
@%- endfor %@
@%- for (enum_name, column_def) in def.num_enums(true) %@
#[rustfmt::skip]
pub use base_domain::models::@{ db|snake|to_var_name }@::@{ group_name|snake|to_var_name }@::@{ mod_name|to_var_name }@::@{ enum_name|pascal }@;
@%- endfor %@
@%- for (enum_name, column_def) in def.str_enums(true) %@
#[rustfmt::skip]
pub use base_domain::models::@{ db|snake|to_var_name }@::@{ group_name|snake|to_var_name }@::@{ mod_name|to_var_name }@::@{ enum_name|pascal }@;
@%- endfor %@
#[rustfmt::skip]
pub use super::_base::_@{ mod_name }@::{join, Joiner_};
#[rustfmt::skip]
pub use super::_base::_@{ mod_name }@::{filter, order, Filter_};
pub use super::_base::_@{ mod_name }@::@{ pascal_name }@Factory;
use super::_base::_@{ mod_name }@::{_@{ pascal_name }@QueryService, _@{ pascal_name }@Repository};
pub use base_domain::models::@{ db|snake|to_var_name }@::@{ group_name|snake|to_var_name }@::@{ mod_name|to_var_name }@::consts;
#[rustfmt::skip]
pub use base_domain::models::@{ db|snake|to_var_name }@::@{ group_name|snake|to_var_name }@::@{ mod_name|to_var_name }@::{
    @{ pascal_name }@, @{ pascal_name }@Cache, @{ pascal_name }@Common, @{ pascal_name }@Updater,
};
@%- for id in def.id() %@
#[rustfmt::skip]
pub use base_domain::models::@{ db|snake|to_var_name }@::@{ group_name|snake|to_var_name }@::@{ mod_name|to_var_name }@::@{ id_name }@;
@%- endfor %@
#[rustfmt::skip]
pub use base_domain::models::@{ db|snake|to_var_name }@::@{ group_name|snake|to_var_name }@::@{ mod_name|to_var_name }@::@{ pascal_name }@Primary;
#[rustfmt::skip]
pub use super::_base::_@{ mod_name }@::{_@{ pascal_name }@QueryFindBuilder, _@{ pascal_name }@RepositoryFindBuilder};
#[cfg(any(feature = "mock", test))]
pub use base_domain::models::@{ db|snake|to_var_name }@::@{ group_name|snake|to_var_name }@::@{ mod_name|to_var_name }@::@{ pascal_name }@Entity;
@%- for (selector, selector_def) in def.selectors %@
#[rustfmt::skip]
pub use super::_base::_@{ mod_name }@::@{ pascal_name }@Repository@{ selector|pascal }@Builder;
@%- endfor %@
@%- for (selector, selector_def) in def.selectors %@
#[rustfmt::skip]
pub use super::_base::_@{ mod_name }@::{@{ pascal_name }@Query@{ selector|pascal }@Builder, @{ pascal_name }@Query@{ selector|pascal }@Cursor, @{ pascal_name }@Query@{ selector|pascal }@Filter, @{ pascal_name }@Query@{ selector|pascal }@Order};
@%- endfor %@
#[cfg(any(feature = "mock", test))]
pub use self::{MockQueryService_ as Mock@{ pascal_name }@QueryService, MockRepository_ as Mock@{ pascal_name }@Repository};
#[cfg(any(feature = "mock", test))]
pub use super::_base::_@{ mod_name }@::Emu@{ pascal_name }@Repository;

pub async fn create(
    repo: &dyn super::@{ group_name|pascal }@Repository,
    obj: Box<dyn @{ pascal_name }@Updater>,
) -> anyhow::Result<Box<dyn @{ pascal_name }@>> {
    let @{ mod_name }@_repo = repo.@{ mod_name|to_var_name }@();
    #[allow(deprecated)]
    let res = @{ mod_name }@_repo.save(obj).await?;
    Ok(res.unwrap())
}
@%- if !def.disable_update() %@

pub async fn import(
    repo: &dyn super::@{ group_name|pascal }@Repository,
    list: Vec<Box<dyn @{ pascal_name }@Updater>>,
    option: Option<domain::models::ImportOption>,
) -> anyhow::Result<()> {
    let @{ mod_name }@_repo = repo.@{ mod_name|to_var_name }@();
    #[allow(deprecated)]
    @{ mod_name }@_repo.import(list, option).await
}

pub async fn update<F>(
    repo: &dyn super::@{ group_name|pascal }@Repository,
    mut obj: Box<dyn @{ pascal_name }@Updater>,
    update_updater: F,
) -> anyhow::Result<Box<dyn @{ pascal_name }@>>
where
    F: FnOnce(&mut dyn @{ pascal_name }@Updater) -> anyhow::Result<()>,
{
    let @{ mod_name }@_repo = repo.@{ mod_name|to_var_name }@();
    update_updater(&mut *obj)?;
    #[allow(deprecated)]
    let res = @{ mod_name }@_repo.save(obj).await?;
    Ok(res.unwrap())
}
@%- endif %@
@%- if !def.disable_delete() %@

pub async fn delete(
    repo: &dyn super::@{ group_name|pascal }@Repository,
    obj: Box<dyn @{ pascal_name }@Updater>,
) -> anyhow::Result<()> {
    let @{ mod_name }@_repo = repo.@{ mod_name|to_var_name }@();
    #[allow(deprecated)]
    @{ mod_name }@_repo.delete(obj).await
}
@%- endif %@

#[async_trait]
pub trait @{ pascal_name }@Repository: _@{ pascal_name }@Repository {}

#[async_trait]
pub trait @{ pascal_name }@QueryService: _@{ pascal_name }@QueryService {}

#[cfg(any(feature = "mock", test))]
mockall::mock! {
    pub Repository_ {}
    #[async_trait]
    impl _@{ pascal_name }@Repository for Repository_ {
        @%- if !def.disable_update() %@
        fn find(&self, id: @{ def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") }@) -> Box<dyn _@{ pascal_name }@RepositoryFindBuilder>;
        @%- endif %@
        fn convert_factory(&self, factory: @{ pascal_name }@Factory) -> Box<dyn @{ pascal_name }@Updater>;
        async fn save(&self, obj: Box<dyn @{ pascal_name }@Updater>) -> anyhow::Result<Option<Box<dyn @{ pascal_name }@>>>;
        @%- if !def.disable_update() %@
        async fn import(&self, list: Vec<Box<dyn @{ pascal_name }@Updater>>, option: Option<domain::models::ImportOption>) -> anyhow::Result<()>;
        @%- endif %@
        @%- if def.use_insert_delayed() %@
        async fn insert_delayed(&self, obj: Box<dyn @{ pascal_name }@Updater>) -> anyhow::Result<()>;
        @%- endif %@
        @%- if !def.disable_delete() %@
        async fn delete(&self, obj: Box<dyn @{ pascal_name }@Updater>) -> anyhow::Result<()>;
        @%- if def.primaries().len() == 1 %@
        async fn delete_by_ids(&self, ids: &[@{ def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") }@]) -> anyhow::Result<u64>;
        @%- endif %@
        async fn delete_all(&self) -> anyhow::Result<()>;
        @%- endif %@
        @%- if def.act_as_job_queue() %@
        async fn fetch(&self, limit: usize) -> anyhow::Result<Vec<Box<dyn @{ pascal_name }@Updater>>>;
        @%- endif %@
        @%- for (selector, selector_def) in def.selectors %@
        fn @{ selector|to_var_name }@(&self) -> Box<dyn @{ pascal_name }@Repository@{ selector|pascal }@Builder>;
        @%- endfor %@
    }
    #[async_trait]
    impl @{ pascal_name }@Repository for Repository_ {}
}

#[cfg(any(feature = "mock", test))]
mockall::mock! {
    pub QueryService_ {}
    #[async_trait]
    impl _@{ pascal_name }@QueryService for QueryService_ {
        @%- if def.use_all_rows_cache() && !def.use_filtered_row_cache() %@
        async fn all(&self) -> anyhow::Result<Box<dyn base_domain::models::EntityIterator<dyn @{ pascal_name }@Cache>>>;
        @%- endif %@
        @%- for (selector, selector_def) in def.selectors %@
        fn @{ selector|to_var_name }@(&self) -> Box<dyn @{ pascal_name }@Query@{ selector|pascal }@Builder>;
        @%- endfor %@
        fn find(&self, id: @{ def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") }@) -> Box<dyn _@{ pascal_name }@QueryFindBuilder>;
    }
    #[async_trait]
    impl @{ pascal_name }@QueryService for QueryService_ {}
}

#[cfg(any(feature = "mock", test))]
#[async_trait]
impl @{ pascal_name }@Repository for Emu@{ pascal_name }@Repository {}

#[cfg(any(feature = "mock", test))]
#[async_trait]
impl @{ pascal_name }@QueryService for Emu@{ pascal_name }@Repository {}
@{-"\n"}@