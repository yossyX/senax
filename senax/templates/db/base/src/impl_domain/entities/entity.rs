#[allow(unused_imports)]
use crate::misc::Updater as _;
use crate::models::@{ group_name|snake|to_var_name }@::@{ mod_name|to_var_name }@::*;
#[allow(unused_imports)]
use anyhow::Context as _;
// #[allow(unused_imports)]
// use domain::repository::@{ db|snake|to_var_name }@::@{ group_name|snake|to_var_name }@::@{ mod_name|to_var_name }@::{self, *};
// use domain::repository::@{ db|snake|to_var_name }@::@{ group_name|snake|to_var_name }@::@{ mod_name|to_var_name }@::*;
use base_domain as domain;
use domain::models::@{ db|snake|to_var_name }@::@{ group_name|snake|to_var_name }@::@{ mod_name|to_var_name }@::*;
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
@%- for (name, rel_def) in def.belongs_to_outer_db() %@
use domain::models::@{ rel_def.db()|snake|to_var_name }@ as _@{ rel_def.db()|snake }@_model_;
@%- endfor %@

type __Getter__ = dyn crate::models::@{ group_name|snake|to_var_name }@::@{ mod_name|to_var_name }@::_@{ pascal_name }@Getter;
@%- if !config.force_disable_cache %@
type __Cache__ = _@{ pascal_name }@Cache;
@%- endif %@
type __Updater__ = _@{ pascal_name }@Updater;

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
        __Getter__::_{raw_var}(self){clone}{convert_impl_domain_inner}
    }", "") }@
@{- parent.only_version()|fmt_join("
    fn {var}(&self) -> {domain_outer} {
        __Getter__::_{raw_var}(self)
    }", "") }@
@{- parent.cache_cols_wo_primaries_and_invisibles()|fmt_join("
    fn {var}(&self) -> {domain_outer} {
        __Getter__::_{raw_var}(self){convert_impl_domain_outer}
    }", "") }@
}
@%- if !config.force_disable_cache %@
impl domain::models::@{ db|snake|to_var_name }@::@{ parent.group_name|to_var_name }@::@{ parent.name|to_var_name }@::@{ parent.name|pascal }@Common for _@{ pascal_name }@Cache {
@{- parent.primaries()|fmt_join("
    fn _{raw_var}(&self) -> {inner} {
        __Cache__::_{raw_var}(self){clone}{convert_impl_domain_inner}
    }", "") }@
@{- parent.only_version()|fmt_join("
    fn {var}(&self) -> {domain_outer} {
        __Cache__::_{raw_var}(self)
    }", "") }@
@{- parent.cache_cols_wo_primaries_and_invisibles()|fmt_join("
    fn {var}(&self) -> {domain_outer} {
        __Cache__::_{raw_var}(self){convert_impl_domain_outer}
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
@{- parent.cache_cols_wo_primaries_and_invisibles()|fmt_join("
    fn {var}(&self) -> {domain_outer} {
        {convert_impl_domain_outer_for_updater}
    }", "") }@
}
@%- if !config.force_disable_cache %@
impl domain::models::@{ db|snake|to_var_name }@::@{ parent.group_name|to_var_name }@::@{ parent.name|to_var_name }@::@{ parent.name|pascal }@Cache for _@{ pascal_name }@Cache {
@{- parent.relations_one_cache(true)|fmt_rel_join("
    fn {rel_name}(&self) -> anyhow::Result<Option<Box<dyn _model_::{class_mod_var}::{class}Cache>>> {
        Ok(__Cache__::_{raw_rel_name}(self)?.map(|v| Box::new(v) as Box<dyn _model_::{class_mod_var}::{class}Cache>))
    }", "") }@
@{- parent.relations_one_uncached(true)|fmt_rel_join("
    fn {rel_name}(&self) -> anyhow::Result<Option<Box<dyn _model_::{class_mod_var}::{class}>>> {
        Ok(__Cache__::_{raw_rel_name}(self)?.map(|v| Box::new(v) as Box<dyn _model_::{class_mod_var}::{class}>))
    }", "") }@
@{- parent.relations_many_cache(true)|fmt_rel_join("
    fn {rel_name}(&self) -> anyhow::Result<Vec<Box<dyn _model_::{class_mod_var}::{class}Cache>>> {
        Ok(__Cache__::_{raw_rel_name}(self)?.into_iter().map(|v| Box::new(v) as Box<dyn _model_::{class_mod_var}::{class}Cache>).collect())
    }", "") }@
@{- parent.relations_many_uncached(true)|fmt_rel_join("
    fn {rel_name}(&self) -> anyhow::Result<Vec<Box<dyn _model_::{class_mod_var}::{class}>>> {
        Ok(__Cache__::_{raw_rel_name}(self)?.into_iter().map(|v| Box::new(v) as Box<dyn _model_::{class_mod_var}::{class}>).collect())
    }", "") }@
@{- parent.relations_belonging_cache(true)|fmt_rel_join("
    #[allow(clippy::question_mark)]
    fn {rel_name}(&self) -> anyhow::Result<Option<Box<dyn _model_::{class_mod_var}::{class}Cache>>> {
        Ok(__Cache__::_{raw_rel_name}(self)?.map(|v| Box::new(v) as Box<dyn _model_::{class_mod_var}::{class}Cache>))
    }", "") }@
@{- parent.relations_belonging_uncached(true)|fmt_rel_join("
    #[allow(clippy::question_mark)]
    fn {rel_name}(&self) -> anyhow::Result<Option<Box<dyn _model_::{class_mod_var}::{class}>>> {
        Ok(__Cache__::_{raw_rel_name}(self)?.map(|v| Box::new(v) as Box<dyn _model_::{class_mod_var}::{class}>))
    }", "") }@
@{- parent.relations_belonging_outer_db(true)|fmt_rel_outer_db_join("
    #[allow(clippy::question_mark)]
    fn {rel_name}(&self) -> anyhow::Result<Option<Box<dyn _{db_snake}_model_::{class_mod_var}::{class}>>> {
        Ok(__Cache__::_{raw_rel_name}(self)?.map(|v| Box::new(v) as Box<dyn _{db_snake}_model_::{class_mod_var}::{class}>))
    }", "") }@}
@%- endif %@
impl domain::models::@{ db|snake|to_var_name }@::@{ parent.group_name|to_var_name }@::@{ parent.name|to_var_name }@::@{ parent.name|pascal }@ for _@{ pascal_name }@ {
@{- parent.non_cache_cols_wo_primaries_and_invisibles()|fmt_join("
    fn {var}(&self) -> {domain_outer} {
        __Getter__::_{raw_var}(self){convert_impl_domain_outer}
    }", "") }@
@{- parent.relations_one_and_belonging(true)|fmt_rel_join("
    #[allow(clippy::question_mark)]
    fn {rel_name}(&self) -> anyhow::Result<Option<&dyn _model_::{class_mod_var}::{class}>> {
        Ok(__Getter__::_{raw_rel_name}(self)?.map(|v| v as &dyn _model_::{class_mod_var}::{class}))
    }", "") }@
@{- parent.relations_many(true)|fmt_rel_join("
    #[allow(clippy::question_mark)]
    fn {rel_name}(&self) -> anyhow::Result<Box<dyn Iterator<Item = &dyn _model_::{class_mod_var}::{class}> + '_>> {
        Ok(Box::new(__Getter__::_{raw_rel_name}(self)?.iter().map(|v| v as &dyn _model_::{class_mod_var}::{class})))
    }", "") }@
@{- parent.relations_belonging_outer_db(true)|fmt_rel_outer_db_join("
    #[allow(clippy::question_mark)]
    fn {rel_name}(&self) -> anyhow::Result<Option<&dyn _{db_snake}_model_::{class_mod_var}::{class}>> {
        Ok(__Getter__::_{raw_rel_name}(self)?.map(|v| v as &dyn _{db_snake}_model_::{class_mod_var}::{class}))
    }", "") }@
}
#[allow(clippy::useless_conversion)]
impl domain::models::@{ db|snake|to_var_name }@::@{ parent.group_name|to_var_name }@::@{ parent.name|to_var_name }@::@{ parent.name|pascal }@Updater for _@{ pascal_name }@Updater {
@{- parent.non_cache_cols_wo_primaries_and_invisibles()|fmt_join("
    fn {var}(&self) -> {domain_outer} {
        {convert_impl_domain_outer_for_updater}
    }", "") }@
@{- parent.non_primaries_wo_invisible_and_read_only(true)|fmt_join("
    fn set_{raw_var}(&mut self, v: {domain_factory}) {
        __Updater__::mut_{raw_var}(self).set(v{convert_domain_inner_type})
    }", "") }@
@{- parent.relations_one(true)|fmt_rel_join("
    fn {rel_name}(&mut self) -> anyhow::Result<Option<&mut dyn _model_::{class_mod_var}::{class}Updater>> {
        Ok(self.{rel_name}.as_mut().context(\"{raw_rel_name} is not joined.\")?.iter_mut().last().filter(|v| !v.will_be_deleted() && !v.has_been_deleted()).map(|v| v as &mut dyn _model_::{class_mod_var}::{class}Updater))
    }
    fn set_{raw_rel_name}(&mut self, v: Box<dyn _model_::{class_mod_var}::{class}Updater>) {
        __Updater__::mut_{raw_rel_name}(self).set(
            if let Ok(v) = v.downcast::<crate::models::{group_var}::{mod_var}::_{class}Updater>() {
                *v
            } else {
                panic!(\"Only _{class}Updater is accepted.\");
            }
        )
    }", "") }@
@{- parent.relations_many(true)|fmt_rel_join("
    fn {rel_name}(&mut self) -> anyhow::Result<Box<dyn domain::models::UpdateIterator<dyn _model_::{class_mod_var}::{class}Updater> + '_>> {
        struct V<'a, T: crate::misc::Updater>(crate::accessor::AccessorHasMany<'a, T>);
        impl<T: crate::misc::Updater + _model_::{class_mod_var}::{class}Updater> domain::models::UpdateIterator<dyn _model_::{class_mod_var}::{class}Updater> for V<'_, T> {
            fn iter_mut(&mut self) -> Box<dyn Iterator<Item = &mut (dyn _model_::{class_mod_var}::{class}Updater + 'static)> + '_> {
                Box::new(self.0.iter_mut().map(|v| v as &mut dyn _model_::{class_mod_var}::{class}Updater))
            }
        }
        self.{rel_name}.as_ref().context(\"{raw_rel_name} is not joined.\")?;
        Ok(Box::new(V(__Updater__::mut_{raw_rel_name}(self))))
    }
    fn take_{raw_rel_name}(&mut self) -> Option<Vec<Box<dyn _model_::{class_mod_var}::{class}Updater>>> {
        __Updater__::mut_{raw_rel_name}(self).take().map(|v| v.into_iter().map(|v| Box::new(v) as Box<dyn _model_::{class_mod_var}::{class}Updater>).collect())
    }
    fn replace_{raw_rel_name}(&mut self, list: Vec<Box<dyn _model_::{class_mod_var}::{class}Updater>>) {
        let mut vec = Vec::new();
        for row in list {
            match row.downcast::<crate::models::{group_var}::{mod_var}::_{class}Updater>() {
                Ok(v) => { vec.push(*v); }
                Err(_) => panic!(\"Only _{class}Updater is accepted.\"),
            }
        }
        __Updater__::mut_{raw_rel_name}(self).replace(vec);
    }
    fn push_{raw_rel_name}(&mut self, v: Box<dyn _model_::{class_mod_var}::{class}Updater>) {
        if let Ok(v) = v.downcast::<crate::models::{group_var}::{mod_var}::_{class}Updater>() {
            __Updater__::mut_{raw_rel_name}(self).push(*v)
        } else {
            panic!(\"Only _{class}Updater is accepted.\");
        }
    }", "") }@
}
@%- endfor %@

impl @{ pascal_name }@Common for _@{ pascal_name }@ {
@{- def.primaries()|fmt_join("
    fn {var}(&self) -> {domain_outer} {
        __Getter__::_{raw_var}(self){convert_impl_domain_outer}
    }", "") }@
@{- def.only_version()|fmt_join("
    fn {var}(&self) -> {domain_outer} {
        __Getter__::_{raw_var}(self)
    }", "") }@
@{- def.cache_cols_wo_primaries_and_invisibles()|fmt_join("
    fn {var}(&self) -> {domain_outer} {
        __Getter__::_{raw_var}(self){convert_impl_domain_outer}
    }", "") }@
}
@%- if !config.force_disable_cache %@

impl @{ pascal_name }@Common for _@{ pascal_name }@Cache {
@{- def.primaries()|fmt_join("
    fn {var}(&self) -> {domain_outer} {
        __Cache__::_{raw_var}(self){convert_impl_domain_outer}
    }", "") }@
@{- def.only_version()|fmt_join("
    fn {var}(&self) -> {domain_outer} {
        __Cache__::_{raw_var}(self)
    }", "") }@
@{- def.cache_cols_wo_primaries_and_invisibles()|fmt_join("
    fn {var}(&self) -> {domain_outer} {
        __Cache__::_{raw_var}(self){convert_impl_domain_outer}
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
@{- def.cache_cols_wo_primaries_and_invisibles()|fmt_join("
    fn {var}(&self) -> {domain_outer} {
        {convert_impl_domain_outer_for_updater}
    }", "") }@
}
@%- if !config.force_disable_cache %@

impl @{ pascal_name }@Cache for _@{ pascal_name }@Cache {
@{- def.relations_one_cache(true)|fmt_rel_join("
    fn {rel_name}(&self) -> anyhow::Result<Option<Box<dyn _model_::{class_mod_var}::{class}Cache>>> {
        Ok(__Cache__::_{raw_rel_name}(self)?.map(|v| Box::new(v) as Box<dyn _model_::{class_mod_var}::{class}Cache>))
    }", "") }@
@{- def.relations_one_uncached(true)|fmt_rel_join("
    fn {rel_name}(&self) -> anyhow::Result<Option<Box<dyn _model_::{class_mod_var}::{class}>>> {
        Ok(__Cache__::_{raw_rel_name}(self)?.map(|v| Box::new(v) as Box<dyn _model_::{class_mod_var}::{class}>))
    }", "") }@
@{- def.relations_many_cache(true)|fmt_rel_join("
    fn {rel_name}(&self) -> anyhow::Result<Vec<Box<dyn _model_::{class_mod_var}::{class}Cache>>> {
        Ok(__Cache__::_{raw_rel_name}(self)?.into_iter().map(|v| Box::new(v) as Box<dyn _model_::{class_mod_var}::{class}Cache>).collect())
    }", "") }@
@{- def.relations_many_uncached(true)|fmt_rel_join("
    fn {rel_name}(&self) -> anyhow::Result<Vec<Box<dyn _model_::{class_mod_var}::{class}>>> {
        Ok(__Cache__::_{raw_rel_name}(self)?.into_iter().map(|v| Box::new(v) as Box<dyn _model_::{class_mod_var}::{class}>).collect())
    }", "") }@
@{- def.relations_belonging_cache(true)|fmt_rel_join("
    #[allow(clippy::question_mark)]
    fn {rel_name}(&self) -> anyhow::Result<Option<Box<dyn _model_::{class_mod_var}::{class}Cache>>> {
        Ok(__Cache__::_{raw_rel_name}(self)?.map(|v| Box::new(v) as Box<dyn _model_::{class_mod_var}::{class}Cache>))
    }", "") }@
@{- def.relations_belonging_uncached(true)|fmt_rel_join("
    #[allow(clippy::question_mark)]
    fn {rel_name}(&self) -> anyhow::Result<Option<Box<dyn _model_::{class_mod_var}::{class}>>> {
        Ok(__Cache__::_{raw_rel_name}(self)?.map(|v| Box::new(v) as Box<dyn _model_::{class_mod_var}::{class}>))
    }", "") }@
@{- def.relations_belonging_outer_db(true)|fmt_rel_outer_db_join("
    #[allow(clippy::question_mark)]
    fn {rel_name}(&self) -> anyhow::Result<Option<Box<dyn _{db_snake}_model_::{class_mod_var}::{class}>>> {
        Ok(__Cache__::_{raw_rel_name}(self)?.map(|v| Box::new(v) as Box<dyn _{db_snake}_model_::{class_mod_var}::{class}>))
    }", "") }@
}
@%- endif %@

impl @{ pascal_name }@ for _@{ pascal_name }@ {
@{- def.non_cache_cols_wo_primaries_and_invisibles()|fmt_join("
    fn {var}(&self) -> {domain_outer} {
        __Getter__::_{raw_var}(self){convert_impl_domain_outer}
    }", "") }@
@{- def.relations_one_and_belonging(true)|fmt_rel_join("
    #[allow(clippy::question_mark)]
    fn {rel_name}(&self) -> anyhow::Result<Option<&dyn _model_::{class_mod_var}::{class}>> {
        Ok(__Getter__::_{raw_rel_name}(self)?.map(|v| v as &dyn _model_::{class_mod_var}::{class}))
    }", "") }@
@{- def.relations_many(true)|fmt_rel_join("
    #[allow(clippy::question_mark)]
    fn {rel_name}(&self) -> anyhow::Result<Box<dyn Iterator<Item = &dyn _model_::{class_mod_var}::{class}> + '_>> {
        Ok(Box::new(__Getter__::_{raw_rel_name}(self)?.iter().map(|v| v as &dyn _model_::{class_mod_var}::{class})))
    }", "") }@
@{- def.relations_belonging_outer_db(true)|fmt_rel_outer_db_join("
    #[allow(clippy::question_mark)]
    fn {rel_name}(&self) -> anyhow::Result<Option<&dyn _{db_snake}_model_::{class_mod_var}::{class}>> {
        Ok(__Getter__::_{raw_rel_name}(self)?.map(|v| v as &dyn _{db_snake}_model_::{class_mod_var}::{class}))
    }", "") }@
}

#[allow(clippy::useless_conversion)]
impl @{ pascal_name }@Updater for _@{ pascal_name }@Updater {
@{- def.non_cache_cols_wo_primaries_and_invisibles()|fmt_join("
    fn {var}(&self) -> {domain_outer} {
        {convert_impl_domain_outer_for_updater}
    }", "") }@
@{- def.non_primaries_wo_invisible_and_read_only(true)|fmt_join("
    fn set_{raw_var}(&mut self, v: {domain_factory}) {
        __Updater__::mut_{raw_var}(self).set(v{convert_domain_inner_type})
    }", "") }@
@{- def.relations_one(true)|fmt_rel_join("
    fn {rel_name}(&mut self) -> anyhow::Result<Option<&mut dyn _model_::{class_mod_var}::{class}Updater>> {
        Ok(self.{rel_name}.as_mut().context(\"{raw_rel_name} is not joined.\")?.iter_mut().last().filter(|v| !v.will_be_deleted() && !v.has_been_deleted()).map(|v| v as &mut dyn _model_::{class_mod_var}::{class}Updater))
    }
    fn set_{raw_rel_name}(&mut self, v: Box<dyn _model_::{class_mod_var}::{class}Updater>) {
        __Updater__::mut_{raw_rel_name}(self).set(
            if let Ok(v) = v.downcast::<crate::models::{group_var}::{mod_var}::_{class}Updater>() {
                *v
            } else {
                panic!(\"Only _{class}Updater is accepted.\");
            }
        )
    }", "") }@
@{- def.relations_many(true)|fmt_rel_join("
    fn {rel_name}(&mut self) -> anyhow::Result<Box<dyn domain::models::UpdateIterator<dyn _model_::{class_mod_var}::{class}Updater> + '_>> {
        struct V<'a, T: crate::misc::Updater>(crate::accessor::AccessorHasMany<'a, T>);
        impl<T: crate::misc::Updater + _model_::{class_mod_var}::{class}Updater> domain::models::UpdateIterator<dyn _model_::{class_mod_var}::{class}Updater> for V<'_, T> {
            fn iter_mut(&mut self) -> Box<dyn Iterator<Item = &mut (dyn _model_::{class_mod_var}::{class}Updater + 'static)> + '_> {
                Box::new(self.0.iter_mut().map(|v| v as &mut dyn _model_::{class_mod_var}::{class}Updater))
            }
        }
        self.{rel_name}.as_ref().context(\"{raw_rel_name} is not joined.\")?;
        Ok(Box::new(V(__Updater__::mut_{raw_rel_name}(self))))
    }
    fn take_{raw_rel_name}(&mut self) -> Option<Vec<Box<dyn _model_::{class_mod_var}::{class}Updater>>> {
        __Updater__::mut_{raw_rel_name}(self).take().map(|v| v.into_iter().map(|v| Box::new(v) as Box<dyn _model_::{class_mod_var}::{class}Updater>).collect())
    }
    fn replace_{raw_rel_name}(&mut self, list: Vec<Box<dyn _model_::{class_mod_var}::{class}Updater>>) {
        let mut vec = Vec::new();
        for row in list {
            match row.downcast::<crate::models::{group_var}::{mod_var}::_{class}Updater>() {
                Ok(v) => { vec.push(*v); }
                Err(_) => panic!(\"Only _{class}Updater is accepted.\"),
            }
        }
        __Updater__::mut_{raw_rel_name}(self).replace(vec);
    }
    fn push_{raw_rel_name}(&mut self, v: Box<dyn _model_::{class_mod_var}::{class}Updater>) {
        if let Ok(v) = v.downcast::<crate::models::{group_var}::{mod_var}::_{class}Updater>() {
            __Updater__::mut_{raw_rel_name}(self).push(*v)
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
@{-"\n"}@