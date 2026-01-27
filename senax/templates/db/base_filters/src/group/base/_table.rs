use ::ahash::AHasher;
use ::core::option::Option;
use ::fxhash::FxHasher64;
use ::log::{debug, error, info, warn};
use ::senax_common::cache::db_cache::HashVal;
use ::senax_common::ShardId;
use ::serde::Serialize;
use ::serde_json::Value;
use ::sqlx::query::Query;
use ::std::boxed::Box;
use ::std::collections::BTreeMap;
use ::std::fmt::Write;
use ::std::hash::{Hash, Hasher};
use ::std::vec::Vec;
use ::db::connection::{DbArguments, DbConn, DbRow, DbType};
use crate::misc::{BindArrayTr, BindTr, ColRelTr, ColTr, FilterTr, OrderTr};
use ::db::misc::{BindValue, Count, Exists, Updater, Size, TrashMode, UpdaterForInner as _};
use ::db::accessor::*;
pub use ::db::models::@{ group_name|snake|ident }@::@{ mod_name|ident }@::*;
@%- if !config.exclude_from_domain %@
#[allow(unused_imports)]
use ::domain::value_objects;
pub use ::domain::repository::@{ db|snake|ident }@::@{ group_name|snake|ident }@::@{ mod_name|ident }@::{join, Joiner_};
@%- for (name, rel_def) in def.belongs_to_outer_db(Joinable::Filter) %@
use ::db_@{ rel_def.db()|snake }@::models::@{ rel_def.get_group_mod_path() }@ as rel_@{ rel_def.get_group_mod_name() }@;
use _repo_@{ rel_def.db()|snake }@_@{ rel_def.get_group_name()|snake }@::repositories::@{ rel_def.get_base_group_mod_path() }@ as repo_@{ rel_def.get_group_mod_name() }@;
@%- endfor %@
@%- endif %@
@%- for mod_name in def.relation_mods(Joinable::Filter) %@
use ::db::models::@{ mod_name[0]|ident }@::@{ mod_name[1]|ident }@ as rel_@{ mod_name[0] }@_@{ mod_name[1] }@;
@%- if unified_group.is_ref(mod_name) %@
use _base_filter_@{ db|snake }@_@{ UnifiedGroup::unified_name_from_rel(unified_groups, mod_name) }@::repositories::@{ mod_name[0]|ident }@::_base::_@{ mod_name[1] }@ as repo_@{ mod_name[0] }@_@{ mod_name[1] }@;
@%- else %@
use crate::repositories::@{ mod_name[0]|ident }@::_base::_@{ mod_name[1] }@ as repo_@{ mod_name[0] }@_@{ mod_name[1] }@;
@%- endif %@
@%- endfor %@
pub const USE_CACHE: bool = @{ def.use_cache() }@;
pub const ENABLE_ALL_ROWS_CACHE: bool = @{ def.enable_all_rows_cache() }@;
pub const ENABLE_UPDATE_NOTICE: bool = @{ def.enable_update_notice() }@;
pub const TABLE_NAME: &str = "@{ table_name }@";
pub const TRASHED_SQL: &str = r#"@{ def.inheritance_cond(" AND ") }@"#;
pub const NOT_TRASHED_SQL: &str = r#"@{ def.soft_delete_tpl("","deleted_at IS NULL AND ","deleted = 0 AND ")}@@{ def.inheritance_cond(" AND ") }@"#;
pub const ONLY_TRASHED_SQL: &str = r#"@{ def.soft_delete_tpl("","deleted_at IS NOT NULL AND ","deleted != 0 AND ")}@@{ def.inheritance_cond(" AND ") }@"#;

type __Updater__ = _@{pascal_name}@Updater;

@%- for (model, rel_name, rel) in def.relations_belonging(Joinable::Filter, false) %@

pub struct RelCol@{ rel_name|pascal }@;
impl RelCol@{ rel_name|pascal }@ {
    pub fn cols() -> &'static str {
        r#"@{ rel.get_local_cols(rel_name, def)|fmt_join_with_paren("{col_esc}", ", ") }@"#
    }
    pub fn cols_with_idx(idx: usize) -> String {
        format!(r#"@{ rel.get_local_cols(rel_name, def)|fmt_join_with_paren("_t{}.{col_esc}", ", ") }@"#, @{ rel.get_local_cols(rel_name, def)|fmt_join("idx", ", ") }@)
    }
}

pub trait RelPk@{ rel_name|pascal }@ {
    fn primary(&self) -> Option<rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Primary>;
}
impl RelPk@{ rel_name|pascal }@ for _@{ pascal_name }@ {
    fn primary(&self) -> Option<rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Primary> {
        @%- if rel.non_equijoin %@
        None
        @%- else %@
        Some(@{ rel.get_local_cols(rel_name, def)|fmt_join_with_paren("self._{raw_name}(){null_question}", ", ") }@.into())
        @%- endif %@
    }
}
impl RelPk@{ rel_name|pascal }@ for _@{ pascal_name }@Updater {
    fn primary(&self) -> Option<rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Primary> {
        @%- if rel.non_equijoin %@
        None
        @%- else %@
        Some(@{ rel.get_local_cols(rel_name, def)|fmt_join_with_paren("self._{raw_name}(){null_question}", ", ") }@.into())
        @%- endif %@
    }
}
@%- if !config.force_disable_cache %@
impl RelPk@{ rel_name|pascal }@ for _@{ pascal_name }@Cache {
    fn primary(&self) -> Option<rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Primary> {
        @%- if rel.non_equijoin %@
        None
        @%- else %@
        Some(@{ rel.get_local_cols(rel_name, def)|fmt_join_with_paren("self._{raw_name}(){null_question}", ", ") }@.into())
        @%- endif %@
    }
}
@%- endif %@
@%- endfor %@
@%- for (model, rel_name, rel) in def.relations_belonging_outer_db(Joinable::Filter, false) %@
pub struct RelCol@{ rel_name|pascal }@;
impl RelCol@{ rel_name|pascal }@ {
    pub fn cols() -> &'static str {
        r#"@{ rel.get_local_cols(rel_name, def)|fmt_join_with_paren("{col_esc}", ", ") }@"#
    }
    pub fn cols_with_idx(idx: usize) -> String {
        format!(r#"@{ rel.get_local_cols(rel_name, def)|fmt_join_with_paren("_t{}.{col_esc}", ", ") }@"#, @{ rel.get_local_cols(rel_name, def)|fmt_join("idx", ", ") }@)
    }
}

pub trait RelPk@{ rel_name|pascal }@ {
    fn primary(&self) -> Option<rel_@{ rel.db()|snake }@_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Primary>;
}
impl RelPk@{ rel_name|pascal }@ for _@{ pascal_name }@ {
    fn primary(&self) -> Option<rel_@{ rel.db()|snake }@_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Primary> {
        @%- if rel.non_equijoin %@
        None
        @%- else %@
        Some(@{ rel.get_local_cols(rel_name, def)|fmt_join_with_paren("self._{raw_name}(){null_question}", ", ") }@.into())
        @%- endif %@
    }
}
impl RelPk@{ rel_name|pascal }@ for _@{ pascal_name }@Updater {
    fn primary(&self) -> Option<rel_@{ rel.db()|snake }@_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Primary> {
        @%- if rel.non_equijoin %@
        None
        @%- else %@
        Some(@{ rel.get_local_cols(rel_name, def)|fmt_join_with_paren("self._{raw_name}(){null_question}", ", ") }@.into())
        @%- endif %@
    }
}
@%- if !config.force_disable_cache %@
impl RelPk@{ rel_name|pascal }@ for _@{ pascal_name }@Cache {
    fn primary(&self) -> Option<rel_@{ rel.db()|snake }@_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Primary> {
        @%- if rel.non_equijoin %@
        None
        @%- else %@
        Some(@{ rel.get_local_cols(rel_name, def)|fmt_join_with_paren("self._{raw_name}(){null_question}", ", ") }@.into())
        @%- endif %@
    }
}
@%- endif %@
@%- endfor %@
@%- for (model, rel_name, rel) in def.relations_one(Joinable::Filter, false) %@
pub struct RelCol@{ rel_name|pascal }@;
impl RelCol@{ rel_name|pascal }@ {
    pub fn cols() -> &'static str {
        r#"@{ rel.get_foreign_cols(def)|fmt_join_foreign("{col_esc}", ", ") }@"#
    }
    pub fn cols_with_paren() -> &'static str {
        r#"@{ rel.get_foreign_cols(def)|fmt_join_foreign_with_paren("{col_esc}", ", ") }@"#
    }
    pub fn set_op_none(op: &mut rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::OpData) {
        @{- rel.get_foreign_cols(def)|fmt_join_foreign("
        op.{ident} = Op::None;", "") }@
    }
}

pub trait RelFil@{ rel_name|pascal }@ where Self: Sized {
    fn filter(&self) -> repo_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Filter_;
    fn in_filter(list: &[Self]) -> repo_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Filter_;
}
impl RelFil@{ rel_name|pascal }@ for _@{ pascal_name }@ {
    fn filter(&self) -> repo_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Filter_ {
        use repo_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@ as repo;
        let pk: Primary = self.into();
        repo::Filter_::new_and()
        @{- rel.get_foreign_cols(def)|fmt_join_foreign("
            .and(repo::Filter_::Eq(repo::ColOne_::{ident}(pk.{index}.inner().into())))", "") }@
    }
    fn in_filter(list: &[Self]) -> repo_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Filter_ {
        use repo_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@ as repo;
        @%- if rel.get_foreign_cols(def).len() == 1 %@
        let mut vec = Vec::new();
        for row in list {
            let pk: Primary = row.into();
            vec.push(pk.0.inner().into());
        }
        @{ rel.get_foreign_cols(def)|fmt_join_foreign("repo::Filter_::In(repo::ColMany_::{ident}(vec))", "") }@
        @%- else %@
        let mut filter = repo::Filter_::new_or();
        for row in list {
            let pk: Primary = row.into();
            filter = filter.or(repo::Filter_::new_and()
            @{- rel.get_foreign_cols(def)|fmt_join_foreign("
                .and(repo::Filter_::Eq(repo::ColOne_::{ident}(pk.{index}.inner().into())))", "") }@);
        }
        filter
        @%- endif %@
    }
}
impl RelFil@{ rel_name|pascal }@ for _@{ pascal_name }@Updater {
    fn filter(&self) -> repo_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Filter_ {
        use repo_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@ as repo;
        let pk: Primary = self.into();
        repo::Filter_::new_and()
        @{- rel.get_foreign_cols(def)|fmt_join_foreign("
            .and(repo::Filter_::Eq(repo::ColOne_::{ident}(pk.{index}.inner().into())))", "") }@
    }
    fn in_filter(list: &[Self]) -> repo_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Filter_ {
        use repo_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@ as repo;
        @%- if rel.get_foreign_cols(def).len() == 1 %@
        let mut vec = Vec::new();
        for row in list {
            let pk: Primary = row.into();
            vec.push(pk.0.inner().into());
        }
        @{ rel.get_foreign_cols(def)|fmt_join_foreign("repo::Filter_::In(repo::ColMany_::{ident}(vec))", "") }@
        @%- else %@
        let mut filter = repo::Filter_::new_or();
        for row in list {
            let pk: Primary = row.into();
            filter = filter.or(repo::Filter_::new_and()
            @{- rel.get_foreign_cols(def)|fmt_join_foreign("
                .and(repo::Filter_::Eq(repo::ColOne_::{ident}(pk.{index}.inner().into())))", "") }@);
        }
        filter
        @%- endif %@
    }
}
impl RelFil@{ rel_name|pascal }@ for &ForInsert {
    fn filter(&self) -> repo_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Filter_ {
        use repo_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@ as repo;
        let pk: Primary = (&self._data).into();
        repo::Filter_::new_and()
        @{- rel.get_foreign_cols(def)|fmt_join_foreign("
            .and(repo::Filter_::Eq(repo::ColOne_::{ident}(pk.{index}.inner().into())))", "") }@
    }
    fn in_filter(list: &[Self]) -> repo_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Filter_ {
        use repo_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@ as repo;
        @%- if rel.get_foreign_cols(def).len() == 1 %@
        let mut vec = Vec::new();
        for row in list {
            let pk: Primary = (&row._data).into();
            vec.push(pk.0.inner().into());
        }
        @{ rel.get_foreign_cols(def)|fmt_join_foreign("repo::Filter_::In(repo::ColMany_::{ident}(vec))", "") }@
        @%- else %@
        let mut filter = repo::Filter_::new_or();
        for row in list {
            let pk: Primary = (&row._data).into();
            filter = filter.or(repo::Filter_::new_and()
            @{- rel.get_foreign_cols(def)|fmt_join_foreign("
                .and(repo::Filter_::Eq(repo::ColOne_::{ident}(pk.{index}.inner().into())))", "") }@);
        }
        filter
        @%- endif %@
    }
}
@%- if !config.force_disable_cache %@
impl RelFil@{ rel_name|pascal }@ for CacheWrapper {
    fn filter(&self) -> repo_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Filter_ {
        use repo_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@ as repo;
        let pk: Primary = self.into();
        repo::Filter_::new_and()
        @{- rel.get_foreign_cols(def)|fmt_join_foreign("
            .and(repo::Filter_::Eq(repo::ColOne_::{ident}(pk.{index}.inner().into())))", "") }@
    }
    fn in_filter(list: &[Self]) -> repo_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Filter_ {
        use repo_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@ as repo;
        @%- if rel.get_foreign_cols(def).len() == 1 %@
        let mut vec = Vec::new();
        for row in list {
            let pk: Primary = row.into();
            vec.push(pk.0.inner().into());
        }
        @{ rel.get_foreign_cols(def)|fmt_join_foreign("repo::Filter_::In(repo::ColMany_::{ident}(vec))", "") }@
        @%- else %@
        let mut filter = repo::Filter_::new_or();
        for row in list {
            let pk: Primary = row.into();
            filter = filter.or(repo::Filter_::new_and()
            @{- rel.get_foreign_cols(def)|fmt_join_foreign("
                .and(repo::Filter_::Eq(repo::ColOne_::{ident}(pk.{index}.inner().into())))", "") }@);
        }
        filter
        @%- endif %@
    }
}
impl RelFil@{ rel_name|pascal }@ for _@{ pascal_name }@Cache {
    fn filter(&self) -> repo_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Filter_ {
        use repo_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@ as repo;
        let pk: Primary = self.into();
        repo::Filter_::new_and()
        @{- rel.get_foreign_cols(def)|fmt_join_foreign("
            .and(repo::Filter_::Eq(repo::ColOne_::{ident}(pk.{index}.inner().into())))", "") }@
    }
    fn in_filter(list: &[Self]) -> repo_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Filter_ {
        use repo_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@ as repo;
        @%- if rel.get_foreign_cols(def).len() == 1 %@
        let mut vec = Vec::new();
        for row in list {
            let pk: Primary = row.into();
            vec.push(pk.0.inner().into());
        }
        @{ rel.get_foreign_cols(def)|fmt_join_foreign("repo::Filter_::In(repo::ColMany_::{ident}(vec))", "") }@
        @%- else %@
        let mut filter = repo::Filter_::new_or();
        for row in list {
            let pk: Primary = row.into();
            filter = filter.or(repo::Filter_::new_and()
            @{- rel.get_foreign_cols(def)|fmt_join_foreign("
                .and(repo::Filter_::Eq(repo::ColOne_::{ident}(pk.{index}.inner().into())))", "") }@);
        }
        filter
        @%- endif %@
    }
}
@%- endif %@
pub trait RelFk@{ rel_name|pascal }@ {
    fn get_fk(&self) -> Option<Primary>;
    fn set_fk(&mut self, pk: InnerPrimary);
}
impl RelFk@{ rel_name|pascal }@ for rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Data {
    fn get_fk(&self) -> Option<Primary> {
        @%- if rel.non_equijoin %@
        None
        @%- else %@
        Some(@{ rel.get_foreign_cols(def)|fmt_join_foreign_with_paren("self.{raw_name}{null_question}{clone}", ", ") }@.into())
        @%- endif %@
    }
    fn set_fk(&mut self, pk: InnerPrimary) {
        @{- rel.get_foreign_cols(def)|fmt_join_foreign_not_null_or_null("
        self.{raw_name} = pk.{index}{raw_to_inner};", "
        self.{raw_name} = Some(pk.{index}{raw_to_inner});", "") }@
    }
}
@%- if !config.force_disable_cache %@
impl RelFk@{ rel_name|pascal }@ for rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::CacheData {
    fn get_fk(&self) -> Option<Primary> {
        @%- if rel.non_equijoin %@
        None
        @%- else %@
        Some(@{ rel.get_foreign_cols(def)|fmt_join_foreign_with_paren("self.{raw_name}{null_question}{clone}", ", ") }@.into())
        @%- endif %@
    }
    fn set_fk(&mut self, pk: InnerPrimary) {
        @{- rel.get_foreign_cols(def)|fmt_join_foreign_not_null_or_null("
        self.{raw_name} = pk.{index}{raw_to_inner};", "
        self.{raw_name} = Some(pk.{index}{raw_to_inner});", "") }@
    }
}
@%- endif %@
impl RelFk@{ rel_name|pascal }@ for rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::ForInsert {
    fn get_fk(&self) -> Option<Primary> {
        @%- if rel.non_equijoin %@
        None
        @%- else %@
        Some(@{ rel.get_foreign_cols(def)|fmt_join_foreign_with_paren("self._data.{raw_name}{null_question}{clone}", ", ") }@.into())
        @%- endif %@
    }
    fn set_fk(&mut self, pk: InnerPrimary) {
        @{- rel.get_foreign_cols(def)|fmt_join_foreign_not_null_or_null("
        self._data.{raw_name} = pk.{index}{raw_to_inner};", "
        self._data.{raw_name} = Some(pk.{index}{raw_to_inner});", "") }@
    }
}
@%- endfor %@
@%- for (model, rel_name, rel) in def.relations_many(Joinable::Filter, false) %@
pub struct RelCol@{ rel_name|pascal }@;
impl RelCol@{ rel_name|pascal }@ {
    pub fn cols() -> &'static str {
        r#"@{ rel.get_foreign_cols(def)|fmt_join_foreign("{col_esc}", ", ") }@"#
    }
    pub fn cols_with_paren() -> &'static str {
        r#"@{ rel.get_foreign_cols(def)|fmt_join_foreign_with_paren("{col_esc}", ", ") }@"#
    }
    pub fn set_op_none(op: &mut rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::OpData) {
        @{- rel.get_foreign_cols(def)|fmt_join_foreign("
        op.{ident} = Op::None;", "") }@
    }
}

pub trait RelFil@{ rel_name|pascal }@ where Self: Sized {
    fn filter(&self) -> repo_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Filter_;
    fn in_filter(list: &[Self]) -> repo_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Filter_;
}
impl RelFil@{ rel_name|pascal }@ for _@{ pascal_name }@ {
    fn filter(&self) -> repo_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Filter_ {
        use repo_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@ as repo;
        let pk: Primary = self.into();
        repo::Filter_::new_and()
        @{- rel.get_foreign_cols(def)|fmt_join_foreign("
            .and(repo::Filter_::Eq(repo::ColOne_::{ident}(pk.{index}.inner().into())))", "") }@
    }
    fn in_filter(list: &[Self]) -> repo_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Filter_ {
        use repo_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@ as repo;
        @%- if rel.get_foreign_cols(def).len() == 1 %@
        let mut vec = Vec::new();
        for row in list {
            let pk: Primary = row.into();
            vec.push(pk.0.inner().into());
        }
        @{ rel.get_foreign_cols(def)|fmt_join_foreign("repo::Filter_::In(repo::ColMany_::{ident}(vec))", "") }@
        @%- else %@
        let mut filter = repo::Filter_::new_or();
        for row in list {
            let pk: Primary = row.into();
            filter = filter.or(repo::Filter_::new_and()
            @{- rel.get_foreign_cols(def)|fmt_join_foreign("
                .and(repo::Filter_::Eq(repo::ColOne_::{ident}(pk.{index}.inner().into())))", "") }@);
        }
        filter
        @%- endif %@
    }
}
impl RelFil@{ rel_name|pascal }@ for _@{ pascal_name }@Updater {
    fn filter(&self) -> repo_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Filter_ {
        use repo_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@ as repo;
        let pk: Primary = self.into();
        repo::Filter_::new_and()
        @{- rel.get_foreign_cols(def)|fmt_join_foreign("
            .and(repo::Filter_::Eq(repo::ColOne_::{ident}(pk.{index}.inner().into())))", "") }@
    }
    fn in_filter(list: &[Self]) -> repo_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Filter_ {
        use repo_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@ as repo;
        @%- if rel.get_foreign_cols(def).len() == 1 %@
        let mut vec = Vec::new();
        for row in list {
            let pk: Primary = row.into();
            vec.push(pk.0.inner().into());
        }
        @{ rel.get_foreign_cols(def)|fmt_join_foreign("repo::Filter_::In(repo::ColMany_::{ident}(vec))", "") }@
        @%- else %@
        let mut filter = repo::Filter_::new_or();
        for row in list {
            let pk: Primary = row.into();
            filter = filter.or(repo::Filter_::new_and()
            @{- rel.get_foreign_cols(def)|fmt_join_foreign("
                .and(repo::Filter_::Eq(repo::ColOne_::{ident}(pk.{index}.inner().into())))", "") }@);
        }
        filter
        @%- endif %@
    }
}
impl RelFil@{ rel_name|pascal }@ for &ForInsert {
    fn filter(&self) -> repo_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Filter_ {
        use repo_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@ as repo;
        let pk: Primary = (&self._data).into();
        repo::Filter_::new_and()
        @{- rel.get_foreign_cols(def)|fmt_join_foreign("
            .and(repo::Filter_::Eq(repo::ColOne_::{ident}(pk.{index}.inner().into())))", "") }@
    }
    fn in_filter(list: &[Self]) -> repo_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Filter_ {
        use repo_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@ as repo;
        @%- if rel.get_foreign_cols(def).len() == 1 %@
        let mut vec = Vec::new();
        for row in list {
            let pk: Primary = (&row._data).into();
            vec.push(pk.0.inner().into());
        }
        @{ rel.get_foreign_cols(def)|fmt_join_foreign("repo::Filter_::In(repo::ColMany_::{ident}(vec))", "") }@
        @%- else %@
        let mut filter = repo::Filter_::new_or();
        for row in list {
            let pk: Primary = (&row._data).into();
            filter = filter.or(repo::Filter_::new_and()
            @{- rel.get_foreign_cols(def)|fmt_join_foreign("
                .and(repo::Filter_::Eq(repo::ColOne_::{ident}(pk.{index}.inner().into())))", "") }@);
        }
        filter
        @%- endif %@
    }
}
@%- if !config.force_disable_cache %@
impl RelFil@{ rel_name|pascal }@ for CacheWrapper {
    fn filter(&self) -> repo_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Filter_ {
        use repo_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@ as repo;
        let pk: Primary = self.into();
        repo::Filter_::new_and()
        @{- rel.get_foreign_cols(def)|fmt_join_foreign("
            .and(repo::Filter_::Eq(repo::ColOne_::{ident}(pk.{index}.inner().into())))", "") }@
    }
    fn in_filter(list: &[Self]) -> repo_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Filter_ {
        use repo_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@ as repo;
        @%- if rel.get_foreign_cols(def).len() == 1 %@
        let mut vec = Vec::new();
        for row in list {
            let pk: Primary = row.into();
            vec.push(pk.0.inner().into());
        }
        @{ rel.get_foreign_cols(def)|fmt_join_foreign("repo::Filter_::In(repo::ColMany_::{ident}(vec))", "") }@
        @%- else %@
        let mut filter = repo::Filter_::new_or();
        for row in list {
            let pk: Primary = row.into();
            filter = filter.or(repo::Filter_::new_and()
            @{- rel.get_foreign_cols(def)|fmt_join_foreign("
                .and(repo::Filter_::Eq(repo::ColOne_::{ident}(pk.{index}.inner().into())))", "") }@);
        }
        filter
        @%- endif %@
    }
}
impl RelFil@{ rel_name|pascal }@ for _@{ pascal_name }@Cache {
    fn filter(&self) -> repo_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Filter_ {
        use repo_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@ as repo;
        let pk: Primary = self.into();
        repo::Filter_::new_and()
        @{- rel.get_foreign_cols(def)|fmt_join_foreign("
            .and(repo::Filter_::Eq(repo::ColOne_::{ident}(pk.{index}.inner().into())))", "") }@
    }
    fn in_filter(list: &[Self]) -> repo_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Filter_ {
        use repo_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@ as repo;
        @%- if rel.get_foreign_cols(def).len() == 1 %@
        let mut vec = Vec::new();
        for row in list {
            let pk: Primary = row.into();
            vec.push(pk.0.inner().into());
        }
        @{ rel.get_foreign_cols(def)|fmt_join_foreign("repo::Filter_::In(repo::ColMany_::{ident}(vec))", "") }@
        @%- else %@
        let mut filter = repo::Filter_::new_or();
        for row in list {
            let pk: Primary = row.into();
            filter = filter.or(repo::Filter_::new_and()
            @{- rel.get_foreign_cols(def)|fmt_join_foreign("
                .and(repo::Filter_::Eq(repo::ColOne_::{ident}(pk.{index}.inner().into())))", "") }@);
        }
        filter
        @%- endif %@
    }
}
@%- endif %@
pub trait RelFk@{ rel_name|pascal }@ {
    fn get_fk(&self) -> Option<Primary>;
    fn set_fk(&mut self, pk: InnerPrimary);
}
impl RelFk@{ rel_name|pascal }@ for rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::Data {
    fn get_fk(&self) -> Option<Primary> {
        @%- if rel.non_equijoin %@
        None
        @%- else %@
        Some(@{ rel.get_foreign_cols(def)|fmt_join_foreign_with_paren("self.{raw_name}{null_question}{clone}", ", ") }@.into())
        @%- endif %@
    }
    fn set_fk(&mut self, pk: InnerPrimary) {
        @{- rel.get_foreign_cols(def)|fmt_join_foreign_not_null_or_null("
        self.{raw_name} = pk.{index}{raw_to_inner};", "
        self.{raw_name} = Some(pk.{index}{raw_to_inner});", "") }@
    }
}
@%- if !config.force_disable_cache %@
impl RelFk@{ rel_name|pascal }@ for rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::CacheData {
    fn get_fk(&self) -> Option<Primary> {
        @%- if rel.non_equijoin %@
        None
        @%- else %@
        Some(@{ rel.get_foreign_cols(def)|fmt_join_foreign_with_paren("self.{raw_name}{null_question}{clone}", ", ") }@.into())
        @%- endif %@
    }
    fn set_fk(&mut self, pk: InnerPrimary) {
        @{- rel.get_foreign_cols(def)|fmt_join_foreign_not_null_or_null("
        self.{raw_name} = pk.{index}{raw_to_inner};", "
        self.{raw_name} = Some(pk.{index}{raw_to_inner});", "") }@
    }
}
@%- endif %@
impl RelFk@{ rel_name|pascal }@ for rel_@{ rel.get_group_name()|snake }@_@{ rel.get_mod_name() }@::ForInsert {
    fn get_fk(&self) -> Option<Primary> {
        @%- if rel.non_equijoin %@
        None
        @%- else %@
        Some(@{ rel.get_foreign_cols(def)|fmt_join_foreign_with_paren("self._data.{raw_name}{null_question}{clone}", ", ") }@.into())
        @%- endif %@
    }
    fn set_fk(&mut self, pk: InnerPrimary) {
        @{- rel.get_foreign_cols(def)|fmt_join_foreign_not_null_or_null("
        self._data.{raw_name} = pk.{index}{raw_to_inner};", "
        self._data.{raw_name} = Some(pk.{index}{raw_to_inner});", "") }@
    }
}
@%- endfor %@

@%- if config.exclude_from_domain %@
@%- for (index_name, index) in def.multi_index(false) %@

#[allow(non_camel_case_types)]
#[derive(PartialEq, Debug, Clone)]
pub struct _@{ pascal_name }@Index_@{ index_name }@(@{ index.join_fields(def, "pub {filter_type}", ", ") }@);
impl<@{ index.join_fields(def, "T{index}", ", ") }@> TryFrom<(@{ index.join_fields(def, "T{index}", ", ") }@)> for _@{ pascal_name }@Index_@{ index_name }@
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
@{ def.all_fields()|fmt_join("    {ident},", "\n") }@
}
#[allow(unreachable_patterns)]
#[allow(clippy::match_single_binding)]
impl Col_ {
    pub fn _name(&self) -> &'static str {
        match self {
            @{- def.all_fields()|fmt_join("
            Col_::{ident} => \"{col}\",", "") }@
            _ => unimplemented!(),
        }
    }
}
@%- else %@
pub use domain::repository::@{ db|snake|ident }@::@{ group_name|snake|ident }@::_base::_@{ mod_name }@::Col_;
@%- endif %@
impl ColTr for Col_ {
    fn name(&self) -> &'static str {
        match self {
@{ def.all_fields()|fmt_join("            Col_::{ident} => r#\"{col_esc}\"#,", "\n") }@
        }
    }
}
@%- if config.exclude_from_domain %@

#[allow(non_camel_case_types)]
#[derive(Clone, Debug)]
pub enum ColOne_ {
@{ def.all_fields_except_json()|fmt_join("    {ident}({filter_type}),", "\n") }@
@%- for (index_name, index) in def.multi_index(false) %@
    @{ index.join_fields(def, "{name}", "_") }@(_@{ pascal_name }@Index_@{ index_name }@),
@%- endfor %@
}
#[allow(unreachable_patterns)]
#[allow(clippy::match_single_binding)]
impl ColOne_ {
    fn _name(&self) -> &'static str {
        match self {
            @{- def.all_fields_except_json()|fmt_join("
            ColOne_::{ident}(_) => \"{col}\",", "") }@
            @%- for (index_name, index) in def.multi_index(false) %@
            ColOne_::@{ index.join_fields(def, "{name}", "_") }@(_) => "<@{ index.join_fields(def, "{name}", ", ") }@>",
            @%- endfor %@
            _ => unimplemented!(),
        }
    }
}
@%- else %@
pub use domain::repository::@{ db|snake|ident }@::@{ group_name|snake|ident }@::_base::_@{ mod_name }@::ColOne_;
@%- endif %@
#[allow(clippy::match_single_binding)]
impl BindTr for ColOne_ {
    fn name(&self) -> &'static str {
        match self {
@{ def.all_fields_except_json()|fmt_join("            ColOne_::{ident}(_) => r#\"{col_esc}\"#,", "\n") }@
@%- for (index_name, index) in def.multi_index(false) %@
            ColOne_::@{ index.join_fields(def, "{name}", "_") }@(_) => r#"(@{ index.join_fields(def, "{col_esc}", ", ") }@)"#,
@%- endfor %@
            _ => unreachable!(),
        }
    }
    fn placeholder(&self) -> &'static str {
        match self {
@{ def.all_fields_except_json()|fmt_join("            ColOne_::{ident}(_) => \"{placeholder}\",", "\n") }@
@%- for (index_name, index) in def.multi_index(false) %@
            ColOne_::@{ index.join_fields(def, "{name}", "_") }@(_) => "(@{ index.join_fields(def, "{placeholder}", ", ") }@)",
@%- endfor %@
            _ => "?",
        }
    }
    fn bind_to_query(
        self,
        query: Query<'_, DbType, DbArguments>,
    ) -> Query<'_, DbType, DbArguments> {
        debug!("bind: {:?}", &self);
        match self {
@{ def.all_fields_except_json()|fmt_join("            ColOne_::{ident}(v) => query.bind(v{bind_as_for_filter}),", "\n") }@
@%- for (index_name, index) in def.multi_index(false) %@
            ColOne_::@{ index.join_fields(def, "{name}", "_") }@(v) => query@{ index.join_fields(def, ".bind(v.{index}{bind_as_for_filter})", "") }@,
@%- endfor %@
            _ => unreachable!(),
        }
    }
}
@%- if config.exclude_from_domain %@

#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Hash, Serialize)]
pub enum ColKey_ {
    @{- def.unique_key()|fmt_index_col("
    {ident}({filter_type}),", "") }@
}
#[allow(unreachable_patterns)]
#[allow(clippy::match_single_binding)]
impl ColKey_ {
    fn _name(&self) -> &'static str {
        match self {
            @{- def.unique_key()|fmt_join("
            ColKey_::{ident}(_) => \"{col}\",", "") }@
            _ => unimplemented!(),
        }
    }
}
@%- else %@
pub use domain::repository::@{ db|snake|ident }@::@{ group_name|snake|ident }@::_base::_@{ mod_name }@::ColKey_;
@%- endif %@
#[allow(clippy::match_single_binding)]
impl BindTr for ColKey_ {
    fn name(&self) -> &'static str {
        match self {
            @{- def.unique_key()|fmt_index_col("
            ColKey_::{ident}(_v) => r#\"{col_esc}\"#,", "") }@
            _ => unreachable!(),
        }
    }
    fn bind_to_query(
        self,
        query: Query<'_, DbType, DbArguments>,
    ) -> Query<'_, DbType, DbArguments> {
        debug!("bind: {:?}", &self);
        match self {
            @{- def.unique_key()|fmt_index_col("
            ColKey_::{ident}(v) => query.bind(v{bind_as_for_filter}),", "") }@
            _ => unreachable!(),
        }
    }
}
pub struct VecColKey(pub Vec<ColKey_>);
@%- if !config.force_disable_cache %@
impl HashVal for VecColKey {
    fn hash_val(&self, shard_id: ShardId) -> u128 {
        let mut hasher = FxHasher64::default();
        COL_KEY_TYPE_ID.hash(&mut hasher);
        shard_id.hash(&mut hasher);
        self.0.hash(&mut hasher);
        let hash = (hasher.finish() as u128) << 64;

        let mut hasher = AHasher::default();
        COL_KEY_TYPE_ID.hash(&mut hasher);
        shard_id.hash(&mut hasher);
        self.0.hash(&mut hasher);
        hash | (hasher.finish() as u128)
    }
}
@%- endif %@
@%- if config.exclude_from_domain %@

#[allow(non_camel_case_types)]
#[derive(Clone, Debug)]
pub enum ColMany_ {
@{ def.all_fields_except_json()|fmt_join("    {ident}(Vec<{filter_type}>),", "\n") }@
@%- for (index_name, index) in def.multi_index(false) %@
    @{ index.join_fields(def, "{name}", "_") }@(Vec<_@{ pascal_name }@Index_@{ index_name }@>),
@%- endfor %@
}
#[allow(unreachable_patterns)]
#[allow(clippy::match_single_binding)]
impl ColMany_ {
    fn _name(&self) -> &'static str {
        match self {
            @{- def.all_fields_except_json()|fmt_join("
            ColMany_::{ident}(_) => \"{col}\",", "") }@
            @%- for (index_name, index) in def.multi_index(false) %@
            ColMany_::@{ index.join_fields(def, "{name}", "_") }@(_) => "<@{ index.join_fields(def, "{name}", ", ") }@>",
            @%- endfor %@
            _ => unimplemented!(),
        }
    }
}
@%- else %@
pub use domain::repository::@{ db|snake|ident }@::@{ group_name|snake|ident }@::_base::_@{ mod_name }@::ColMany_;
@%- endif %@
#[allow(clippy::match_single_binding)]
impl BindTr for ColMany_ {
    fn name(&self) -> &'static str {
        match self {
@{ def.all_fields_except_json()|fmt_join("            ColMany_::{ident}(_) => r#\"{col_esc}\"#,", "\n") }@
@%- for (index_name, index) in def.multi_index(false) %@
            ColMany_::@{ index.join_fields(def, "{name}", "_") }@(_v) => r#"(@{ index.join_fields(def, "{col_esc}", ", ") }@)"#,
@%- endfor %@
            _ => unreachable!(),
        }
    }
    fn placeholder(&self) -> &'static str {
        match self {
@{ def.all_fields_except_json()|fmt_join("            ColMany_::{ident}(_) => \"{placeholder}\",", "\n") }@
@%- for (index_name, index) in def.multi_index(false) %@
            ColMany_::@{ index.join_fields(def, "{name}", "_") }@(_v) => "(@{ index.join_fields(def, "{placeholder}", ", ") }@)",
@%- endfor %@
            _ => "?",
        }
    }
    fn len(&self) -> usize {
        match self {
@{ def.all_fields_except_json()|fmt_join("            ColMany_::{ident}(v) => v.len(),", "\n") }@
@%- for (index_name, index) in def.multi_index(false) %@
            ColMany_::@{ index.join_fields(def, "{name}", "_") }@(v) => v.len(),
@%- endfor %@
            _ => unreachable!(),
        }
    }
    fn bind_to_query(
        self,
        mut query: Query<'_, DbType, DbArguments>,
    ) -> Query<'_, DbType, DbArguments> {
        debug!("bind: {:?}", &self);
        match self {
@{ def.all_fields_except_json()|fmt_join("            ColMany_::{ident}(v) => {for v in v { query = query.bind(v{bind_as_for_filter}); } query},", "\n") }@
@%- for (index_name, index) in def.multi_index(false) %@
            ColMany_::@{ index.join_fields(def, "{name}", "_") }@(v) => {for v in v { query = query@{ index.join_fields(def, ".bind(v.{index}{bind_as_for_filter})", "") }@; } query},
@%- endfor %@
            _ => unreachable!(),
        }
    }
}
@%- if config.exclude_from_domain %@

#[allow(non_camel_case_types)]
#[derive(Clone, Debug)]
pub enum ColJson_ {
@{- def.all_fields_only_json()|fmt_join("
    {ident}(Value),", "") }@
}
#[allow(unreachable_patterns)]
#[allow(clippy::match_single_binding)]
impl ColJson_ {
    fn _name(&self) -> &'static str {
        match self {
            @{- def.all_fields_only_json()|fmt_join("
            ColJson_::{ident}(_) => \"{col}\",", "") }@
            _ => unimplemented!(),
        }
    }
}
@%- else %@
pub use domain::repository::@{ db|snake|ident }@::@{ group_name|snake|ident }@::_base::_@{ mod_name }@::ColJson_;
@%- endif %@
#[allow(clippy::match_single_binding)]
impl BindTr for ColJson_ {
    fn name(&self) -> &'static str {
        match self {
@{- def.all_fields_only_json()|fmt_join("
            ColJson_::{ident}(_v) => r#\"{col_esc}\"#,", "") }@
            _ => unreachable!(),
        }
    }
    fn bind_to_query(
        self,
        query: Query<'_, DbType, DbArguments>,
    ) -> Query<'_, DbType, DbArguments> {
        debug!("bind: {:?}", &self);
        match self {
@{- def.all_fields_only_json()|fmt_join("
            ColJson_::{ident}(v) => query.bind(v{bind_as_for_filter}),", "") }@
            _ => unreachable!(),
        }
    }
}
@%- if config.exclude_from_domain %@

#[allow(non_camel_case_types)]
#[derive(Clone, Debug)]
pub enum ColJsonArray_ {
@{- def.all_fields_only_json()|fmt_join("
    {ident}(Vec<Value>),", "") }@
}
#[allow(unreachable_patterns)]
#[allow(clippy::match_single_binding)]
impl ColJsonArray_ {
    fn _name(&self) -> &'static str {
        match self {
            @{- def.all_fields_only_json()|fmt_join("
            ColJsonArray_::{ident}(_) => \"{col}\",", "") }@
            _ => unimplemented!(),
        }
    }
}
@%- else %@
pub use domain::repository::@{ db|snake|ident }@::@{ group_name|snake|ident }@::_base::_@{ mod_name }@::ColJsonArray_;
@%- endif %@
#[allow(clippy::match_single_binding)]
impl BindTr for ColJsonArray_ {
    fn name(&self) -> &'static str {
        match self {
@{- def.all_fields_only_json()|fmt_join("
            ColJsonArray_::{ident}(_v) => r#\"{col_esc}\"#,", "") }@
            _ => unreachable!(),
        }
    }
    fn bind_to_query(
        self,
        query: Query<'_, DbType, DbArguments>,
    ) -> Query<'_, DbType, DbArguments> {
        debug!("bind: {:?}", &self);
        match self {
@{- def.all_fields_only_json()|fmt_join("
            ColJsonArray_::{ident}(v) => query.bind(sqlx::types::Json(v{bind_as_for_filter})),", "") }@
            _ => unreachable!(),
        }
    }
}
impl BindArrayTr for ColJsonArray_ {
    fn query_each_bind(
        self,
        mut query: Query<'_, DbType, DbArguments>,
    ) -> Query<'_, DbType, DbArguments> {
        debug!("bind: {:?}", &self);
        match self {
@{- def.all_fields_only_json()|fmt_join("
            ColJsonArray_::{ident}(v) => {for v in v { query = query.bind(v{bind_as_for_filter}); } query},", "") }@
            _ => unreachable!(),
        }
    }
}
@%- if config.exclude_from_domain %@

#[allow(non_camel_case_types)]
#[derive(Clone, Debug)]
pub enum ColGeo_ {
@{- def.all_fields_only_geo()|fmt_join("
    {ident}(Value, i32),", "") }@
}
#[allow(unreachable_patterns)]
#[allow(clippy::match_single_binding)]
impl ColGeo_ {
    fn _name(&self) -> &'static str {
        match self {
            @{- def.all_fields_only_geo()|fmt_join("
            ColGeo_::{ident}(_, _) => \"{col}\",", "") }@
            _ => unimplemented!(),
        }
    }
}
@%- else %@
pub use domain::repository::@{ db|snake|ident }@::@{ group_name|snake|ident }@::_base::_@{ mod_name }@::ColGeo_;
@%- endif %@
#[allow(clippy::match_single_binding)]
impl BindTr for ColGeo_ {
    fn name(&self) -> &'static str {
        match self {
@{- def.all_fields_only_geo()|fmt_join("
            ColGeo_::{ident}(_, _) => r#\"{col_esc}\"#,", "") }@
            _ => unreachable!(),
        }
    }
    fn bind_to_query(
        self,
        query: Query<'_, DbType, DbArguments>,
    ) -> Query<'_, DbType, DbArguments> {
        debug!("bind: {:?}", &self);
        match self {
@{- def.all_fields_only_geo()|fmt_join("
            ColGeo_::{ident}(v, srid) => query.bind(v{bind_as_for_filter}).bind(srid),", "") }@
            _ => unreachable!(),
        }
    }
}
@%- if config.exclude_from_domain %@

#[allow(non_camel_case_types)]
#[derive(Clone, Debug)]
pub enum ColGeoDistance_ {
@{- def.all_fields_only_geo()|fmt_join("
    {ident}(Value, f64, i32),", "") }@
}
#[allow(unreachable_patterns)]
#[allow(clippy::match_single_binding)]
impl ColGeoDistance_ {
    fn _name(&self) -> &'static str {
        match self {
            @{- def.all_fields_only_geo()|fmt_join("
            ColGeoDistance_::{ident}(_, _, _) => \"{col}\",", "") }@
            _ => unimplemented!(),
        }
    }
}
@%- else %@
pub use domain::repository::@{ db|snake|ident }@::@{ group_name|snake|ident }@::_base::_@{ mod_name }@::ColGeoDistance_;
@%- endif %@
#[allow(clippy::match_single_binding)]
impl BindTr for ColGeoDistance_ {
    fn name(&self) -> &'static str {
        match self {
@{- def.all_fields_only_geo()|fmt_join("
            ColGeoDistance_::{ident}(_, _, _) => r#\"{col_esc}\"#,", "") }@
            _ => unreachable!(),
        }
    }
    fn bind_to_query(
        self,
        query: Query<'_, DbType, DbArguments>,
    ) -> Query<'_, DbType, DbArguments> {
        debug!("bind: {:?}", &self);
        match self {
@%- if !config.is_mysql() %@
@{- def.all_fields_only_geo()|fmt_join("
            ColGeoDistance_::{ident}(v, d, srid) => query.bind(v.clone(){bind_as_for_filter}).bind(srid).bind(d).bind(v{bind_as_for_filter}).bind(srid).bind(d),", "") }@
@%- else %@
@{- def.all_fields_only_geo()|fmt_join("
            ColGeoDistance_::{ident}(v, d, srid) => query.bind(v.clone(){bind_as_for_filter}).bind(srid).bind(d),", "") }@
@%- endif %@
            _ => unreachable!(),
        }
    }
}
@%- if config.exclude_from_domain %@

#[allow(non_camel_case_types)]
#[derive(Clone, Debug)]
pub enum ColRel_ {
@{- def.relations_one_and_belonging(Joinable::Filter, false)|fmt_rel_join("\n    {rel_name}(Option<Box<repo_{class_mod}::Filter_>>),", "") }@
@{- def.relations_many(Joinable::Filter, false)|fmt_rel_join("\n    {rel_name}(Option<Box<repo_{class_mod}::Filter_>>),", "") }@
}
#[allow(unreachable_patterns)]
#[allow(clippy::match_single_binding)]
impl std::fmt::Display for ColRel_ {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            @{- def.relations_one_and_belonging(Joinable::Filter, false)|fmt_rel_join("
            ColRel_::{rel_name}(v) => if let Some(v) = v {
                write!(_f, \"{raw_rel_name}:<{}>\", v)
            } else {
                write!(_f, \"{raw_rel_name}\")
            },", "") }@
            @{- def.relations_many(Joinable::Filter, false)|fmt_rel_join("
            ColRel_::{rel_name}(v) => if let Some(v) = v {
                write!(_f, \"{raw_rel_name}:<{}>\", v)
            } else {
                write!(_f, \"{raw_rel_name}\")
            },", "") }@
            @{- def.relations_belonging_outer_db(Joinable::Filter, false)|fmt_rel_outer_db_join("
            ColRel_::{rel_name}(v) => if let Some(v) = v {
                write!(_f, \"{raw_rel_name}:<{}>\", v)
            } else {
                write!(_f, \"{raw_rel_name}\")
            },", "") }@
            _ => unimplemented!(),
        }
    }
}
@%- else %@
pub use domain::repository::@{ db|snake|ident }@::@{ group_name|snake|ident }@::_base::_@{ mod_name }@::ColRel_;
@%- endif %@
impl ColRelTr for ColRel_ {
    #[allow(unused_mut)]
    #[allow(clippy::ptr_arg)]
    fn write_rel(&self, buf: &mut String, idx: usize, without_key: bool, shard_id: ShardId, is_outer: bool) {
@%- if def.relations_one_and_belonging(Joinable::Filter, false).len() + def.relations_many(Joinable::Filter, false).len() + def.relations_belonging_outer_db(Joinable::Filter, false).len() > 0 %@
        match self {
@{- def.relations_belonging(Joinable::Filter, false)|fmt_rel_join("
            ColRel_::{rel_name}(c) => {
                repo_{class_mod}::write_belonging_rel(buf, c, RelCol{rel_name_pascal}::cols_with_idx(idx), idx, without_key, shard_id, is_outer);
            }", "") }@
@{- def.relations_belonging_outer_db(Joinable::Filter, false)|fmt_rel_outer_db_join("
            ColRel_::{rel_name}(c) => {
                repo_{class_mod}::write_belonging_rel(buf, c, RelCol{rel_name_pascal}::cols_with_idx(idx), idx, without_key, shard_id, true);
            }", "") }@
@{- def.relations_one(Joinable::Filter, false)|fmt_rel_join("
            ColRel_::{rel_name}(c) => {
                repo_{class_mod}::write_having_rel(buf, c, RelCol{rel_name_pascal}::cols(), Primary::cols_with_idx(idx), RelCol{rel_name_pascal}::cols_with_paren(), idx, without_key, shard_id, is_outer);
            }", "") }@
@{- def.relations_many(Joinable::Filter, false)|fmt_rel_join("
            ColRel_::{rel_name}(c) => {
                repo_{class_mod}::write_having_rel(buf, c, RelCol{rel_name_pascal}::cols(), Primary::cols_with_idx(idx), RelCol{rel_name_pascal}::cols_with_paren(), idx, without_key, shard_id, is_outer);
            }", "") }@
        };
@%- endif %@
    }
    #[allow(unused_mut)]
    #[allow(clippy::ptr_arg)]
    fn write_key(&self, buf: &mut String) {
@%- if def.relations_one_and_belonging(Joinable::Filter, false).len() + def.relations_many(Joinable::Filter, false).len() + def.relations_belonging_outer_db(Joinable::Filter, false).len() > 0 %@
        match self {
@{- def.relations_belonging(Joinable::Filter, false)|fmt_rel_join("
            ColRel_::{rel_name}(c) => {
                buf.push_str(RelCol{rel_name_pascal}::cols());
            }", "") }@
@{- def.relations_belonging_outer_db(Joinable::Filter, false)|fmt_rel_outer_db_join("
            ColRel_::{rel_name}(c) => {
                buf.push_str(RelCol{rel_name_pascal}::cols());
            }", "") }@
@{- def.relations_one(Joinable::Filter, false)|fmt_rel_join("
            ColRel_::{rel_name}(c) => {
                buf.push_str(Primary::cols());
            }", "") }@
@{- def.relations_many(Joinable::Filter, false)|fmt_rel_join("
            ColRel_::{rel_name}(c) => {
                buf.push_str(Primary::cols());
            }", "") }@
        };
@%- endif %@
    }
    fn bind_to_query(
        self,
        query: Query<'_, DbType, DbArguments>,
    ) -> Query<'_, DbType, DbArguments> {
@%- if def.relations_one_and_belonging(Joinable::Filter, false).len() + def.relations_many(Joinable::Filter, false).len() + def.relations_belonging_outer_db(Joinable::Filter, false).len() > 0 %@
        match self {
@{- def.relations_one_and_belonging(Joinable::Filter, false)|fmt_rel_join("
            ColRel_::{rel_name}(c) => {
                repo_{class_mod}::bind_for_rel_query(c, query)
            }", "") }@
@{- def.relations_belonging_outer_db(Joinable::Filter, false)|fmt_rel_outer_db_join("
            ColRel_::{rel_name}(c) => {
                repo_{class_mod}::bind_for_rel_query(c, query)
            }", "") }@
@{- def.relations_many(Joinable::Filter, false)|fmt_rel_join("
            ColRel_::{rel_name}(c) => {
                repo_{class_mod}::bind_for_rel_query(c, query)
            }", "") }@
        }
@%- else %@
        query
@%- endif %@
    }
}

pub fn write_belonging_rel(buf: &mut String, filter: &Option<Box<Filter_>>, cols: String, idx: usize, without_key: bool, shard_id: ShardId, is_outer: bool) {
    let db = if is_outer {
        DbConn::real_db_name(shard_id)
    } else {
        ""
    };
    @%- if def.dummy_always_joinable() %@
    if without_key {
        write!(buf, r#"SELECT {} FROM (SELECT {cols} as {}) as _t{} WHERE "#, Primary::cols(), Primary::cols_with_paren(), idx + 1).unwrap();
    } else {
        write!(buf, r#"SELECT @%- if !config.enable_semijoin() %@ /*+ NO_SEMIJOIN() */@%- endif %@ * FROM (SELECT {cols} as {}) as _t{} WHERE "#, Primary::cols_with_paren(), idx + 1).unwrap();
    }
    @%- else %@
    if without_key {
        write!(buf, r#"SELECT {} FROM {db}@{ table_name|db_esc }@ as _t{} WHERE "#, Primary::cols(), idx + 1).unwrap();
    } else if !cols.is_empty() {
        write!(buf, r#"SELECT @%- if !config.enable_semijoin() %@ /*+ NO_SEMIJOIN() */@%- endif %@ * FROM {db}@{ table_name|db_esc }@ as _t{} WHERE {}={} AND "#, idx + 1, Primary::cols_with_paren(), cols).unwrap();
    } else {
        write!(buf, r#"SELECT @%- if !config.enable_semijoin() %@ /*+ NO_SEMIJOIN() */@%- endif %@ * FROM {db}@{ table_name|db_esc }@ as _t{} WHERE "#, idx + 1).unwrap();
    }
    @%- endif %@
    let mut trash_mode = TrashMode::Not;
    if let Some(filter) = filter {
        filter.write(buf, idx + 1, &mut trash_mode, shard_id, is_outer);
    }
    if trash_mode == TrashMode::Not {
        buf.push_str(NOT_TRASHED_SQL)
    } else if trash_mode == TrashMode::Only {
        buf.push_str(ONLY_TRASHED_SQL)
    } else {
        buf.push_str(TRASHED_SQL)
    }
    if buf.ends_with(" AND ") {
        buf.truncate(buf.len() - " AND ".len());
    }
    if buf.ends_with(" WHERE ") {
        buf.truncate(buf.len() - " WHERE ".len());
    }
}

pub fn write_having_rel(buf: &mut String, filter: &Option<Box<Filter_>>, cols1: &str, cols2: String, cols3: &str, idx: usize, without_key: bool, shard_id: ShardId, is_outer: bool) {
    let db = if is_outer {
        DbConn::real_db_name(shard_id)
    } else {
        ""
    };
    @%- if def.dummy_always_joinable() %@
    if without_key {
        write!(buf, r#"SELECT {} FROM (SELECT {cols3} as {cols1}) as _t{} WHERE "#, cols1, idx + 1).unwrap();
    } else {
        write!(buf, r#"SELECT @%- if !config.enable_semijoin() %@ /*+ NO_SEMIJOIN() */@%- endif %@ * FROM (SELECT {cols3} as {cols1}) as _t{} WHERE "#, idx + 1).unwrap();
    }
    @%- else %@
    if without_key {
        write!(buf, r#"SELECT {} FROM {db}@{ table_name|db_esc }@ as _t{} WHERE "#, cols1, idx + 1).unwrap();
    } else if !cols3.is_empty() {
        write!(buf, r#"SELECT @%- if !config.enable_semijoin() %@ /*+ NO_SEMIJOIN() */@%- endif %@ * FROM {db}@{ table_name|db_esc }@ as _t{} WHERE {}={} AND "#, idx + 1, cols2, cols3).unwrap();
    } else {
        write!(buf, r#"SELECT @%- if !config.enable_semijoin() %@ /*+ NO_SEMIJOIN() */@%- endif %@ * FROM {db}@{ table_name|db_esc }@ as _t{} WHERE "#, idx + 1).unwrap();
    }
    @%- endif %@
    let mut trash_mode = TrashMode::Not;
    if let Some(filter) = filter {
        filter.write(buf, idx + 1, &mut trash_mode, shard_id, is_outer);
    }
    if trash_mode == TrashMode::Not {
        buf.push_str(NOT_TRASHED_SQL)
    } else if trash_mode == TrashMode::Only {
        buf.push_str(ONLY_TRASHED_SQL)
    } else {
        buf.push_str(TRASHED_SQL)
    }
    if buf.ends_with(" AND ") {
        buf.truncate(buf.len() - " AND ".len());
    }
    if buf.ends_with(" WHERE ") {
        buf.truncate(buf.len() - " WHERE ".len());
    }
}

pub fn bind_for_rel_query(filter: Option<Box<Filter_>>, query: Query<'_, DbType, DbArguments>) -> Query<'_, DbType, DbArguments> {
    if let Some(filter) = filter {
        filter.bind_to_query(query)
    } else {
        query
    }
}
@%- if config.exclude_from_domain %@

#[derive(Clone, Debug)]
pub enum Filter_ {
    WithTrashed,
    OnlyTrashed,
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
#[derive(Clone, Debug, Default)]
pub struct WithFilterFlag {
    pub filters: BTreeMap<&'static str, Filter_>,
}
impl std::fmt::Display for Filter_ {
    #[allow(bindings_with_variant_name)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Filter_::WithTrashed => write!(f, "WithTrashed"),
            Filter_::OnlyTrashed => write!(f, "OnlyTrashed"),
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
            Filter_::Not(filter) => write!(f, "Not:<{}>", filter),
            Filter_::And(filters) => write!(f, "And:<{}>", filters.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(",")),
            Filter_::Or(filters) => write!(f, "Or:<{}>", filters.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(",")),
            Filter_::Exists(col_rel) => write!(f, "Exists:<{}>", col_rel),
            Filter_::NotExists(col_rel) => write!(f, "NotExists:<{}>", col_rel),
            Filter_::EqAny(col_rel) => write!(f, "EqAny:<{}>", col_rel),
            Filter_::NotAll(col_rel) => write!(f, "NotAll:<{}>", col_rel),
            Filter_::Raw(sql) => write!(f, "Raw:<{}>", sql),
            Filter_::RawWithParam(sql, _) => write!(f, "Raw:<{}>", sql),
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
}
@%- else %@
pub use domain::repository::@{ db|snake|ident }@::@{ group_name|snake|ident }@::_base::_@{ mod_name }@::Filter_;
@%- endif %@
impl FilterTr for Filter_ {
    crate::misc::filter!(Data);
}
@% let filter_macro_name = "filter_{}_{}_{}"|format(db|snake, group_name|snake, model_name) -%@
@% let model_path = "$crate::repositories::{}::_base::_{}"|format(group_name|snake|ident, mod_name) -%@
@%- if config.exclude_from_domain %@

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
@%- for (model_def, col_name, rel_def) in def.relations_one_and_belonging(Joinable::Filter, false) %@
    (@{ col_name }@) => (@{ model_path }@::ColRel_::@{ col_name|ident }@(None));
    (@{ col_name }@ $t:tt) => (@{ model_path }@::ColRel_::@{ col_name|ident }@(Some(Box::new($crate::repositories::@{ rel_def.get_group_name()|snake|ident }@::_base::_@{ rel_def.get_mod_name() }@::filter!($t)))));
@%- endfor %@
@%- for (model_def, col_name, rel_def) in def.relations_many(Joinable::Filter, false) %@
    (@{ col_name }@) => (@{ model_path }@::ColRel_::@{ col_name|ident }@(None));
    (@{ col_name }@ $t:tt) => (@{ model_path }@::ColRel_::@{ col_name|ident }@(Some(Box::new($crate::repositories::@{ rel_def.get_group_name()|snake|ident }@::_base::_@{ rel_def.get_mod_name() }@::filter!($t)))));
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
@%- endif %@
@%- if config.exclude_from_domain %@

#[derive(Clone, Debug)]
pub enum Order_ {
    Asc(Col_),
    Desc(Col_),
    IsNullAsc(Col_),
    IsNullDesc(Col_),
}
@%- else %@
pub use domain::repository::@{ db|snake|ident }@::@{ group_name|snake|ident }@::_base::_@{ mod_name }@::Order_;
@%- endif %@
impl OrderTr for Order_ {
    crate::misc::order!();
}
@%- if config.exclude_from_domain %@

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
@%- endif %@
@{-"\n"}@