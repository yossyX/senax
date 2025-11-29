use async_trait::async_trait;
#[allow(unused_imports)]
use db::misc::{Updater as _, ToJsonRawValue as _};
#[allow(unused_imports)]
use domain::models::ToRawValue as _;
#[allow(unused_imports)]
use domain::repository::@{ db|snake|ident }@::@{ base_group_name|snake|ident }@::_super::@{ group_name|snake|ident }@::_base::_@{ mod_name }@::{self, *};
use domain::repository::@{ db|snake|ident }@::@{ base_group_name|snake|ident }@::_super::@{ group_name|snake|ident }@::@{ mod_name|ident }@::*;
#[allow(unused_imports)]
use senax_common::types::geo_point::ToGeoPoint as _;
#[allow(unused_imports)]
use senax_common::types::point::ToPoint as _;
#[allow(unused_imports)]
use std::ops::{Deref as _, DerefMut as _};
#[allow(unused_imports)]
use domain::models::@{ db|snake|ident }@ as _model_;
@%- for (name, rel_def) in def.belongs_to_outer_db() %@
#[allow(unused_imports)]
use domain::models::@{ rel_def.db()|snake|ident }@ as _@{ rel_def.db()|snake }@_model_;
@%- endfor %@

use crate::repositories::@{ group_name|snake|ident }@::@{ mod_name|ident }@::*;

#[derive(derive_new::new, Clone)]
pub struct @{ pascal_name }@RepositoryImpl {
    _conn: std::sync::Arc<tokio::sync::Mutex<db::DbConn>>,
}

#[allow(clippy::needless_update)]
fn updater_from_factory(_v: domain::repository::@{ db|snake|ident }@::@{ base_group_name|snake|ident }@::_super::@{ group_name|snake|ident }@::@{ mod_name|ident }@::@{ pascal_name }@Factory) -> _@{ pascal_name }@Updater {
    _@{ pascal_name }@Updater {
        _data: ::db::models::@{ group_name|snake|ident }@::@{ mod_name|ident }@::Data {
@{ def.for_factory()|fmt_join("            {var}: _v.{var}{convert_domain_factory}{convert_from_entity},", "\n") }@
            ..Default::default()
        },
        _update: Default::default(),
        _is_new: true,
        _do_delete: false,
        _upsert: false,
        _is_loaded: true,
        _op: Default::default(),
@{- def.relations_one(false)|fmt_rel_join("\n        {rel_name}: None,", "") }@
@{- def.relations_many(false)|fmt_rel_join("\n        {rel_name}: None,", "") }@
@{- def.relations_belonging(false)|fmt_rel_join("\n        {rel_name}: None,", "") }@
@{- def.relations_belonging_outer_db(false)|fmt_rel_outer_db_join("\n        {rel_name}: None,", "") }@
    }
}

#[allow(clippy::clone_on_copy)]
#[async_trait]
impl _@{ pascal_name }@Repository for @{ pascal_name }@RepositoryImpl {
    @%- if !def.disable_update() %@
    fn find(&self, id: @{ def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") }@) -> Box<dyn _@{ pascal_name }@RepositoryFindBuilder> {
        struct V {
            conn: std::sync::Arc<tokio::sync::Mutex<db::DbConn>>,
            id: @{ def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") }@,
            filter: Option<Filter_>,
            @%- if def.is_soft_delete() %@
            with_trashed: bool,
            @%- endif %@
            joiner: Option<Box<Joiner_>>,
        }
        #[allow(unused_imports)]
        use _@{ pascal_name }@RepositoryFindBuilder as _RepositoryFindBuilder;
        #[async_trait]
        impl _@{ pascal_name }@RepositoryFindBuilder for V {
            async fn query_for_update(self: Box<Self>) -> anyhow::Result<Box<dyn @{ pascal_name }@Updater>> {
                let mut conn = self.conn.lock().await;
                let conn = conn.deref_mut();
                #[allow(unused_mut)]
                @%- if def.is_soft_delete() %@
                let obj = if self.with_trashed {
                    _@{ pascal_name }@_::find_for_update_with_trashed(conn, @{ def.primaries()|fmt_join_with_paren2("self.id{convert_from_entity}", "self.id.{index}{convert_from_entity}", ", ") }@, self.joiner, self.filter).await?
                } else {
                    _@{ pascal_name }@_::find_for_update(conn, @{ def.primaries()|fmt_join_with_paren2("self.id{convert_from_entity}", "self.id.{index}{convert_from_entity}", ", ") }@, self.joiner, self.filter).await?
                };
                @%- else %@
                let obj = _@{ pascal_name }@_::find_for_update(conn, @{ def.primaries()|fmt_join_with_paren2("self.id{convert_from_entity}", "self.id.{index}{convert_from_entity}", ", ") }@, self.joiner, self.filter).await?;
                @%- endif %@
                Ok(Box::new(obj) as Box<dyn @{ pascal_name }@Updater>)
            }
            async fn query(self: Box<Self>) -> anyhow::Result<Option<Box<dyn @{ pascal_name }@>>> {
                let mut conn = self.conn.lock().await;
                let conn = conn.deref_mut();
                @%- if def.is_soft_delete() %@
                let obj = if self.with_trashed {
                    _@{ pascal_name }@_::find_optional_with_trashed(conn, @{ def.primaries()|fmt_join_with_paren2("self.id{convert_from_entity}", "self.id.{index}{convert_from_entity}", ", ") }@, self.joiner, self.filter).await?
                } else {
                    _@{ pascal_name }@_::find_optional(conn, @{ def.primaries()|fmt_join_with_paren2("self.id{convert_from_entity}", "self.id.{index}{convert_from_entity}", ", ") }@, self.joiner, self.filter).await?
                };
                @%- else %@
                let obj = _@{ pascal_name }@_::find_optional(conn, @{ def.primaries()|fmt_join_with_paren2("self.id{convert_from_entity}", "self.id.{index}{convert_from_entity}", ", ") }@, self.joiner, self.filter).await?;
                @%- endif %@
                if let Some(obj) = obj {
                    Ok(Some(Box::new(obj) as Box<dyn @{ pascal_name }@>))
                } else {
                    Ok(None)
                }
            }
            fn filter(mut self: Box<Self>, filter: Filter_) -> Box<dyn _RepositoryFindBuilder> {
                self.filter = Some(filter);
                self
            }
            @%- if def.is_soft_delete() %@
            fn with_trashed(mut self: Box<Self>, mode: bool) -> Box<dyn _RepositoryFindBuilder> {
                self.with_trashed = mode;
                self
            }
            @%- endif %@
            fn join(mut self: Box<Self>, joiner: Option<Box<Joiner_>>) -> Box<dyn _RepositoryFindBuilder> {
                self.joiner = Joiner_::merge(self.joiner, joiner);
                self
            }
        }
        Box::new(V {
            conn: self._conn.clone(),
            id,
            filter: None,
            @%- if def.is_soft_delete() %@
            with_trashed: false,
            @%- endif %@
            joiner: None,
        })
    }
    @%- endif %@
    fn convert_factory(&self, factory: @{ pascal_name }@Factory) -> Box<dyn @{ pascal_name }@Updater> {
        Box::new(updater_from_factory(factory))
    }
    #[allow(unused_mut)]
    async fn save(&self, obj: Box<dyn @{ pascal_name }@Updater>) -> anyhow::Result<Option<Box<dyn @{ pascal_name }@>>> {
        let obj: _@{ pascal_name }@Updater = match (obj as Box<dyn std::any::Any>).downcast::<_@{ pascal_name }@Updater>() {
            Ok(obj) => *obj,
            Err(_) => panic!("Only _@{ pascal_name }@Updater is accepted."),
        };
        Ok(_@{ pascal_name }@_::save(self._conn.lock().await.deref_mut(), obj).await?.map(|v| Box::new(v) as Box<dyn @{ pascal_name }@>))
    }
    @%- if !def.disable_update() %@
    async fn import(&self, list: Vec<Box<dyn @{ pascal_name }@Updater>>, option: Option<domain::models::ImportOption>) -> anyhow::Result<()> {
        let list = list.into_iter().map(|obj| {
            let obj: _@{ pascal_name }@Updater = match (obj as Box<dyn std::any::Any>).downcast::<_@{ pascal_name }@Updater>() {
                Ok(obj) => *obj,
                Err(_) => panic!("Only _@{ pascal_name }@Updater is accepted."),
            };
            obj
        }).collect();
        let option = option.unwrap_or_default();
        if option.replace.unwrap_or_default() {
            _@{ pascal_name }@_::bulk_replace(self._conn.lock().await.deref_mut(), list).await?;
        } else if option.overwrite.unwrap_or_default() {
            _@{ pascal_name }@_::bulk_overwrite(self._conn.lock().await.deref_mut(), list).await?;
        } else {
            _@{ pascal_name }@_::bulk_insert(self._conn.lock().await.deref_mut(), list, option.ignore.unwrap_or_default()).await?;
        }
        Ok(())
    }
    @%- endif %@
    @%- if def.use_insert_delayed() %@
    async fn insert_delayed(&self, obj: Box<dyn @{ pascal_name }@Updater>) -> anyhow::Result<()> {
        let Ok(obj) = (obj as Box<dyn std::any::Any>).downcast::<_@{ pascal_name }@Updater>() else {
            panic!("Only _@{ pascal_name }@Updater is accepted.");
        };
        _@{ pascal_name }@_::insert_delayed(self._conn.lock().await.deref_mut(), *obj).await?;
        Ok(())
    }
    @%- endif %@
    @%- if !def.disable_delete() %@
    async fn delete(&self, obj: Box<dyn @{ pascal_name }@Updater>) -> anyhow::Result<()> {
        let Ok(obj) = (obj as Box<dyn std::any::Any>).downcast::<_@{ pascal_name }@Updater>() else {
            panic!("Only _@{ pascal_name }@Updater is accepted.");
        };
        _@{ pascal_name }@_::delete(self._conn.lock().await.deref_mut(), *obj).await
    }
    @%- if def.primaries().len() == 1 %@
    async fn delete_by_ids(&self, ids: &[@{ def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") }@]) -> anyhow::Result<u64> {
        _@{ pascal_name }@_::delete_by_ids(self._conn.lock().await.deref_mut(), ids.iter().map(|v| @{ def.primaries()|fmt_join_with_paren2("v{convert_from_entity}", "v.{index}{convert_from_entity}", ", ") }@)).await
    }
    @%- endif %@
    async fn delete_all(&self) -> anyhow::Result<()> {
        _@{ pascal_name }@_::query().delete(self._conn.lock().await.deref_mut()).await?;
        Ok(())
    }
    @%- endif %@
    @%- if def.act_as_job_queue() %@
    async fn fetch(&self, limit: usize) -> anyhow::Result<Vec<Box<dyn @{ pascal_name }@Updater>>> {
        let list = _@{ pascal_name }@_::query().order_by(order!(@{ def.primaries()|fmt_join_with_paren("{raw_name}", ", ") }@)).limit(limit).skip_locked().select_for_update(self._conn.lock().await.deref_mut()).await?;
        Ok(list.into_iter().map(|v| Box::new(v) as Box<dyn @{ pascal_name }@Updater>).collect())
    }
    @%- endif %@
    @%- for (selector, selector_def) in def.selectors %@
    fn @{ selector|ident }@(&self) -> Box<dyn @{ pascal_name }@Repository@{ selector|pascal }@Builder> {
        struct V {
            conn: std::sync::Arc<tokio::sync::Mutex<db::DbConn>>,
            selector_filter: Option<_@{ mod_name }@::@{ pascal_name }@Query@{ selector|pascal }@Filter>,
            extra_filter: Option<Filter_>,
            @%- if def.is_soft_delete() %@
            with_trashed: bool,
            @%- endif %@
            joiner: Option<Box<Joiner_>>,
        }
        impl V {
            fn _query(
                selector_filter: Option<_@{ mod_name }@::@{ pascal_name }@Query@{ selector|pascal }@Filter>,
                extra_filter: Option<Filter_>,
                @%- if def.is_soft_delete() %@
                with_trashed: bool,
                @%- endif %@
                joiner: Option<Box<Joiner_>>,
            ) -> anyhow::Result<crate::repositories::@{ group_name|snake|ident }@::_base::_@{ mod_name }@::QueryBuilder> {
                let mut query = _@{ pascal_name }@_::query();
                let mut fltr = if let Some(filter) = selector_filter {
                    _filter_@{ selector }@(&filter)?
                } else {
                    filter!()
                };
                if let Some(filter) = extra_filter {
                    fltr = fltr.and(filter);
                }
                query = query.filter(fltr);
                query = query.join(joiner);
                @%- if def.is_soft_delete() %@
                query = query.when(with_trashed, |v| v.with_trashed());
                @%- endif %@
                Ok(query)
            }
        }
        #[allow(unused_imports)]
        use @{ pascal_name }@Repository@{ selector|pascal }@Builder as _Repository@{ selector|pascal }@Builder;
        #[async_trait]
        impl @{ pascal_name }@Repository@{ selector|pascal }@Builder for V {
            async fn query_for_update(self: Box<Self>) -> anyhow::Result<Vec<Box<dyn @{ pascal_name }@Updater>>> {
                let mut conn = self.conn.lock().await;
                let conn = conn.deref_mut();
                let query = Self::_query(self.selector_filter, self.extra_filter,@% if def.is_soft_delete() %@ self.with_trashed,@% endif %@ self.joiner)?;
                Ok(query.select_for_update(conn).await?.into_iter().map(|v| Box::new(v) as Box<dyn @{ pascal_name }@Updater>).collect())
            }
            async fn query(self: Box<Self>) -> anyhow::Result<Vec<Box<dyn @{ pascal_name }@>>> {
                let mut conn = self.conn.lock().await;
                let conn = conn.deref_mut();
                let query = Self::_query(self.selector_filter, self.extra_filter,@% if def.is_soft_delete() %@ self.with_trashed,@% endif %@ self.joiner)?;
                Ok(query.select(conn).await?.into_iter().map(|v| Box::new(v) as Box<dyn @{ pascal_name }@>).collect())
            }
            async fn count(self: Box<Self>) -> anyhow::Result<i64> {
                let mut conn = self.conn.lock().await;
                let conn = conn.deref_mut();
                let query = Self::_query(self.selector_filter, self.extra_filter,@% if def.is_soft_delete() %@ self.with_trashed,@% endif %@ None)?;
                Ok(query.count(conn).await?)
            }
            fn selector_filter(mut self: Box<Self>, filter: _@{ mod_name }@::@{ pascal_name }@Query@{ selector|pascal }@Filter) -> Box<dyn _Repository@{ selector|pascal }@Builder> {
                self.selector_filter = Some(filter);
                self
            }
            fn extra_filter(mut self: Box<Self>, filter: Filter_) -> Box<dyn _Repository@{ selector|pascal }@Builder> { self.extra_filter = Some(filter); self }
            @%- if def.is_soft_delete() %@
            fn with_trashed(mut self: Box<Self>, mode: bool) -> Box<dyn _Repository@{ selector|pascal }@Builder> { self.with_trashed = mode; self  }
            @%- endif %@
            fn join(mut self: Box<Self>, joiner: Option<Box<Joiner_>>) -> Box<dyn _Repository@{ selector|pascal }@Builder>  {
                self.joiner = Joiner_::merge(self.joiner, joiner);
                self
            }
        }
        Box::new(V {
            conn: self._conn.clone(),
            selector_filter: None,
            extra_filter: None,
            @%- if def.is_soft_delete() %@
            with_trashed: false,
            @%- endif %@
            joiner: None,
        })
    }
    @%- endfor %@
}
@%- for (selector, selector_def) in def.selectors %@
@%- for filter_map in selector_def.nested_filters(selector, def) %@
#[allow(unused_variables)]
#[allow(unused_mut)]
fn _filter@{ filter_map.suffix }@(filter: &_@{ mod_name }@::@{ pascal_name }@Query@{ selector|pascal }@@{ filter_map.pascal_name }@Filter) -> anyhow::Result<crate::repositories::@{ filter_map.model_group()|snake|ident }@::_base::_@{ filter_map.model_name()|snake }@::Filter_> {
    #[allow(unused_imports)]
    @%- if config.exclude_from_domain %@
    use crate::repository::@{ filter_map.model_group()|snake|ident }@::@{ filter_map.model_name()|snake|ident }@::filter;
    @%- else %@
    use domain::repository::@{ db|snake|ident }@::@{ base_group_name|snake|ident }@::_super::@{ filter_map.model_group()|snake|ident }@::@{ filter_map.model_name()|snake|ident }@::filter;
    @%- endif %@
    #[allow(unused_imports)]
    use anyhow::Context;
    let mut fltr = filter!();
    @%- for (filter, filter_def) in filter_map.filters %@
    @{- filter_def.db_str(filter, filter_map.model, filter_map.suffix) }@
    @%- endfor %@
    if let Some(_and) = &filter._and {
        let mut filters = filter!();
        for f in _and {
            filters = filters.and(_filter@{ filter_map.suffix }@(f)?);
        }
        fltr = fltr.and(filters);
    }
    if let Some(_or) = &filter._or {
        let mut filters = filter!();
        for f in _or {
            filters = filters.or(_filter@{ filter_map.suffix }@(f)?);
        }
        fltr = fltr.and(filters);
    }
    Ok(fltr)
}
@%- endfor %@
@%- endfor %@

#[allow(clippy::clone_on_copy)]
#[async_trait]
impl _@{ pascal_name }@QueryService for @{ pascal_name }@RepositoryImpl {
    @%- if def.use_all_rows_cache() && !def.use_filtered_row_cache() %@
    async fn all(&self) -> anyhow::Result<Box<dyn domain::models::EntityIterator<dyn @{ pascal_name }@Cache>>> {
        struct V(std::sync::Arc<Vec<_@{ pascal_name }@Cache>>);
        impl domain::models::EntityIterator<dyn @{ pascal_name }@Cache> for V {
            fn iter(&self) -> Box<dyn Iterator<Item = &(dyn @{ pascal_name }@Cache + 'static)> + '_> {
                Box::new(self.0.iter().map(|v| v as &dyn @{ pascal_name }@Cache))
            }
            fn into_iter(self) -> Box<dyn Iterator<Item = Box<dyn @{ pascal_name }@Cache>>> {
                Box::new(Vec::clone(&self.0).into_iter().map(|v| Box::new(v) as Box<dyn @{ pascal_name }@Cache>))
            }
        }
        Ok(Box::new(V(_@{ pascal_name }@_::find_all_from_cache(self._conn.lock().await.deref(), None).await?)))
    }
    @%- endif %@
    @%- for (selector, selector_def) in def.selectors %@
    fn @{ selector|ident }@(&self) -> Box<dyn @{ pascal_name }@Query@{ selector|pascal }@Builder> {
        struct V {
            conn: std::sync::Arc<tokio::sync::Mutex<db::DbConn>>,
            selector_filter: Option<_@{ mod_name }@::@{ pascal_name }@Query@{ selector|pascal }@Filter>,
            extra_filter: Option<Filter_>,
            cursor: Option<_@{ mod_name }@::@{ pascal_name }@Query@{ selector|pascal }@Cursor>,
            order: Option<_@{ mod_name }@::@{ pascal_name }@Query@{ selector|pascal }@Order>,
            reverse: bool,
            limit: usize,
            offset: usize,
            @%- if def.is_soft_delete() %@
            with_trashed: bool,
            @%- endif %@
            joiner: Option<Box<Joiner_>>,
        }
        #[allow(unused_mut)]
        #[allow(unused_variables)]
        #[allow(clippy::match_single_binding)]
        fn _cursor(mut fltr: crate::repositories::@{ group_name|snake|ident }@::_base::_@{ mod_name }@::Filter_, cursor: &@{ pascal_name }@Query@{ selector|pascal }@Cursor) -> anyhow::Result<crate::repositories::@{ group_name|snake|ident }@::_base::_@{ mod_name }@::Filter_> {
            @%- if !selector_def.orders.is_empty() %@
            match cursor {
                @%- for (cursor, cursor_def) in selector_def.orders %@
                @{ pascal_name }@Query@{ selector|pascal }@Cursor::@{ cursor|pascal }@(c) => {
                    match c {
                        @{- cursor_def.db_str() }@
                    }
                }
                @%- endfor %@
            }
            @%- endif %@
            Ok(fltr)
        }
        #[allow(unused_imports)]
        use @{ pascal_name }@Query@{ selector|pascal }@Builder as _Query@{ selector|pascal }@Builder;
        #[async_trait]
        #[allow(clippy::if_same_then_else)]
        impl @{ pascal_name }@Query@{ selector|pascal }@Builder for V {
            async fn query(self: Box<Self>) -> anyhow::Result<Vec<Box<dyn @{ pascal_name }@@% if def.use_cache() %@Cache@% endif %@>>> {
                let mut conn = self.conn.lock().await;
                let conn = conn.deref_mut();
                let mut query = _@{ pascal_name }@_::query();
                let mut fltr = if let Some(filter) = self.selector_filter {
                    _filter_@{ selector }@(&filter)?
                } else {
                    filter!()
                };
                @%- if def.use_cache() %@
                let extra_filter = self.extra_filter.clone().unwrap_or_default();
                @%- endif %@
                if let Some(filter) = self.extra_filter {
                    fltr = fltr.and(filter);
                }
                if let Some(cursor) = self.cursor {
                    fltr = _cursor(fltr, &cursor)?;
                }
                query = query.filter(fltr);
                query = query.join(self.joiner);
                match self.order.unwrap_or_default() {
                    @%- for (order, fields) in selector_def.orders %@
                    _@{ mod_name }@::@{ pascal_name }@Query@{ selector|pascal }@Order::@{ order|pascal }@ => {
                        if !self.reverse {
                            query = query.@{ selector_def.db_order(order, false) }@;
                        } else {
                            query = query.@{ selector_def.db_order(order, true) }@;
                        }
                    }
                    @%- endfor %@
                    _@{ mod_name }@::@{ pascal_name }@Query@{ selector|pascal }@Order::_None => {}
                }
                query = query.when(self.limit > 0, |v| v.limit(self.limit));
                query = query.when(self.offset > 0, |v| v.offset(self.offset));
                @%- if def.is_soft_delete() %@
                query = query.when(self.with_trashed, |v| v.with_trashed());
                @%- endif %@
                @%- if def.use_cache() %@
                use domain::models::Check_ as _;
                let mut excluded = 0;
                let result = query
                    .select_from_cache(conn)
                    .await?
                    .into_iter()
                    .filter_map(|v| {
                        let v = Box::new(v) as Box<dyn @{ pascal_name }@Cache>;
                        if extra_filter.check(&*v).unwrap_or(true) {
                            Some(v)
                        } else {
                            excluded += 1;
                            None
                        }
                    })
                    .collect();
                if excluded > 0 {
                    // This is usually not an issue, but if it occurs frequently, please review the process.
                    log::warn!(ctx = conn.ctx_no(); "Objects updated while querying have been excluded: count={}.", excluded);
                }
                Ok(result)
                @%- else %@
                Ok(query.select(conn).await?.into_iter().map(|v| Box::new(v) as Box<dyn @{ pascal_name }@>).collect())
                @%- endif %@
            }
            async fn stream(self: Box<Self>, single_transaction: bool) -> anyhow::Result<std::pin::Pin<Box<dyn futures::Stream<Item=anyhow::Result<Box<dyn @{ pascal_name }@@% if def.use_cache() %@Cache@% endif %@>>> + Send>>> {
                let mut conn = self.conn.clone().lock_owned().await;
                let conn = conn.deref_mut();
                conn.begin_read_tx().await?;
                let mut query = _@{ pascal_name }@_::query();
                let mut fltr = if let Some(filter) = self.selector_filter {
                    _filter_@{ selector }@(&filter)?
                } else {
                    filter!()
                };
                let extra_filter = self.extra_filter.clone().unwrap_or_default();
                if let Some(filter) = self.extra_filter {
                    fltr = fltr.and(filter);
                }
                if let Some(cursor) = self.cursor {
                    fltr = _cursor(fltr, &cursor)?;
                }
                query = query.filter(fltr.clone());
                let joiner = self.joiner;
                match self.order.unwrap_or_default() {
                    @%- for (order, fields) in selector_def.orders %@
                    _@{ mod_name }@::@{ pascal_name }@Query@{ selector|pascal }@Order::@{ order|pascal }@ => {
                        if !self.reverse {
                            query = query.@{ selector_def.db_order(order, false) }@;
                        } else {
                            query = query.@{ selector_def.db_order(order, true) }@;
                        }
                    }
                    @%- endfor %@
                    _@{ mod_name }@::@{ pascal_name }@Query@{ selector|pascal }@Order::_None => {}
                }
                query = query.when(self.limit > 0, |v| v.limit(self.limit));
                query = query.when(self.offset > 0, |v| v.offset(self.offset));
                @%- if def.is_soft_delete() %@
                query = query.when(self.with_trashed, |v| v.with_trashed());
                @%- endif %@
                use db::models::@{ group_name|snake|ident }@::@{ mod_name|ident }@::InnerPrimary as _InnerPrimary_;
                let list: Vec<_InnerPrimary_> = query.select_for(conn).await?;
                if !single_transaction {
                    conn.release_read_tx()?;
                }

                let conn = self.conn;
                use domain::models::Check_ as _;
                use futures::StreamExt as _;
                Ok(async_stream::stream! {
                    let chunks = list.chunks(db::STREAM_CHUNK_SIZE);
                    let mut ret = None;
                    let mut excluded = 0;
                    for chunk in chunks {
                        let pks: Vec<_InnerPrimary_> = chunk.to_vec();
                        let joiner = joiner.clone();
                        let mut conn_lock = conn.clone().lock_owned().await;
                        let handle = tokio::spawn(async move {
                            async fn func(conn: &mut db::DbConn, pks: &Vec<_InnerPrimary_>, joiner: Option<Box<Joiner_>>) -> anyhow::Result<Vec<_@{ pascal_name }@@% if def.use_cache() %@Cache@% endif %@>> {
                                conn.begin_read_tx().await?;
                                let list = _@{ pascal_name }@_::find_many@% if def.use_cache() %@_from_cache@% endif %@@% if def.is_soft_delete() %@_with_trashed@% endif %@(conn, pks, joiner@% if !def.use_cache() %@, None@% endif %@).await?;
                                conn.release_read_tx()?;
                                let mut map: ahash::HashMap<_InnerPrimary_, _>  = list.into_iter().map(|v| ((&v).into(), v)).collect();
                                let mut ret = vec![];
                                for pk in pks {
                                    if let Some(v) = map.remove(pk) {
                                        ret.push(v);
                                    }
                                }
                                Ok(ret)
                            }
                            let conn = conn_lock.deref_mut();
                            match func(conn, &pks, joiner.clone()).await {
                                Ok(r) => Ok(r),
                                Err(e) => {
                                    if db::DbConn::is_retryable_error(&e) {
                                        conn.reset_tx();
                                        func(conn, &pks, joiner).await
                                    } else {
                                        Err(e)
                                    }
                                }
                            }
                        });
                        if let Some(list) = ret.take() {
                            for v in list {
                                let v = Box::new(v) as Box<dyn @{ pascal_name }@@% if def.use_cache() %@Cache@% endif %@>;
                                if extra_filter.check(&*v).unwrap_or(true) {
                                    yield Ok(v);
                                } else {
                                    excluded += 1;
                                }
                            }
                        }
                        ret = Some(handle.await??);
                    }
                    if let Some(list) = ret.take() {
                        for v in list {
                            let v = Box::new(v) as Box<dyn @{ pascal_name }@@% if def.use_cache() %@Cache@% endif %@>;
                            if extra_filter.check(&*v).unwrap_or(true) {
                                yield Ok(v);
                            } else {
                                excluded += 1;
                            }
                        }
                    }
                    let mut conn_lock = conn.clone().lock_owned().await;
                    let conn = conn_lock.deref_mut();
                    if excluded > 0 {
                        // This is usually not an issue, but if it occurs frequently, please review the process.
                        log::warn!(ctx = conn.ctx_no(); "Objects updated while streaming have been excluded: count={}.", excluded);
                    }
                    if single_transaction {
                        conn.release_read_tx()?;
                    }
                }.boxed())
            }
            async fn count(self: Box<Self>) -> anyhow::Result<i64> {
                let mut conn = self.conn.lock().await;
                let conn = conn.deref_mut();
                let mut query = _@{ pascal_name }@_::query();
                let mut fltr = if let Some(filter) = self.selector_filter {
                    _filter_@{ selector }@(&filter)?
                } else {
                    filter!()
                };
                if let Some(filter) = self.extra_filter {
                    fltr = fltr.and(filter);
                }
                if let Some(cursor) = self.cursor {
                    fltr = _cursor(fltr, &cursor)?;
                }
                query = query.filter(fltr);
                @%- if def.is_soft_delete() %@
                query = query.when(self.with_trashed, |v| v.with_trashed());
                @%- endif %@
                Ok(query.count(conn).await?)
            }
            fn selector_filter(mut self: Box<Self>, filter: _@{ mod_name }@::@{ pascal_name }@Query@{ selector|pascal }@Filter) -> Box<dyn _Query@{ selector|pascal }@Builder> { self.selector_filter = Some(filter); self }
            fn extra_filter(mut self: Box<Self>, filter: Filter_) -> Box<dyn _Query@{ selector|pascal }@Builder> { self.extra_filter = Some(filter); self }
            fn cursor(mut self: Box<Self>, cursor: _@{ mod_name }@::@{ pascal_name }@Query@{ selector|pascal }@Cursor) -> Box<dyn _Query@{ selector|pascal }@Builder> { self.cursor = Some(cursor); self }
            fn order_by(mut self: Box<Self>, order: _@{ mod_name }@::@{ pascal_name }@Query@{ selector|pascal }@Order) -> Box<dyn _Query@{ selector|pascal }@Builder> { self.order = Some(order); self  }
            fn reverse(mut self: Box<Self>, mode: bool) -> Box<dyn _Query@{ selector|pascal }@Builder> { self.reverse = mode; self  }
            fn limit(mut self: Box<Self>, limit: usize) -> Box<dyn _Query@{ selector|pascal }@Builder> { self.limit = limit; self  }
            fn offset(mut self: Box<Self>, offset: usize) -> Box<dyn _Query@{ selector|pascal }@Builder> { self.offset = offset; self  }
            @%- if def.is_soft_delete() %@
            fn with_trashed(mut self: Box<Self>, mode: bool) -> Box<dyn _Query@{ selector|pascal }@Builder> { self.with_trashed = mode; self  }
            @%- endif %@
            fn join(mut self: Box<Self>, joiner: Option<Box<Joiner_>>) -> Box<dyn _Query@{ selector|pascal }@Builder>  {
                self.joiner = Joiner_::merge(self.joiner, joiner);
                self
            }
        }
        Box::new(V {
            conn: self._conn.clone(),
            selector_filter: None,
            extra_filter: None,
            cursor: None,
            order: None,
            reverse: false,
            limit: 0,
            offset: 0,
            @%- if def.is_soft_delete() %@
            with_trashed: false,
            @%- endif %@
            joiner: None,
        })
    }
    @%- endfor %@
    @%- if def.use_cache() %@
    fn find(&self, id: @{ def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") }@) -> Box<dyn _@{ pascal_name }@QueryFindBuilder> {
        struct V {
            conn: std::sync::Arc<tokio::sync::Mutex<db::DbConn>>,
            id: @{ def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") }@,
            filter: Option<Filter_>,
            @%- if def.is_soft_delete() %@
            with_trashed: bool,
            @%- endif %@
            joiner: Option<Box<Joiner_>>,
        }
        #[allow(unused_imports)]
        use _@{ pascal_name }@QueryFindBuilder as _QueryFindBuilder;
        #[async_trait]
        impl _@{ pascal_name }@QueryFindBuilder for V {
            async fn query(self: Box<Self>) -> anyhow::Result<Option<Box<dyn @{ pascal_name }@Cache>>> {
                let mut conn = self.conn.lock().await;
                let conn = conn.deref_mut();
                let joiner = if let Some(filter) = &self.filter {
                    Joiner_::merge(self.joiner, filter.joiner())
                } else {
                    self.joiner
                };
                @%- if def.is_soft_delete() %@
                let obj = if self.with_trashed {
                    _@{ pascal_name }@_::find_optional_from_cache_with_trashed(conn, @{ def.primaries()|fmt_join_with_paren2("self.id{convert_from_entity}.clone()", "self.id.{index}.clone(){convert_from_entity}", ", ") }@, joiner).await?
                } else {
                    _@{ pascal_name }@_::find_optional_from_cache(conn, @{ def.primaries()|fmt_join_with_paren2("self.id{convert_from_entity}.clone()", "self.id.{index}.clone(){convert_from_entity}", ", ") }@, joiner).await?
                };
                @%- else %@
                let obj = _@{ pascal_name }@_::find_optional_from_cache(conn, @{ def.primaries()|fmt_join_with_paren2("self.id{convert_from_entity}.clone()", "self.id.{index}.clone(){convert_from_entity}", ", ") }@, joiner).await?;
                @%- endif %@
                if let Some(obj) = obj {
                    if let Some(filter) = self.filter {
                        use domain::models::Check_ as _;
                        match filter.check(&obj as &dyn @{ pascal_name }@Cache) {
                            Ok(true) => Ok(Some(Box::new(obj) as Box<dyn @{ pascal_name }@Cache>)),
                            Ok(false) => {
                                log::warn!(ctx = conn.ctx_no(); "Forbidden: {:?}", @{ def.primaries()|fmt_join_with_paren2("self.id.clone()", "self.id.{index}.clone()", ", ") }@);
                                Ok(None)
                            }
                            Err(_) => {
                                @%- if def.is_soft_delete() %@
                                let exists = if self.with_trashed {
                                    _@{ pascal_name }@_::exists_with_trashed(conn, @{ def.primaries()|fmt_join_with_paren2("self.id{convert_from_entity}.clone()", "self.id.{index}.clone(){convert_from_entity}", ", ") }@, Some(filter)).await?
                                } else {
                                    _@{ pascal_name }@_::exists(conn, @{ def.primaries()|fmt_join_with_paren2("self.id{convert_from_entity}.clone()", "self.id.{index}.clone(){convert_from_entity}", ", ") }@, Some(filter)).await?
                                };
                                @%- else %@
                                let exists = _@{ pascal_name }@_::exists(conn, @{ def.primaries()|fmt_join_with_paren2("self.id{convert_from_entity}.clone()", "self.id.{index}.clone(){convert_from_entity}", ", ") }@, Some(filter)).await?;
                                @%- endif %@
                                if exists {
                                    Ok(Some(Box::new(obj) as Box<dyn @{ pascal_name }@Cache>))
                                } else {
                                    log::warn!(ctx = conn.ctx_no(); "Forbidden: {:?}", @{ def.primaries()|fmt_join_with_paren2("self.id", "self.id.{index}", ", ") }@);
                                    Ok(None)
                                }
                            }
                        }
                    } else {
                        Ok(Some(Box::new(obj) as Box<dyn @{ pascal_name }@Cache>))
                    }
                } else {
                    Ok(None)
                }
            }
            fn filter(mut self: Box<Self>, filter: Filter_) -> Box<dyn _QueryFindBuilder> {
                self.filter = Some(filter);
                self
            }
            @%- if def.is_soft_delete() %@
            fn with_trashed(mut self: Box<Self>, mode: bool) -> Box<dyn _QueryFindBuilder> {
                self.with_trashed = mode;
                self
            }
            @%- endif %@
            fn join(mut self: Box<Self>, joiner: Option<Box<Joiner_>>) -> Box<dyn _QueryFindBuilder> {
                self.joiner = Joiner_::merge(self.joiner, joiner);
                self
            }
        }
        Box::new(V {
            conn: self._conn.clone(),
            id,
            filter: None,
            @%- if def.is_soft_delete() %@
            with_trashed: false,
            @%- endif %@
            joiner: None,
        })
    }
    @%- else %@
    fn find(&self, id: @{ def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") }@) -> Box<dyn _@{ pascal_name }@QueryFindBuilder> {
        struct V {
            conn: std::sync::Arc<tokio::sync::Mutex<db::DbConn>>,
            id: @{ def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") }@,
            filter: Option<Filter_>,
            @%- if def.is_soft_delete() %@
            with_trashed: bool,
            @%- endif %@
            joiner: Option<Box<Joiner_>>,
        }
        #[allow(unused_imports)]
        use _@{ pascal_name }@QueryFindBuilder as _QueryFindBuilder;
        #[async_trait]
        impl _@{ pascal_name }@QueryFindBuilder for V {
            async fn query(self: Box<Self>) -> anyhow::Result<Option<Box<dyn @{ pascal_name }@>>> {
                let mut conn = self.conn.lock().await;
                let conn = conn.deref_mut();
                @%- if def.is_soft_delete() %@
                let obj = if self.with_trashed {
                    _@{ pascal_name }@_::find_optional_with_trashed(conn, @{ def.primaries()|fmt_join_with_paren2("self.id{convert_from_entity}", "self.id.{index}{convert_from_entity}", ", ") }@, self.joiner, self.filter).await?
                } else {
                    _@{ pascal_name }@_::find_optional(conn, @{ def.primaries()|fmt_join_with_paren2("self.id{convert_from_entity}", "self.id.{index}{convert_from_entity}", ", ") }@, self.joiner, self.filter).await?
                };
                @%- else %@
                let obj = _@{ pascal_name }@_::find_optional(conn, @{ def.primaries()|fmt_join_with_paren2("self.id{convert_from_entity}", "self.id.{index}{convert_from_entity}", ", ") }@, self.joiner, self.filter).await?;
                @%- endif %@
                if let Some(obj) = obj {
                    Ok(Some(Box::new(obj) as Box<dyn @{ pascal_name }@>))
                } else {
                    Ok(None)
                }
            }
            fn filter(mut self: Box<Self>, filter: Filter_) -> Box<dyn _QueryFindBuilder> {
                self.filter = Some(filter);
                self
            }
            @%- if def.is_soft_delete() %@
            fn with_trashed(mut self: Box<Self>, mode: bool) -> Box<dyn _QueryFindBuilder> {
                self.with_trashed = mode;
                self
            }
            @%- endif %@
            fn join(mut self: Box<Self>, joiner: Option<Box<Joiner_>>) -> Box<dyn _QueryFindBuilder> {
                self.joiner = Joiner_::merge(self.joiner, joiner);
                self
            }
        }
        Box::new(V {
            conn: self._conn.clone(),
            id,
            filter: None,
            @%- if def.is_soft_delete() %@
            with_trashed: false,
            @%- endif %@
            joiner: None,
        })
    }
    @%- endif %@
}
@{-"\n"}@