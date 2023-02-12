use async_graphql::types::connection::{query, Connection, Edge};
use async_graphql::{ErrorExtensions, InputObject, Lookahead, Object, SimpleObject, ID};
use serde::{Deserialize, Serialize};
#[allow(unused_imports)]
use std::collections::HashMap;
use validator::{Validate, ValidationErrors};

#[allow(unused_imports)]
use crate::graphql::{GqiError, Session, _SessionStore};
use db_@{ db }@::misc::ForUpdateTr;
use db_@{ db }@::DbConn as @{ db|pascal }@Conn;

const VER: usize = 1;

// Do not modify this line. (GqiModelBegin)
// Do not modify this line. (GqiModelEnd)

pub struct GqiQuery@{ db|pascal }@@{ group|pascal }@@{ mod_name|pascal }@;
#[Object]
impl GqiQuery@{ db|pascal }@@{ group|pascal }@@{ mod_name|pascal }@ {
    @%- if def.use_cache_all() && !def.use_cache_all_with_condition() %@
    async fn all(
        &self,
        ctx: &async_graphql::Context<'_>,
    ) -> async_graphql::Result<Vec<@{ db|pascal }@@{ group|pascal }@@{ mod_name|pascal }@>> {
        let mut conn = @{ db|pascal }@Conn::new();
        let list = _@{ pascal_name }@::find_all_from_cache(&conn, None)
            .await
            .map_err(|e| GqiError::ServerError(e.to_string()).extend())?;
        let mut list = list.iter().cloned().collect();
        fetch_cache_list(&mut list, &mut conn, ctx.look_ahead()).await?;
        Ok(list.into_iter().map(|v| v.into()).collect())
    }
    @%- endif %@
@%- if def.primaries().len() > 0 %@
    @%- if def.use_cache() %@
    async fn find(
        &self,
        ctx: &async_graphql::Context<'_>,
        @{- def.primaries()|fmt_join("
        {var}: {inner}", ", ") }@,
    ) -> async_graphql::Result<@{ db|pascal }@@{ group|pascal }@@{ mod_name|pascal }@> {
        let mut conn = @{ db|pascal }@Conn::new();
        let mut obj = _@{ pascal_name }@::find_optional_from_cache(&conn, @{ def.primaries()|fmt_join_with_paren("{var}", ", ") }@)
            .await
            .map_err(|e| GqiError::ServerError(e.to_string()).extend())?
            .ok_or_else(|| GqiError::NotFound.extend())?;
        fetch_cache_obj(&mut obj, &mut conn, ctx.look_ahead()).await?;
        Ok(obj.into())
    }
    @%- else %@
    async fn find(
        &self,
        ctx: &async_graphql::Context<'_>,
        @{ def.primaries()|fmt_join("
        {var}: {inner}", ", ") }@,
    ) -> async_graphql::Result<@{ db|pascal }@@{ group|pascal }@@{ mod_name|pascal }@> {
        let mut conn = @{ db|pascal }@Conn::new();
        let mut obj = _@{ pascal_name }@::find_optional(&mut conn, @{ def.primaries()|fmt_join_with_paren("{var}", ", ") }@)
            .await
            .map_err(|e| GqiError::ServerError(e.to_string()).extend())?
            .ok_or_else(|| GqiError::NotFound.extend())?;
        fetch_obj(&mut obj, &mut conn, ctx.look_ahead()).await?;
        Ok(obj.into())
    }
    @%- endif %@
@%- endif %@
    @%- if def.main_primary().len() > 0 %@
    async fn list(
        &self,
        ctx: &async_graphql::Context<'_>,
        after: Option<String>,
        before: Option<String>,
        #[graphql(validator(maximum = 100))] first: Option<i32>,
        #[graphql(validator(maximum = 100))] last: Option<i32>,
        offset: Option<usize>,
    ) -> async_graphql::Result<Connection<@{ def.main_primary()|fmt_join("{inner}", ", ") }@, @{ db|pascal }@@{ group|pascal }@@{ mod_name|pascal }@>> {
        query(
            after,
            before,
            first,
            last,
            |after: Option<@{ def.main_primary()|fmt_join("{inner}", ", ") }@>, before: Option<@{ def.main_primary()|fmt_join("{inner}", ", ") }@>, first, last| async move {
                let mut conn = @{ db|pascal }@Conn::new();
                let mut query = _@{ pascal_name }@::query();
                let mut cond = db_@{ db }@::cond_@{ group }@_@{ name }@!();
                let mut previous = false;
                let mut limit = 100;
                if first.is_some() || after.is_some() {
                    if let Some(after) = after {
                        cond = cond.and(db_@{ db }@::cond_@{ group }@_@{ name }@!(@{ def.main_primary()|fmt_join("{col} > after", "") }@));
                        if ctx
                            .look_ahead()
                            .field("pageInfo")
                            .field("hasPreviousPage")
                            .exists()
                        {
                            previous = !_@{ pascal_name }@::query()
                                .cond(db_@{ db }@::cond_@{ group }@_@{ name }@!(@{ def.main_primary()|fmt_join("{col} < after", "") }@))
                                .limit(1)
                                .select(&mut conn)
                                .await?
                                .is_empty();
                        }
                    }
                    if let Some(first) = first {
                        limit = first;
                    }
                    query = query.order_by(db_@{ db }@::order_by_@{ group }@_@{ name }@!(@{ def.main_primary()|fmt_join("{col} ASC", "") }@));
                }
                if last.is_some() || before.is_some() {
                    if let Some(before) = before {
                        cond = cond.and(db_@{ db }@::cond_@{ group }@_@{ name }@!(@{ def.main_primary()|fmt_join("{col} < before", "") }@));
                        if ctx
                            .look_ahead()
                            .field("pageInfo")
                            .field("hasPreviousPage")
                            .exists()
                        {
                            previous = !_@{ pascal_name }@::query()
                                .cond(db_@{ db }@::cond_@{ group }@_@{ name }@!(@{ def.main_primary()|fmt_join("{col} > before", "") }@))
                                .limit(1)
                                .select(&mut conn)
                                .await?
                                .is_empty();
                        }
                    }
                    if let Some(last) = last {
                        limit = last;
                    }
                    query = query.order_by(db_@{ db }@::order_by_@{ group }@_@{ name }@!(@{ def.main_primary()|fmt_join("{col} DESC", "") }@));
                }
                if let Some(offset) = offset {
                    previous = offset > 0;
                    query = query.offset(offset);
                }
                query = query.limit(limit + 1);
                query = query.cond(cond);
                let mut list = query.@% if def.use_cache() %@select_from_cache@% else %@select@% endif %@(&mut conn).await?;
                @% if def.use_cache() %@fetch_cache_list@% else %@fetch_list@% endif %@(
                    &mut list,
                    &mut conn,
                    ctx.look_ahead().field("edges").field("node"),
                )
                .await?;
                let mut connection = Connection::new(previous, list.len() > limit);
                list.truncate(limit);
                if last.is_some() {
                    list.reverse();
                }
                connection.edges.extend(
                    list.into_iter()
                        .map(|obj| Edge::new(@{ def.main_primary()|fmt_join("{inner}::from(obj.{var}())", ", ") }@, @{ db|pascal }@@{ group|pascal }@@{ mod_name|pascal }@::from(obj))),
                );
                Ok::<_, async_graphql::Error>(connection)
            },
        )
        .await
    }
    @%- endif %@
}

pub struct GqiMutation@{ db|pascal }@@{ group|pascal }@@{ mod_name|pascal }@;
#[Object]
impl GqiMutation@{ db|pascal }@@{ group|pascal }@@{ mod_name|pascal }@ {
    async fn create(
        &self,
        ctx: &async_graphql::Context<'_>,
        data: Req@{ db|pascal }@@{ group|pascal }@@{ mod_name|pascal }@,
    ) -> async_graphql::Result<@{ db|pascal }@@{ group|pascal }@@{ mod_name|pascal }@> {
        let mut conn = @{ db|pascal }@Conn::new();
        conn.begin().await?;
        validate(&mut conn, &data)
            .await
            .map_err(|e| GqiError::ValidationError(e).extend())?;
        let mut obj = create(&mut conn, data).await?;
        fetch_obj(&mut obj, &mut conn, ctx.look_ahead()).await?;
        conn.commit().await?;
        Ok(obj.into())
    }

    async fn update(
        &self,
        ctx: &async_graphql::Context<'_>,
        @{- def.primaries()|fmt_join("
        {var}: {inner},", "") }@
        data: Req@{ db|pascal }@@{ group|pascal }@@{ mod_name|pascal }@,
    ) -> async_graphql::Result<@{ db|pascal }@@{ group|pascal }@@{ mod_name|pascal }@> {
        let mut conn = @{ db|pascal }@Conn::new();
        conn.begin().await?;
        validate(&mut conn, &data)
            .await
            .map_err(|e| GqiError::ValidationError(e).extend())?;
        let mut obj = update(&mut conn, @{ def.primaries()|fmt_join("{var}", ", ") }@, data).await?;
        fetch_obj(&mut obj, &mut conn, ctx.look_ahead()).await?;
        conn.commit().await?;
        Ok(obj.into())
    }

    async fn delete(
        &self,
        _ctx: &async_graphql::Context<'_>,
        @{- def.primaries()|fmt_join("
        {var}: {inner},", "") }@
    ) -> async_graphql::Result<bool> {
        let mut conn = @{ db|pascal }@Conn::new();
        conn.begin().await?;
        delete(&mut conn, @{ def.primaries()|fmt_join("{var}", ", ") }@).await?;
        conn.commit().await?;
        Ok(true)
    }
}

pub async fn validate(
    _conn: &mut @{ db|pascal }@Conn,
    data: &Req@{ db|pascal }@@{ group|pascal }@@{ mod_name|pascal }@,
) -> Result<(), ValidationErrors> {
    data.validate()?;
    Ok(())
}

pub async fn create(conn: &mut @{ db|pascal }@Conn, data: Req@{ db|pascal }@@{ group|pascal }@@{ mod_name|pascal }@) -> anyhow::Result<_@{ pascal_name }@> {
    let obj = prepare_create(conn, data);
    let res = _@{ pascal_name }@::save(conn, obj).await?;
    Ok(res.unwrap())
}

pub async fn update(conn: &mut @{ db|pascal }@Conn, @{ def.primaries()|fmt_join("{var}: {inner}", ", ") }@, data: Req@{ db|pascal }@@{ group|pascal }@@{ mod_name|pascal }@) -> anyhow::Result<_@{ pascal_name }@> {
    let mut obj = _@{ pascal_name }@::find_for_update(conn, @{ def.primaries()|fmt_join_with_paren("{var}", ", ") }@).await?;
    prepare_update(conn, &mut obj, data).await?;
    let res = _@{ pascal_name }@::save(conn, obj).await?;
    Ok(res.unwrap())
}

pub async fn delete(conn: &mut @{ db|pascal }@Conn, @{ def.primaries()|fmt_join("{var}: {inner}", ", ") }@) -> anyhow::Result<()> {
    let obj = _@{ pascal_name }@::find_for_update(conn, @{ def.primaries()|fmt_join_with_paren("{var}", ", ") }@).await?;
    _@{ pascal_name }@::delete(conn, obj).await
}
@{-"\n"}@