// This code is auto-generated and will always be overwritten.
#[allow(unused_imports)]
use crate::misc::Updater as _;
use crate::models::@{ group_name|to_var_name }@::@{ mod_name|to_var_name }@::*;
#[allow(unused_imports)]
use anyhow::Context as _;
use async_trait::async_trait;
#[allow(unused_imports)]
use domain::models::@{ db|snake|to_var_name }@::@{ group_name|to_var_name }@::_base::_@{ mod_name }@::{self, *};
use domain::models::@{ db|snake|to_var_name }@::@{ group_name|to_var_name }@::@{ mod_name|to_var_name }@::*;
#[allow(unused_imports)]
use domain::models::{self, ToGeoPoint as _, ToPoint as _};
#[allow(unused_imports)]
use domain::value_objects;
#[allow(unused_imports)]
use senax_common::types::geo_point::ToGeoPoint as _;
#[allow(unused_imports)]
use senax_common::types::point::ToPoint as _;
#[allow(unused_imports)]
use std::ops::{Deref as _, DerefMut as _};
#[allow(unused_imports)]
use domain::models::@{ db|snake|to_var_name }@ as _model_;

type _Getter_ = dyn crate::models::@{ group_name|to_var_name }@::_base::_@{ mod_name }@::_@{ pascal_name }@Getter;
@%- if !config.force_disable_cache %@
type _Cache_ = _@{ pascal_name }@Cache;
@%- endif %@
type _Updater_ = _@{ pascal_name }@Updater;

@% for (name, column_def) in def.id() -%@
impl From<@{ pascal_name }@Id> for _@{ pascal_name }@Id {
    fn from(id: @{ pascal_name }@Id) -> Self {
        Self(id.inner())
    }
}
impl From<&@{ pascal_name }@Id> for _@{ pascal_name }@Id {
    fn from(id: &@{ pascal_name }@Id) -> Self {
        Self(id.inner())
    }
}
impl From<_@{ pascal_name }@Id> for @{ pascal_name }@Id {
    fn from(id: _@{ pascal_name }@Id) -> Self {
        id.inner().into()
    }
}
impl From<&_@{ pascal_name }@Id> for @{ pascal_name }@Id {
    fn from(id: &_@{ pascal_name }@Id) -> Self {
        id.inner().into()
    }
}
@%- endfor %@
@%- for parent in def.parents() %@

impl domain::models::@{ db|snake|to_var_name }@::@{ parent.group_name|to_var_name }@::@{ parent.name|to_var_name }@::@{ parent.name|pascal }@Common for _@{ pascal_name }@ {
@{- parent.primaries()|fmt_join("
    fn _{raw_var}(&self) -> {inner} {
        _Getter_::_{raw_var}(self){clone}{convert_impl_domain_inner}
    }", "") }@
@{- parent.only_version()|fmt_join("
    fn {var}(&self) -> {domain_outer} {
        _Getter_::_{raw_var}(self)
    }", "") }@
@{- parent.cache_cols_wo_primaries_and_read_only()|fmt_join("
    fn {var}(&self) -> {domain_outer} {
        _Getter_::_{raw_var}(self){convert_impl_domain_outer}
    }", "") }@
}
@%- if !config.force_disable_cache %@
impl domain::models::@{ db|snake|to_var_name }@::@{ parent.group_name|to_var_name }@::@{ parent.name|to_var_name }@::@{ parent.name|pascal }@Common for _@{ pascal_name }@Cache {
@{- parent.primaries()|fmt_join("
    fn _{raw_var}(&self) -> {inner} {
        _Cache_::_{raw_var}(self){clone}{convert_impl_domain_inner}
    }", "") }@
@{- parent.only_version()|fmt_join("
    fn {var}(&self) -> {domain_outer} {
        _Cache_::_{raw_var}(self)
    }", "") }@
@{- parent.cache_cols_wo_primaries_and_read_only()|fmt_join("
    fn {var}(&self) -> {domain_outer} {
        _Cache_::_{raw_var}(self){convert_impl_domain_outer}
    }", "") }@
}
@%- endif %@
impl domain::models::@{ db|snake|to_var_name }@::@{ parent.group_name|to_var_name }@::@{ parent.name|to_var_name }@::@{ parent.name|pascal }@Common for _@{ pascal_name }@Updater {
@{- parent.primaries()|fmt_join("
    fn _{raw_var}(&self) -> {inner} {
        self._data.{var}{clone}
    }", "") }@
@{- parent.only_version()|fmt_join("
    fn {var}(&self) -> {domain_outer} {
        self._data.{var}
    }", "") }@
@{- parent.cache_cols_wo_primaries_and_read_only()|fmt_join("
    fn {var}(&self) -> {domain_outer} {
        {convert_impl_domain_outer_for_updater}
    }", "") }@
}
@%- if !config.force_disable_cache %@
impl domain::models::@{ db|snake|to_var_name }@::@{ parent.group_name|to_var_name }@::@{ parent.name|to_var_name }@::@{ parent.name|pascal }@Cache for _@{ pascal_name }@Cache {
@{- parent.relations_one_cache(true)|fmt_rel_join("
    fn {rel_name}(&self) -> Option<Box<dyn _model_::{class_mod_var}::{class}Cache>> {
        _Cache_::_{raw_rel_name}(self).map(|v| Box::new(v) as Box<dyn _model_::{class_mod_var}::{class}Cache>)
    }", "") }@
@{- parent.relations_one_uncached(true)|fmt_rel_join("
    fn {rel_name}(&self) -> Option<Box<dyn _model_::{class_mod_var}::{class}>> {
        if self.{rel_name}.is_none() {
            return None;
        }
        _Cache_::_{raw_rel_name}(self).map(|v| Box::new(v) as Box<dyn _model_::{class_mod_var}::{class}>)
    }", "") }@
@{- parent.relations_many_cache(true)|fmt_rel_join("
    fn {rel_name}(&self) -> Vec<Box<dyn _model_::{class_mod_var}::{class}Cache>> {
        _Cache_::_{raw_rel_name}(self).into_iter().map(|v| Box::new(v) as Box<dyn _model_::{class_mod_var}::{class}Cache>).collect()
    }", "") }@
@{- parent.relations_many_uncached(true)|fmt_rel_join("
    fn {rel_name}(&self) -> Vec<Box<dyn _model_::{class_mod_var}::{class}>> {
        if self.{rel_name}.is_none() {
            return Vec::new();
        }
        _Cache_::_{raw_rel_name}(self).into_iter().map(|v| Box::new(v) as Box<dyn _model_::{class_mod_var}::{class}>).collect()
    }", "") }@
@{- parent.relations_belonging_cache(true)|fmt_rel_join("
    #[allow(clippy::question_mark)]
    fn {rel_name}(&self) -> Option<Box<dyn _model_::{class_mod_var}::{class}Cache>> {
        if self.{rel_name}.is_none() {
            return None;
        }
        _Cache_::_{raw_rel_name}(self).map(|v| Box::new(v) as Box<dyn _model_::{class_mod_var}::{class}Cache>)
    }", "") }@
@{- parent.relations_belonging_uncached(true)|fmt_rel_join("
    #[allow(clippy::question_mark)]
    fn {rel_name}(&self) -> Option<Box<dyn _model_::{class_mod_var}::{class}>> {
        if self.{rel_name}.is_none() {
            return None;
        }
        _Cache_::_{raw_rel_name}(self).map(|v| Box::new(v) as Box<dyn _model_::{class_mod_var}::{class}>)
    }", "") }@
}
@%- endif %@
impl domain::models::@{ db|snake|to_var_name }@::@{ parent.group_name|to_var_name }@::@{ parent.name|to_var_name }@::@{ parent.name|pascal }@ for _@{ pascal_name }@ {
@{- parent.non_cache_cols_wo_primaries_and_read_only()|fmt_join("
    fn {var}(&self) -> {domain_outer} {
        _Getter_::_{raw_var}(self){convert_impl_domain_outer}
    }", "") }@
@{- parent.relations_one_and_belonging(true)|fmt_rel_join("
    #[allow(clippy::question_mark)]
    fn {rel_name}(&self) -> Option<&dyn _model_::{class_mod_var}::{class}> {
        if self.{rel_name}.is_none() {
            return None;
        }
        _Getter_::_{raw_rel_name}(self).map(|v| v as &dyn _model_::{class_mod_var}::{class})
    }", "") }@
@{- parent.relations_many(true)|fmt_rel_join("
    #[allow(clippy::question_mark)]
    fn {rel_name}(&self) -> Box<dyn Iterator<Item = &dyn _model_::{class_mod_var}::{class}> + '_> {
        if self.{rel_name}.is_none() {
            return Box::new(std::iter::empty::<&dyn _model_::{class_mod_var}::{class}>());
        }
        Box::new(_Getter_::_{raw_rel_name}(self).iter().map(|v| v as &dyn _model_::{class_mod_var}::{class}))
    }", "") }@
}
#[allow(clippy::useless_conversion)]
impl domain::models::@{ db|snake|to_var_name }@::@{ parent.group_name|to_var_name }@::@{ parent.name|to_var_name }@::@{ parent.name|pascal }@UpdaterBase for _@{ pascal_name }@Updater {
@{- parent.non_cache_cols_wo_primaries_and_read_only()|fmt_join("
    fn {var}(&self) -> {domain_outer} {
        {convert_impl_domain_outer_for_updater}
    }", "") }@
@{- parent.non_primaries_wo_read_only(true)|fmt_join("
    fn set_{raw_var}(&mut self, v: {domain_outer_owned}) {
        _Updater_::mut_{raw_var}(self).set(v{convert_domain_inner_type})
    }", "") }@
@{- parent.relations_one(true)|fmt_rel_join("
    fn {rel_name}(&mut self) -> Option<&dyn _model_::{class_mod_var}::{class}Updater> {
        self.{rel_name}.as_ref().unwrap_or_else(|| panic!(\"{raw_rel_name} is not fetched.\")).into_iter().filter(|v| !v.will_be_deleted()).last().map(|v| v as &dyn _model_::{class_mod_var}::{class}Updater)
    }
    fn set_{raw_rel_name}(&mut self, v: Box<dyn _model_::{class_mod_var}::{class}Updater>) {
        _Updater_::mut_{raw_rel_name}(self).set(
            if let Ok(v) = v.downcast::<crate::models::{group_var}::{mod_var}::_{class}Updater>() {
                *v
            } else {
                panic!(\"Only _{class}Updater is accepted.\");
            }
        )
    }", "") }@
@{- parent.relations_many(true)|fmt_rel_join("
    fn {rel_name}(&mut self) -> Box<dyn domain::models::UpdateIterator<dyn _model_::{class_mod_var}::{class}Updater> + '_> {
        struct V<'a, T: crate::misc::Updater>(crate::accessor::AccessorHasMany<'a, T>);
        impl<T: crate::misc::Updater + _model_::{class_mod_var}::{class}Updater> domain::models::UpdateIterator<dyn _model_::{class_mod_var}::{class}Updater> for V<'_, T> {
            fn iter_mut(&mut self) -> Box<dyn Iterator<Item = &mut (dyn _model_::{class_mod_var}::{class}Updater + 'static)> + '_> {
                Box::new(self.0.iter_mut().map(|v| v as &mut dyn _model_::{class_mod_var}::{class}Updater))
            }
        }
        Box::new(V(_Updater_::mut_{raw_rel_name}(self)))
    }
    fn take_{raw_rel_name}(&mut self) -> Option<Vec<Box<dyn _model_::{class_mod_var}::{class}Updater>>> {
        _Updater_::mut_{raw_rel_name}(self).take().map(|v| v.into_iter().map(|v| Box::new(v) as Box<dyn _model_::{class_mod_var}::{class}Updater>).collect())
    }
    fn replace_{raw_rel_name}(&mut self, list: Vec<Box<dyn _model_::{class_mod_var}::{class}Updater>>) {
        let mut vec = Vec::new();
        for row in list {
            match row.downcast::<crate::models::{group_var}::{mod_var}::_{class}Updater>() {
                Ok(v) => { vec.push(*v); }
                Err(_) => panic!(\"Only _{class}Updater is accepted.\"),
            }
        }
        _Updater_::mut_{raw_rel_name}(self).replace(vec);
    }
    fn push_{raw_rel_name}(&mut self, v: Box<dyn _model_::{class_mod_var}::{class}Updater>) {
        if let Ok(v) = v.downcast::<crate::models::{group_var}::{mod_var}::_{class}Updater>() {
            _Updater_::mut_{raw_rel_name}(self).push(*v)
        } else {
            panic!(\"Only _{class}Updater is accepted.\");
        }
    }", "") }@
}
@%- endfor %@

impl @{ pascal_name }@Common for _@{ pascal_name }@ {
@{- def.primaries()|fmt_join("
    fn {var}(&self) -> {domain_outer} {
        _Getter_::_{raw_var}(self){convert_impl_domain_outer}
    }", "") }@
@{- def.only_version()|fmt_join("
    fn {var}(&self) -> {domain_outer} {
        _Getter_::_{raw_var}(self)
    }", "") }@
@{- def.cache_cols_wo_primaries_and_read_only()|fmt_join("
    fn {var}(&self) -> {domain_outer} {
        _Getter_::_{raw_var}(self){convert_impl_domain_outer}
    }", "") }@
}
@%- if !config.force_disable_cache %@

impl @{ pascal_name }@Common for _@{ pascal_name }@Cache {
@{- def.primaries()|fmt_join("
    fn {var}(&self) -> {domain_outer} {
        _Cache_::_{raw_var}(self){convert_impl_domain_outer}
    }", "") }@
@{- def.only_version()|fmt_join("
    fn {var}(&self) -> {domain_outer} {
        _Cache_::_{raw_var}(self)
    }", "") }@
@{- def.cache_cols_wo_primaries_and_read_only()|fmt_join("
    fn {var}(&self) -> {domain_outer} {
        _Cache_::_{raw_var}(self){convert_impl_domain_outer}
    }", "") }@
}
@%- endif %@

impl @{ pascal_name }@Common for _@{ pascal_name }@Updater {
@{- def.primaries()|fmt_join("
    fn {var}(&self) -> {domain_outer} {
        {convert_impl_domain_outer_for_updater}
    }", "") }@
@{- def.only_version()|fmt_join("
    fn {var}(&self) -> {domain_outer} {
        self._data.{var}
    }", "") }@
@{- def.cache_cols_wo_primaries_and_read_only()|fmt_join("
    fn {var}(&self) -> {domain_outer} {
        {convert_impl_domain_outer_for_updater}
    }", "") }@
}
@%- if !config.force_disable_cache %@

impl @{ pascal_name }@Cache for _@{ pascal_name }@Cache {
@{- def.relations_one_cache(true)|fmt_rel_join("
    fn {rel_name}(&self) -> Option<Box<dyn _model_::{class_mod_var}::{class}Cache>> {
        _Cache_::_{raw_rel_name}(self).map(|v| Box::new(v) as Box<dyn _model_::{class_mod_var}::{class}Cache>)
    }", "") }@
@{- def.relations_one_uncached(true)|fmt_rel_join("
    fn {rel_name}(&self) -> Option<Box<dyn _model_::{class_mod_var}::{class}>> {
        if self.{rel_name}.is_none() {
            return None;
        }
        _Cache_::_{raw_rel_name}(self).map(|v| Box::new(v) as Box<dyn _model_::{class_mod_var}::{class}>)
    }", "") }@
@{- def.relations_many_cache(true)|fmt_rel_join("
    fn {rel_name}(&self) -> Vec<Box<dyn _model_::{class_mod_var}::{class}Cache>> {
        _Cache_::_{raw_rel_name}(self).into_iter().map(|v| Box::new(v) as Box<dyn _model_::{class_mod_var}::{class}Cache>).collect()
    }", "") }@
@{- def.relations_many_uncached(true)|fmt_rel_join("
    fn {rel_name}(&self) -> Vec<Box<dyn _model_::{class_mod_var}::{class}>> {
        if self.{rel_name}.is_none() {
            return Vec::new();
        }
        _Cache_::_{raw_rel_name}(self).into_iter().map(|v| Box::new(v) as Box<dyn _model_::{class_mod_var}::{class}>).collect()
    }", "") }@
@{- def.relations_belonging_cache(true)|fmt_rel_join("
    #[allow(clippy::question_mark)]
    fn {rel_name}(&self) -> Option<Box<dyn _model_::{class_mod_var}::{class}Cache>> {
        if self.{rel_name}.is_none() {
            return None;
        }
        _Cache_::_{raw_rel_name}(self).map(|v| Box::new(v) as Box<dyn _model_::{class_mod_var}::{class}Cache>)
    }", "") }@
@{- def.relations_belonging_uncached(true)|fmt_rel_join("
    #[allow(clippy::question_mark)]
    fn {rel_name}(&self) -> Option<Box<dyn _model_::{class_mod_var}::{class}>> {
        if self.{rel_name}.is_none() {
            return None;
        }
        _Cache_::_{raw_rel_name}(self).map(|v| Box::new(v) as Box<dyn _model_::{class_mod_var}::{class}>)
    }", "") }@
}
@%- endif %@

impl @{ pascal_name }@ for _@{ pascal_name }@ {
@{- def.non_cache_cols_wo_primaries_and_read_only()|fmt_join("
    fn {var}(&self) -> {domain_outer} {
        _Getter_::_{raw_var}(self){convert_impl_domain_outer}
    }", "") }@
@{- def.relations_one_and_belonging(true)|fmt_rel_join("
    #[allow(clippy::question_mark)]
    fn {rel_name}(&self) -> Option<&dyn _model_::{class_mod_var}::{class}> {
        if self.{rel_name}.is_none() {
            return None;
        }
        _Getter_::_{raw_rel_name}(self).map(|v| v as &dyn _model_::{class_mod_var}::{class})
    }", "") }@
@{- def.relations_many(true)|fmt_rel_join("
    #[allow(clippy::question_mark)]
    fn {rel_name}(&self) -> Box<dyn Iterator<Item = &dyn _model_::{class_mod_var}::{class}> + '_> {
        if self.{rel_name}.is_none() {
            return Box::new(std::iter::empty::<&dyn _model_::{class_mod_var}::{class}>());
        }
        Box::new(_Getter_::_{raw_rel_name}(self).iter().map(|v| v as &dyn _model_::{class_mod_var}::{class}))
    }", "") }@
}

#[allow(clippy::useless_conversion)]
impl @{ pascal_name }@UpdaterBase for _@{ pascal_name }@Updater {
@{- def.non_cache_cols_wo_primaries_and_read_only()|fmt_join("
    fn {var}(&self) -> {domain_outer} {
        {convert_impl_domain_outer_for_updater}
    }", "") }@
@{- def.non_primaries_wo_read_only(true)|fmt_join("
    fn set_{raw_var}(&mut self, v: {domain_outer_owned}) {
        _Updater_::mut_{raw_var}(self).set(v{convert_domain_inner_type})
    }", "") }@
@{- def.relations_one(true)|fmt_rel_join("
    fn {rel_name}(&mut self) -> Option<&mut dyn _model_::{class_mod_var}::{class}Updater> {
        self.{rel_name}.as_mut().unwrap_or_else(|| panic!(\"{raw_rel_name} is not fetched.\")).iter_mut().filter(|v| !v.will_be_deleted()).last().map(|v| v as &mut dyn _model_::{class_mod_var}::{class}Updater)
    }
    fn set_{raw_rel_name}(&mut self, v: Box<dyn _model_::{class_mod_var}::{class}Updater>) {
        _Updater_::mut_{raw_rel_name}(self).set(
            if let Ok(v) = v.downcast::<crate::models::{group_var}::{mod_var}::_{class}Updater>() {
                *v
            } else {
                panic!(\"Only _{class}Updater is accepted.\");
            }
        )
    }", "") }@
@{- def.relations_many(true)|fmt_rel_join("
    fn {rel_name}(&mut self) -> Box<dyn domain::models::UpdateIterator<dyn _model_::{class_mod_var}::{class}Updater> + '_> {
        struct V<'a, T: crate::misc::Updater>(crate::accessor::AccessorHasMany<'a, T>);
        impl<T: crate::misc::Updater + _model_::{class_mod_var}::{class}Updater> domain::models::UpdateIterator<dyn _model_::{class_mod_var}::{class}Updater> for V<'_, T> {
            fn iter_mut(&mut self) -> Box<dyn Iterator<Item = &mut (dyn _model_::{class_mod_var}::{class}Updater + 'static)> + '_> {
                Box::new(self.0.iter_mut().map(|v| v as &mut dyn _model_::{class_mod_var}::{class}Updater))
            }
        }
        Box::new(V(_Updater_::mut_{raw_rel_name}(self)))
    }
    fn take_{raw_rel_name}(&mut self) -> Option<Vec<Box<dyn _model_::{class_mod_var}::{class}Updater>>> {
        _Updater_::mut_{raw_rel_name}(self).take().map(|v| v.into_iter().map(|v| Box::new(v) as Box<dyn _model_::{class_mod_var}::{class}Updater>).collect())
    }
    fn replace_{raw_rel_name}(&mut self, list: Vec<Box<dyn _model_::{class_mod_var}::{class}Updater>>) {
        let mut vec = Vec::new();
        for row in list {
            match row.downcast::<crate::models::{group_var}::{mod_var}::_{class}Updater>() {
                Ok(v) => { vec.push(*v); }
                Err(_) => panic!(\"Only _{class}Updater is accepted.\"),
            }
        }
        _Updater_::mut_{raw_rel_name}(self).replace(vec);
    }
    fn push_{raw_rel_name}(&mut self, v: Box<dyn _model_::{class_mod_var}::{class}Updater>) {
        if let Ok(v) = v.downcast::<crate::models::{group_var}::{mod_var}::_{class}Updater>() {
            _Updater_::mut_{raw_rel_name}(self).push(*v)
        } else {
            panic!(\"Only _{class}Updater is accepted.\");
        }
    }", "") }@
}
impl domain::models::MarkForDelete for _@{ pascal_name }@Updater {
    fn mark_for_delete(&mut self) {
        crate::misc::Updater::mark_for_delete(self);
    }
    fn unmark_for_delete(&mut self) {
        crate::misc::Updater::unmark_for_delete(self);
    }
}

#[derive(derive_new::new, Clone)]
pub struct @{ pascal_name }@RepositoryImpl(std::sync::Arc<tokio::sync::Mutex<crate::DbConn>>);

#[allow(clippy::clone_on_copy)]
#[async_trait]
impl _@{ pascal_name }@Repository for @{ pascal_name }@RepositoryImpl {
    @%- if !def.disable_update() %@
    fn find_for_update(&self, id: @{ def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") }@) -> Box<dyn @{ pascal_name }@RepositoryFindForUpdateBuilder> {
        struct V {
            conn: std::sync::Arc<tokio::sync::Mutex<crate::DbConn>>,
            id: @{ def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") }@,
            visibility_filter: Option<Filter_>,
            @%- if def.is_soft_delete() %@
            with_trashed: bool,
            @%- endif %@
            joiner: Option<Box<Joiner_>>,
        }
        #[allow(unused_imports)]
        use @{ pascal_name }@RepositoryFindForUpdateBuilder as _RepositoryFindForUpdateBuilder;
        #[async_trait]
        impl @{ pascal_name }@RepositoryFindForUpdateBuilder for V {
            async fn query(self: Box<Self>) -> anyhow::Result<Box<dyn @{ pascal_name }@Updater>> {
                let mut conn = self.conn.lock().await;
                let conn = conn.deref_mut();
                #[allow(unused_mut)]
                @%- if def.is_soft_delete() %@
                let mut obj = if self.with_trashed {
                    _@{ pascal_name }@::find_for_update_with_trashed(conn, @{ def.primaries()|fmt_join_with_paren2("self.id{convert_from_entity}", "self.id.{index}{convert_from_entity}", ", ") }@, self.visibility_filter).await?
                } else {
                    _@{ pascal_name }@::find_for_update(conn, @{ def.primaries()|fmt_join_with_paren2("self.id{convert_from_entity}", "self.id.{index}{convert_from_entity}", ", ") }@, self.visibility_filter).await?
                };
                @%- else %@
                let mut obj = _@{ pascal_name }@::find_for_update(conn, @{ def.primaries()|fmt_join_with_paren2("self.id{convert_from_entity}", "self.id.{index}{convert_from_entity}", ", ") }@, self.visibility_filter).await?;
                @%- endif %@
                _@{ pascal_name }@Joiner::join(&mut obj, conn, self.joiner).await?;
                Ok(Box::new(obj) as Box<dyn @{ pascal_name }@Updater>)
            }
            fn visibility_filter(mut self: Box<Self>, filter: Filter_) -> Box<dyn _RepositoryFindForUpdateBuilder> {
                self.visibility_filter = Some(filter);
                self
            }
            @%- if def.is_soft_delete() %@
            fn with_trashed(mut self: Box<Self>, mode: bool) -> Box<dyn _RepositoryFindForUpdateBuilder> {
                self.with_trashed = mode;
                self
            }
            @%- endif %@
            fn join(mut self: Box<Self>, joiner: Option<Box<Joiner_>>) -> Box<dyn _RepositoryFindForUpdateBuilder> {
                self.joiner = Joiner_::merge(self.joiner, joiner);
                self
            }
        }
        Box::new(V {
            conn: self.0.clone(),
            id,
            visibility_filter: None,
            @%- if def.is_soft_delete() %@
            with_trashed: false,
            @%- endif %@
            joiner: None,
        })
    }
    @%- endif %@
    fn convert_factory(&self, factory: @{ pascal_name }@Factory) -> Box<dyn @{ pascal_name }@Updater> {
        Box::new(_@{ pascal_name }@Updater::from(factory))
    }
    #[allow(unused_mut)]
    async fn save(&self, obj: Box<dyn @{ pascal_name }@Updater>) -> anyhow::Result<Option<Box<dyn @{ pascal_name }@>>> {
        let obj: _@{ pascal_name }@Updater = match obj.downcast::<_@{ pascal_name }@Updater>() {
            Ok(obj) => *obj,
            Err(_) => panic!("Only _@{ pascal_name }@Updater is accepted."),
        };
        Ok(_@{ pascal_name }@::save(self.0.lock().await.deref_mut(), obj).await?.map(|v| Box::new(v) as Box<dyn @{ pascal_name }@>))
    }
    @%- if !def.disable_update() %@
    async fn import(&self, list: Vec<Box<dyn @{ pascal_name }@Updater>>, option: Option<domain::models::ImportOption>) -> anyhow::Result<()> {
        let list = list.into_iter().map(|obj| {
            let obj: _@{ pascal_name }@Updater = match obj.downcast::<_@{ pascal_name }@Updater>() {
                Ok(obj) => *obj,
                Err(_) => panic!("Only _@{ pascal_name }@Updater is accepted."),
            };
            obj
        }).collect();
        let option = option.unwrap_or_default();
        if option.overwrite.unwrap_or_default() {
            _@{ pascal_name }@::bulk_overwrite(self.0.lock().await.deref_mut(), list).await?;
        } else {
            _@{ pascal_name }@::bulk_insert(self.0.lock().await.deref_mut(), list, option.ignore.unwrap_or_default()).await?;
        }
        Ok(())
    }
    @%- endif %@
    @%- if def.use_insert_delayed() %@
    async fn insert_delayed(&self, obj: Box<dyn @{ pascal_name }@Updater>) -> anyhow::Result<()> {
        let obj: _@{ pascal_name }@Updater = if obj.is::<_@{ pascal_name }@Updater>() {
            *obj.downcast::<_@{ pascal_name }@Updater>().unwrap()
        } else {
            panic!("Only _@{ pascal_name }@Updater is accepted.");
        };
        _@{ pascal_name }@::insert_delayed(self.0.lock().await.deref_mut(), obj).await?;
        Ok(())
    }
    @%- endif %@
    @%- if !def.disable_update() %@
    async fn delete(&self, obj: Box<dyn @{ pascal_name }@Updater>) -> anyhow::Result<()> {
        let obj = if let Ok(obj) = obj.downcast::<_@{ pascal_name }@Updater>() {
            obj
        } else {
            panic!("Only _@{ pascal_name }@Updater is accepted.");
        };
        _@{ pascal_name }@::delete(self.0.lock().await.deref_mut(), *obj).await
    }
    @%- if def.primaries().len() == 1 %@
    async fn delete_by_ids(&self, ids: &[@{ def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") }@]) -> anyhow::Result<u64> {
        _@{ pascal_name }@::delete_by_ids(self.0.lock().await.deref_mut(), ids.iter().map(|v| @{ def.primaries()|fmt_join_with_paren2("v{convert_from_entity}", "v.{index}{convert_from_entity}", ", ") }@)).await
    }
    @%- endif %@
    async fn delete_all(&self) -> anyhow::Result<()> {
        _@{ pascal_name }@::query().delete(self.0.lock().await.deref_mut()).await?;
        Ok(())
    }
    @%- endif %@
    @%- if def.act_as_job_queue() %@
    async fn fetch(&self, limit: usize) -> anyhow::Result<Vec<Box<dyn @{ pascal_name }@Updater>>> {
        let list = _@{ pascal_name }@::query().order_by(order!(@{ def.primaries()|fmt_join_with_paren("{raw_var}", ", ") }@)).limit(limit).skip_locked().select_for_update(self.0.lock().await.deref_mut()).await?;
        Ok(list.into_iter().map(|v| Box::new(v) as Box<dyn @{ pascal_name }@Updater>).collect())
    }
    @%- endif %@
    @%- for (selector, selector_def) in def.selectors %@
    fn @{ selector|to_var_name }@(&self) -> Box<dyn @{ pascal_name }@Repository@{ selector|pascal }@Builder> {
        struct V {
            conn: std::sync::Arc<tokio::sync::Mutex<crate::DbConn>>,
            query_filter: Option<_@{ mod_name }@::@{ pascal_name }@Query@{ selector|pascal }@Filter>,
            visibility_filter: Option<Filter_>,
            @%- if def.is_soft_delete() %@
            with_trashed: bool,
            @%- endif %@
            joiner: Option<Box<Joiner_>>,
        }
        #[allow(unused_imports)]
        use @{ pascal_name }@Repository@{ selector|pascal }@Builder as _Repository@{ selector|pascal }@Builder;
        #[async_trait]
        impl @{ pascal_name }@Repository@{ selector|pascal }@Builder for V {
            async fn query(self: Box<Self>) -> anyhow::Result<Vec<Box<dyn @{ pascal_name }@Updater>>> {
                let mut conn = self.conn.lock().await;
                let conn = conn.deref_mut();
                let mut query = _@{ pascal_name }@::query();
                let mut fltr = if let Some(filter) = self.query_filter {
                    _filter_@{ selector }@(&filter)?
                } else {
                    filter!()
                };
                if let Some(filter) = self.visibility_filter {
                    query = query.join(Joiner_::merge(self.joiner, filter.joiner()));
                    fltr = fltr.and(filter);
                } else {
                    query = query.join(self.joiner);
                }
                query = query.filter(fltr);
                @%- if def.is_soft_delete() %@
                query = query.when(self.with_trashed, |v| v.with_trashed());
                @%- endif %@
                Ok(query.select_for_update(conn).await?.into_iter().map(|v| Box::new(v) as Box<dyn @{ pascal_name }@Updater>).collect())
            }
            async fn count(self: Box<Self>) -> anyhow::Result<i64> {
                let mut conn = self.conn.lock().await;
                let conn = conn.deref_mut();
                let mut query = _@{ pascal_name }@::query();
                let mut fltr = if let Some(filter) = self.query_filter {
                    _filter_@{ selector }@(&filter)?
                } else {
                    filter!()
                };
                if let Some(filter) = self.visibility_filter {
                    fltr = fltr.and(filter);
                }
                query = query.filter(fltr);
                @%- if def.is_soft_delete() %@
                query = query.when(self.with_trashed, |v| v.with_trashed());
                @%- endif %@
                Ok(query.count(conn).await?)
            }
            fn query_filter(mut self: Box<Self>, filter: _@{ mod_name }@::@{ pascal_name }@Query@{ selector|pascal }@Filter) -> Box<dyn _Repository@{ selector|pascal }@Builder> {
                self.query_filter = Some(filter);
                self
            }
            fn visibility_filter(mut self: Box<Self>, filter: Filter_) -> Box<dyn _Repository@{ selector|pascal }@Builder> { self.visibility_filter = Some(filter); self }
            @%- if def.is_soft_delete() %@
            fn with_trashed(mut self: Box<Self>, mode: bool) -> Box<dyn _Repository@{ selector|pascal }@Builder> { self.with_trashed = mode; self  }
            @%- endif %@
            fn join(mut self: Box<Self>, joiner: Option<Box<Joiner_>>) -> Box<dyn _Repository@{ selector|pascal }@Builder>  {
                self.joiner = Joiner_::merge(self.joiner, joiner);
                self
            }
        }
        Box::new(V {
            conn: self.0.clone(),
            query_filter: None,
            visibility_filter: None,
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
fn _filter@{ filter_map.suffix }@(filter: &_@{ mod_name }@::@{ pascal_name }@Query@{ selector|pascal }@@{ filter_map.pascal_name }@Filter) -> anyhow::Result<crate::models::@{ filter_map.model_group()|snake|to_var_name }@::_base::_@{ filter_map.model_name()|snake }@::Filter_> {
    #[allow(unused_imports)]
    @%- if config.excluded_from_domain %@
    use crate::models::@{ filter_map.model_group()|snake|to_var_name }@::@{ filter_map.model_name()|snake|to_var_name }@::filter;
    @%- else %@
    use domain::models::@{ db|snake|to_var_name }@::@{ filter_map.model_group()|snake|to_var_name }@::@{ filter_map.model_name()|snake|to_var_name }@::filter;
    @%- endif %@
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
impl _@{ pascal_name }@Query for @{ pascal_name }@RepositoryImpl {
    @%- if def.use_all_row_cache() && !def.use_filtered_row_cache() %@
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
        Ok(Box::new(V(_@{ pascal_name }@::find_all_from_cache(self.0.lock().await.deref(), None).await?)))
    }
    @%- endif %@
    @%- for (selector, selector_def) in def.selectors %@
    fn @{ selector|to_var_name }@(&self) -> Box<dyn @{ pascal_name }@Query@{ selector|pascal }@Builder> {
        struct V {
            conn: std::sync::Arc<tokio::sync::Mutex<crate::DbConn>>,
            query_filter: Option<_@{ mod_name }@::@{ pascal_name }@Query@{ selector|pascal }@Filter>,
            visibility_filter: Option<Filter_>,
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
        fn _cursor(mut fltr: crate::models::@{ group_name|to_var_name }@::_base::_@{ mod_name }@::Filter_, cursor: &@{ pascal_name }@Query@{ selector|pascal }@Cursor) -> anyhow::Result<crate::models::@{ group_name|to_var_name }@::_base::_@{ mod_name }@::Filter_> {
            match cursor {
                @%- for (cursor, cursor_def) in selector_def.orders %@
                @{ pascal_name }@Query@{ selector|pascal }@Cursor::@{ cursor|pascal }@(c) => {
                    match c {
                        @{- cursor_def.db_str() }@
                    }
                }
                @%- endfor %@
            }
            Ok(fltr)
        }
        #[allow(unused_imports)]
        use @{ pascal_name }@Query@{ selector|pascal }@Builder as _Query@{ selector|pascal }@Builder;
        #[async_trait]
        impl @{ pascal_name }@Query@{ selector|pascal }@Builder for V {
            async fn query(self: Box<Self>) -> anyhow::Result<Vec<Box<dyn @{ pascal_name }@@% if def.use_cache() %@Cache@% endif %@>>> {
                let mut conn = self.conn.lock().await;
                let conn = conn.deref_mut();
                let mut query = _@{ pascal_name }@::query();
                let mut fltr = if let Some(filter) = self.query_filter {
                    _filter_@{ selector }@(&filter)?
                } else {
                    filter!()
                };
                if let Some(filter) = self.visibility_filter {
                    query = query.join(Joiner_::merge(self.joiner, filter.joiner()));
                    fltr = fltr.and(filter);
                } else {
                    query = query.join(self.joiner);
                }
                if let Some(cursor) = self.cursor {
                    fltr = _cursor(fltr, &cursor)?;
                }
                query = query.filter(fltr);
                match self.order.unwrap_or_default() {
                    @%- for (order, fields) in selector_def.orders %@
                    _@{ mod_name }@::@{ pascal_name }@Query@{ selector|pascal }@Order::@{ order|pascal }@ => {
                        if !self.reverse {
                            query = query.order_by(order!(@{ selector_def.db_order(order, false) }@));
                        } else {
                            query = query.order_by(order!(@{ selector_def.db_order(order, true) }@));
                        }
                    }
                    @%- endfor %@
                }
                query = query.when(self.limit > 0, |v| v.limit(self.limit));
                query = query.when(self.offset > 0, |v| v.offset(self.offset));
                @%- if def.is_soft_delete() %@
                query = query.when(self.with_trashed, |v| v.with_trashed());
                @%- endif %@
                Ok(query.select@% if def.use_cache() %@_from_cache@% endif %@(conn).await?.into_iter().map(|v| Box::new(v) as Box<dyn @{ pascal_name }@@% if def.use_cache() %@Cache@% endif %@>).collect())
            }
            async fn count(self: Box<Self>) -> anyhow::Result<i64> {
                let mut conn = self.conn.lock().await;
                let conn = conn.deref_mut();
                let mut query = _@{ pascal_name }@::query();
                let mut fltr = if let Some(filter) = self.query_filter {
                    _filter_@{ selector }@(&filter)?
                } else {
                    filter!()
                };
                if let Some(filter) = self.visibility_filter {
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
            fn query_filter(mut self: Box<Self>, filter: _@{ mod_name }@::@{ pascal_name }@Query@{ selector|pascal }@Filter) -> Box<dyn _Query@{ selector|pascal }@Builder> { self.query_filter = Some(filter); self }
            fn visibility_filter(mut self: Box<Self>, filter: Filter_) -> Box<dyn _Query@{ selector|pascal }@Builder> { self.visibility_filter = Some(filter); self }
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
            conn: self.0.clone(),
            query_filter: None,
            visibility_filter: None,
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
    fn find(&self, id: @{ def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") }@) -> Box<dyn @{ pascal_name }@QueryFindBuilder> {
        struct V {
            conn: std::sync::Arc<tokio::sync::Mutex<crate::DbConn>>,
            id: @{ def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") }@,
            visibility_filter: Option<Filter_>,
            @%- if def.is_soft_delete() %@
            with_trashed: bool,
            @%- endif %@
            joiner: Option<Box<Joiner_>>,
        }
        #[allow(unused_imports)]
        use @{ pascal_name }@QueryFindBuilder as _QueryFindBuilder;
        #[async_trait]
        impl @{ pascal_name }@QueryFindBuilder for V {
            async fn query(self: Box<Self>) -> anyhow::Result<Option<Box<dyn @{ pascal_name }@Cache>>> {
                let mut conn = self.conn.lock().await;
                let conn = conn.deref_mut();
                @%- if def.is_soft_delete() %@
                let obj = if self.with_trashed {
                    _@{ pascal_name }@::find_optional_from_cache_with_trashed(conn, @{ def.primaries()|fmt_join_with_paren2("self.id{convert_from_entity}", "self.id.{index}{convert_from_entity}", ", ") }@).await?
                } else {
                    _@{ pascal_name }@::find_optional_from_cache(conn, @{ def.primaries()|fmt_join_with_paren2("self.id{convert_from_entity}", "self.id.{index}{convert_from_entity}", ", ") }@).await?
                };
                @%- else %@
                let obj = _@{ pascal_name }@::find_optional_from_cache(conn, @{ def.primaries()|fmt_join_with_paren2("self.id{convert_from_entity}", "self.id.{index}{convert_from_entity}", ", ") }@).await?;
                @%- endif %@
                if let Some(mut obj) = obj {
                    if let Some(filter) = self.visibility_filter {
                        _@{ pascal_name }@Joiner::join(&mut obj, conn, Joiner_::merge(self.joiner, filter.joiner())).await?;
                        use domain::models::Check_ as _;
                        if filter.check(&obj as &dyn @{ pascal_name }@Cache) {
                            Ok(Some(Box::new(obj) as Box<dyn @{ pascal_name }@Cache>))
                        } else {
                            Ok(None)
                        }
                    } else {
                        _@{ pascal_name }@Joiner::join(&mut obj, conn, self.joiner).await?;
                        Ok(Some(Box::new(obj) as Box<dyn @{ pascal_name }@Cache>))
                    }
                } else {
                    Ok(None)
                }
            }
            fn visibility_filter(mut self: Box<Self>, filter: Filter_) -> Box<dyn _QueryFindBuilder> {
                self.visibility_filter = Some(filter);
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
            conn: self.0.clone(),
            id,
            visibility_filter: None,
            @%- if def.is_soft_delete() %@
            with_trashed: false,
            @%- endif %@
            joiner: None,
        })
    }
    @%- else %@
    fn find(&self, id: @{ def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") }@) -> Box<dyn @{ pascal_name }@QueryFindDirectlyBuilder> {
        self.find_directly(id)
    }
    @%- endif %@
    fn find_directly(&self, id: @{ def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") }@) -> Box<dyn @{ pascal_name }@QueryFindDirectlyBuilder> {
        struct V {
            conn: std::sync::Arc<tokio::sync::Mutex<crate::DbConn>>,
            id: @{ def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") }@,
            visibility_filter: Option<Filter_>,
            @%- if def.is_soft_delete() %@
            with_trashed: bool,
            @%- endif %@
            joiner: Option<Box<Joiner_>>,
        }
        #[allow(unused_imports)]
        use @{ pascal_name }@QueryFindDirectlyBuilder as _QueryFindDirectlyBuilder;
        #[async_trait]
        impl @{ pascal_name }@QueryFindDirectlyBuilder for V {
            async fn query(self: Box<Self>) -> anyhow::Result<Option<Box<dyn @{ pascal_name }@>>> {
                let mut conn = self.conn.lock().await;
                let conn = conn.deref_mut();
                @%- if def.is_soft_delete() %@
                let obj = if self.with_trashed {
                    _@{ pascal_name }@::find_optional_with_trashed(conn, @{ def.primaries()|fmt_join_with_paren2("self.id{convert_from_entity}", "self.id.{index}{convert_from_entity}", ", ") }@, self.visibility_filter).await?
                } else {
                    _@{ pascal_name }@::find_optional(conn, @{ def.primaries()|fmt_join_with_paren2("self.id{convert_from_entity}", "self.id.{index}{convert_from_entity}", ", ") }@, self.visibility_filter).await?
                };
                @%- else %@
                let obj = _@{ pascal_name }@::find_optional(conn, @{ def.primaries()|fmt_join_with_paren2("self.id{convert_from_entity}", "self.id.{index}{convert_from_entity}", ", ") }@, self.visibility_filter).await?;
                @%- endif %@
                if let Some(mut obj) = obj {
                    _@{ pascal_name }@Joiner::join(&mut obj, conn, self.joiner).await?;
                    Ok(Some(Box::new(obj) as Box<dyn @{ pascal_name }@>))
                } else {
                    Ok(None)
                }
            }
            fn visibility_filter(mut self: Box<Self>, filter: Filter_) -> Box<dyn _QueryFindDirectlyBuilder> {
                self.visibility_filter = Some(filter);
                self
            }
            @%- if def.is_soft_delete() %@
            fn with_trashed(mut self: Box<Self>, mode: bool) -> Box<dyn _QueryFindDirectlyBuilder> {
                self.with_trashed = mode;
                self
            }
            @%- endif %@
            fn join(mut self: Box<Self>, joiner: Option<Box<Joiner_>>) -> Box<dyn _QueryFindDirectlyBuilder> {
                self.joiner = Joiner_::merge(self.joiner, joiner);
                self
            }
        }
        Box::new(V {
            conn: self.0.clone(),
            id,
            visibility_filter: None,
            @%- if def.is_soft_delete() %@
            with_trashed: false,
            @%- endif %@
            joiner: None,
        })
    }
}
@{-"\n"}@