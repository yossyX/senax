use ::async_trait::async_trait;

#[allow(unused_imports)]
use ::base_domain as domain;
use ::base_domain::models::{@{ db|snake|ident }@::@{ group_name|snake|ident }@::@{ mod_name|ident }@::{@{ pascal_name }@, @{ pascal_name }@Updater as _Updater}};
#[allow(unused_imports)]
use ::base_domain::models::{self, ToGeoPoint as _, ToPoint as _};
#[allow(unused_imports)]
use ::base_domain::value_objects;

#[allow(unused_imports)]
use ::base_domain::models::@{ db|snake|ident }@ as _model_;
#[allow(unused_imports)]
use crate::repositories as _repository_;
@%- for (name, rel_def) in def.belongs_to_outer_db(Joinable::Join) %@
pub use ::base_domain::models::@{ rel_def.db()|snake|ident }@ as _@{ rel_def.db()|snake }@_model_;
@%- endfor %@
#[cfg(any(feature = "mock", test))]
use ::base_domain::models::@{ db|snake|ident }@::@{ group_name|snake|ident }@::@{ mod_name|ident }@::@{ pascal_name }@Entity;
#[cfg(any(feature = "mock", test))]
use ::base_domain::models::Check_;
pub use ::base_relations_@{ db|snake }@::@{ group_name|snake|ident }@::@{ mod_name|ident }@::*;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct @{ pascal_name }@Factory {
@{- def.non_auto_primary_for_factory()|fmt_join("
{label}{comment}    pub {ident}: {domain_factory},", "") }@
}

impl @{ pascal_name }@Factory {
    pub fn from(value: ::serde_json::Value) -> anyhow::Result<Self> {
        Ok(::serde_json::from_value(value)?)
    }
    pub fn create(self, repo: Box<dyn crate::repositories::Repository_>) -> Box<dyn _Updater> {
        let repo = repo.@{ group_name|snake|ident }@().@{ mod_name|ident }@();
        repo.convert_factory(self)
    }
}

#[allow(unused_imports)]
use _@{ pascal_name }@RepositoryFindBuilder as _RepositoryFindBuilder;

#[async_trait]
pub trait _@{ pascal_name }@RepositoryFindBuilder: Send + Sync {
    async fn query_for_update(self: Box<Self>) -> anyhow::Result<Box<dyn _Updater>>;
    async fn query(self: Box<Self>) -> anyhow::Result<Option<Box<dyn @{ pascal_name }@>>>;
    fn filter(self: Box<Self>, filter: Filter_) -> Box<dyn _RepositoryFindBuilder>;
    fn with_filter_flag(self: Box<Self>, name: &'static str, filter_flag: Filter_) -> Box<dyn _RepositoryFindBuilder>;
    fn with_filter_flag_when(self: Box<Self>, condition: bool, name: &'static str, filter_flag: Filter_) -> Box<dyn _RepositoryFindBuilder>;
    @%- if def.is_soft_delete() %@
    fn with_trashed(self: Box<Self>, mode: bool) -> Box<dyn _RepositoryFindBuilder>;
    @%- endif %@
    fn join(self: Box<Self>, joiner: Option<Box<Joiner_>>) -> Box<dyn _RepositoryFindBuilder>;
}

#[async_trait]
pub trait _@{ pascal_name }@Repository: Send + Sync {
@%- if !def.disable_update() %@
    fn find(&self, id: @{ def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") }@) -> Box<dyn _@{ pascal_name }@RepositoryFindBuilder>;
@%- endif %@
    async fn query_virtual_row(&self, obj: &Box<dyn _Updater>, filter_flag: Filter_) -> anyhow::Result<bool>;
    fn convert_factory(&self, factory: @{ pascal_name }@Factory) -> Box<dyn _Updater>;
    #[deprecated(note = "This method should not be used outside the domain.")]
    async fn save(&self, obj: Box<dyn _Updater>) -> anyhow::Result<Option<Box<dyn @{ pascal_name }@>>>;
@%- if !def.disable_update() %@
    #[deprecated(note = "This method should not be used outside the domain.")]
    async fn import(&self, list: Vec<Box<dyn _Updater>>, option: Option<base_domain::models::ImportOption>) -> anyhow::Result<()>;
@%- endif %@
@%- if def.enable_delayed_insert() %@
    #[deprecated(note = "This method should not be used outside the domain.")]
    async fn delayed_insert(&self, obj: Box<dyn _Updater>) -> anyhow::Result<()>;
@%- endif %@
@%- if !def.disable_delete() %@
    #[deprecated(note = "This method should not be used outside the domain.")]
    async fn delete(&self, obj: Box<dyn _Updater>) -> anyhow::Result<()>;
    @%- if def.primaries().len() == 1 %@
    #[deprecated(note = "This method should not be used outside the domain.")]
    async fn delete_by_ids(&self, ids: &[@{ def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") }@]) -> anyhow::Result<u64>;
    @%- endif %@
    #[deprecated(note = "This method should not be used outside the domain.")]
    async fn delete_all(&self) -> anyhow::Result<()>;
@%- endif %@
@%- if def.act_as_job_queue() %@
    async fn fetch(&self, limit: usize) -> anyhow::Result<Vec<Box<dyn _Updater>>>;
@%- endif %@
@%- for (selector, selector_def) in def.selectors %@
    fn @{ selector|ident }@(&self) -> Box<dyn @{ pascal_name }@Repository@{ selector|pascal }@Builder>;
@%- endfor %@
}
@%- for (selector, selector_def) in def.selectors %@

#[allow(unused_imports)]
use @{ pascal_name }@Repository@{ selector|pascal }@Builder as _Repository@{ selector|pascal }@Builder;

#[async_trait]
pub trait @{ pascal_name }@Repository@{ selector|pascal }@Builder: Send + Sync {
    async fn query_for_update(self: Box<Self>) -> anyhow::Result<Vec<Box<dyn _Updater>>>;
    async fn query(self: Box<Self>) -> anyhow::Result<Vec<Box<dyn @{ pascal_name }@>>>;
    async fn count(self: Box<Self>) -> anyhow::Result<i64>;
    fn selector_filter(self: Box<Self>, filter: @{ pascal_name }@Query@{ selector|pascal }@Filter) -> Box<dyn _Repository@{ selector|pascal }@Builder>;
    fn selector_filter_in_json(self: Box<Self>, filter: ::serde_json::Value) -> anyhow::Result<Box<dyn _Repository@{ selector|pascal }@Builder>> {
        Ok(self.selector_filter(::serde_json::from_value(filter)?))
    }
    fn extra_filter(self: Box<Self>, filter: Filter_) -> Box<dyn _Repository@{ selector|pascal }@Builder>;
    fn with_filter_flag(self: Box<Self>, name: &'static str, filter: Filter_) -> Box<dyn _Repository@{ selector|pascal }@Builder>;
    fn with_filter_flag_when(self: Box<Self>, condition: bool, name: &'static str, filter: Filter_) -> Box<dyn _Repository@{ selector|pascal }@Builder>;
    @%- if def.is_soft_delete() %@
    fn with_trashed(self: Box<Self>, mode: bool) -> Box<dyn _Repository@{ selector|pascal }@Builder>;
    @%- endif %@
    fn join(self: Box<Self>, joiner: Option<Box<Joiner_>>) -> Box<dyn _Repository@{ selector|pascal }@Builder>;
}
@%- endfor %@
@%- for (selector, selector_def) in def.selectors %@

#[allow(unused_imports)]
use @{ pascal_name }@Query@{ selector|pascal }@Builder as _Query@{ selector|pascal }@Builder;

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Eq, Debug, Clone, Copy, Default, async_graphql::Enum)]
#[serde(deny_unknown_fields)]
#[graphql(name = "@{ config.layer_name(db, group_name) }@@{ pascal_name }@Query@{ selector|pascal }@Order")]
#[derive(utoipa::ToSchema)]
#[schema(as = @{ config.layer_name(db, group_name) }@@{ pascal_name }@Query@{ selector|pascal }@Order)]
pub enum @{ pascal_name }@Query@{ selector|pascal }@Order {
    #[default]
    @%- for (order, _) in selector_def.orders %@
    @{ order|pascal }@,
    @%- endfor %@
    #[graphql(name = "_NONE")]
    _None,
}

#[allow(unused_parens)]
impl @{ pascal_name }@Query@{ selector|pascal }@Order {
    #[allow(clippy::borrowed_box)]
    pub fn to_cursor<T: @{ pascal_name }@ + ?Sized>(&self, _obj: &Box<T>) -> Option<String> {
        match self {
            @%- for (order, order_def) in selector_def.orders %@
            @{ pascal_name }@Query@{ selector|pascal }@Order::@{ order|pascal }@ => {
                @%- if order_def.direct_sql.is_some() %@
                None
                @%- else %@
                use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
                let v = @{ order_def.field_tuple(def) }@;
                let mut buf = Vec::new();
                ciborium::into_writer(&v, &mut buf).unwrap();
                Some(URL_SAFE_NO_PAD.encode(buf))
                @%- endif %@
            }
            @%- endfor %@
            @{ pascal_name }@Query@{ selector|pascal }@Order::_None => None,
        }
    }
}

#[allow(unused_parens)]
#[derive(Debug, Clone)]
pub enum @{ pascal_name }@Query@{ selector|pascal }@Cursor {
    @%- for (order, order_def) in selector_def.orders %@
    @{ order|pascal }@(models::Cursor<@{ order_def.type_str(def) }@>),
    @%- endfor %@
}
#[allow(unused_parens)]
impl @{ pascal_name }@Query@{ selector|pascal }@Cursor {
    @%- for (order, order_def) in selector_def.orders %@
    pub fn @{ order }@_from_str(_v: &str) -> anyhow::Result<@{ order_def.type_str(def) }@> {
        @%- if order_def.direct_sql.is_some() %@
        Ok(())
        @%- else %@
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
        Ok(ciborium::from_reader(URL_SAFE_NO_PAD.decode(_v)?.as_slice())?)
        @%- endif %@
    }
    @%- endfor %@
}

#[async_trait]
pub trait @{ pascal_name }@Query@{ selector|pascal }@Builder: Send + Sync {
    async fn query(self: Box<Self>) -> anyhow::Result<Vec<Box<dyn @{ pascal_name }@>>>;
    async fn stream(self: Box<Self>, single_transaction: bool) -> anyhow::Result<std::pin::Pin<Box<dyn futures::Stream<Item=anyhow::Result<Box<dyn @{ pascal_name }@>>> + Send>>>;
    async fn count(self: Box<Self>) -> anyhow::Result<i64>;
    fn selector_filter(self: Box<Self>, filter: @{ pascal_name }@Query@{ selector|pascal }@Filter) -> Box<dyn _Query@{ selector|pascal }@Builder>;
    fn selector_filter_in_json(self: Box<Self>, filter: ::serde_json::Value) -> anyhow::Result<Box<dyn _Query@{ selector|pascal }@Builder>> {
        Ok(self.selector_filter(::serde_json::from_value(filter)?))
    }
    fn extra_filter(self: Box<Self>, filter: Filter_) -> Box<dyn _Query@{ selector|pascal }@Builder>;
    fn with_filter_flag(self: Box<Self>, name: &'static str, filter: Filter_) -> Box<dyn _Query@{ selector|pascal }@Builder>;
    fn with_filter_flag_when(self: Box<Self>, condition: bool, name: &'static str, filter: Filter_) -> Box<dyn _Query@{ selector|pascal }@Builder>;
    fn cursor(self: Box<Self>, cursor: @{ pascal_name }@Query@{ selector|pascal }@Cursor) -> Box<dyn _Query@{ selector|pascal }@Builder>;
    fn order_by(self: Box<Self>, order: @{ pascal_name }@Query@{ selector|pascal }@Order) -> Box<dyn _Query@{ selector|pascal }@Builder>;
    fn reverse(self: Box<Self>, mode: bool) -> Box<dyn _Query@{ selector|pascal }@Builder>;
    fn limit(self: Box<Self>, limit: usize) -> Box<dyn _Query@{ selector|pascal }@Builder>;
    fn offset(self: Box<Self>, offset: usize) -> Box<dyn _Query@{ selector|pascal }@Builder>;
    @%- if def.is_soft_delete() %@
    fn with_trashed(self: Box<Self>, mode: bool) -> Box<dyn _Query@{ selector|pascal }@Builder>;
    @%- endif %@
    fn join(self: Box<Self>, joiner: Option<Box<Joiner_>>) -> Box<dyn _Query@{ selector|pascal }@Builder>;
}
@%- endfor %@

#[allow(unused_imports)]
use _@{ pascal_name }@QueryFindBuilder as _QueryFindBuilder;

#[async_trait]
pub trait _@{ pascal_name }@QueryFindBuilder: Send + Sync {
    async fn query(self: Box<Self>) -> anyhow::Result<Option<Box<dyn @{ pascal_name }@>>>;
    fn filter(self: Box<Self>, filter: Filter_) -> Box<dyn _QueryFindBuilder>;
    fn with_filter_flag(self: Box<Self>, name: &'static str, filter: Filter_) -> Box<dyn _QueryFindBuilder>;
    fn with_filter_flag_when(self: Box<Self>, condition: bool, name: &'static str, filter: Filter_) -> Box<dyn _QueryFindBuilder>;
    @%- if def.is_soft_delete() %@
    fn with_trashed(self: Box<Self>, mode: bool) -> Box<dyn _QueryFindBuilder>;
    @%- endif %@
    fn join(self: Box<Self>, joiner: Option<Box<Joiner_>>) -> Box<dyn _QueryFindBuilder>;
}

#[async_trait]
pub trait _@{ pascal_name }@QueryService: Send + Sync {
    @%- if def.enable_all_rows_cache() && !def.enable_filtered_rows_cache() %@
    async fn all(&self) -> anyhow::Result<Box<dyn base_domain::models::EntityIterator<dyn @{ pascal_name }@>>>;
    @%- endif %@
    @%- for (selector, selector_def) in def.selectors %@
    fn @{ selector|ident }@(&self) -> Box<dyn @{ pascal_name }@Query@{ selector|pascal }@Builder>;
    @%- endfor %@
    fn find(&self, id: @{ def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") }@) -> Box<dyn _@{ pascal_name }@QueryFindBuilder>;
}

#[cfg(any(feature = "mock", test))]
#[derive(derive_new::new, Clone)]
pub struct Emu@{ pascal_name }@Repository {
    pub(crate) _repo: ::std::sync::Arc<::std::sync::Mutex<::std::collections::HashMap<::std::any::TypeId, Box<dyn ::std::any::Any + Send + Sync>>>>,
    pub(crate) _data: ::std::sync::Arc<::std::sync::Mutex<::std::collections::BTreeMap<@{- def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") }@, @{ pascal_name }@Entity>>>
}

#[cfg(any(feature = "mock", test))]
impl Emu@{ pascal_name }@Repository {
    pub fn _load(&self, data: &Vec<@{ pascal_name }@Entity>) {
        let mut map = self._data.lock().unwrap();
        for v in data {
            map.insert(@{- def.primaries()|fmt_join_with_paren("v.{ident}{clone}", ", ") }@, v.clone());
        }
    }
}
#[cfg(any(feature = "mock", test))]
#[async_trait]
impl _@{ pascal_name }@Repository for Emu@{ pascal_name }@Repository {
    @%- if !def.disable_update() %@
    fn find(&self, id: @{ def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") }@) -> Box<dyn _@{ pascal_name }@RepositoryFindBuilder> {
        struct V(Option<@{ pascal_name }@Entity>, Option<Filter_>, std::collections::BTreeMap<&'static str, Filter_>@% if def.is_soft_delete() %@, bool@% endif %@);
        #[async_trait]
        impl _@{ pascal_name }@RepositoryFindBuilder for V {
            async fn query_for_update(self: Box<Self>) -> anyhow::Result<Box<dyn _Updater>> {
                use anyhow::Context;
                let filter = self.1;
                self.0.filter(|v| filter.map(|f| f.check(v as &dyn @{ pascal_name }@)).unwrap_or(Ok(true)).unwrap())@{- def.soft_delete_tpl2("",".filter(|v| self.3 || v.deleted_at.is_none())",".filter(|v| self.3 || !v.deleted)",".filter(|v| self.3 || v.deleted == 0)")}@
                    .map(|v| Box::new(v) as Box<dyn _Updater>)
                    .with_context(|| "Not Found")
            }
            async fn query(self: Box<Self>) -> anyhow::Result<Option<Box<dyn @{ pascal_name }@>>> {
                let filter = self.1;
                Ok(self.0.filter(|v| filter.map(|f| f.check(v as &dyn @{ pascal_name }@)).unwrap_or(Ok(true)).unwrap())@{- def.soft_delete_tpl2("",".filter(|v| self.3 || v.deleted_at.is_none())",".filter(|v| self.3 || !v.deleted)",".filter(|v| self.3 || v.deleted == 0)")}@.map(|v| Box::new(v) as Box<dyn @{ pascal_name }@>))
            }
            fn filter(mut self: Box<Self>, filter: Filter_) -> Box<dyn _RepositoryFindBuilder> { self.1 = Some(filter); self }
            fn with_filter_flag(mut self: Box<Self>, name: &'static str, filter_flag: Filter_) -> Box<dyn _RepositoryFindBuilder> {
                self.2.insert(name, filter_flag);
                self
            }
            fn with_filter_flag_when(self: Box<Self>, condition: bool, name: &'static str, filter_flag: Filter_) -> Box<dyn _RepositoryFindBuilder> {
                if condition {
                    self.with_filter_flag(name, filter_flag)
                } else {
                    self
                }
            }
            @%- if def.is_soft_delete() %@
            fn with_trashed(mut self: Box<Self>, mode: bool) -> Box<dyn _RepositoryFindBuilder> { self.3 = mode; self }
            @%- endif %@
            fn join(self: Box<Self>, _join: Option<Box<Joiner_>>) -> Box<dyn _RepositoryFindBuilder> { self }
        }
        let map = self._data.lock().unwrap();
        Box::new(V(map.get(&id).cloned(), None, Default::default()@% if def.is_soft_delete() %@, false@% endif %@))
    }
    @%- endif %@
    async fn query_virtual_row(&self, obj: &Box<dyn _Updater>, filter_flag: Filter_) -> anyhow::Result<bool> {
        use domain::models::Check_ as _;
        if let Ok(flag) = filter_flag.check(&**obj as &dyn @{ pascal_name }@) {
            return Ok(flag)
        }
        unimplemented!();
    }
    fn convert_factory(&self, _factory: @{ pascal_name }@Factory) -> Box<dyn _Updater> {
        #[allow(unused_imports)]
        use base_domain::models::ToRawValue as _;
        Box::new(@{ pascal_name }@Entity {
@{- def.non_auto_primary_for_factory()|fmt_join("
            {ident}: _factory.{ident}{convert_domain_factory},", "") }@
            ..Default::default()
        })
    }
    #[allow(unused_mut)]
    async fn save(&self, obj: Box<dyn _Updater>) -> anyhow::Result<Option<Box<dyn @{ pascal_name }@>>> {
        let Ok(mut obj) = (obj as Box<dyn std::any::Any>).downcast::<@{ pascal_name }@Entity>() else {
            panic!("Only @{ pascal_name }@Entity is accepted.");
        };
        if obj._delete {
            @%- if !def.disable_delete() %@
            #[allow(deprecated)]
            self.delete(obj).await?;
            @%- endif %@
            Ok(None)
        } else {
            let mut map = self._data.lock().unwrap();
            @%- for (name, column_def) in def.auto_inc_or_seq() %@
            if obj.@{ name|ident }@ == 0.into() {
                obj.@{ name|ident }@ = (map.iter().map(|(_k, v)| @{ column_def.get_inner_type(true, false) }@::from(v.@{ name|ident }@)).max().unwrap_or_default() + 1).into();
            }
            @%- endfor %@
            @%- for (name, column_def) in def.auto_uuid() %@
            if obj.@{ name|ident }@.is_nil() {
                obj.@{ name|ident }@ = uuid::Uuid::new_v4().into();
            }
            @%- endfor %@
            map.insert(@{- def.primaries()|fmt_join_with_paren("obj.{ident}{clone}", ", ") }@, *obj.clone());
            Ok(Some(obj as Box<dyn @{ pascal_name }@>))
        }
    }
    @%- if !def.disable_update() %@
    #[allow(unused_mut)]
    async fn import(&self, list: Vec<Box<dyn _Updater>>, _option: Option<base_domain::models::ImportOption>) -> anyhow::Result<()> {
        for obj in list {
            let Ok(mut obj) = (obj as Box<dyn std::any::Any>).downcast::<@{ pascal_name }@Entity>() else {
                panic!("Only @{ pascal_name }@Entity is accepted.");
            };
            if obj._delete {
                @%- if !def.disable_delete() %@
                #[allow(deprecated)]
                self.delete(obj).await?;
                @%- endif %@
            } else {
                let mut map = self._data.lock().unwrap();
                @%- for (name, column_def) in def.auto_inc_or_seq() %@
                if obj.@{ name|ident }@ == 0.into() {
                    obj.@{ name|ident }@ = (map.iter().map(|(_k, v)| @{ column_def.get_inner_type(true, false) }@::from(v.@{ name|ident }@)).max().unwrap_or_default() + 1).into();
                }
                @%- endfor %@
                @%- for (name, column_def) in def.auto_uuid() %@
                if obj.@{ name|ident }@.is_nil() {
                    obj.@{ name|ident }@ = uuid::Uuid::new_v4().into();
                }
                @%- endfor %@
                map.insert(@{- def.primaries()|fmt_join_with_paren("obj.{ident}{clone}", ", ") }@, *obj.clone());
            }
        }
        Ok(())
    }
    @%- endif %@
    @%- if def.enable_delayed_insert() %@
    #[allow(unused_mut)]
    async fn delayed_insert(&self, obj: Box<dyn _Updater>) -> anyhow::Result<()> {
        let Ok(mut obj) = (obj as Box<dyn std::any::Any>).downcast::<@{ pascal_name }@Entity>() else {
            panic!("Only @{ pascal_name }@Entity is accepted.");
        };
        let mut map = self._data.lock().unwrap();
        @%- for (name, column_def) in def.auto_inc_or_seq() %@
        if obj.@{ name|ident }@ == 0.into() {
            obj.@{ name|ident }@ = (map.iter().map(|(_k, v)| @{ column_def.get_inner_type(true, false) }@::from(v.@{ name|ident }@)).max().unwrap_or_default() + 1).into();
        }
        @%- endfor %@
        @%- for (name, column_def) in def.auto_uuid() %@
        if obj.@{ name|ident }@.is_nil() {
            obj.@{ name|ident }@ = uuid::Uuid::new_v4().into();
        }
        @%- endfor %@
        map.insert(@{- def.primaries()|fmt_join_with_paren("obj.{ident}{clone}", ", ") }@, *obj.clone());
        Ok(())
    }
    @%- endif %@
    @%- if !def.disable_delete() %@
    async fn delete(&self, obj: Box<dyn _Updater>) -> anyhow::Result<()> {
        let mut map = self._data.lock().unwrap();
        map.remove(&@{- def.primaries()|fmt_join_with_paren("obj.{ident}(){clone}", ", ") }@);
        Ok(())
    }
    @%- if def.primaries().len() == 1 %@
    async fn delete_by_ids(&self, ids: &[@{ def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") }@]) -> anyhow::Result<u64> {
        let mut count = 0;
        let mut map = self._data.lock().unwrap();
        for id in ids {
            if map.remove(id).is_some() {
                count += 1;
            }
        }
        Ok(count)
    }
    @%- endif %@
    async fn delete_all(&self) -> anyhow::Result<()> {
        let mut map = self._data.lock().unwrap();
        map.clear();
        Ok(())
    }
    @%- endif %@
    @%- if def.act_as_job_queue() %@
    async fn fetch(&self, limit: usize) -> anyhow::Result<Vec<Box<dyn _Updater>>> {
        let map = self._data.lock().unwrap();
        Ok(map.iter().take(limit).map(|(_, v)| Box::new(v.clone()) as Box<dyn _Updater>).collect())
    }
    @%- endif %@
    @%- for (selector, selector_def) in def.selectors %@
    fn @{ selector|ident }@(&self) -> Box<dyn @{ pascal_name }@Repository@{ selector|pascal }@Builder> {
        #[derive(Default)]
        struct V {
            _list: Vec<@{ pascal_name }@Entity>,
            selector_filter: Option<@{ pascal_name }@Query@{ selector|pascal }@Filter>,
            extra_filter: Option<Filter_>,
            filter_flag: std::collections::BTreeMap<&'static str, Filter_>,
            @%- if def.is_soft_delete() %@
            with_trashed: bool,
            @%- endif %@
        }
        #[async_trait]
        impl @{ pascal_name }@Repository@{ selector|pascal }@Builder for V {
            async fn query_for_update(self: Box<Self>) -> anyhow::Result<Vec<Box<dyn _Updater>>> {
                let list: Vec<_> = self._list.into_iter()
                    .filter(|v| {
                        if let Some(filter) = &self.selector_filter {
                            if !_filter_@{ selector }@(v, filter) {
                                return false;
                            }
                        }
                        if let Some(filter) = &self.extra_filter {
                            if !filter.check(v as &dyn @{ pascal_name }@).unwrap() {
                                return false;
                            }
                        }
                        @{ def.soft_delete_tpl2("true","self.with_trashed || v.deleted_at.is_none()","self.with_trashed || !v.deleted","self.with_trashed || v.deleted == 0")}@
                    })
                    .map(|v| Box::new(v) as Box<dyn _Updater>).collect();
                Ok(list)
            }
            async fn query(self: Box<Self>) -> anyhow::Result<Vec<Box<dyn @{ pascal_name }@>>> {
                let list: Vec<_> = self._list.into_iter()
                    .filter(|v| {
                        if let Some(filter) = &self.selector_filter {
                            if !_filter_@{ selector }@(v, filter) {
                                return false;
                            }
                        }
                        if let Some(filter) = &self.extra_filter {
                            if !filter.check(v as &dyn @{ pascal_name }@).unwrap() {
                                return false;
                            }
                        }
                        @{ def.soft_delete_tpl2("true","self.with_trashed || v.deleted_at.is_none()","self.with_trashed || !v.deleted","self.with_trashed || v.deleted == 0")}@
                    })
                    .map(|v| Box::new(v) as Box<dyn @{ pascal_name }@>).collect();
                Ok(list)
            }
            async fn count(self: Box<Self>) -> anyhow::Result<i64> {
                let list: Vec<_> = self._list.into_iter()
                    .filter(|v| {
                        if let Some(filter) = &self.selector_filter {
                            if !_filter_@{ selector }@(v, filter) {
                                return false;
                            }
                        }
                        if let Some(filter) = &self.extra_filter {
                            if !filter.check(v as &dyn @{ pascal_name }@).unwrap() {
                                return false;
                            }
                        }
                        @{ def.soft_delete_tpl2("true","self.with_trashed || v.deleted_at.is_none()","self.with_trashed || !v.deleted","self.with_trashed || v.deleted == 0")}@
                    })
                    .map(|v| Box::new(v) as Box<dyn @{ pascal_name }@>).collect();
                Ok(list.len() as i64)
            }
            fn selector_filter(mut self: Box<Self>, filter: @{ pascal_name }@Query@{ selector|pascal }@Filter) -> Box<dyn _Repository@{ selector|pascal }@Builder> { self.selector_filter = Some(filter); self }
            fn extra_filter(mut self: Box<Self>, filter: Filter_) -> Box<dyn _Repository@{ selector|pascal }@Builder> { self.extra_filter = Some(filter); self }
            fn with_filter_flag(mut self: Box<Self>, name: &'static str, filter: Filter_) -> Box<dyn _Repository@{ selector|pascal }@Builder> { self.filter_flag.insert(name, filter); self }
            fn with_filter_flag_when(self: Box<Self>, condition: bool, name: &'static str, filter: Filter_) -> Box<dyn _Repository@{ selector|pascal }@Builder> {
                if condition {
                    self.with_filter_flag(name, filter)
                } else {
                    self
                }
            }
            @%- if def.is_soft_delete() %@
            fn with_trashed(mut self: Box<Self>, mode: bool) -> Box<dyn _Repository@{ selector|pascal }@Builder> { self.with_trashed = mode; self  }
            @%- endif %@
            fn join(self: Box<Self>, _join: Option<Box<Joiner_>>) -> Box<dyn _Repository@{ selector|pascal }@Builder> { self }
        }
        Box::new(V{_list: self._data.lock().unwrap().values().map(|v| v.clone()).collect(), ..Default::default()})
    }
    @%- endfor %@
}
@%- for (selector, selector_def) in def.selectors %@
@%- for filter_map in selector_def.nested_filters(selector, def) %@
#[cfg(any(feature = "mock", test))]
#[allow(unused_variables)]
#[allow(unused_imports)]
fn _filter@{ filter_map.suffix }@(v: &impl base_domain::models::@{ filter_map.db()|snake|ident }@::@{ filter_map.model_group()|snake|ident }@::@{ filter_map.model_name()|snake|ident }@::@{ filter_map.model_name()|pascal }@, filter: &@{ pascal_name }@Query@{ selector|pascal }@@{ filter_map.pascal_name }@Filter) -> bool {
    use base_domain::models::@{ filter_map.db()|snake|ident }@::@{ filter_map.model_group()|snake|ident }@::@{ filter_map.model_name()|snake|ident }@::*;
    use crate::PartialOrdering_;
    @%- for (filter, filter_def) in filter_map.filters %@
    @{- filter_def.emu_str(filter, filter_map.model) }@
    @%- endfor %@
    if let Some(_and) = &filter._and {
        if !_and.iter().all(|f| _filter@{ filter_map.suffix }@(v, f)) {
            return false;
        }
    }
    if let Some(_or) = &filter._or {
        if !_or.iter().any(|f| _filter@{ filter_map.suffix }@(v, f)) {
            return false;
        }
    }
    true
}
@%- endfor %@
@%- endfor %@

#[cfg(any(feature = "mock", test))]
#[async_trait]
impl _@{ pascal_name }@QueryService for Emu@{ pascal_name }@Repository {
    @%- if def.enable_all_rows_cache() && !def.enable_filtered_rows_cache() %@
    async fn all(&self) -> anyhow::Result<Box<dyn base_domain::models::EntityIterator<dyn @{ pascal_name }@>>> {
        struct V(std::collections::BTreeMap<@{- def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") }@, @{ pascal_name }@Entity>);
        impl base_domain::models::EntityIterator<dyn @{ pascal_name }@> for V {
            fn iter(&self) -> Box<dyn Iterator<Item = &(dyn @{ pascal_name }@ + 'static)> + '_> {
                Box::new(self.0.iter().map(|(_, v)| v as &dyn @{ pascal_name }@))
            }
            fn into_iter(self) -> Box<dyn Iterator<Item = Box<dyn @{ pascal_name }@>>> {
                Box::new(self.0.into_iter().map(|(_, v)| Box::new(v) as Box<dyn @{ pascal_name }@>))
            }
        }
        Ok(Box::new(V(self._data.lock().unwrap().clone())))
    }
    @%- endif %@
    @%- for (selector, selector_def) in def.selectors %@
    fn @{ selector|ident }@(&self) -> Box<dyn @{ pascal_name }@Query@{ selector|pascal }@Builder> {
        #[derive(Default)]
        struct V {
            _list: Vec<@{ pascal_name }@Entity>,
            selector_filter: Option<@{ pascal_name }@Query@{ selector|pascal }@Filter>,
            extra_filter: Option<Filter_>,
            filter_flag: std::collections::BTreeMap<&'static str, Filter_>,
            cursor: Option<@{ pascal_name }@Query@{ selector|pascal }@Cursor>,
            order: Option<@{ pascal_name }@Query@{ selector|pascal }@Order>,
            reverse: bool,
            limit: usize,
            offset: usize,
            @%- if def.is_soft_delete() %@
            with_trashed: bool,
            @%- endif %@
        }
        #[allow(unused_variables)]
        #[allow(unreachable_code)]
        fn _cursor(v: &@{ pascal_name }@Entity, cursor: &@{ pascal_name }@Query@{ selector|pascal }@Cursor) -> bool {
            @%- if !selector_def.orders.is_empty() %@
            use crate::PartialOrdering_;
            match cursor {
                @%- for (cursor, cursor_def) in selector_def.orders %@
                @{ pascal_name }@Query@{ selector|pascal }@Cursor::@{ cursor|pascal }@(c) => {
                    match c {
                        @{- cursor_def.emu_str(def) }@
                    }
                }
                @%- endfor %@
            }
            @%- endif %@
            true
        }
        #[async_trait]
        impl @{ pascal_name }@Query@{ selector|pascal }@Builder for V {
            async fn query(self: Box<Self>) -> anyhow::Result<Vec<Box<dyn @{ pascal_name }@>>> {
                let mut list: Vec<_> = self._list.into_iter()
                    .filter(|v| {
                        if let Some(filter) = &self.selector_filter {
                            if !_filter_@{ selector }@(v, filter) {
                                return false;
                            }
                        }
                        if let Some(filter) = &self.extra_filter {
                            if !filter.check(v as &dyn @{ pascal_name }@).unwrap() {
                                return false;
                            }
                        }
                        if let Some(cursor) = &self.cursor {
                            if !_cursor(v, cursor) {
                                return false;
                            }
                        }
                        @{ def.soft_delete_tpl2("true","self.with_trashed || v.deleted_at.is_none()","self.with_trashed || !v.deleted","self.with_trashed || v.deleted == 0")}@
                    })
                    .map(|v| Box::new(v) as Box<dyn @{ pascal_name }@>).collect();
                match self.order.unwrap_or_default() {
                    @%- for (order, fields) in selector_def.orders %@
                    @{ pascal_name }@Query@{ selector|pascal }@Order::@{ order|pascal }@ => @{ selector_def.emu_order(order) }@,
                    @%- endfor %@
                    @{ pascal_name }@Query@{ selector|pascal }@Order::_None => {},
                }
                if self.reverse {
                    list.reverse();
                }
                if self.offset > 0 {
                    list = list.split_off(std::cmp::min(list.len(), self.offset));
                }
                if self.limit > 0 {
                    list.truncate(self.limit);
                }
                Ok(list)
            }
            async fn stream(self: Box<Self>, _single_transaction: bool) -> anyhow::Result<std::pin::Pin<Box<dyn futures::Stream<Item=anyhow::Result<Box<dyn @{ pascal_name }@>>> + Send>>> {
                use futures::StreamExt as _;
                let list = self.query().await?;
                Ok(async_stream::stream! {
                    for obj in list {
                        yield Ok(obj);
                    }
                }.boxed())
            }
            async fn count(self: Box<Self>) -> anyhow::Result<i64> {
                let list: Vec<_> = self._list.into_iter()
                    .filter(|v| {
                        if let Some(filter) = &self.selector_filter {
                            if !_filter_@{ selector }@(v, filter) {
                                return false;
                            }
                        }
                        if let Some(filter) = &self.extra_filter {
                            if !filter.check(v as &dyn @{ pascal_name }@).unwrap() {
                                return false;
                            }
                        }
                        if let Some(cursor) = &self.cursor {
                            if !_cursor(v, cursor) {
                                return false;
                            }
                        }
                        @{ def.soft_delete_tpl2("true","self.with_trashed || v.deleted_at.is_none()","self.with_trashed || !v.deleted","self.with_trashed || v.deleted == 0")}@
                    })
                    .map(|v| Box::new(v) as Box<dyn @{ pascal_name }@>).collect();
                Ok(list.len() as i64)
            }
            fn selector_filter(mut self: Box<Self>, filter: @{ pascal_name }@Query@{ selector|pascal }@Filter) -> Box<dyn _Query@{ selector|pascal }@Builder> { self.selector_filter = Some(filter); self }
            fn extra_filter(mut self: Box<Self>, filter: Filter_) -> Box<dyn _Query@{ selector|pascal }@Builder> { self.extra_filter = Some(filter); self }
            fn with_filter_flag(mut self: Box<Self>, name: &'static str, filter: Filter_) -> Box<dyn _Query@{ selector|pascal }@Builder> { self.filter_flag.insert(name, filter); self }
            fn with_filter_flag_when(self: Box<Self>, condition: bool, name: &'static str, filter: Filter_) -> Box<dyn _Query@{ selector|pascal }@Builder> {
                if condition {
                    self.with_filter_flag(name, filter)
                } else {
                    self
                }
            }
            fn cursor(mut self: Box<Self>, cursor: @{ pascal_name }@Query@{ selector|pascal }@Cursor) -> Box<dyn _Query@{ selector|pascal }@Builder> { self.cursor = Some(cursor); self }
            fn order_by(mut self: Box<Self>, order: @{ pascal_name }@Query@{ selector|pascal }@Order) -> Box<dyn _Query@{ selector|pascal }@Builder> { self.order = Some(order); self  }
            fn reverse(mut self: Box<Self>, mode: bool) -> Box<dyn _Query@{ selector|pascal }@Builder> { self.reverse = mode; self  }
            fn limit(mut self: Box<Self>, limit: usize) -> Box<dyn _Query@{ selector|pascal }@Builder> { self.limit = limit; self  }
            fn offset(mut self: Box<Self>, offset: usize) -> Box<dyn _Query@{ selector|pascal }@Builder> { self.offset = offset; self  }
            @%- if def.is_soft_delete() %@
            fn with_trashed(mut self: Box<Self>, mode: bool) -> Box<dyn _Query@{ selector|pascal }@Builder> { self.with_trashed = mode; self  }
            @%- endif %@
            fn join(self: Box<Self>, _join: Option<Box<Joiner_>>) -> Box<dyn _Query@{ selector|pascal }@Builder> { self }
        }
        Box::new(V{_list: self._data.lock().unwrap().values().map(|v| v.clone()).collect(), ..Default::default()})
    }
    @%- endfor %@
    @%- if def.use_cache() %@
    fn find(&self, id: @{ def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") }@) -> Box<dyn _@{ pascal_name }@QueryFindBuilder> {
        struct V(Option<@{ pascal_name }@Entity>, Option<Filter_>, std::collections::BTreeMap<&'static str, Filter_>@% if def.is_soft_delete() %@, bool@% endif %@);
        #[async_trait]
        impl _@{ pascal_name }@QueryFindBuilder for V {
            async fn query(self: Box<Self>) -> anyhow::Result<Option<Box<dyn @{ pascal_name }@>>> {
                let filter = self.1;
                Ok(self.0.filter(|v| filter.map(|f| f.check(v as &dyn @{ pascal_name }@)).unwrap_or(Ok(true)).unwrap())@{- def.soft_delete_tpl2("",".filter(|v| self.3 || v.deleted_at.is_none())",".filter(|v| self.3 || !v.deleted)",".filter(|v| self.3 || v.deleted == 0)")}@.map(|v| Box::new(v) as Box<dyn @{ pascal_name }@>))
            }
            fn filter(mut self: Box<Self>, filter: Filter_) -> Box<dyn _QueryFindBuilder> { self.1 = Some(filter); self }
            fn with_filter_flag(mut self: Box<Self>, name: &'static str, filter: Filter_) -> Box<dyn _QueryFindBuilder> {
                self.2.insert(name, filter);
                self
            }
            fn with_filter_flag_when(self: Box<Self>, condition: bool, name: &'static str, filter: Filter_) -> Box<dyn _QueryFindBuilder> {
                if condition {
                    self.with_filter_flag(name, filter)
                } else {
                    self
                }
            }
            @%- if def.is_soft_delete() %@
            fn with_trashed(mut self: Box<Self>, mode: bool) -> Box<dyn _QueryFindBuilder> { self.3 = mode; self }
            @%- endif %@
            fn join(self: Box<Self>, _join: Option<Box<Joiner_>>) -> Box<dyn _QueryFindBuilder> { self }
        }
        let map = self._data.lock().unwrap();
        Box::new(V(map.get(&id).cloned(), None, Default::default()@% if def.is_soft_delete() %@, false@% endif %@))
    }
    @%- else %@
    fn find(&self, id: @{ def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") }@) -> Box<dyn _@{ pascal_name }@QueryFindBuilder> {
        struct V(Option<@{ pascal_name }@Entity>, Option<Filter_>, std::collections::BTreeMap<&'static str, Filter_>@% if def.is_soft_delete() %@, bool@% endif %@);
        #[async_trait]
        impl _@{ pascal_name }@QueryFindBuilder for V {
            async fn query(self: Box<Self>) -> anyhow::Result<Option<Box<dyn @{ pascal_name }@>>> {
                let filter = self.1;
                Ok(self.0.filter(|v| filter.map(|f| f.check(v as &dyn @{ pascal_name }@)).unwrap_or(Ok(true)).unwrap())@{- def.soft_delete_tpl2("",".filter(|v| self.3 || v.deleted_at.is_none())",".filter(|v| self.3 || !v.deleted)",".filter(|v| self.3 || v.deleted == 0)")}@.map(|v| Box::new(v) as Box<dyn @{ pascal_name }@>))
            }
            fn filter(mut self: Box<Self>, filter: Filter_) -> Box<dyn _QueryFindBuilder> { self.1 = Some(filter); self }
            fn with_filter_flag(mut self: Box<Self>, name: &'static str, filter: Filter_) -> Box<dyn _QueryFindBuilder> {
                self.2.insert(name, filter);
                self
            }
            fn with_filter_flag_when(self: Box<Self>, condition: bool, name: &'static str, filter: Filter_) -> Box<dyn _QueryFindBuilder> {
                if condition {
                    self.with_filter_flag(name, filter)
                } else {
                    self
                }
            }
            @%- if def.is_soft_delete() %@
            fn with_trashed(mut self: Box<Self>, mode: bool) -> Box<dyn _QueryFindBuilder> { self.3 = mode; self }
            @%- endif %@
            fn join(self: Box<Self>, _join: Option<Box<Joiner_>>) -> Box<dyn _QueryFindBuilder> { self }
        }
        let map = self._data.lock().unwrap();
        Box::new(V(map.get(&id).cloned(), None, Default::default()@% if def.is_soft_delete() %@, false@% endif %@))
    }
    @%- endif %@
}
@{-"\n"}@