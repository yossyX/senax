use ::async_trait::async_trait;
#[allow(unused_imports)]
use ::validator::Validate as _;

#[allow(unused_imports)]
use ::base_domain as domain;
use ::base_domain::models::{@{ db|snake|ident }@::@{ group_name|snake|ident }@::@{ mod_name|ident }@::{@{ pascal_name }@, @{ pascal_name }@Cache, @{ pascal_name }@Common, @{ pascal_name }@Updater as _Updater}, Check_};
#[allow(unused_imports)]
use ::base_domain::models::{self, ToGeoPoint as _, ToPoint as _};
#[allow(unused_imports)]
use ::base_domain::value_objects;

#[allow(unused_imports)]
use ::base_domain::models::@{ db|snake|ident }@ as _model_;
#[allow(unused_imports)]
use crate::repositories as _repository_;
@%- for (name, rel_def) in def.belongs_to_outer_db() %@
pub use base_domain::models::@{ rel_def.db()|snake|ident }@ as _@{ rel_def.db()|snake }@_model_;
pub use repository_@{ rel_def.db()|snake }@_@{ rel_def.get_group_name()|snake }@::repositories as _@{ rel_def.db()|snake }@_repository_;
@%- endfor %@
#[cfg(any(feature = "mock", test))]
use ::base_domain::models::@{ db|snake|ident }@::@{ group_name|snake|ident }@::@{ mod_name|ident }@::@{ pascal_name }@Entity;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct @{ pascal_name }@Factory {
@{- def.non_auto_primary_for_factory()|fmt_join("
{label}{comment}    pub {var}: {domain_factory},", "") }@
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

#[derive(Debug, Clone, Default)]
pub struct Joiner_ {
@{- def.relations()|fmt_rel_join("
    pub {rel_name}: Option<Box<_repository_::{class_mod_path}::Joiner_>>,", "") }@
@{- def.relations_belonging_outer_db(false)|fmt_rel_outer_db_join("
    pub {rel_name}: Option<Box<_{db_snake}_repository_::{class_mod_path}::Joiner_>>,", "") }@
}
impl Joiner_ {
    #[allow(clippy::nonminimal_bool)]
    pub fn has_some(&self) -> bool {
        false
        @{- def.relations()|fmt_rel_join("
            || self.{rel_name}.is_some()", "") }@
        @{- def.relations_belonging_outer_db(false)|fmt_rel_outer_db_join("
            || self.{rel_name}.is_some()", "") }@
    }
    #[allow(unused_variables)]
    pub fn merge(lhs: Option<Box<Self>>, rhs: Option<Box<Self>>) -> Option<Box<Self>> {
        if let Some(lhs) = lhs {
            if let Some(rhs) = rhs {
                Some(Box::new(Joiner_{
                    @{- def.relations()|fmt_rel_join("
                    {rel_name}: _repository_::{class_mod_path}::Joiner_::merge(lhs.{rel_name}, rhs.{rel_name}),", "") }@
                    @{- def.relations_belonging_outer_db(false)|fmt_rel_outer_db_join("
                    {rel_name}: _{db_snake}_repository_::{class_mod_path}::Joiner_::merge(lhs.{rel_name}, rhs.{rel_name}),", "") }@
                }))
            } else {
                Some(lhs)
            }
        } else {
            rhs
        }
    }
}
@%- let fetch_macro_name = "{}_{}_{}"|format(db|snake, group_name, model_name) %@
@% let model_path = "$crate::models::{}::{}::{}"|format(db|snake|ident, group_name|ident, mod_name|ident) -%@
@% let base_path = "$crate::models::{}::{}::_base::_{}"|format(db|snake|ident, group_name|ident, mod_name) -%@
#[macro_export]
macro_rules! _join_@{ fetch_macro_name }@ {
@{- def.relations()|fmt_rel_join("
    ({rel_name}) => ($crate::models::--1--::{group_ident}::{mod_ident}::join!({}));
    ({rel_name}: $p:tt) => ($crate::models::--1--::{group_ident}::{mod_ident}::join!($p));", "")|replace1(db|snake|ident) }@
@{- def.relations_belonging_outer_db(false)|fmt_rel_outer_db_join("
    ({rel_name}) => (--1--::_{db_snake}_model_::{group_ident}::{mod_ident}::join!({}));
    ({rel_name}: $p:tt) => (--1--::_{db_snake}_model_::{group_ident}::{mod_ident}::join!($p));", "")|replace1(base_path) }@
    () => ();
}
pub use _join_@{ fetch_macro_name }@ as _join;
#[macro_export]
macro_rules! join_@{ fetch_macro_name }@ {
    ({$($i:ident $(: $p:tt)?),*}) => (Some(Box::new(@{ model_path }@::Joiner_ {
        $($i: @{ base_path }@::_join!($i $(: $p)?),)*
        ..Default::default()
    })));
}
pub use join_@{ fetch_macro_name }@ as join;

#[allow(unused_imports)]
use _@{ pascal_name }@RepositoryFindBuilder as _RepositoryFindBuilder;

#[async_trait]
pub trait _@{ pascal_name }@RepositoryFindBuilder: Send + Sync {
    async fn query_for_update(self: Box<Self>) -> anyhow::Result<Box<dyn _Updater>>;
    async fn query(self: Box<Self>) -> anyhow::Result<Option<Box<dyn @{ pascal_name }@>>>;
    fn filter(self: Box<Self>, filter: Filter_) -> Box<dyn _RepositoryFindBuilder>;
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
    fn convert_factory(&self, factory: @{ pascal_name }@Factory) -> Box<dyn _Updater>;
    #[deprecated(note = "This method should not be used outside the domain.")]
    async fn save(&self, obj: Box<dyn _Updater>) -> anyhow::Result<Option<Box<dyn @{ pascal_name }@>>>;
@%- if !def.disable_update() %@
    #[deprecated(note = "This method should not be used outside the domain.")]
    async fn import(&self, list: Vec<Box<dyn _Updater>>, option: Option<base_domain::models::ImportOption>) -> anyhow::Result<()>;
@%- endif %@
@%- if def.use_insert_delayed() %@
    #[deprecated(note = "This method should not be used outside the domain.")]
    async fn insert_delayed(&self, obj: Box<dyn _Updater>) -> anyhow::Result<()>;
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
    @%- if def.is_soft_delete() %@
    fn with_trashed(self: Box<Self>, mode: bool) -> Box<dyn _Repository@{ selector|pascal }@Builder>;
    @%- endif %@
    fn join(self: Box<Self>, joiner: Option<Box<Joiner_>>) -> Box<dyn _Repository@{ selector|pascal }@Builder>;
}
@%- endfor %@
@%- for (selector, selector_def) in def.selectors %@

#[allow(unused_imports)]
use @{ pascal_name }@Query@{ selector|pascal }@Builder as _Query@{ selector|pascal }@Builder;
@%- for filter_map in selector_def.nested_filters(selector, def) %@

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Debug, Clone, Default, validator::Validate, async_graphql::InputObject)]
#[serde(deny_unknown_fields)]
#[allow(non_camel_case_types)]
#[graphql(name = "@{ config.layer_name(db, group_name) }@@{ pascal_name }@Query@{ selector|pascal }@@{ filter_map.pascal_name }@Filter")]
#[derive(utoipa::ToSchema)]
#[schema(as = @{ config.layer_name(db, group_name) }@@{ pascal_name }@Query@{ selector|pascal }@@{ filter_map.pascal_name }@Filter)]
pub struct @{ pascal_name }@Query@{ selector|pascal }@@{ filter_map.pascal_name }@Filter {
    @%- for (filter, filter_def) in filter_map.filters %@
    #[graphql(name = "@{ filter }@")]
    @%- if !filter_def.required %@
    #[serde(default, skip_serializing_if = "Option::is_none")]
    @%- endif %@
    @%- if filter_def.has_default() %@
    #[validate(custom(function = "base_domain::models::reject_empty_filter"))]
    @%- endif %@
    pub @{ filter|ident }@: @{ filter_def.type_str(filter, pascal_name, selector, filter_map.pascal_name) }@,
    @%- endfor %@
    #[graphql(name = "_and")]
    #[schema(no_recursion)]
    #[validate(nested)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub _and: Option<Vec<@{ pascal_name }@Query@{ selector|pascal }@@{ filter_map.pascal_name }@Filter>>,
    #[graphql(name = "_or")]
    #[schema(no_recursion)]
    #[validate(nested)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub _or: Option<Vec<@{ pascal_name }@Query@{ selector|pascal }@@{ filter_map.pascal_name }@Filter>>,
}
@%- for (name, type_name) in filter_map.ranges(pascal_name, selector, filter_map.pascal_name) %@

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Debug, Clone, Default, async_graphql::InputObject)]
#[serde(deny_unknown_fields)]
#[allow(non_camel_case_types)]
#[graphql(name = "@{ config.layer_name(db, group_name) }@@{ pascal_name }@Query@{ selector|pascal }@Range@{ filter_map.pascal_name }@_@{ name|pascal }@")]
#[derive(utoipa::ToSchema)]
#[schema(as = @{ config.layer_name(db, group_name) }@@{ pascal_name }@Query@{ selector|pascal }@Range@{ filter_map.pascal_name }@_@{ name|pascal }@)]
pub struct @{ pascal_name }@Query@{ selector|pascal }@Range@{ filter_map.pascal_name }@_@{ name|pascal }@ {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eq: Option<@{ type_name }@>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lt: Option<@{ type_name }@>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lte: Option<@{ type_name }@>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gt: Option<@{ type_name }@>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gte: Option<@{ type_name }@>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_null: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_not_null: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_null_or_lt: Option<@{ type_name }@>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_null_or_lte: Option<@{ type_name }@>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_null_or_gt: Option<@{ type_name }@>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_null_or_gte: Option<@{ type_name }@>,
}
@%- endfor %@
@%- for (name, fields) in filter_map.range_tuples() %@

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Debug, Clone, Default, async_graphql::InputObject)]
#[serde(deny_unknown_fields)]
#[allow(non_camel_case_types)]
#[graphql(name = "@{ config.layer_name(db, group_name) }@@{ pascal_name }@Query@{ selector|pascal }@RangeValues@{ filter_map.pascal_name }@_@{ name|pascal }@")]
#[derive(utoipa::ToSchema)]
#[schema(as = @{ config.layer_name(db, group_name) }@@{ pascal_name }@Query@{ selector|pascal }@RangeValues@{ filter_map.pascal_name }@_@{ name|pascal }@)]
pub struct @{ pascal_name }@Query@{ selector|pascal }@RangeValues@{ filter_map.pascal_name }@_@{ name|pascal }@ {
    @%- for (field, _type) in fields.clone() %@
    #[graphql(name = "@{ field }@")]
    pub @{ field|ident }@: @{ _type }@,
    @%- endfor %@
}
impl @{ pascal_name }@Query@{ selector|pascal }@@{ filter_map.pascal_name }@RangeValues@{ filter_map.pascal_name }@_@{ name|pascal }@ {
    pub fn values(&self) -> (@% for (field, _type) in fields.clone() %@@{ _type }@, @% endfor %@) {
        (@% for (field, _type) in fields.clone() %@self.@{ field|ident }@, @% endfor %@)
    }
}
@%- endfor %@
@%- for (name, type_name) in filter_map.identities(pascal_name, selector, filter_map.pascal_name) %@

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Debug, Clone, Default, async_graphql::InputObject)]
#[serde(deny_unknown_fields)]
#[allow(non_camel_case_types)]
#[graphql(name = "@{ config.layer_name(db, group_name) }@@{ pascal_name }@Query@{ selector|pascal }@Identity@{ filter_map.pascal_name }@_@{ name|pascal }@")]
#[derive(utoipa::ToSchema)]
#[schema(as = @{ config.layer_name(db, group_name) }@@{ pascal_name }@Query@{ selector|pascal }@Identity@{ filter_map.pascal_name }@_@{ name|pascal }@)]
pub struct @{ pascal_name }@Query@{ selector|pascal }@Identity@{ filter_map.pascal_name }@_@{ name|pascal }@ {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eq: Option<@{ type_name }@>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#in: Option<Vec<@{ type_name }@>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_null: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_not_null: Option<bool>,
}
@%- endfor %@
@%- for (name, fields) in filter_map.identity_tuples() %@

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Debug, Clone, Default, async_graphql::InputObject)]
#[serde(deny_unknown_fields)]
#[allow(non_camel_case_types)]
#[graphql(name = "@{ config.layer_name(db, group_name) }@@{ pascal_name }@Query@{ selector|pascal }@IdentityValues@{ filter_map.pascal_name }@_@{ name|pascal }@")]
#[derive(utoipa::ToSchema)]
#[schema(as = @{ config.layer_name(db, group_name) }@@{ pascal_name }@Query@{ selector|pascal }@IdentityValues@{ filter_map.pascal_name }@_@{ name|pascal }@)]
pub struct @{ pascal_name }@Query@{ selector|pascal }@IdentityValues@{ filter_map.pascal_name }@_@{ name|pascal }@ {
    @%- for (field, _type) in fields.clone() %@
    #[graphql(name = "@{ field }@")]
    pub @{ field|ident }@: @{ _type }@,
    @%- endfor %@
}
impl @{ pascal_name }@Query@{ selector|pascal }@IdentityValues@{ filter_map.pascal_name }@_@{ name|pascal }@ {
    pub fn values(&self) -> (@% for (field, _type) in fields.clone() %@@{ _type }@, @% endfor %@) {
        (@% for (field, _type) in fields.clone() %@self.@{ field|ident }@, @% endfor %@)
    }
}
@%- endfor %@
@%- endfor %@

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
    pub fn to_cursor<T: @{ pascal_name }@Common + ?Sized>(&self, _obj: &Box<T>) -> Option<String> {
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
    async fn query(self: Box<Self>) -> anyhow::Result<Vec<Box<dyn @{ pascal_name }@@% if def.use_cache() %@Cache@% endif %@>>>;
    async fn stream(self: Box<Self>, single_transaction: bool) -> anyhow::Result<std::pin::Pin<Box<dyn futures::Stream<Item=anyhow::Result<Box<dyn @{ pascal_name }@@% if def.use_cache() %@Cache@% endif %@>>> + Send>>>;
    async fn count(self: Box<Self>) -> anyhow::Result<i64>;
    fn selector_filter(self: Box<Self>, filter: @{ pascal_name }@Query@{ selector|pascal }@Filter) -> Box<dyn _Query@{ selector|pascal }@Builder>;
    fn selector_filter_in_json(self: Box<Self>, filter: ::serde_json::Value) -> anyhow::Result<Box<dyn _Query@{ selector|pascal }@Builder>> {
        Ok(self.selector_filter(::serde_json::from_value(filter)?))
    }
    fn extra_filter(self: Box<Self>, filter: Filter_) -> Box<dyn _Query@{ selector|pascal }@Builder>;
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
    async fn query(self: Box<Self>) -> anyhow::Result<Option<Box<dyn @{ pascal_name }@@% if def.use_cache() %@Cache@% endif %@>>>;
    fn filter(self: Box<Self>, filter: Filter_) -> Box<dyn _QueryFindBuilder>;
    @%- if def.is_soft_delete() %@
    fn with_trashed(self: Box<Self>, mode: bool) -> Box<dyn _QueryFindBuilder>;
    @%- endif %@
    fn join(self: Box<Self>, joiner: Option<Box<Joiner_>>) -> Box<dyn _QueryFindBuilder>;
}

#[async_trait]
pub trait _@{ pascal_name }@QueryService: Send + Sync {
    @%- if def.use_all_rows_cache() && !def.use_filtered_row_cache() %@
    async fn all(&self) -> anyhow::Result<Box<dyn base_domain::models::EntityIterator<dyn @{ pascal_name }@Cache>>>;
    @%- endif %@
    @%- for (selector, selector_def) in def.selectors %@
    fn @{ selector|ident }@(&self) -> Box<dyn @{ pascal_name }@Query@{ selector|pascal }@Builder>;
    @%- endfor %@
    fn find(&self, id: @{ def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") }@) -> Box<dyn _@{ pascal_name }@QueryFindBuilder>;
}
@%- for (index_name, index) in def.multi_index(false) %@

#[allow(non_camel_case_types)]
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct @{ pascal_name }@Index_@{ index_name }@(@{ index.join_fields(def, "pub {filter_type}", ", ") }@);
impl<@{ index.join_fields(def, "T{index}", ", ") }@> TryFrom<(@{ index.join_fields(def, "T{index}", ", ") }@)> for @{ pascal_name }@Index_@{ index_name }@
where@{ index.join_fields(def, "
    T{index}: TryInto<{filter_type}>,
    T{index}::Error: Into<anyhow::Error>,", "") }@
{
    type Error = anyhow::Error;
    fn try_from(value: (@{ index.join_fields(def, "T{index}", ", ") }@)) -> Result<Self, Self::Error> {
        Ok(Self(@{ index.join_fields(def, "value.{index}.try_into().map_err(|e| e.into())?", ", ") }@))
    }
}
@%- endfor %@

#[allow(non_camel_case_types)]
#[derive(Clone, Debug)]
pub enum Col_ {
@{ def.all_fields()|fmt_join("    {var},", "\n") }@
}
#[allow(unreachable_patterns)]
#[allow(clippy::match_single_binding)]
impl Col_ {
    fn _name(&self) -> &'static str {
        match self {
            @{- def.all_fields()|fmt_join("
            Col_::{var} => \"`{col}`\",", "") }@
            _ => unimplemented!(),
        }
    }
    pub fn check_null<T: @{ pascal_name }@Common + ?Sized>(&self, _obj: &T) -> bool {
        match self {
            @{- def.primaries()|fmt_join("
            Col_::{var} => {filter_check_null},", "") }@
            @{- def.cache_cols_except_primaries_and_invisibles()|fmt_join("
            Col_::{var} => {filter_check_null},", "") }@
            _ => unimplemented!(),
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug)]
pub enum ColOne_ {
@{ def.all_fields_except_json()|fmt_join("    {var}({filter_type}),", "\n") }@
@%- for (index_name, index) in def.multi_index(false) %@
    @{ index.join_fields(def, "{name}", "_") }@(@{ pascal_name }@Index_@{ index_name }@),
@%- endfor %@
}
#[allow(unreachable_patterns)]
#[allow(clippy::match_single_binding)]
impl ColOne_ {
    fn _name(&self) -> &'static str {
        match self {
            @{- def.all_fields_except_json()|fmt_join("
            ColOne_::{var}(_) => \"`{col}`\",", "") }@
            @%- for (index_name, index) in def.multi_index(false) %@
            ColOne_::@{ index.join_fields(def, "{name}", "_") }@(_) => "<@{ index.join_fields(def, "`{name}`", ", ") }@>",
            @%- endfor %@
            _ => unimplemented!(),
        }
    }
    pub fn check_eq<T: @{ pascal_name }@Common + ?Sized>(&self, _obj: &T) -> bool {
        match self {
            @{- def.equivalence_cache_fields_except_json()|fmt_join("
            ColOne_::{var}(c) => _obj.{var}(){filter_check_eq},", "") }@
            @%- for (index_name, index) in def.multi_index(true) %@
            ColOne_::@{ index.join_fields(def, "{name}", "_") }@(@{ pascal_name }@Index_@{ index_name }@(@{ index.join_fields(def, "c{index}", ", ") }@)) => @{ index.join_fields(def, "(_obj.{var}(){filter_check_eq})", " && ") }@,
            @%- endfor %@
            _ => unimplemented!(),
        }
    }
    pub fn check_cmp<T: @{ pascal_name }@Common + ?Sized>(&self, _obj: &T, order: std::cmp::Ordering, eq: bool) -> Result<bool, bool> {
        let o = match self {
            @{- def.comparable_cache_fields_except_json()|fmt_join("
            ColOne_::{var}(c) => _obj.{var}(){filter_check_cmp},", "") }@
            @%- for (index_name, index) in def.multi_index(true) %@
            ColOne_::@{ index.join_fields(def, "{name}", "_") }@(@{ pascal_name }@Index_@{ index_name }@(@{ index.join_fields(def, "c{index}", ", ") }@)) => @{ index.join_fields(def, "(_obj.{var}(){filter_check_cmp})", ".then") }@,
            @%- endfor %@
            _ => unimplemented!(),
        };
        Ok(o == order || eq && o == std::cmp::Ordering::Equal)
    }
    pub fn check_like<T: @{ pascal_name }@Common + ?Sized>(&self, _obj: &T) -> bool {
        #[allow(unused_imports)]
        use models::Like as _;
        match self {
            @{- def.string_cache_fields()|fmt_join("
            ColOne_::{var}(c) => _obj.{var}(){filter_like},", "") }@
            _ => unimplemented!(),
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Hash, serde::Serialize)]
pub enum ColKey_ {
    @{- def.unique_key()|fmt_index_col("
    {var}({filter_type}),", "") }@
}
#[allow(unreachable_patterns)]
#[allow(clippy::match_single_binding)]
impl ColKey_ {
    fn _name(&self) -> &'static str {
        match self {
            @{- def.unique_key()|fmt_join("
            ColKey_::{var}(_) => \"`{col}`\",", "") }@
            _ => unimplemented!(),
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug)]
pub enum ColMany_ {
@{ def.all_fields_except_json()|fmt_join("    {var}(Vec<{filter_type}>),", "\n") }@
@%- for (index_name, index) in def.multi_index(false) %@
    @{ index.join_fields(def, "{name}", "_") }@(Vec<@{ pascal_name }@Index_@{ index_name }@>),
@%- endfor %@
}
#[allow(unreachable_patterns)]
#[allow(clippy::match_single_binding)]
impl ColMany_ {
    fn _name(&self) -> &'static str {
        match self {
            @{- def.all_fields_except_json()|fmt_join("
            ColMany_::{var}(_) => \"`{col}`\",", "") }@
            @%- for (index_name, index) in def.multi_index(false) %@
            ColMany_::@{ index.join_fields(def, "{name}", "_") }@(_) => "<@{ index.join_fields(def, "`{name}`", ", ") }@>",
            @%- endfor %@
            _ => unimplemented!(),
        }
    }
    #[allow(bindings_with_variant_name)]
    pub fn check_in<T: @{ pascal_name }@Common + ?Sized>(&self, _obj: &T) -> bool {
        match self {
            @{- def.equivalence_cache_fields_except_json()|fmt_join("
            ColMany_::{var}(list) => list.iter().any(|c| _obj.{var}(){filter_check_eq}),", "") }@
            @%- for (index_name, index) in def.multi_index(true) %@
            ColMany_::@{ index.join_fields(def, "{name}", "_") }@(list) => list.iter().any(|@{ pascal_name }@Index_@{ index_name }@(@{ index.join_fields(def, "c{index}", ", ") }@)| @{ index.join_fields(def, "(_obj.{var}(){filter_check_eq})", " && ") }@),
            @%- endfor %@
            _ => unimplemented!(),
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug)]
pub enum ColJson_ {
@{- def.all_fields_only_json()|fmt_join("
    {var}(::serde_json::Value),", "") }@
}
#[allow(unreachable_patterns)]
#[allow(clippy::match_single_binding)]
impl ColJson_ {
    fn _name(&self) -> &'static str {
        match self {
            @{- def.all_fields_only_json()|fmt_join("
            ColJson_::{var}(_) => \"`{col}`\",", "") }@
            _ => unimplemented!(),
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug)]
pub enum ColJsonArray_ {
@{- def.all_fields_only_json()|fmt_join("
    {var}(Vec<::serde_json::Value>),", "") }@
}
#[allow(unreachable_patterns)]
#[allow(clippy::match_single_binding)]
impl ColJsonArray_ {
    fn _name(&self) -> &'static str {
        match self {
            @{- def.all_fields_only_json()|fmt_join("
            ColJsonArray_::{var}(_) => \"`{col}`\",", "") }@
            _ => unimplemented!(),
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug)]
pub enum ColGeo_ {
@{- def.all_fields_only_geo()|fmt_join("
    {var}(::serde_json::Value, i32),", "") }@
}
#[allow(unreachable_patterns)]
#[allow(clippy::match_single_binding)]
impl ColGeo_ {
    fn _name(&self) -> &'static str {
        match self {
            @{- def.all_fields_only_geo()|fmt_join("
            ColGeo_::{var}(_, _) => \"`{col}`\",", "") }@
            _ => unimplemented!(),
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug)]
pub enum ColGeoDistance_ {
@{- def.all_fields_only_geo()|fmt_join("
    {var}(::serde_json::Value, f64, i32),", "") }@
}
#[allow(unreachable_patterns)]
#[allow(clippy::match_single_binding)]
impl ColGeoDistance_ {
    fn _name(&self) -> &'static str {
        match self {
            @{- def.all_fields_only_geo()|fmt_join("
            ColGeoDistance_::{var}(_, _, _) => \"`{col}`\",", "") }@
            _ => unimplemented!(),
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug)]
pub enum ColRel_ {
@{- def.relations_one_and_belonging(false)|fmt_rel_join("
    {rel_name}(Option<Box<_repository_::{base_class_mod_path}::Filter_>>),", "") }@
@{- def.relations_many(false)|fmt_rel_join("
    {rel_name}(Option<Box<_repository_::{base_class_mod_path}::Filter_>>),", "") }@
@{- def.relations_belonging_outer_db(false)|fmt_rel_outer_db_join("
    {rel_name}(Option<Box<_{db_snake}_repository_::{base_class_mod_path}::Filter_>>),", "") }@
}
#[allow(unreachable_patterns)]
#[allow(clippy::match_single_binding)]
impl std::fmt::Display for ColRel_ {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            @{- def.relations_one_and_belonging(false)|fmt_rel_join("
            ColRel_::{rel_name}(v) => if let Some(v) = v {
                write!(_f, \"{raw_rel_name}:<{}>\", v)
            } else {
                write!(_f, \"{raw_rel_name}\")
            },", "") }@
            @{- def.relations_many(false)|fmt_rel_join("
            ColRel_::{rel_name}(v) => if let Some(v) = v {
                write!(_f, \"{raw_rel_name}:<{}>\", v)
            } else {
                write!(_f, \"{raw_rel_name}\")
            },", "") }@
            @{- def.relations_belonging_outer_db(false)|fmt_rel_outer_db_join("
            ColRel_::{rel_name}(v) => if let Some(v) = v {
                write!(_f, \"{raw_rel_name}:<{}>\", v)
            } else {
                write!(_f, \"{raw_rel_name}\")
            },", "") }@
            _ => unimplemented!(),
        }
    }
}
impl ColRel_ {
    #[allow(unreachable_patterns)]
    #[allow(clippy::needless_update)]
    #[allow(clippy::match_single_binding)]
    fn joiner(&self) -> Option<Box<Joiner_>> {
        match self {
            @{- def.relations_one_and_belonging(false)|fmt_rel_join("
            ColRel_::{rel_name}(c) => Some(Box::new(Joiner_{
                {rel_name}: Some(c.as_ref().and_then(|c| c.joiner()).unwrap_or_default()),
                ..Default::default()
            })),", "") }@
            @{- def.relations_many(false)|fmt_rel_join("
            ColRel_::{rel_name}(c) => Some(Box::new(Joiner_{
                {rel_name}: Some(c.as_ref().and_then(|c| c.joiner()).unwrap_or_default()),
                ..Default::default()
            })),", "") }@
            @{- def.relations_belonging_outer_db(false)|fmt_rel_outer_db_join("
            ColRel_::{rel_name}(c) => Some(Box::new(Joiner_{
                {rel_name}: Some(c.as_ref().and_then(|c| c.joiner()).unwrap_or_default()),
                ..Default::default()
            })),", "") }@
            _ => unreachable!()
        }
    }
    #[allow(unreachable_patterns)]
    #[allow(clippy::needless_update)]
    #[allow(clippy::match_single_binding)]
    fn joiner_cache_only(&self) -> Option<Box<Joiner_>> {
        match self {
            @{- def.relations_belonging_cache(false)|fmt_rel_join("
            ColRel_::{rel_name}(c) => Some(Box::new(Joiner_{
                {rel_name}: Some(c.as_ref().and_then(|c| c.joiner_cache_only()).unwrap_or_default()),
                ..Default::default()
            })),", "") }@
            @{- def.relations_one_cache(false)|fmt_rel_join("
            ColRel_::{rel_name}(c) => Some(Box::new(Joiner_{
                {rel_name}: Some(c.as_ref().and_then(|c| c.joiner_cache_only()).unwrap_or_default()),
                ..Default::default()
            })),", "") }@
            @{- def.relations_many_cache(false)|fmt_rel_join("
            ColRel_::{rel_name}(c) => Some(Box::new(Joiner_{
                {rel_name}: Some(c.as_ref().and_then(|c| c.joiner_cache_only()).unwrap_or_default()),
                ..Default::default()
            })),", "") }@
            _ => None
        }
    }
}
impl Check_<dyn @{ pascal_name }@Cache> for ColRel_ {
    #[allow(unreachable_patterns)]
    #[allow(unreachable_code)]
    #[allow(clippy::match_single_binding)]
    fn check(&self, _obj: &dyn @{ pascal_name }@Cache) -> anyhow::Result<bool> {
        Ok(match self {
            @{- def.relations_one_and_belonging(false)|fmt_rel_join("
            ColRel_::{rel_name}(None) => _obj.{rel_name}()?.is_some(),
            ColRel_::{rel_name}(Some(f)) => _obj.{rel_name}()?.map(|v| f.check(&*v)).unwrap_or(Ok(false))?,", "") }@
            @{- def.relations_many(false)|fmt_rel_join("
            ColRel_::{rel_name}(None) => !_obj.{rel_name}()?.is_empty(),
            ColRel_::{rel_name}(Some(f)) => _obj.{rel_name}()?.iter().try_fold(false, |acc, v| Ok::<bool, anyhow::Error>(acc || f.check(v.as_ref())?))?,", "") }@
            @{- def.relations_belonging_outer_db(false)|fmt_rel_outer_db_join("
            ColRel_::{rel_name}(None) => _obj.{rel_name}()?.is_some(),
            ColRel_::{rel_name}(Some(f)) => _obj.{rel_name}()?.map(|v| f.check(&*v)).unwrap_or(Ok(false))?,", "") }@
            _ => unreachable!()
        })
    }
}
impl Check_<dyn @{ pascal_name }@> for ColRel_ {
    #[allow(unreachable_patterns)]
    #[allow(unreachable_code)]
    #[allow(clippy::match_single_binding)]
    fn check(&self, _obj: &dyn @{ pascal_name }@) -> anyhow::Result<bool> {
        Ok(match self {
            @{- def.relations_one_and_belonging(false)|fmt_rel_join("
            ColRel_::{rel_name}(None) => _obj.{rel_name}()?.is_some(),
            ColRel_::{rel_name}(Some(f)) => _obj.{rel_name}()?.map(|v| f.check(v)).unwrap_or(Ok(false))?,", "") }@
            @{- def.relations_many(false)|fmt_rel_join("
            ColRel_::{rel_name}(None) => _obj.{rel_name}()?.next().is_some(),
            ColRel_::{rel_name}(Some(f)) => _obj.{rel_name}()?.try_fold(false, |acc, v| Ok::<bool, anyhow::Error>(acc || f.check(v)?))?,", "") }@
            @{- def.relations_belonging_outer_db(false)|fmt_rel_outer_db_join("
            ColRel_::{rel_name}(None) => _obj.{rel_name}()?.is_some(),
            ColRel_::{rel_name}(Some(f)) => _obj.{rel_name}()?.map(|v| f.check(v)).unwrap_or(Ok(false))?,", "") }@
            _ => unreachable!()
        })
    }
}

#[derive(Clone, Debug)]
pub enum Filter_ {
    WithTrashed,
    OnlyTrashed,
    Match(Vec<Col_>, String),
    MatchBoolean(Vec<Col_>, String),
    MatchExpansion(Vec<Col_>, String),
    IsNull(Col_),
    IsNotNull(Col_),
    Eq(ColOne_),
    EqKey(ColKey_),
    NotEq(ColOne_),
    Gt(ColOne_),
    Gte(ColOne_),
    Lt(ColOne_),
    Lte(ColOne_),
    Like(ColOne_),
    AllBits(ColMany_),
    AnyBits(ColOne_),
    In(ColMany_),
    NotIn(ColMany_),
    Contains(ColJsonArray_, Option<String>),
    JsonIn(ColJsonArray_, String),
    JsonContainsPath(ColJson_, String),
    JsonEq(ColJson_, String),
    JsonIsNull(ColJson_, String),
    JsonIsNotNull(ColJson_, String),
    JsonLt(ColJson_, String),
    JsonLte(ColJson_, String),
    JsonGt(ColJson_, String),
    JsonGte(ColJson_, String),
    GeoEquals(ColGeo_),
    Within(ColGeo_),
    Intersects(ColGeo_),
    Crosses(ColGeo_),
    DWithin(ColGeoDistance_),
    Not(Box<Filter_>),
    And(Vec<Filter_>),
    Or(Vec<Filter_>),
    Exists(ColRel_),
    NotExists(ColRel_),
    EqAny(ColRel_),
    NotAll(ColRel_),
    Raw(String),
    RawWithParam(String, Vec<String>),
    Boolean(bool),
}
impl Default for Filter_ {
    fn default() -> Self {
        Filter_::new_and()
    }
}
impl std::fmt::Display for Filter_ {
    #[allow(bindings_with_variant_name)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Filter_::WithTrashed => write!(f, "WithTrashed"),
            Filter_::OnlyTrashed => write!(f, "OnlyTrashed"),
            Filter_::Match(cols, _) => write!(f, "Match:<{}>", cols.iter().map(|v| v._name()).collect::<Vec<_>>().join(",")),
            Filter_::MatchBoolean(cols, _) => write!(f, "MatchBoolean:<{}>", cols.iter().map(|v| v._name()).collect::<Vec<_>>().join(",")),
            Filter_::MatchExpansion(cols, _) => write!(f, "MatchExpansion:<{}>", cols.iter().map(|v| v._name()).collect::<Vec<_>>().join(",")),
            Filter_::IsNull(col) => write!(f, "IsNull:{}", col._name()),
            Filter_::IsNotNull(col) => write!(f, "IsNotNull:{}", col._name()),
            Filter_::Eq(col) => write!(f, "Eq:{}", col._name()),
            Filter_::EqKey(col) => write!(f, "EqKey:{}", col._name()),
            Filter_::NotEq(col) => write!(f, "NotEq:{}", col._name()),
            Filter_::Gt(col) => write!(f, "Gt:{}", col._name()),
            Filter_::Gte(col) => write!(f, "Gte:{}", col._name()),
            Filter_::Lt(col) => write!(f, "Lt:{}", col._name()),
            Filter_::Lte(col) => write!(f, "Lte:{}", col._name()),
            Filter_::Like(col) => write!(f, "Like:{}", col._name()),
            Filter_::AllBits(col) => write!(f, "AllBits:{}", col._name()),
            Filter_::AnyBits(col) => write!(f, "AnyBits:{}", col._name()),
            Filter_::In(col) => write!(f, "In:{}", col._name()),
            Filter_::NotIn(col) => write!(f, "NotIn:{}", col._name()),
            Filter_::Contains(col, _) => write!(f, "Contains:{}", col._name()),
            Filter_::JsonIn(col, _) => write!(f, "JsonIn:{}", col._name()),
            Filter_::JsonContainsPath(col, _) => write!(f, "JsonContainsPath:{}", col._name()),
            Filter_::JsonEq(col, _) => write!(f, "JsonEq:{}", col._name()),
            Filter_::JsonIsNull(col, _) => write!(f, "JsonIsNull:{}", col._name()),
            Filter_::JsonIsNotNull(col, _) => write!(f, "JsonIsNotNull:{}", col._name()),
            Filter_::JsonLt(col, _) => write!(f, "JsonLt:{}", col._name()),
            Filter_::JsonLte(col, _) => write!(f, "JsonLte:{}", col._name()),
            Filter_::JsonGt(col, _) => write!(f, "JsonGt:{}", col._name()),
            Filter_::JsonGte(col, _) => write!(f, "JsonGte:{}", col._name()),
            Filter_::GeoEquals(col) => write!(f, "GeoEquals:{}", col._name()),
            Filter_::Within(col) => write!(f, "Within:{}", col._name()),
            Filter_::Intersects(col) => write!(f, "Intersects:{}", col._name()),
            Filter_::Crosses(col) => write!(f, "Crosses:{}", col._name()),
            Filter_::DWithin(col) => write!(f, "DWithin:{}", col._name()),
            Filter_::Not(_filter) => write!(f, "Not:<...>"),
            Filter_::And(filters) => write!(f, "And:<{}>", filters.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(",")),
            Filter_::Or(filters) => write!(f, "Or:<{}>", filters.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(",")),
            Filter_::Exists(_col_rel) => write!(f, "Exists:<...>"),
            Filter_::NotExists(_col_rel) => write!(f, "NotExists:<...>"),
            Filter_::EqAny(_col_rel) => write!(f, "EqAny:<...>"),
            Filter_::NotAll(_col_rel) => write!(f, "NotAll:<...>"),
            Filter_::Raw(_sql) => write!(f, "Raw:<...>"),
            Filter_::RawWithParam(_sql, _) => write!(f, "Raw:<...>"),
            Filter_::Boolean(v) => write!(f, "Boolean:{}", v),
        }
    }
}
impl Filter_ {
    pub fn new_and() -> Filter_ {
        Filter_::And(vec![])
    }
    pub fn new_or() -> Filter_ {
        Filter_::Or(vec![])
    }
    pub fn and(mut self, filter: Filter_) -> Filter_ {
        match self {
            Filter_::And(ref mut v) => {
                v.push(filter);
                self
            },
            _ => Filter_::And(vec![self, filter]),
        }
    }
    pub fn or(mut self, filter: Filter_) -> Filter_ {
        match self {
            Filter_::Or(ref mut v) => {
                v.push(filter);
                self
            },
            Filter_::And(ref v) if v.is_empty() => {
                Filter_::Or(vec![filter])
            },
            _ => Filter_::Or(vec![self, filter]),
        }
    }
    pub fn when<F>(self, condition: bool, f: F) -> Self
    where
        F: FnOnce(Self) -> Self,
    {
        if condition {
            f(self)
        } else {
            self
        }
    }
    pub fn if_let_some<T, F>(self, value: &Option<T>, f: F) -> Self
    where
        F: FnOnce(Self, &T) -> Self,
    {
        if let Some(v) = value {
            f(self, v)
        } else {
            self
        }
    }
    pub fn joiner(&self) -> Option<Box<Joiner_>> {
        match self {
            Filter_::And(list) => list.iter().fold(None, |acc, c| Joiner_::merge(acc, c.joiner())),
            Filter_::Or(list) => list.iter().fold(None, |acc, c| Joiner_::merge(acc, c.joiner())),
            Filter_::Exists(c) => c.joiner(),
            Filter_::NotExists(c) => c.joiner(),
            Filter_::EqAny(c) => c.joiner(),
            Filter_::NotAll(c) => c.joiner(),
            _ => None
        }
    }
    pub fn joiner_cache_only(&self) -> Option<Box<Joiner_>> {
        match self {
            Filter_::And(list) => list.iter().fold(None, |acc, c| Joiner_::merge(acc, c.joiner_cache_only())),
            Filter_::Or(list) => list.iter().fold(None, |acc, c| Joiner_::merge(acc, c.joiner_cache_only())),
            Filter_::Exists(c) => c.joiner_cache_only(),
            Filter_::NotExists(c) => c.joiner_cache_only(),
            Filter_::EqAny(c) => c.joiner_cache_only(),
            Filter_::NotAll(c) => c.joiner_cache_only(),
            _ => None
        }
    }
}
impl Check_<dyn @{ pascal_name }@Cache> for Filter_ {
    fn check(&self, obj: &dyn @{ pascal_name }@Cache) -> anyhow::Result<bool> {
        Ok(match self {
            Filter_::IsNull(c) => c.check_null(obj),
            Filter_::IsNotNull(c) => !c.check_null(obj),
            Filter_::Eq(c) => c.check_eq(obj),
            Filter_::NotEq(c) => !c.check_eq(obj),
            Filter_::Gt(c) => c.check_cmp(obj, std::cmp::Ordering::Greater, false).unwrap_or_else(|x| x),
            Filter_::Gte(c) => c.check_cmp(obj, std::cmp::Ordering::Greater, true).unwrap_or_else(|x| x),
            Filter_::Lt(c) => c.check_cmp(obj, std::cmp::Ordering::Less, false).unwrap_or_else(|x| x),
            Filter_::Lte(c) => c.check_cmp(obj, std::cmp::Ordering::Less, true).unwrap_or_else(|x| x),
            Filter_::Like(c) => c.check_like(obj),
            Filter_::In(c) => c.check_in(obj),
            Filter_::NotIn(c) => !c.check_in(obj),
            Filter_::Not(c) => !c.check(obj)?,
            Filter_::And(list) => list.iter().try_fold(true, |acc, c| Ok::<bool, anyhow::Error>(acc && c.check(obj)?))?,
            Filter_::Or(list) => list.iter().try_fold(false, |acc, c| Ok::<bool, anyhow::Error>(acc || c.check(obj)?))?,
            Filter_::Exists(c) => c.check(obj)?,
            Filter_::NotExists(c) => !c.check(obj)?,
            Filter_::EqAny(c) => c.check(obj)?,
            Filter_::NotAll(c) => !c.check(obj)?,
            Filter_::Boolean(c) => *c,
            _ => anyhow::bail!("unsupported check operation!"),
        })
    }
}
impl Check_<dyn @{ pascal_name }@> for Filter_ {
    fn check(&self, obj: &dyn @{ pascal_name }@) -> anyhow::Result<bool> {
        Ok(match self {
            Filter_::IsNull(c) => c.check_null(obj),
            Filter_::IsNotNull(c) => !c.check_null(obj),
            Filter_::Eq(c) => c.check_eq(obj),
            Filter_::NotEq(c) => !c.check_eq(obj),
            Filter_::Gt(c) => c.check_cmp(obj, std::cmp::Ordering::Greater, false).unwrap_or_else(|x| x),
            Filter_::Gte(c) => c.check_cmp(obj, std::cmp::Ordering::Greater, true).unwrap_or_else(|x| x),
            Filter_::Lt(c) => c.check_cmp(obj, std::cmp::Ordering::Less, false).unwrap_or_else(|x| x),
            Filter_::Lte(c) => c.check_cmp(obj, std::cmp::Ordering::Less, true).unwrap_or_else(|x| x),
            Filter_::Like(c) => c.check_like(obj),
            Filter_::In(c) => c.check_in(obj),
            Filter_::NotIn(c) => !c.check_in(obj),
            Filter_::Not(c) => !c.check(obj)?,
            Filter_::And(list) => list.iter().try_fold(true, |acc, c| Ok::<bool, anyhow::Error>(acc && c.check(obj)?))?,
            Filter_::Or(list) => list.iter().try_fold(false, |acc, c| Ok::<bool, anyhow::Error>(acc || c.check(obj)?))?,
            Filter_::Exists(c) => c.check(obj)?,
            Filter_::NotExists(c) => !c.check(obj)?,
            Filter_::EqAny(c) => c.check(obj)?,
            Filter_::NotAll(c) => !c.check(obj)?,
            Filter_::Boolean(c) => *c,
            _ => anyhow::bail!("unsupported check operation!"),
        })
    }
}

@% let filter_macro_name = "filter_{}_{}_{}"|format(db|snake, group_name|snake, model_name) -%@
@% let model_path = "$crate::repositories::{}::_base::_{}"|format(group_name|snake|ident, mod_name) -%@
#[macro_export]
macro_rules! @{ filter_macro_name }@_null {
@%- for (col_name, column_def) in def.nullable() %@
    (@{ col_name }@) => (@{ model_path }@::Col_::@{ col_name|ident }@);
@%- endfor %@
    () => (); // For empty case
}
pub use @{ filter_macro_name }@_null as filter_null;

#[macro_export]
macro_rules! @{ filter_macro_name }@_text {
@%- for (col_name, column_def) in def.text() %@
    (@{ col_name }@) => (@{ model_path }@::Col_::@{ col_name|ident }@);
@%- endfor %@
    () => (); // For empty case
}
pub use @{ filter_macro_name }@_text as filter_text;

#[macro_export]
macro_rules! @{ filter_macro_name }@_one {
@%- for (col_name, column_def) in def.all_fields_except_json() %@
    (@{ col_name }@ $e:expr) => (@{ model_path }@::ColOne_::@{ col_name|ident }@($e.clone().try_into()?));
@%- endfor %@
}
pub use @{ filter_macro_name }@_one as filter_one;

#[macro_export]
macro_rules! @{ filter_macro_name }@_many {
@%- for (col_name, column_def) in def.all_fields_except_json() %@
    (@{ col_name }@ [$($e:expr),*]) => (@{ model_path }@::ColMany_::@{ col_name|ident }@(vec![ $( $e.clone().try_into()? ),* ]));
    (@{ col_name }@ $e:expr) => (@{ model_path }@::ColMany_::@{ col_name|ident }@($e.into_iter().map(|v| v.clone().try_into()).collect::<Result<Vec<_>, _>>()?));
@%- endfor %@
}
pub use @{ filter_macro_name }@_many as filter_many;

#[macro_export]
macro_rules! @{ filter_macro_name }@_json {
@%- for (col_name, column_def) in def.all_fields_only_json() %@
    (@{ col_name }@ $e:expr) => (@{ model_path }@::ColJson_::@{ col_name|ident }@($e.clone().try_into()?));
@%- endfor %@
    () => ();
}
pub use @{ filter_macro_name }@_json as filter_json;

#[macro_export]
macro_rules! @{ filter_macro_name }@_json_array {
@%- for (col_name, column_def) in def.all_fields_only_json() %@
    (@{ col_name }@ $e:expr) => (@{ model_path }@::ColJsonArray_::@{ col_name|ident }@($e.iter().map(|v| v.clone().try_into()).collect::<Result<Vec<_>, _>>()?));
@%- endfor %@
    () => ();
}
pub use @{ filter_macro_name }@_json_array as filter_json_array;

#[macro_export]
macro_rules! @{ filter_macro_name }@_geo {
@%- for (col_name, column_def) in def.all_fields_only_geo() %@
    (@{ col_name }@ $e:expr) => (@{ model_path }@::ColGeo_::@{ col_name|ident }@($e.clone().try_into()?, @{ column_def.srid() }@));
@%- endfor %@
    () => ();
}
pub use @{ filter_macro_name }@_geo as filter_geo;

#[macro_export]
macro_rules! @{ filter_macro_name }@_geo_distance {
@%- for (col_name, column_def) in def.all_fields_only_geo() %@
    (@{ col_name }@ $e:expr, $d:expr) => (@{ model_path }@::ColGeoDistance_::@{ col_name|ident }@($e.clone().try_into()?, $d, @{ column_def.srid() }@));
@%- endfor %@
    () => ();
}
pub use @{ filter_macro_name }@_geo_distance as filter_geo_distance;

#[macro_export]
macro_rules! @{ filter_macro_name }@_rel {
@%- for (model_def, col_name, rel_def) in def.relations_one_and_belonging(false) %@
    (@{ col_name }@) => (@{ model_path }@::ColRel_::@{ col_name|ident }@(None));
    (@{ col_name }@ $t:tt) => (@{ model_path }@::ColRel_::@{ col_name|ident }@(Some(Box::new($crate::models::@{ db|snake|ident }@::@{ rel_def.get_group_name()|snake|ident }@::_base::_@{ rel_def.get_mod_name() }@::filter!($t)))));
@%- endfor %@
@%- for (model_def, col_name, rel_def) in def.relations_many(false) %@
    (@{ col_name }@) => (@{ model_path }@::ColRel_::@{ col_name|ident }@(None));
    (@{ col_name }@ $t:tt) => (@{ model_path }@::ColRel_::@{ col_name|ident }@(Some(Box::new($crate::models::@{ db|snake|ident }@::@{ rel_def.get_group_name()|snake|ident }@::_base::_@{ rel_def.get_mod_name() }@::filter!($t)))));
@%- endfor %@
@%- for (model_def, col_name, rel_def) in def.relations_belonging_outer_db(false) %@
    (@{ col_name }@) => (@{ model_path }@::ColRel_::@{ col_name|ident }@(None));
    (@{ col_name }@ $t:tt) => (@{ model_path }@::ColRel_::@{ col_name|ident }@(Some(Box::new(@{ model_path }@::_@{ rel_def.db()|snake }@_model_::@{ rel_def.get_group_name()|snake|ident }@::_base::_@{ rel_def.get_mod_name() }@::filter!($t)))));
@%- endfor %@
    () => ();
}
pub use @{ filter_macro_name }@_rel as filter_rel;

#[macro_export]
macro_rules! @{ filter_macro_name }@ {
    () => (@{ model_path }@::Filter_::new_and());
@%- for (index_name, index) in def.multi_index(false) %@
    ((@{ index.join_fields(def, "{name}", ", ") }@) = (@{ index.join_fields(def, "$e{index}:expr", ", ") }@)) => (@{ model_path }@::Filter_::Eq(@{ model_path }@::ColOne_::@{ index.join_fields(def, "{name}", "_") }@((@{ index.join_fields(def, "$e{index}.clone()", ", ") }@).try_into()?)));
    ((@{ index.join_fields(def, "{name}", ", ") }@) > (@{ index.join_fields(def, "$e{index}:expr", ", ") }@)) => (@{ model_path }@::Filter_::Gt(@{ model_path }@::ColOne_::@{ index.join_fields(def, "{name}", "_") }@((@{ index.join_fields(def, "$e{index}.clone()", ", ") }@).try_into()?)));
    ((@{ index.join_fields(def, "{name}", ", ") }@) >= (@{ index.join_fields(def, "$e{index}:expr", ", ") }@)) => (@{ model_path }@::Filter_::Gte(@{ model_path }@::ColOne_::@{ index.join_fields(def, "{name}", "_") }@((@{ index.join_fields(def, "$e{index}.clone()", ", ") }@).try_into()?)));
    ((@{ index.join_fields(def, "{name}", ", ") }@) < (@{ index.join_fields(def, "$e{index}:expr", ", ") }@)) => (@{ model_path }@::Filter_::Lt(@{ model_path }@::ColOne_::@{ index.join_fields(def, "{name}", "_") }@((@{ index.join_fields(def, "$e{index}.clone()", ", ") }@).try_into()?)));
    ((@{ index.join_fields(def, "{name}", ", ") }@) <= (@{ index.join_fields(def, "$e{index}:expr", ", ") }@)) => (@{ model_path }@::Filter_::Lte(@{ model_path }@::ColOne_::@{ index.join_fields(def, "{name}", "_") }@((@{ index.join_fields(def, "$e{index}.clone()", ", ") }@).try_into()?)));
    ((@{ index.join_fields(def, "{name}", ", ") }@) = $e:expr) => (@{ model_path }@::Filter_::Eq(@{ model_path }@::ColOne_::@{ index.join_fields(def, "{name}", "_") }@((@{ index.join_fields(def, "$e.{index}.clone()", ", ") }@).try_into()?)));
    ((@{ index.join_fields(def, "{name}", ", ") }@) IN $e:expr) => (@{ model_path }@::Filter_::In(@{ model_path }@::ColMany_::@{ index.join_fields(def, "{name}", "_") }@($e.into_iter().map(|v| (@{ index.join_fields(def, "v.{index}.clone()", ", ") }@).try_into()).collect::<Result<_, _>>()?)));
    ((@{ index.join_fields(def, "{name}", ", ") }@) NOT IN $e:expr) => (@{ model_path }@::Filter_::NotIn(@{ model_path }@::ColMany_::@{ index.join_fields(def, "{name}", "_") }@($e.into_iter().map(|v| (@{ index.join_fields(def, "v.{index}.clone()", ", ") }@).try_into()).collect::<Result<_, _>>()?)));
    ((@{ index.join_fields(def, "{name}", ", ") }@) > $e:expr) => (@{ model_path }@::Filter_::Gt(@{ model_path }@::ColOne_::@{ index.join_fields(def, "{name}", "_") }@((@{ index.join_fields(def, "$e.{index}.clone()", ", ") }@).try_into()?)));
    ((@{ index.join_fields(def, "{name}", ", ") }@) >= $e:expr) => (@{ model_path }@::Filter_::Gte(@{ model_path }@::ColOne_::@{ index.join_fields(def, "{name}", "_") }@((@{ index.join_fields(def, "$e.{index}.clone()", ", ") }@).try_into()?)));
    ((@{ index.join_fields(def, "{name}", ", ") }@) < $e:expr) => (@{ model_path }@::Filter_::Lt(@{ model_path }@::ColOne_::@{ index.join_fields(def, "{name}", "_") }@((@{ index.join_fields(def, "$e.{index}.clone()", ", ") }@).try_into()?)));
    ((@{ index.join_fields(def, "{name}", ", ") }@) <= $e:expr) => (@{ model_path }@::Filter_::Lte(@{ model_path }@::ColOne_::@{ index.join_fields(def, "{name}", "_") }@((@{ index.join_fields(def, "$e.{index}.clone()", ", ") }@).try_into()?)));
@%- endfor %@
    (($($t:tt)*)) => (@{ model_path }@::filter!($($t)*));
    (NOT $t:tt) => (@{ model_path }@::Filter_::Not(Box::new(@{ model_path }@::filter!($t))));
    (WITH_TRASHED) => (@{ model_path }@::Filter_::WithTrashed);
    (ONLY_TRASHED) => (@{ model_path }@::Filter_::OnlyTrashed);
    (BOOLEAN $e:expr) => (@{ model_path }@::Filter_::Boolean($e));
    (RAW $e:expr) => (@{ model_path }@::Filter_::Raw($e.to_string()));
    (RAW $e:expr , [$($p:expr),*] ) => (@{ model_path }@::Filter_::RawWithParam($e.to_string(), vec![ $( $p.to_string() ),* ]));
    (RAW $e:expr , $p:expr ) => (@{ model_path }@::Filter_::RawWithParam($e.to_string(), $p.iter().map(|v| v.to_string()).collect()));
    (MATCH ( $($i:ident),+ ) AGAINST ($e:expr) IN BOOLEAN MODE) => (@{ model_path }@::Filter_::MatchBoolean(vec![ $( @{ model_path }@::filter_text!($i) ),* ], $e.to_string()));
    (MATCH ( $($i:ident),+ ) AGAINST ($e:expr) WITH QUERY EXPANSION) => (@{ model_path }@::Filter_::MatchExpansion(vec![ $( @{ model_path }@::filter_text!($i) ),* ], $e.to_string()));
    (MATCH ( $($i:ident),+ ) AGAINST ($e:expr)) => (@{ model_path }@::Filter_::Match(vec![ $( @{ model_path }@::filter_text!($i) ),* ], $e.to_string()));
    ($i:ident EXISTS) => (@{ model_path }@::Filter_::Exists(@{ model_path }@::filter_rel!($i)));
    ($i:ident EXISTS $t:tt) => (@{ model_path }@::Filter_::Exists(@{ model_path }@::filter_rel!($i $t)));
    ($i:ident NOT EXISTS) => (@{ model_path }@::Filter_::NotExists(@{ model_path }@::filter_rel!($i)));
    ($i:ident NOT EXISTS $t:tt) => (@{ model_path }@::Filter_::NotExists(@{ model_path }@::filter_rel!($i $t)));
    ($i:ident = ANY $t:tt) => (@{ model_path }@::Filter_::EqAny(@{ model_path }@::filter_rel!($i $t)));
    ($i:ident NOT ALL $t:tt) => (@{ model_path }@::Filter_::NotAll(@{ model_path }@::filter_rel!($i $t)));
    ($i:ident IS NULL) => (@{ model_path }@::Filter_::IsNull(@{ model_path }@::filter_null!($i)));
    ($i:ident IS NOT NULL) => (@{ model_path }@::Filter_::IsNotNull(@{ model_path }@::filter_null!($i)));
    ($i:ident = $e:expr) => (@{ model_path }@::Filter_::Eq(@{ model_path }@::filter_one!($i $e)));
    ($i:ident != $e:expr) => (@{ model_path }@::Filter_::NotEq(@{ model_path }@::filter_one!($i $e)));
    ($i:ident > $e:expr) => (@{ model_path }@::Filter_::Gt(@{ model_path }@::filter_one!($i $e)));
    ($i:ident >= $e:expr) => (@{ model_path }@::Filter_::Gte(@{ model_path }@::filter_one!($i $e)));
    ($i:ident < $e:expr) => (@{ model_path }@::Filter_::Lt(@{ model_path }@::filter_one!($i $e)));
    ($i:ident <= $e:expr) => (@{ model_path }@::Filter_::Lte(@{ model_path }@::filter_one!($i $e)));
    ($i:ident LIKE $e:expr) => (@{ model_path }@::Filter_::Like(@{ model_path }@::filter_one!($i $e)));
    ($i:ident ALL_BITS $e:expr) => (@{ model_path }@::Filter_::AllBits(@{ model_path }@::filter_many!($i [$e, $e])));
    ($i:ident ANY_BITS $e:expr) => (@{ model_path }@::Filter_::AnyBits(@{ model_path }@::filter_one!($i $e)));
    ($i:ident BETWEEN ($e1:expr, $e2:expr)) => (@{ model_path }@::filter!(($i >= $e1) AND ($i <= $e2)));
    ($i:ident RIGHT_OPEN ($e1:expr, $e2:expr)) => (@{ model_path }@::filter!(($i >= $e1) AND ($i < $e2)));
    ($i:ident IN ( $($e:expr),* )) => (@{ model_path }@::Filter_::In(@{ model_path }@::filter_many!($i [ $( $e ),* ])));
    ($i:ident IN $e:expr) => (@{ model_path }@::Filter_::In(@{ model_path }@::filter_many!($i $e)));
    ($i:ident NOT IN ( $($e:expr),* )) => (@{ model_path }@::Filter_::NotIn(@{ model_path }@::filter_many!($i [ $( $e ),* ])));
    ($i:ident NOT IN $e:expr) => (@{ model_path }@::Filter_::NotIn(@{ model_path }@::filter_many!($i $e)));
    ($i:ident CONTAINS [ $($e:expr),* ]) => (@{ model_path }@::Filter_::Contains(@{ model_path }@::filter_json_array!($i vec![ $( $e ),* ]), None));
    ($i:ident CONTAINS $e:expr) => (@{ model_path }@::Filter_::Contains(@{ model_path }@::filter_json_array!($i $e), None));
    ($i:ident -> ($p:expr) CONTAINS [ $($e:expr),* ]) => (@{ model_path }@::Filter_::Contains(@{ model_path }@::filter_json_array!($i vec![ $( $e ),* ]), Some($p.to_string())));
    ($i:ident -> ($p:expr) CONTAINS $e:expr) => (@{ model_path }@::Filter_::Contains(@{ model_path }@::filter_json_array!($i $e), Some($p.to_string())));
    ($i:ident -> ($p:expr) IN [ $($e:expr),* ]) => (@{ model_path }@::Filter_::JsonIn(@{ model_path }@::filter_json_array!($i vec![ $( $e ),* ]), $p.to_string()));
    ($i:ident -> ($p:expr) IN $e:expr) => (@{ model_path }@::Filter_::JsonIn(@{ model_path }@::filter_json_array!($i $e), $p.to_string()));
    ($i:ident JSON_CONTAINS_PATH ($p:expr)) => (@{ model_path }@::Filter_::JsonContainsPath(@{ model_path }@::filter_json!($i 0), $p.to_string()));
    ($i:ident -> ($p:expr) = $e:expr) => (@{ model_path }@::Filter_::JsonEq(@{ model_path }@::filter_json!($i $e), $p.to_string()));
    ($i:ident -> ($p:expr) IS NULL) => (@{ model_path }@::Filter_::JsonIsNull(@{ model_path }@::filter_json!($i 0), $p.to_string()));
    ($i:ident -> ($p:expr) IS NOT NULL) => (@{ model_path }@::Filter_::JsonIsNotNull(@{ model_path }@::filter_json!($i 0), $p.to_string()));
    ($i:ident -> ($p:expr) < $e:expr) => (@{ model_path }@::Filter_::JsonLt(@{ model_path }@::filter_json!($i $e), $p.to_string()));
    ($i:ident -> ($p:expr) <= $e:expr) => (@{ model_path }@::Filter_::JsonLte(@{ model_path }@::filter_json!($i $e), $p.to_string()));
    ($i:ident -> ($p:expr) > $e:expr) => (@{ model_path }@::Filter_::JsonGt(@{ model_path }@::filter_json!($i $e), $p.to_string()));
    ($i:ident -> ($p:expr) >= $e:expr) => (@{ model_path }@::Filter_::JsonGte(@{ model_path }@::filter_json!($i $e), $p.to_string()));
    ($i:ident GEO_EQUALS $e:expr) => (@{ model_path }@::Filter_::GeoEquals(@{ model_path }@::filter_geo!($i $e)));
    ($i:ident WITHIN $e:expr) => (@{ model_path }@::Filter_::Within(@{ model_path }@::filter_geo!($i $e)));
    ($i:ident INTERSECTS $e:expr) => (@{ model_path }@::Filter_::Intersects(@{ model_path }@::filter_geo!($i $e)));
    ($i:ident CROSSES $e:expr) => (@{ model_path }@::Filter_::Crosses(@{ model_path }@::filter_geo!($i $e)));
    ($i:ident D_WITHIN $e:expr, $d:expr) => (@{ model_path }@::Filter_::DWithin(@{ model_path }@::filter_geo_distance!($i $e, $d)));
    ($t1:tt AND $($t2:tt)AND+) => (@{ model_path }@::Filter_::And(vec![ @{ model_path }@::filter!($t1), $( @{ model_path }@::filter!($t2) ),* ]));
    ($t1:tt OR $($t2:tt)OR+) => (@{ model_path }@::Filter_::Or(vec![ @{ model_path }@::filter!($t1), $( @{ model_path }@::filter!($t2) ),* ]));
}
pub use @{ filter_macro_name }@ as filter;

#[derive(Clone, Debug)]
pub enum Order_ {
    Asc(Col_),
    Desc(Col_),
    IsNullAsc(Col_),
    IsNullDesc(Col_),
}

@% let order_macro_name = "order_{}_{}_{}"|format(db|snake, group_name, model_name) -%@
#[macro_export]
macro_rules! @{ order_macro_name }@_col {
@%- for (col_name, column_def) in def.all_fields() %@
    (@{ col_name }@) => (@{ model_path }@::Col_::@{ col_name|ident }@);
@%- endfor %@
}
pub use @{ order_macro_name }@_col as order_by_col;

#[macro_export]
macro_rules! @{ order_macro_name }@_one {
    ($i:ident) => (@{ model_path }@::Order_::Asc(@{ model_path }@::order_by_col!($i)));
    ($i:ident ASC) => (@{ model_path }@::Order_::Asc(@{ model_path }@::order_by_col!($i)));
    ($i:ident DESC) => (@{ model_path }@::Order_::Desc(@{ model_path }@::order_by_col!($i)));
    ($i:ident IS NULL ASC) => (@{ model_path }@::Order_::IsNullAsc(@{ model_path }@::order_by_col!($i)));
    ($i:ident IS NULL DESC) => (@{ model_path }@::Order_::IsNullDesc(@{ model_path }@::order_by_col!($i)));
}
pub use @{ order_macro_name }@_one as order_by_one;

#[macro_export]
macro_rules! @{ order_macro_name }@ {
    ($($($i:ident)+),+) => (vec![$( @{ model_path }@::order_by_one!($($i)+)),+]);
}
pub use @{ order_macro_name }@ as order;


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
            map.insert(@{- def.primaries()|fmt_join_with_paren("v.{var}{clone}", ", ") }@, v.clone());
        }
    }
}
#[cfg(any(feature = "mock", test))]
#[async_trait]
impl _@{ pascal_name }@Repository for Emu@{ pascal_name }@Repository {
    @%- if !def.disable_update() %@
    fn find(&self, id: @{ def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") }@) -> Box<dyn _@{ pascal_name }@RepositoryFindBuilder> {
        struct V(Option<@{ pascal_name }@Entity>, Option<Filter_>@% if def.is_soft_delete() %@, bool@% endif %@);
        #[async_trait]
        impl _@{ pascal_name }@RepositoryFindBuilder for V {
            async fn query_for_update(self: Box<Self>) -> anyhow::Result<Box<dyn _Updater>> {
                use anyhow::Context;
                let filter = self.1;
                self.0.filter(|v| filter.map(|f| f.check(v as &dyn @{ pascal_name }@)).unwrap_or(Ok(true)).unwrap())@{- def.soft_delete_tpl2("",".filter(|v| self.2 || v.deleted_at.is_none())",".filter(|v| self.2 || !v.deleted)",".filter(|v| self.2 || v.deleted == 0)")}@
                    .map(|v| Box::new(v) as Box<dyn _Updater>)
                    .with_context(|| "Not Found")
            }
            async fn query(self: Box<Self>) -> anyhow::Result<Option<Box<dyn @{ pascal_name }@>>> {
                let filter = self.1;
                Ok(self.0.filter(|v| filter.map(|f| f.check(v as &dyn @{ pascal_name }@)).unwrap_or(Ok(true)).unwrap())@{- def.soft_delete_tpl2("",".filter(|v| self.2 || v.deleted_at.is_none())",".filter(|v| self.2 || !v.deleted)",".filter(|v| self.2 || v.deleted == 0)")}@.map(|v| Box::new(v) as Box<dyn @{ pascal_name }@>))
            }
            fn filter(mut self: Box<Self>, filter: Filter_) -> Box<dyn _RepositoryFindBuilder> { self.1 = Some(filter); self }
            @%- if def.is_soft_delete() %@
            fn with_trashed(mut self: Box<Self>, mode: bool) -> Box<dyn _RepositoryFindBuilder> { self.2 = mode; self }
            @%- endif %@
            fn join(self: Box<Self>, _join: Option<Box<Joiner_>>) -> Box<dyn _RepositoryFindBuilder> { self }
        }
        let map = self._data.lock().unwrap();
        Box::new(V(map.get(&id).cloned(), None@% if def.is_soft_delete() %@, false@% endif %@))
    }
    @%- endif %@
    fn convert_factory(&self, _factory: @{ pascal_name }@Factory) -> Box<dyn _Updater> {
        #[allow(unused_imports)]
        use base_domain::models::ToRawValue as _;
        Box::new(@{ pascal_name }@Entity {
@{- def.non_auto_primary_for_factory()|fmt_join("
            {var}: _factory.{var}{convert_domain_factory},", "") }@
            ..Default::default()
        })
    }
    #[allow(unused_mut)]
    async fn save(&self, obj: Box<dyn _Updater>) -> anyhow::Result<Option<Box<dyn @{ pascal_name }@>>> {
        let mut obj = if let Ok(obj) = (obj as Box<dyn std::any::Any>).downcast::<@{ pascal_name }@Entity>() {
            obj
        } else {
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
            map.insert(@{- def.primaries()|fmt_join_with_paren("obj.{var}{clone}", ", ") }@, *obj.clone());
            Ok(Some(obj as Box<dyn @{ pascal_name }@>))
        }
    }
    @%- if !def.disable_update() %@
    #[allow(unused_mut)]
    async fn import(&self, list: Vec<Box<dyn _Updater>>, _option: Option<base_domain::models::ImportOption>) -> anyhow::Result<()> {
        for obj in list {
            let mut obj = if let Ok(obj) = (obj as Box<dyn std::any::Any>).downcast::<@{ pascal_name }@Entity>() {
                obj
            } else {
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
                map.insert(@{- def.primaries()|fmt_join_with_paren("obj.{var}{clone}", ", ") }@, *obj.clone());
            }
        }
        Ok(())
    }
    @%- endif %@
    @%- if def.use_insert_delayed() %@
    #[allow(unused_mut)]
    async fn insert_delayed(&self, obj: Box<dyn _Updater>) -> anyhow::Result<()> {
        let mut obj = if let Ok(obj) = (obj as Box<dyn std::any::Any>).downcast::<@{ pascal_name }@Entity>() {
            obj
        } else {
            panic!("Only @{ pascal_name }@Entity is accepted.");
        };
        let mut map = self._data.lock().unwrap();
        @%- for (name, column_def) in def.auto_inc_or_seq() %@
        if obj.@{ name|ident }@ == 0.into() {
            obj.@{ name|ident }@ = (map.iter().map(|(_k, v)| @{ column_def.get_inner_type(true, false) }@::from(v.@{ name|ident }@)).max().unwrap_or_default() + 1).into();
        }
        @%- endfor %@
        @%- for (name, column_def) in def.auto_uuid() %@
        if obj.@{ name|ident }@.is_empty() {
            obj.@{ name|ident }@ = uuid::Uuid::new_v4().to_string().into();
        }
        @%- endfor %@
        map.insert(@{- def.primaries()|fmt_join_with_paren("obj.{var}{clone}", ", ") }@, *obj.clone());
        Ok(())
    }
    @%- endif %@
    @%- if !def.disable_delete() %@
    async fn delete(&self, obj: Box<dyn _Updater>) -> anyhow::Result<()> {
        let mut map = self._data.lock().unwrap();
        map.remove(&@{- def.primaries()|fmt_join_with_paren("obj.{var}(){clone}", ", ") }@);
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
                    .map(|v| Box::new(v) as Box<dyn @{ pascal_name }@@% if def.use_cache() %@Cache@% endif %@>).collect();
                Ok(list.len() as i64)
            }
            fn selector_filter(mut self: Box<Self>, filter: @{ pascal_name }@Query@{ selector|pascal }@Filter) -> Box<dyn _Repository@{ selector|pascal }@Builder> { self.selector_filter = Some(filter); self }
            fn extra_filter(mut self: Box<Self>, filter: Filter_) -> Box<dyn _Repository@{ selector|pascal }@Builder> { self.extra_filter = Some(filter); self }
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
fn _filter@{ filter_map.suffix }@(v: &impl super::super::super::@{ filter_map.model_group()|snake|ident }@::@{ filter_map.model_name()|snake|ident }@::@{ filter_map.model_name()|pascal }@, filter: &@{ pascal_name }@Query@{ selector|pascal }@@{ filter_map.pascal_name }@Filter) -> bool {
    use super::super::super::@{ filter_map.model_group()|snake|ident }@::@{ filter_map.model_name()|snake|ident }@::*;
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
    @%- if def.use_all_rows_cache() && !def.use_filtered_row_cache() %@
    async fn all(&self) -> anyhow::Result<Box<dyn base_domain::models::EntityIterator<dyn @{ pascal_name }@Cache>>> {
        struct V(std::collections::BTreeMap<@{- def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") }@, @{ pascal_name }@Entity>);
        impl base_domain::models::EntityIterator<dyn @{ pascal_name }@Cache> for V {
            fn iter(&self) -> Box<dyn Iterator<Item = &(dyn @{ pascal_name }@Cache + 'static)> + '_> {
                Box::new(self.0.iter().map(|(_, v)| v as &dyn @{ pascal_name }@Cache))
            }
            fn into_iter(self) -> Box<dyn Iterator<Item = Box<dyn @{ pascal_name }@Cache>>> {
                Box::new(self.0.into_iter().map(|(_, v)| Box::new(v) as Box<dyn @{ pascal_name }@Cache>))
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
            async fn query(self: Box<Self>) -> anyhow::Result<Vec<Box<dyn @{ pascal_name }@@% if def.use_cache() %@Cache@% endif %@>>> {
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
                    .map(|v| Box::new(v) as Box<dyn @{ pascal_name }@@% if def.use_cache() %@Cache@% endif %@>).collect();
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
            async fn stream(self: Box<Self>, _single_transaction: bool) -> anyhow::Result<std::pin::Pin<Box<dyn futures::Stream<Item=anyhow::Result<Box<dyn @{ pascal_name }@@% if def.use_cache() %@Cache@% endif %@>>> + Send>>> {
                use futures::StreamExt;
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
                    .map(|v| Box::new(v) as Box<dyn @{ pascal_name }@@% if def.use_cache() %@Cache@% endif %@>).collect();
                Ok(list.len() as i64)
            }
            fn selector_filter(mut self: Box<Self>, filter: @{ pascal_name }@Query@{ selector|pascal }@Filter) -> Box<dyn _Query@{ selector|pascal }@Builder> { self.selector_filter = Some(filter); self }
            fn extra_filter(mut self: Box<Self>, filter: Filter_) -> Box<dyn _Query@{ selector|pascal }@Builder> { self.extra_filter = Some(filter); self }
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
        struct V(Option<@{ pascal_name }@Entity>, Option<Filter_>@% if def.is_soft_delete() %@, bool@% endif %@);
        #[async_trait]
        impl _@{ pascal_name }@QueryFindBuilder for V {
            async fn query(self: Box<Self>) -> anyhow::Result<Option<Box<dyn @{ pascal_name }@@% if def.use_cache() %@Cache@% endif %@>>> {
                let filter = self.1;
                Ok(self.0.filter(|v| filter.map(|f| f.check(v as &dyn @{ pascal_name }@)).unwrap_or(Ok(true)).unwrap())@{- def.soft_delete_tpl2("",".filter(|v| self.2 || v.deleted_at.is_none())",".filter(|v| self.2 || !v.deleted)",".filter(|v| self.2 || v.deleted == 0)")}@.map(|v| Box::new(v) as Box<dyn @{ pascal_name }@@% if def.use_cache() %@Cache@% endif %@>))
            }
            fn filter(mut self: Box<Self>, filter: Filter_) -> Box<dyn _QueryFindBuilder> { self.1 = Some(filter); self }
            @%- if def.is_soft_delete() %@
            fn with_trashed(mut self: Box<Self>, mode: bool) -> Box<dyn _QueryFindBuilder> { self.2 = mode; self }
            @%- endif %@
            fn join(self: Box<Self>, _join: Option<Box<Joiner_>>) -> Box<dyn _QueryFindBuilder> { self }
        }
        let map = self._data.lock().unwrap();
        Box::new(V(map.get(&id).cloned(), None@% if def.is_soft_delete() %@, false@% endif %@))
    }
    @%- else %@
    fn find(&self, id: @{ def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") }@) -> Box<dyn _@{ pascal_name }@QueryFindBuilder> {
        struct V(Option<@{ pascal_name }@Entity>, Option<Filter_>@% if def.is_soft_delete() %@, bool@% endif %@);
        #[async_trait]
        impl _@{ pascal_name }@QueryFindBuilder for V {
            async fn query(self: Box<Self>) -> anyhow::Result<Option<Box<dyn @{ pascal_name }@>>> {
                let filter = self.1;
                Ok(self.0.filter(|v| filter.map(|f| f.check(v as &dyn @{ pascal_name }@)).unwrap_or(Ok(true)).unwrap())@{- def.soft_delete_tpl2("",".filter(|v| self.2 || v.deleted_at.is_none())",".filter(|v| self.2 || !v.deleted)",".filter(|v| self.2 || v.deleted == 0)")}@.map(|v| Box::new(v) as Box<dyn @{ pascal_name }@>))
            }
            fn filter(mut self: Box<Self>, filter: Filter_) -> Box<dyn _QueryFindBuilder> { self.1 = Some(filter); self }
            @%- if def.is_soft_delete() %@
            fn with_trashed(mut self: Box<Self>, mode: bool) -> Box<dyn _QueryFindBuilder> { self.2 = mode; self }
            @%- endif %@
            fn join(self: Box<Self>, _join: Option<Box<Joiner_>>) -> Box<dyn _QueryFindBuilder> { self }
        }
        let map = self._data.lock().unwrap();
        Box::new(V(map.get(&id).cloned(), None@% if def.is_soft_delete() %@, false@% endif %@))
    }
    @%- endif %@
}
@{-"\n"}@