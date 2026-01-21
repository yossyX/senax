#[allow(unused_imports)]
use actix_web::post;
#[allow(unused_imports)]
use anyhow::Context as _;
#[allow(unused_imports)]
use async_graphql::types::connection as graphql_conn;
use async_graphql::ErrorExtensions as _;
#[allow(unused_imports)]
use async_graphql::GuardExt as _;
use domain::repository::Repository as _;
#[allow(unused_imports)]
use domain::models::FromRawValue as _;
#[allow(unused_imports)]
use domain::value_objects;
#[allow(unused_imports)]
use senax_common::types::blob::{ApiToBlob as _, BlobToApi as _};
#[allow(unused_imports)]
use serde::{Deserialize, Serialize};
#[allow(unused_imports)]
use std::collections::HashMap;
#[allow(unused_imports)]
use validator::Validate as _;

use ::_@{ server_name|snake }@ as _server_;
use _server_::db::RepositoryImpl;
use _server_::{auth::AuthInfo, auto_api::GqlError};
#[allow(unused_imports)]
use _server_::{
    auth::Role,
    auto_api::{NoGuard, RoleGuard},
};

// Do not modify below this line. (GqlModelStart)
// Do not modify above this line. (GqlModelEnd)

async fn find(
    gql_ctx: &async_graphql::Context<'_>,
    repo: Box<dyn _QueryService>,
    auth: &AuthInfo,
    primary: &_domain_::@{ pascal_name }@Primary,
) -> anyhow::Result<Option<Box<dyn _domain_::@{ pascal_name }@>>> {
    repo.begin_read_tx().await?;
    let @{ mod_name }@_repo = repo.@{ group|ident }@().@{ mod_name|ident }@();
    let look_ahead = gql_ctx.look_ahead();
    let result = @{ mod_name }@_repo
        .find(primary.clone().into_inner())
        .join(joiner(&look_ahead)?)
        .with_filter_flag("_readable", readable_filter(auth)?)
        .with_filter_flag_when(look_ahead.field("_updatable").exists(), "_updatable", updatable_filter(auth)?)
        .with_filter_flag_when(look_ahead.field("_deletable").exists(), "_deletable", deletable_filter(auth)?)
        .query()
        .await;
    repo.release_read_tx().await?;
    result
}
@%- if !api_def.disable_mutation %@
@%- if !def.disable_update() %@

async fn find_for_update(
    gql_ctx: &async_graphql::Context<'_>,
    repo: Box<dyn _Repository>,
    auth: &AuthInfo,
    primary: &_domain_::@{ pascal_name }@Primary,
) -> anyhow::Result<Option<Box<dyn _domain_::@{ pascal_name }@>>> {
    let @{ mod_name }@_repo = repo.@{ mod_name|ident }@();
    let look_ahead = gql_ctx.look_ahead();
    @{ mod_name }@_repo
        .find(primary.clone().into_inner())
        .join(joiner(&look_ahead)?)
        .with_filter_flag("_readable", updatable_filter(auth)?)
        .with_filter_flag_when(look_ahead.field("_updatable").exists(), "_updatable", updatable_filter(auth)?)
        .with_filter_flag_when(look_ahead.field("_deletable").exists(), "_deletable", deletable_filter(auth)?)
        .query()
        .await
}
@%- endif %@
@%- if !def.disable_delete() %@

async fn delete(
    repo: Box<dyn _Repository>,
    auth: &AuthInfo,
    primary: _domain_::@{ mod_name|pascal }@Primary,
) -> anyhow::Result<()> {
    let @{ mod_name }@_repo = repo.@{ mod_name|ident }@();
    let mut query = @{ mod_name }@_repo.find(primary.into_inner());
    query = query.with_filter_flag("_deletable", deletable_filter(auth)?);
    match query.query_for_update().await {
        Ok(obj) => {
            if !domain::models::FilterFlag::get_flag(obj.as_ref(), "_deletable").unwrap_or_default() {
                anyhow::bail!(GqlError::Forbidden);
            }
            _repository_::delete(repo.as_ref().into(), obj).await?;
        }
        Err(e) => {
            if !e.is::<senax_common::err::RowNotFound>() {
                return Err(e);
            }
        }
    }
    Ok(())
}
@%- endif %@
@%- endif %@

pub struct GqlQuery@{ graphql_name }@;
#[async_graphql::Object]
impl GqlQuery@{ graphql_name }@ {
    #[graphql(name = "_permission")]
    async fn _permission(
        &self,
        gql_ctx: &async_graphql::Context<'_>,
    ) -> async_graphql::Result<Vec<&'static str>> {
        use async_graphql::Guard;
        let mut permission = Vec::new();
        if query_guard().check(gql_ctx).await.is_ok() {
            permission.push("query");
        }
        @%- if !api_def.disable_mutation %@
        if create_guard().check(gql_ctx).await.is_ok() {
            permission.push("create");
        }
        @%- if !def.disable_update() %@
        @%- if api_def.enable_import %@
        if import_guard().check(gql_ctx).await.is_ok() {
            permission.push("import");
        }
        @%- endif %@
        if update_guard().check(gql_ctx).await.is_ok() {
            permission.push("update");
        }
        if delete_guard().check(gql_ctx).await.is_ok() {
            permission.push("delete");
        }
        @%- endif %@
        @%- endif %@
        Ok(permission)
    }
    @%- if def.enable_all_rows_cache() && !def.enable_filtered_rows_cache() %@

    #[graphql(guard = "query_guard()")]
    async fn all(
        &self,
        gql_ctx: &async_graphql::Context<'_>,
    ) -> async_graphql::Result<Vec<ResObj>> {
        let repo = RepositoryImpl::new_with_ctx(gql_ctx.data()?);
        let auth: &AuthInfo = gql_ctx.data()?;
        let @{ db|snake }@_query = repo.@{ db|snake }@_query();
        @{ db|snake }@_query.begin_read_tx().await?;
        let @{ mod_name }@_repo = @{ db|snake }@_query.@{ group|ident }@().@{ mod_name|ident }@();
        let list = @{ mod_name }@_repo
            .all()
            .await
            .map_err(|e| GqlError::server_error(gql_ctx, e))?;
        @{ db|snake }@_query.release_read_tx().await?;
        let result: anyhow::Result<Vec<_>> = list
            .iter()
            .map(|v| ResObj::try_from_(v, None))
            .collect();
        Ok(result?)
    }
    @%- endif %@
    @%- if api_def.enable_find_by_pk %@

    #[graphql(guard = "query_guard()")]
    async fn find_by_pk(
        &self,
        gql_ctx: &async_graphql::Context<'_>,
        @%- if camel_case %@
        @{- def.primaries()|fmt_join("
        {ident}: {inner},", "") }@
        @%- else %@
        @{- def.primaries()|fmt_join("
        #[graphql(name = \"{raw_name}\")] {ident}: {inner},", "") }@
        @%- endif %@
    ) -> async_graphql::Result<ResObj> {
        let repo = RepositoryImpl::new_with_ctx(gql_ctx.data()?);
        let auth: &AuthInfo = gql_ctx.data()?;
        let primary: _domain_::@{ pascal_name }@Primary = @{ def.primaries()|fmt_join_with_paren("{ident}", ", ") }@.into();
        crate::gql_find!(find(gql_ctx, repo.@{ db|snake }@_query(), auth, &primary), repo, gql_ctx)
    }
    @%- endif %@

    #[graphql(guard = "query_guard()")]
    async fn find(
        &self,
        gql_ctx: &async_graphql::Context<'_>,
        #[graphql(name = "_id")] _id: async_graphql::ID,
    ) -> async_graphql::Result<ResObj> {
        let repo = RepositoryImpl::new_with_ctx(gql_ctx.data()?);
        let auth: &AuthInfo = gql_ctx.data()?;
        let primary: _domain_::@{ pascal_name }@Primary = (&_id).try_into()?;
        crate::gql_find!(find(gql_ctx, repo.@{ db|snake }@_query(), auth, &primary), repo, gql_ctx)
    }
    @%- for (selector, selector_def) in def.selectors %@
    @%- for api_selector_def in api_def.selector(selector) %@

    #[allow(clippy::too_many_arguments)]
    #[rustfmt::skip]
    #[graphql(guard = "query_guard()")]
    async fn @{ selector|ident }@(
        &self,
        gql_ctx: &async_graphql::Context<'_>,
        after: Option<String>,
        before: Option<String>,
        @{ api_selector_def.limit_validator() }@first: Option<i32>,
        @{ api_selector_def.limit_validator() }@last: Option<i32>,
        @%- if selector_def.filter_is_required() %@
        filter: _repository_::@{ pascal_name }@Query@{ selector|pascal }@Filter,
        @%- else %@
        filter: Option<_repository_::@{ pascal_name }@Query@{ selector|pascal }@Filter>,
        @%- endif %@
        order: Option<_repository_::@{ pascal_name }@Query@{ selector|pascal }@Order>,
        offset: Option<usize>,
    ) -> async_graphql::Result<graphql_conn::Connection<String, ResObj>> {
        use graphql_conn::Edge;
        @%- if selector_def.filter_is_required() %@
        filter.validate().map_err(|e| GqlError::ValidationError(e).extend())?;
        @%- else %@
        if let Some(filter) = &filter {
            filter
                .validate()
                .map_err(|e| GqlError::ValidationError(e).extend())?;
        }
        @%- endif %@

        #[allow(unused_imports)]
        #[allow(clippy::let_unit_value)]
        async fn _fetch(
            gql_ctx: &async_graphql::Context<'_>,
            repo: &RepositoryImpl,
            auth: &AuthInfo,
            after: &Option<String>,
            before: &Option<String>,
            first: Option<usize>,
            last: Option<usize>,
            @%- if selector_def.filter_is_required() %@
            filter: &_repository_::@{ pascal_name }@Query@{ selector|pascal }@Filter,
            @%- else %@
            filter: &Option<_repository_::@{ pascal_name }@Query@{ selector|pascal }@Filter>,
            @%- endif %@
            order: _repository_::@{ pascal_name }@Query@{ selector|pascal }@Order,
            offset: Option<usize>,
        ) -> anyhow::Result<(Vec<Box<dyn _domain_::@{ pascal_name }@>>, bool, Option<usize>)> {
            use domain::models::Cursor;
            let @{ db|snake }@_query = repo.@{ db|snake }@_query();
            @{ db|snake }@_query.begin_read_tx().await?;
            let @{ mod_name }@_repo = @{ db|snake }@_query.@{ group|ident }@().@{ mod_name|ident }@();
            let node = if gql_ctx.look_ahead().field("nodes").exists() {
                gql_ctx.look_ahead().field("nodes")
            } else {
                gql_ctx.look_ahead().field("edges").field("node")
            };
            let joiner = joiner(&node)?;
            let mut query = @{ mod_name }@_repo.@{ selector|ident }@().join(joiner);
            @%- if selector_def.filter_is_required() %@
            query = query.selector_filter(filter.clone());
            @%- else %@
            if let Some(filter) = filter {
                query = query.selector_filter(filter.clone());
            }
            @%- endif %@
            query = query
                .extra_filter(readable_filter(auth)?)
                .with_filter_flag_when(node.field("_updatable").exists(), "_updatable", updatable_filter(auth)?)
                .with_filter_flag_when(node.field("_deletable").exists(), "_deletable", deletable_filter(auth)?);
            let mut previous = false;
            @{- api_selector_def.limit_def() }@
            let mut limit = @{ api_selector_def.limit_str() }@;
            query = query.order_by(order);
            if first.is_some() || after.is_some() {
                previous = after.is_some();
                match order {
                    @%- for (order, order_def) in selector_def.orders %@
                    _repository_::@{ pascal_name }@Query@{ selector|pascal }@Order::@{ order|pascal }@ => {
                        if let Some(after) = after {
                            let c = _repository_::@{ pascal_name }@Query@{ selector|pascal }@Cursor::@{ order }@_from_str(
                                after,
                            )?;
                            query = query.cursor(
                                _repository_::@{ pascal_name }@Query@{ selector|pascal }@Cursor::@{ order|pascal }@(
                                    Cursor::After(c),
                                ),
                            );
                        }
                    }
                    @%- endfor %@
                    _repository_::@{ pascal_name }@Query@{ selector|pascal }@Order::_None => {}
                }
                if first.is_some() {
                    limit = first@{ api_selector_def.check_limit() }@;
                }
            }
            if last.is_some() || before.is_some() {
                previous = before.is_some();
                match order {
                    @%- for (order, order_def) in selector_def.orders %@
                    _repository_::@{ pascal_name }@Query@{ selector|pascal }@Order::@{ order|pascal }@ => {
                        if let Some(before) = before {
                            let c = _repository_::@{ pascal_name }@Query@{ selector|pascal }@Cursor::@{ order }@_from_str(
                                before,
                            )?;
                            query = query.cursor(
                                _repository_::@{ pascal_name }@Query@{ selector|pascal }@Cursor::@{ order|pascal }@(
                                    Cursor::Before(c),
                                ),
                            )
                            .reverse(true);
                        }
                    }
                    @%- endfor %@
                    _repository_::@{ pascal_name }@Query@{ selector|pascal }@Order::_None => {}
                }
                if last.is_some() {
                    limit = last@{ api_selector_def.check_limit() }@;
                }
            }
            if let Some(offset) = offset {
                previous = previous || offset > 0;
                query = query.offset(offset);
            }
            if let Some(limit) = limit {
                query = query.limit(limit + 1);
            }
            let list = query.query().await?;
            @{ db|snake }@_query.release_read_tx().await?;
            Ok((list, previous, limit))
        }

        graphql_conn::query(
            after,
            before,
            first,
            last,
            |after: Option<String>, before: Option<String>, first, last| async move {
                let repo = RepositoryImpl::new_with_ctx(gql_ctx.data()?);
                let auth: &AuthInfo = gql_ctx.data()?;
                let order = order.unwrap_or_default();
                let (mut list, previous, limit) = crate::gql_selector!(
                    _fetch(
                        gql_ctx, &repo, auth, &after, &before, first, last, &filter, order, offset,
                    ),
                    repo,
                    gql_ctx
                );
                let mut connection = graphql_conn::Connection::new(previous, limit.map(|l| list.len() > l).unwrap_or(false));
                if let Some(limit) = limit {
                    list.truncate(limit);
                }
                if last.is_some() {
                    list.reverse();
                }
                let connection = tokio::task::spawn_blocking(move || {
                    connection.edges.extend(list.into_iter().map(|obj| {
                        let cursor = order.to_cursor(&obj);
                        Edge::new(
                            cursor.clone().unwrap_or_default(),
                            ResObj::try_from_(&*obj, cursor).unwrap(),
                        )
                    }));
                    connection
                })
                .await?;
                Ok::<_, async_graphql::Error>(connection)
            },
        )
        .await
    }

    #[graphql(guard = "query_guard()")]
    async fn count_@{ selector }@(
        &self,
        gql_ctx: &async_graphql::Context<'_>,
        @%- if selector_def.filter_is_required() %@
        filter: _repository_::@{ pascal_name }@Query@{ selector|pascal }@Filter,
        @%- else %@
        filter: Option<_repository_::@{ pascal_name }@Query@{ selector|pascal }@Filter>,
        @%- endif %@
    ) -> async_graphql::Result<i64> {
        @%- if selector_def.filter_is_required() %@
        filter.validate().map_err(|e| GqlError::ValidationError(e).extend())?;
        @%- else %@
        if let Some(filter) = &filter {
            filter
                .validate()
                .map_err(|e| GqlError::ValidationError(e).extend())?;
        }
        @%- endif %@
        let repo = RepositoryImpl::new_with_ctx(gql_ctx.data()?);
        let auth: &AuthInfo = gql_ctx.data()?;

        async fn _count(
            repo: &RepositoryImpl,
            auth: &AuthInfo,
            @%- if selector_def.filter_is_required() %@
            filter: &_repository_::@{ pascal_name }@Query@{ selector|pascal }@Filter,
            @%- else %@
            filter: &Option<_repository_::@{ pascal_name }@Query@{ selector|pascal }@Filter>,
            @%- endif %@
        ) -> anyhow::Result<i64> {
            let @{ db|snake }@_query = repo.@{ db|snake }@_query();
            @{ db|snake }@_query.begin_read_tx().await?;
            let @{ mod_name }@_repo = @{ db|snake }@_query.@{ group|ident }@().@{ mod_name|ident }@();
            let mut query = @{ mod_name }@_repo.@{ selector|ident }@();
            @%- if selector_def.filter_is_required() %@
            query = query.selector_filter(filter.clone());
            @%- else %@
            if let Some(filter) = filter {
                query = query.selector_filter(filter.clone());
            }
            @%- endif %@
            query = query.extra_filter(readable_filter(auth)?);
            let count = query.count().await?;
            @{ db|snake }@_query.release_read_tx().await?;
            Ok(count)
        }

        crate::gql_count!(_count(&repo, auth, &filter), repo, gql_ctx)
    }
    @%- endfor %@
    @%- endfor %@
}

pub struct GqlMutation@{ graphql_name }@;
#[async_graphql::Object]
impl GqlMutation@{ graphql_name }@ {
    @%- if !api_def.disable_mutation %@
    @%- if !def.disable_update() %@
    @%- if api_def.enable_find_by_pk %@

    #[graphql(guard = "update_guard()")]
    async fn find_for_update_by_pk(
        &self,
        gql_ctx: &async_graphql::Context<'_>,
        @%- if camel_case %@
        @{- def.primaries()|fmt_join("
        {ident}: {inner},", "") }@
        @%- else %@
        @{- def.primaries()|fmt_join("
        #[graphql(name = \"{raw_name}\")] {ident}: {inner},", "") }@
        @%- endif %@
    ) -> async_graphql::Result<ResObj> {
        let repo: &RepositoryImpl = gql_ctx.data()?;
        let auth: &AuthInfo = gql_ctx.data()?;
        let primary: _domain_::@{ pascal_name }@Primary = @{ def.primaries()|fmt_join_with_paren("{ident}", ", ") }@.into();
        crate::gql_find!(find_for_update(gql_ctx, repo.@{ db|snake }@_repository().@{ group|ident }@(), auth, &primary), repo, gql_ctx)
    }
    @%- endif %@

    #[graphql(guard = "update_guard()")]
    async fn find_for_update(
        &self,
        gql_ctx: &async_graphql::Context<'_>,
        #[graphql(name = "_id")] _id: async_graphql::ID,
    ) -> async_graphql::Result<ResObj> {
        let repo: &RepositoryImpl = gql_ctx.data()?;
        let auth: &AuthInfo = gql_ctx.data()?;
        let primary: _domain_::@{ pascal_name }@Primary = (&_id).try_into()?;
        crate::gql_find!(find_for_update(gql_ctx, repo.@{ db|snake }@_repository().@{ group|ident }@(), auth, &primary), repo, gql_ctx)
    }
    @%- endif %@

    #[graphql(guard = "create_guard()")]
    async fn create(
        &self,
        gql_ctx: &async_graphql::Context<'_>,
        data: ReqObj,
        check_only: Option<bool>,
    ) -> async_graphql::Result<ResObj> {
        let repo: &RepositoryImpl = gql_ctx.data()?;
        let @{ group|snake }@_repo = repo.@{ db|snake }@_repository().@{ group|ident }@();
        let auth: &AuthInfo = gql_ctx.data()?;
        data.validate()
            .map_err(|e| GqlError::ValidationError(e).extend())?;
        let entity = create_entity(data, @{ group|snake }@_repo.as_ref(), auth);
        let creatable = @{ group|snake }@_repo.@{ mod_name|ident }@()
            .query_virtual_row(&entity, creatable_filter(auth)?)
            .await
            .map_err(|e| GqlError::server_error(gql_ctx, e))?;
        if !creatable {
            return Err(GqlError::Forbidden.extend());
        }
        if check_only.unwrap_or_default() {
            return Ok(ResObj::try_from_(&*entity, None)?);
        }
        let obj = _repository_::create(@{ group|snake }@_repo.as_ref().into(), entity)
            .await
            .map_err(|e| GqlError::server_error(gql_ctx, e))?;
        Ok(ResObj::try_from_(&*obj, None)?)
    }
    @%- if !def.disable_update() %@
    @%- if api_def.enable_import %@

    #[graphql(guard = "import_guard()")]
    async fn import(
        &self,
        gql_ctx: &async_graphql::Context<'_>,
        list: Vec<ReqObj>,
        @%- if !def.has_auto_primary() %@
        option: Option<domain::models::ImportOption>,
        @%- endif %@
        check_only: Option<bool>,
    ) -> async_graphql::Result<bool> {
        let repo: &RepositoryImpl = gql_ctx.data()?;
        let auth: &AuthInfo = gql_ctx.data()?;
        let mut errors = std::collections::BTreeMap::new();
        for (idx, data) in list.iter().enumerate() {
            if let Err(e) = data.validate() {
                errors.insert(idx + 1, e);
            }
        }
        if !errors.is_empty() {
            return Err(GqlError::ValidationErrorList(errors).extend());
        }
        let @{ group|snake }@_repo = repo.@{ db|snake }@_repository().@{ group|ident }@();
        @%- if def.has_auto_primary() %@
        let mut create_list = Vec::new();
        for (idx, data) in list.into_iter().enumerate() {
            #[allow(clippy::manual_map)]
            let primary: Option<_domain_::@{ pascal_name }@Primary> = if let Some(_id) = &data._id {
                Some(_id.try_into()?)
            } else if @{ def.primaries()|fmt_join_auto_or_not("let Some({ident}) = data.{ident}{clone}", "let {ident} = data.{ident}{clone}", " && ") }@ {
                Some(@{ def.primaries()|fmt_join_with_paren("{ident}", ", ") }@.into())
            } else {
                None
            };
            if let Some(primary) = primary {
                let query = @{ group|snake }@_repo.@{ mod_name|ident }@().find(primary.into_inner()).with_filter_flag("_updatable", updatable_filter(auth)?);
                match query.join(updater_joiner()).query_for_update().await {
                    Ok(obj) => {
                        if !domain::models::FilterFlag::get_flag(obj.as_ref(), "_updatable").unwrap_or_default() {
                            let mut e = validator::ValidationErrors::new();
                            e.add("_", validator::ValidationError::new("forbidden"));
                            errors.insert(idx + 1, e);
                        } else {
                            _repository_::update(@{ group|snake }@_repo.as_ref().into(), obj, |obj| update_updater(&mut *obj, data, @{ group|snake }@_repo.as_ref(), auth))
                                .await
                                .map_err(|e| GqlError::server_error(gql_ctx, e))?;
                        }
                    }
                    Err(e) => {
                        if e.is::<senax_common::err::RowNotFound>() {
                            let mut e = validator::ValidationErrors::new();
                            e.add("_", validator::ValidationError::new("not_found"));
                            errors.insert(idx + 1, e);
                        } else {
                            return Err(GqlError::server_error(gql_ctx, e));
                        }
                    }
                }
            } else {
                let entity = create_entity(data, @{ group|snake }@_repo.as_ref(), auth);
                let creatable = @{ group|snake }@_repo.@{ mod_name|ident }@()
                    .query_virtual_row(&entity, creatable_filter(auth)?)
                    .await
                    .map_err(|e| GqlError::server_error(gql_ctx, e))?;
                if creatable {
                    create_list.push(entity);
                } else {
                    let mut e = validator::ValidationErrors::new();
                    e.add("_", validator::ValidationError::new("forbidden"));
                    errors.insert(idx + 1, e);
                }
            }
        }
        if !errors.is_empty() {
            return Err(GqlError::ValidationErrorList(errors).extend());
        }
        if check_only.unwrap_or_default() {
            return Ok(true);
        }
        if !create_list.is_empty() {
            _repository_::import(@{ group|snake }@_repo.as_ref().into(), create_list, None).await
                .map_err(|e| GqlError::server_error(gql_ctx, e))?;
        }
        @%- else %@
        let list = create_list(list, @{ group|snake }@_repo.as_ref(), auth);
        for (idx, entity) in list.iter().enumerate() {
            let creatable = @{ group|snake }@_repo.@{ mod_name|ident }@()
                .query_virtual_row(&entity, creatable_filter(auth)?)
                .await
                .map_err(|e| GqlError::server_error(gql_ctx, e))?;
            if !creatable {
                let mut e = validator::ValidationErrors::new();
                e.add("_", validator::ValidationError::new("forbidden"));
                errors.insert(idx + 1, e);
            }
        }
        if !errors.is_empty() {
            return Err(GqlError::ValidationErrorList(errors).extend());
        }
        if check_only.unwrap_or_default() {
            return Ok(true);
        }
        _repository_::import(@{ group|snake }@_repo.as_ref().into(), list, option)
            .await
            .map_err(|e| GqlError::server_error(gql_ctx, e))?;
        @%- endif %@
        Ok(true)
    }
    @%- endif %@

    #[graphql(guard = "update_guard()")]
    async fn update(
        &self,
        gql_ctx: &async_graphql::Context<'_>,
        data: ReqObj,
        check_only: Option<bool>,
    ) -> async_graphql::Result<ResObj> {
        let repo: &RepositoryImpl = gql_ctx.data()?;
        let auth: &AuthInfo = gql_ctx.data()?;
        data.validate()
            .map_err(|e| GqlError::ValidationError(e).extend())?;
        #[allow(irrefutable_let_patterns)]
        let primary: _domain_::@{ pascal_name }@Primary = if let Some(_id) = &data._id {
            _id.try_into()?
        } else if @{ def.primaries()|fmt_join_auto_or_not("let Some({ident}) = data.{ident}{clone}", "let {ident} = data.{ident}{clone}", " && ") }@ {
            @{ def.primaries()|fmt_join_with_paren("{ident}", ", ") }@.into()
        } else {
            let mut e = validator::ValidationErrors::new();
            e.add("_id", validator::ValidationError::new("required"));
            return Err(GqlError::ValidationError(e).extend());
        };
        let @{ group|snake }@_repo = repo.@{ db|snake }@_repository().@{ group|ident }@();
        let mut query = @{ group|snake }@_repo.@{ mod_name|ident }@().find(primary.into_inner());
        query = query.with_filter_flag("_updatable", updatable_filter(auth)?);
        let obj = query
            .join(updater_joiner())
            .query_for_update()
            .await
            .map_err(|e| GqlError::server_error(gql_ctx, e))?;
        if !domain::models::FilterFlag::get_flag(obj.as_ref(), "_updatable").unwrap_or_default() {
            return Err(GqlError::Forbidden.extend());
        }
        @%- if def.versioned %@
        if let Some(v) = data.@{ version_col }@ && v != obj.@{ version_col }@() {
            return Err(GqlError::VersionMismatch.extend());
        }
        @%- endif %@
        if check_only.unwrap_or_default() {
            return Ok(ResObj::try_from_(&*obj, None)?);
        }
        let obj = _repository_::update(@{ group|snake }@_repo.as_ref().into(), obj, |obj| update_updater(&mut *obj, data, @{ group|snake }@_repo.as_ref(), auth))
            .await
            .map_err(|e| GqlError::server_error(gql_ctx, e))?;
        Ok(ResObj::try_from_(&*obj, None)?)
    }
    @%- for (selector, selector_def) in def.selectors %@
    @%- for api_selector_def in api_def.selector(selector) %@
    @%- for (js_name, js_def) in api_selector_def.js_updater %@

    #[cfg(feature = "js_updater")]
    #[graphql(guard = "update_guard()")]
    async fn @{ js_name }@(
        &self,
        gql_ctx: &async_graphql::Context<'_>,
        @%- if selector_def.filter_is_required() %@
        filter: _repository_::@{ pascal_name }@Query@{ selector|pascal }@Filter,
        @%- else %@
        filter: Option<_repository_::@{ pascal_name }@Query@{ selector|pascal }@Filter>,
        @%- endif %@
        value: serde_json::Value,
        #[graphql(default = false)] create_if_empty: bool,
    ) -> async_graphql::Result<Vec<ResObj>> {
        @%- if selector_def.filter_is_required() %@
        filter.validate().map_err(|e| GqlError::ValidationError(e).extend())?;
        @%- else %@
        if let Some(filter) = &filter {
            filter
                .validate()
                .map_err(|e| GqlError::ValidationError(e).extend())?;
        }
        @%- endif %@
        let repo: &RepositoryImpl = gql_ctx.data()?;
        if create_if_empty {
            repo.@{ db|snake }@_repository()
                .lock(&format!("@{ group }@.@{ mod_name }@.{}", serde_json::to_string(&filter)?), 10)
                .await?;
        }
        let auth: &AuthInfo = gql_ctx.data()?;
        let @{ group|snake }@_repo = repo.@{ db|snake }@_repository().@{ group|ident }@();
        let mut query = @{ group|snake }@_repo.@{ mod_name|ident }@().@{ selector|ident }@().join(updater_joiner());
        @%- if selector_def.filter_is_required() %@
        query = query.selector_filter(filter);
        @%- else %@
        if let Some(filter) = filter {
            query = query.selector_filter(filter);
        }
        @%- endif %@
        query = query.extra_filter(updatable_filter(auth)?);
        let mut updater_map: HashMap<async_graphql::ID, _> = query
            .query_for_update()
            .await
            .map_err(|e| GqlError::server_error(gql_ctx, e))?
            .into_iter()
            .map(|v| ((&*v).into(), v))
            .collect();
        let list: Vec<String> = updater_map
            .iter_mut()
            .map(|(_, v)| serde_json::to_string(&ReqObj::from(&mut **v))?)
            .collect();

        let mut result = Vec::new();
        let script = @{ js_def.esc_script() }@;
        if create_if_empty && list.is_empty() {
            let list: Vec<String> =
                crate::auto_api::js_update(script, vec!["null".to_string()], value, auth)
                    .await
                    .map_err(|e| GqlError::server_error(gql_ctx, e))?;
            for row in list {
                let data: Option<ReqObj> = serde_json::from_str(&row)?;
                if let Some(data) = data {
                    data.validate()
                        .map_err(|e| GqlError::ValidationError(e).extend())?;
                    let obj = _repository_::create(@{ group|snake }@_repo.as_ref().into(), create_entity(data, @{ group|snake }@_repo.as_ref(), auth))
                        .await
                        .map_err(|e| GqlError::server_error(gql_ctx, e))?;
                    result.push(ResObj::try_from_(&*obj, None)?);
                }
            }
        } else {
            let list: Vec<String> = crate::auto_api::js_update(script, list, value, auth)
                .await
                .map_err(|e| GqlError::server_error(gql_ctx, e))?;
            for row in list {
                let data: Option<ReqObj> = serde_json::from_str(&row)?;
                if let Some(data) = data {
                    data.validate()
                        .map_err(|e| GqlError::ValidationError(e).extend())?;
                    if let Some(obj) = updater_map.remove(data._id.as_ref()?) {
                        let obj = _repository_::update(@{ group|snake }@_repo.as_ref().into(), obj, |obj| {
                                update_updater(&mut *obj, data, @{ group|snake }@_repo.as_ref(), auth)
                            })
                            .await
                            .map_err(|e| GqlError::server_error(gql_ctx, e))?;
                        result.push(ResObj::try_from_(&*obj, None)?);
                    }
                }
            }
        }
        Ok(result)
    }
    @%- endfor %@
    @%- if api_selector_def.enable_update_by_operator %@

    #[graphql(guard = "update_guard()")]
    async fn update_by_@{ selector }@(
        &self,
        gql_ctx: &async_graphql::Context<'_>,
        @%- if selector_def.filter_is_required() %@
        filter: _repository_::@{ pascal_name }@Query@{ selector|pascal }@Filter,
        @%- else %@
        filter: Option<_repository_::@{ pascal_name }@Query@{ selector|pascal }@Filter>,
        @%- endif %@
        operator: serde_json::Value,
    ) -> async_graphql::Result<Vec<ResObj>> {
        @%- if selector_def.filter_is_required() %@
        filter.validate().map_err(|e| GqlError::ValidationError(e).extend())?;
        @%- else %@
        if let Some(filter) = &filter {
            filter
                .validate()
                .map_err(|e| GqlError::ValidationError(e).extend())?;
        }
        @%- endif %@
        let repo: &RepositoryImpl = gql_ctx.data()?;
        let auth: &AuthInfo = gql_ctx.data()?;
        let ctx: &::_server::context::Ctx = gql_ctx.data()?;
        let @{ group|snake }@_repo = repo.@{ db|snake }@_repository().@{ group|ident }@();
        let mut query = @{ group|snake }@_repo.@{ mod_name|ident }@().@{ selector|ident }@().join(updater_joiner());
        @%- if selector_def.filter_is_required() %@
        query = query.selector_filter(filter);
        @%- else %@
        if let Some(filter) = filter {
            query = query.selector_filter(filter);
        }
        @%- endif %@
        query = query.extra_filter(updatable_filter(auth)?);

        let mut result = Vec::new();
        for mut obj in query
            .query_for_update()
            .await
            .map_err(|e| GqlError::server_error(gql_ctx, e))?
        {
            let org = serde_json::to_value(ReqObj::from(&mut *obj))?;
            let val = senax_common::update_operator::apply_operator(org, &operator, ctx.utc())?;
            let data: ReqObj = serde_json::from_value(val)?;
            data.validate()
                .map_err(|e| GqlError::ValidationError(e).extend())?;
            let obj =
                _repository_::update(@{ group|snake }@_repo.as_ref().into(), obj, |obj| update_updater(&mut *obj, data, @{ group|snake }@_repo.as_ref(), auth))
                    .await
                    .map_err(|e| GqlError::server_error(gql_ctx, e))?;
            result.push(ResObj::try_from_(&*obj, None)?);
        }
        Ok(result)
    }
    @%- endif %@
    @%- if api_selector_def.enable_delete_by_selector %@

    #[graphql(guard = "delete_guard()")]
    async fn delete_by_@{ selector }@(
        &self,
        gql_ctx: &async_graphql::Context<'_>,
        @%- if selector_def.filter_is_required() %@
        filter: _repository_::@{ pascal_name }@Query@{ selector|pascal }@Filter,
        @%- else %@
        filter: Option<_repository_::@{ pascal_name }@Query@{ selector|pascal }@Filter>,
        @%- endif %@
    ) -> async_graphql::Result<Vec<async_graphql::ID>> {
        @%- if selector_def.filter_is_required() %@
        filter.validate().map_err(|e| GqlError::ValidationError(e).extend())?;
        @%- else %@
        if let Some(filter) = &filter {
            filter
                .validate()
                .map_err(|e| GqlError::ValidationError(e).extend())?;
        }
        @%- endif %@
        let repo: &RepositoryImpl = gql_ctx.data()?;
        let auth: &AuthInfo = gql_ctx.data()?;
        let @{ group|snake }@_repo = repo.@{ db|snake }@_repository().@{ group|ident }@();
        let mut query = @{ group|snake }@_repo.@{ mod_name|ident }@().@{ selector|ident }@();
        @%- if selector_def.filter_is_required() %@
        query = query.selector_filter(filter);
        @%- else %@
        if let Some(filter) = filter {
            query = query.selector_filter(filter);
        }
        @%- endif %@
        query = query.extra_filter(deletable_filter(auth)?);
        let mut result = Vec::new();
        for obj in query
            .query_for_update()
            .await
            .map_err(|e| GqlError::server_error(gql_ctx, e))?
        {
            result.push((&*obj).into());
            _repository_::delete(@{ group|snake }@_repo.as_ref().into(), obj)
                .await
                .map_err(|e| GqlError::server_error(gql_ctx, e))?;
        }
        Ok(result)
    }
    @%- endif %@
    @%- endfor %@
    @%- endfor %@
    @%- endif %@
    @%- endif %@
    @%- if !def.disable_delete() %@
    @%- if api_def.enable_delete_by_pk %@

    #[graphql(guard = "delete_guard()")]
    async fn delete_by_pk(
        &self,
        gql_ctx: &async_graphql::Context<'_>,
        @%- if camel_case %@
        @{- def.primaries()|fmt_join("
        {ident}: {inner},", "") }@
        @%- else %@
        @{- def.primaries()|fmt_join("
        #[graphql(name = \"{raw_name}\")] {ident}: {inner},", "") }@
        @%- endif %@
    ) -> async_graphql::Result<bool> {
        let repo: &RepositoryImpl = gql_ctx.data()?;
        let auth: &AuthInfo = gql_ctx.data()?;
        delete(repo.@{ db|snake }@_repository().@{ group|ident }@(), auth, @{ def.primaries()|fmt_join_with_paren("{ident}", ", ") }@.into())
            .await
            .map_err(|e| {
                if let Some(e) = e.downcast_ref::<GqlError>() {
                    e.extend()
                } else {
                    GqlError::server_error(gql_ctx, e)
                }
            })?;
        Ok(true)
    }
    @%- endif %@

    #[graphql(guard = "delete_guard()")]
    async fn delete(
        &self,
        gql_ctx: &async_graphql::Context<'_>,
        #[graphql(name = "_id")] _id: async_graphql::ID,
    ) -> async_graphql::Result<bool> {
        let repo: &RepositoryImpl = gql_ctx.data()?;
        let auth: &AuthInfo = gql_ctx.data()?;
        delete(repo.@{ db|snake }@_repository().@{ group|ident }@(), auth, (&_id).try_into()?)
            .await
            .map_err(|e| {
                if let Some(e) = e.downcast_ref::<GqlError>() {
                    e.extend()
                } else {
                    GqlError::server_error(gql_ctx, e)
                }
            })?;
        Ok(true)
    }
    @%- endif %@
    @%- if def.disable_update() && def.disable_delete() %@
    async fn dummy(&self) -> bool {
        false
    }
    @%- endif %@
}

pub fn route_config(_cfg: &mut utoipa_actix_web::service_config::ServiceConfig) {
    @%- for (selector, selector_def) in def.selectors %@
    @%- for api_selector_def in api_def.selector(selector) %@
    @%- if api_selector_def.enable_streaming_api() || api_def.enable_json_api() %@
    _cfg.service(@{ selector }@_handler);
    @%- endif %@
    @%- if api_def.enable_json_api() %@
    _cfg.service(count_@{ selector }@_handler);
    @%- endif %@
    @%- endfor %@
    @%- endfor %@
}
@%- for (selector, selector_def) in def.selectors %@
@%- for api_selector_def in api_def.selector(selector) %@
@%- if api_selector_def.enable_streaming_api() || api_def.enable_json_api() %@

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct @{ selector|pascal }@Request {
    @%- if selector_def.filter_is_required() %@
    filter: _repository_::@{ pascal_name }@Query@{ selector|pascal }@Filter,
    @%- else %@
    #[serde(default, skip_serializing_if = "Option::is_none")]
    filter: Option<_repository_::@{ pascal_name }@Query@{ selector|pascal }@Filter>,
    @%- endif %@
    #[serde(default, skip_serializing_if = "Option::is_none")]
    order: Option<_repository_::@{ pascal_name }@Query@{ selector|pascal }@Order>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    limit: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    offset: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    after: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    ndjson: Option<bool>,
}

#[utoipa::path(
    responses(
        (status = 200, body = Vec<ResObj>)
    )
)]
#[post("/@{ selector }@")]
async fn @{ selector }@_handler(
    data: actix_web::web::Json<@{ selector|pascal }@Request>,
    http_req: actix_web::HttpRequest,
) -> impl actix_web::Responder {
    use _server::response::ApiError;
    use anyhow::Result;
    use futures::{Stream, StreamExt as _};
    use std::pin::Pin;

    #[allow(clippy::let_unit_value)]
    async fn _fetch(
        repo: &RepositoryImpl,
        auth: &AuthInfo,
        data: &@{ selector|pascal }@Request,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<Box<dyn _domain_::@{ pascal_name }@>>> + Send>>>
    {
        let @{ db|snake }@_query = repo.@{ db|snake }@_query();
        let @{ mod_name }@_repo = @{ db|snake }@_query.@{ group|ident }@().@{ mod_name|ident }@();

        let mut query = @{ mod_name }@_repo.@{ selector|ident }@().join(reader_joiner());
        @%- if selector_def.filter_is_required() %@
        query = query.selector_filter(data.filter.clone());
        @%- else %@
        if let Some(filter) = &data.filter {
            query = query.selector_filter(filter.clone());
        }
        @%- endif %@
        query = query.extra_filter(readable_filter(auth)?);
        if let Some(offset) = data.offset {
            query = query.offset(offset);
        }
        if let Some(limit) = data.limit {
            query = query.limit(limit);
        }
        let order = data.order.unwrap_or_default();
        if let Some(_after) = &data.after {
            match order {
                @%- for (order, order_def) in selector_def.orders %@
                _repository_::@{ pascal_name }@Query@{ selector|pascal }@Order::@{ order|pascal }@ => {
                    let c = _repository_::@{ pascal_name }@Query@{ selector|pascal }@Cursor::@{ order }@_from_str(
                        _after,
                    )?;
                    query = query.cursor(
                        _repository_::@{ pascal_name }@Query@{ selector|pascal }@Cursor::@{ order|pascal }@(
                            domain::models::Cursor::After(c),
                        ),
                    );
                }
                @%- endfor %@
                _repository_::@{ pascal_name }@Query@{ selector|pascal }@Order::_None => {}
            }
        }
        query = query.order_by(order);
        query.stream(_server::auto_api::USE_SINGLE_TRANSACTION_FOR_STREAM).await
    }

    let ctx = ::_server::context::Ctx::get(&http_req);
    let ndjson = data.ndjson.unwrap_or_default();
    let result = async move {
        let auth = AuthInfo::retrieve(&http_req).unwrap_or_default();
        if !api_query_guard(&auth).ok_or(ApiError::Unauthorized)? {
            anyhow::bail!(ApiError::Forbidden);
        }
        @%- if selector_def.filter_is_required() %@
        data.filter.validate().map_err(ApiError::ValidationError)?;
        @%- else %@
        if let Some(filter) = &data.filter {
            filter
                .validate()
                .map_err(ApiError::ValidationError)?;
        }
        @%- endif %@
        let repo = RepositoryImpl::new_with_ctx(&ctx);
        let stream = crate::api_selector!(_fetch(&repo, &auth, &data), repo);
        let order = data.order.unwrap_or_default();
        let stream = stream.map(move |obj| {
            obj.and_then(|obj| ResObj::try_from_(&*obj, order.to_cursor(&obj)))
        });
        Ok::<_, anyhow::Error>(stream)
    }
    .await;
    _server::response::json_stream_response(result, ctx, ndjson)
}
@%- endif %@
@%- if api_def.enable_json_api() %@

#[utoipa::path(
    responses(
        (status = 200, body = i64)
    )
)]
#[post("/count_@{ selector }@")]
async fn count_@{ selector }@_handler(
    @%- if selector_def.filter_is_required() %@
    filter: actix_web::web::Json<_repository_::@{ pascal_name }@Query@{ selector|pascal }@Filter>,
    @%- else %@
    filter: actix_web::web::Json<Option<_repository_::@{ pascal_name }@Query@{ selector|pascal }@Filter>>,
    @%- endif %@
    http_req: actix_web::HttpRequest,
) -> impl actix_web::Responder {
    use _server::response::ApiError;

    async fn _count(
        repo: &RepositoryImpl,
        auth: &AuthInfo,
        @%- if selector_def.filter_is_required() %@
        filter: &_repository_::@{ pascal_name }@Query@{ selector|pascal }@Filter,
        @%- else %@
        filter: &Option<_repository_::@{ pascal_name }@Query@{ selector|pascal }@Filter>,
        @%- endif %@
    ) -> anyhow::Result<i64> {
        let @{ db|snake }@_query = repo.@{ db|snake }@_query();
        let @{ mod_name }@_repo = @{ db|snake }@_query.@{ group|ident }@().@{ mod_name|ident }@();
        let mut query = @{ mod_name }@_repo.@{ selector|ident }@();
        @%- if selector_def.filter_is_required() %@
        query = query.selector_filter(filter.clone());
        @%- else %@
        if let Some(filter) = filter {
            query = query.selector_filter(filter.clone());
        }
        @%- endif %@
        query = query.extra_filter(readable_filter(auth)?);
        let count = query.count().await?;
        Ok(count)
    }

    let ctx = ::_server::context::Ctx::get(&http_req);
    let filter = filter.into_inner();
    let result = async move {
        let auth = AuthInfo::retrieve(&http_req).unwrap_or_default();
        if !api_query_guard(&auth).ok_or(ApiError::Unauthorized)? {
            anyhow::bail!(ApiError::Forbidden);
        }
        @%- if selector_def.filter_is_required() %@
        filter.validate().map_err(ApiError::ValidationError)?;
        @%- else %@
        if let Some(filter) = &filter {
            filter
                .validate()
                .map_err(ApiError::ValidationError)?;
        }
        @%- endif %@
        let repo = RepositoryImpl::new_with_ctx(&ctx);
        Ok(crate::api_selector!(_count(&repo, &auth, &filter), repo))
    }
    .await;
    crate::response::json_response(result, ctx)
}
@%- endif %@
@%- endfor %@
@%- endfor %@
@{-"\n"}@