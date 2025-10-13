// Do not modify below this line. (ModStart)
// Do not modify above this line. (ModEnd)

pub async fn create(
    repo: Box<dyn crate::repositories::Repository_>,
    obj: Box<dyn @{ pascal_name }@Updater>,
) -> anyhow::Result<Box<dyn @{ pascal_name }@>> {
    let @{ mod_name }@_repo = repo.@{ group_name|snake|to_var_name }@().@{ mod_name|to_var_name }@();
    #[allow(deprecated)]
    let res = @{ mod_name }@_repo.save(obj).await?;
    Ok(res.unwrap())
}
@%- if !def.disable_update() %@

pub async fn import(
    repo: Box<dyn crate::repositories::Repository_>,
    list: Vec<Box<dyn @{ pascal_name }@Updater>>,
    option: Option<domain::models::ImportOption>,
) -> anyhow::Result<()> {
    let @{ mod_name }@_repo = repo.@{ group_name|snake|to_var_name }@().@{ mod_name|to_var_name }@();
    #[allow(deprecated)]
    @{ mod_name }@_repo.import(list, option).await
}

pub async fn update<F>(
    repo: Box<dyn crate::repositories::Repository_>,
    mut obj: Box<dyn @{ pascal_name }@Updater>,
    update_updater: F,
) -> anyhow::Result<Box<dyn @{ pascal_name }@>>
where
    F: FnOnce(&mut dyn @{ pascal_name }@Updater) -> anyhow::Result<()>,
{
    let @{ mod_name }@_repo = repo.@{ group_name|snake|to_var_name }@().@{ mod_name|to_var_name }@();
    update_updater(&mut *obj)?;
    #[allow(deprecated)]
    let res = @{ mod_name }@_repo.save(obj).await?;
    Ok(res.unwrap())
}
@%- endif %@
@%- if !def.disable_delete() %@

pub async fn delete(
    repo: Box<dyn crate::repositories::Repository_>,
    obj: Box<dyn @{ pascal_name }@Updater>,
) -> anyhow::Result<()> {
    let @{ mod_name }@_repo = repo.@{ group_name|snake|to_var_name }@().@{ mod_name|to_var_name }@();
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
    // Do not modify below this line. (RepositoryMockStart)
    // Do not modify above this line. (RepositoryMockEnd)
    #[async_trait]
    impl @{ pascal_name }@Repository for Repository_ {}
}

#[cfg(any(feature = "mock", test))]
mockall::mock! {
    pub QueryService_ {}
    // Do not modify below this line. (QueryServiceMockStart)
    // Do not modify above this line. (QueryServiceMockEnd)
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