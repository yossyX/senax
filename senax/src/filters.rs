use crate::{common::if_then_else, schema::*};
use convert_case::{Case, Casing};
use std::sync::atomic::{AtomicBool, Ordering};

pub static SHOW_LABEL: AtomicBool = AtomicBool::new(true);
pub static SHOW_COMMNET: AtomicBool = AtomicBool::new(true);

pub fn _to_db_col(s: &str, esc: bool) -> String {
    if esc {
        format!("\"{}\"", s)
    } else {
        s.to_owned()
    }
}

fn _raw_var_name(s: &str) -> String {
    s.to_owned()
}

pub fn to_var_name<S: AsRef<str>>(s: S) -> ::askama::Result<String> {
    Ok(_to_var_name(s.as_ref()))
}

pub fn to_pascal_name(s: &str) -> ::askama::Result<String> {
    Ok(_to_var_name(&s.to_case(Case::Pascal)))
}
pub fn pascal(s: &str) -> ::askama::Result<String> {
    Ok(s.to_case(Case::Pascal))
}
#[allow(dead_code)]
pub fn camel(s: &str) -> ::askama::Result<String> {
    Ok(s.to_case(Case::Camel))
}
pub fn gql_pascal(s: &str) -> ::askama::Result<String> {
    use inflector::Inflector;
    Ok(s.to_pascal_case())
}
pub fn gql_camel(s: &str) -> ::askama::Result<String> {
    use inflector::Inflector;
    Ok(s.to_camel_case())
}
pub fn snake<S: AsRef<str>>(s: S) -> ::askama::Result<String> {
    Ok(s.as_ref().to_case(Case::Snake))
}
pub fn upper_snake(s: &str) -> ::askama::Result<String> {
    Ok(s.to_case(Case::UpperSnake))
}
pub fn db_esc(s: &str) -> ::askama::Result<String> {
    Ok(_to_db_col(s, true))
}

pub fn fmt_join_with_paren(
    v: Vec<(&String, &FieldDef)>,
    f: &str,
    sep: &str,
) -> ::askama::Result<String> {
    if v.len() > 1 {
        Ok(format!("({})", fmt_join(v, f, sep)?))
    } else {
        fmt_join(v, f, sep)
    }
}

pub fn fmt_join_with_paren2(
    v: Vec<(&String, &FieldDef)>,
    f1: &str,
    f2: &str,
    sep: &str,
) -> ::askama::Result<String> {
    if v.len() > 1 {
        Ok(format!("({})", fmt_join(v, f2, sep)?))
    } else {
        fmt_join(v, f1, sep)
    }
}

pub fn fmt_join(v: Vec<(&String, &FieldDef)>, f: &str, sep: &str) -> ::askama::Result<String> {
    let mut index = -1;
    Ok(v.iter()
        .map(|(name, col)| {
            index += 1;
            _fmt_join(f, name, col, index, &Vec::new())
        })
        .collect::<Vec<_>>()
        .join(sep))
}

pub fn fmt_join_with_foreign_default(
    v: Vec<(&String, &FieldDef)>,
    f: &str,
    sep: &str,
    foreign: &[String],
) -> ::askama::Result<String> {
    let mut index = -1;
    Ok(v.iter()
        .map(|(name, col)| {
            index += 1;
            _fmt_join(f, name, col, index, foreign)
        })
        .collect::<Vec<_>>()
        .join(sep))
}

pub fn fmt_join_cache_or_not(
    v: Vec<(&String, &FieldDef)>,
    th: &str,
    el: &str,
    sep: &str,
) -> ::askama::Result<String> {
    let mut index = -1;
    Ok(v.iter()
        .map(|(name, col)| {
            index += 1;
            if !col.exclude_from_cache() {
                _fmt_join(th, name, col, index, &Vec::new())
            } else {
                _fmt_join(el, name, col, index, &Vec::new())
            }
        })
        .collect::<Vec<_>>()
        .join(sep))
}

pub fn fmt_join_foreign_with_paren(
    v: Vec<(String, FieldDef)>,
    f: &str,
    sep: &str,
) -> ::askama::Result<String> {
    if v.len() > 1 {
        Ok(format!("({})", fmt_join_foreign(v, f, sep)?))
    } else {
        fmt_join_foreign(v, f, sep)
    }
}

pub fn fmt_join_foreign(
    v: Vec<(String, FieldDef)>,
    f: &str,
    sep: &str,
) -> ::askama::Result<String> {
    let mut index = -1;
    Ok(v.iter()
        .map(|(name, col)| {
            index += 1;
            _fmt_join(f, &name, &col, index, &Vec::new())
        })
        .collect::<Vec<_>>()
        .join(sep))
}

pub fn fmt_join_foreign_not_null_or_null(
    v: Vec<(String, FieldDef)>,
    not_null_case: &str,
    null_case: &str,
    sep: &str,
) -> ::askama::Result<String> {
    let mut index = -1;
    Ok(v.iter()
        .map(|(name, col)| {
            let f = if col.not_null {
                not_null_case
            } else {
                null_case
            };
            index += 1;
            _fmt_join(f, &name, &col, index, &Vec::new())
        })
        .collect::<Vec<_>>()
        .join(sep))
}

fn _fmt_join(f: &str, name: &&String, col: &&FieldDef, index: i32, foreign: &[String]) -> String {
    f.replace("{col}", &_to_db_col(name, false))
        .replace("{col_esc}", &_to_db_col(&col.get_col_name(name), true))
        .replace("{col_query}", &col.get_col_query(&col.get_col_name(name)))
        .replace("{var}", &_to_var_name(name))
        .replace("{raw_var}", &_raw_var_name(name))
        .replace("{var_pascal}", &name.to_case(Case::Pascal))
        .replace("{upper}", &name.to_case(Case::UpperSnake))
        .replace("{raw_inner}", &col.get_inner_type(true, false))
        .replace(
            "{raw_inner_without_option}",
            &col.get_inner_type(true, true),
        )
        .replace("{inner}", &col.get_inner_type(false, false))
        .replace("{inner_without_option}", &col.get_inner_type(false, true))
        .replace("{inner_to_raw}", col.get_inner_to_raw())
        .replace("{raw_to_inner}", col.get_raw_to_inner())
        .replace("{may_null}", col.get_may_null())
        .replace("{null_question}", col.get_null_question())
        .replace("{serde}", &col.get_serde_default())
        .replace("{default}", &col.get_default())
        .replace("{column_query}", &col.get_column_query(name))
        .replace("{validate}", &col.get_validate(name))
        .replace("{api_validate_const}", &col.get_api_validate_const(name))
        .replace("{api_validate}", &col.get_api_validate(name))
        .replace("{api_default}", &col.get_api_default(name))
        .replace("{api_serde_default}", &col.get_api_serde_default(name))
        .replace("{graphql_secret}", col.graphql_secret())
        .replace("{outer}", &col.get_outer_type(false))
        .replace("{domain_outer}", &col.get_outer_type(true))
        .replace("{outer_ref}", &col.get_outer_ref_type())
        .replace("{outer_owned}", &col.get_outer_owned_type(false, false))
        .replace(
            "{domain_outer_owned}",
            &col.get_outer_owned_type(true, false),
        )
        .replace("{domain_factory}", &col.get_outer_owned_type(true, true))
        .replace("{accessor}", &col.accessor(false, ""))
        .replace("{accessor_with_type}", &col.accessor(true, ""))
        .replace("{accessor_with_sep_type}", &col.accessor(true, "::"))
        .replace("{convert_inner}", &col.convert_inner_type())
        .replace("{convert_outer_prefix}", col.convert_outer_prefix())
        .replace("{convert_outer}", col.convert_outer_type())
        .replace(
            "{convert_domain_outer_prefix}",
            col.convert_domain_outer_prefix(),
        )
        .replace(
            "{convert_domain_outer}",
            col.convert_domain_outer_type(false, false),
        )
        .replace(
            "{convert_impl_domain_outer}",
            col.convert_domain_outer_type(true, false),
        )
        .replace(
            "{convert_impl_domain_inner}",
            col.convert_domain_outer_type(true, true),
        )
        .replace(
            "{convert_domain_inner_type}",
            col.convert_domain_inner_type(),
        )
        .replace("{convert_domain_factory}", col.convert_domain_factory())
        .replace(
            "{convert_impl_domain_outer_for_updater}",
            &col.convert_impl_domain_outer_for_updater(name),
        )
        .replace("{convert_serialize}", col.convert_serialize())
        .replace("{factory}", &col.get_factory_type())
        .replace("{factory_default}", col.get_factory_default())
        .replace("{convert_factory}", &col.convert_factory_type())
        .replace("{convert_from_entity}", &col.convert_from_entity())
        .replace("{res_api_type}", &col.get_api_type(false, false))
        .replace("{req_api_option_type}", &col.get_api_type(true, true))
        .replace("{req_api_type}", &col.get_api_type(false, true))
        .replace("{gql_type}", &col.get_gql_type())
        .replace("{ts_type}", col.get_ts_type())
        .replace("{to_res_api_type}", col.get_to_api_type(false))
        .replace("{to_req_api_type}", col.get_to_api_type(true))
        .replace(
            "{from_api_type}",
            &col.get_from_api_type(name, false, foreign, false),
        )
        .replace(
            "{from_api_type_for_update}",
            &col.get_from_api_type(name, false, foreign, true),
        )
        .replace(
            "{from_api_rel_type}",
            &col.get_from_api_type(name, true, foreign, false),
        )
        .replace("{filter_type}", &col.get_filter_type(domain_mode()))
        .replace(
            "{filter_check_null}",
            &col.get_filter_null(&_to_var_name(name)),
        )
        .replace("{filter_check_eq}", &col.get_filter_eq(None, false))
        .replace("{filter_check_cmp}", &col.get_filter_cmp(None))
        .replace("{filter_like}", col.get_filter_like())
        .replace("{bind_as_for_filter}", col.get_bind_as_for_filter())
        .replace("{bind_as}", col.get_bind_as())
        .replace("{from_row}", &col.get_from_row(name, index))
        .replace("{index}", &index.to_string())
        .replace("{clone}", col.clone_str())
        .replace("{clone_for_outer}", col.clone_for_outer_str())
        .replace("{placeholder}", &col.placeholder())
        .replace(
            "{label}",
            &label4(if_then_else!(
                SHOW_LABEL.load(Ordering::Relaxed),
                &col.label,
                &None
            ))
            .unwrap(),
        )
        .replace(
            "{label_wo_hash}",
            &label4_wo_hash(if_then_else!(
                SHOW_LABEL.load(Ordering::Relaxed),
                &col.label,
                &None
            ))
            .unwrap(),
        )
        .replace(
            "{comment}",
            &comment4(if_then_else!(
                SHOW_COMMNET.load(Ordering::Relaxed),
                &col.comment,
                &None
            ))
            .unwrap(),
        )
        .replace("{comma}", if { index } > 0 { ", " } else { "" })
        .replace("{disp}", if col.is_displayable() { "{}" } else { "{:?}" })
}

pub fn fmt_rel_join(
    v: Vec<(&ModelDef, &String, &RelDef)>,
    f: &str,
    sep: &str,
) -> ::askama::Result<String> {
    let mut index = -1;
    Ok(v.iter()
        .map(|(model, name, rel)| {
            index += 1;
            _fmt_rel(f, rel, name, model, index)
        })
        .collect::<Vec<_>>()
        .join(sep))
}

fn _fmt_rel(f: &str, rel: &&RelDef, name: &&String, model: &&ModelDef, index: i32) -> String {
    let foreign_model = rel.get_foreign_model();
    let asc = if_then_else!(rel.desc, "Desc", "Asc");
    let list_order = if_then_else!(rel.desc, ".reverse()", "");
    let class_mod = rel.get_group_mod_name();
    let rel_name = _to_var_name(name);
    let additional_filter = if let Some(additional_filter) = &rel.additional_filter {
        format!(".and(join_{}::filter!({}))", class_mod, additional_filter)
    } else {
        "".to_string()
    };
    let mut local_keys: Vec<_> = rel
        .get_local_cols(name, model)
        .iter()
        .map(|(k, v)| {
            let name = _to_var_name(k);
            if v.not_null {
                format!("self.{name}()")
            } else {
                format!("self.{name}()?")
            }
        })
        .collect();
    let local_keys = if local_keys.len() == 1 {
        local_keys.pop().unwrap()
    } else {
        format!("({})", local_keys.join(", "))
    };
    let (order, list_sort, list_sort_for_update, cache_list_sort) = if let Some(col) = &rel.order_by
    {
        let col = if let Some(local_col) = model.merged_fields.get(col) {
            local_col.get_col_name(col)
        } else {
            col.into()
        };
        let col = _to_var_name(&col);
        (
            format!("repo_{class_mod}::Order_::{asc}(repo_{class_mod}::Col_::{col})"),
            format!("l.sort_by(|v1, v2| v1._inner.{col}.cmp(&v2._inner.{col}){list_order});"),
            format!("l.sort_by(|v1, v2| v1._data.{col}.cmp(&v2._data.{col}){list_order});"),
            format!(
                "cache.{rel_name}.sort_by(|v1, v2| v1._inner.{col}.cmp(&v2._inner.{col}){list_order});"
            ),
        )
    } else {
        let tmpl1 = format!("repo_{class_mod}::Order_::{asc}(repo_{class_mod}::Col_::{{var}})");
        let tmpl2 = "(v1._inner.{var}.cmp(&v2._inner.{var}))".to_string();
        let tmpl3 = "(v1._data.{var}.cmp(&v2._data.{var}))".to_string();
        (
            fmt_join(foreign_model.primaries(), &tmpl1, ",").unwrap(),
            format!(
                "l.sort_by(|v1, v2| {}{list_order});",
                fmt_join(foreign_model.primaries(), &tmpl2, ".then").unwrap()
            ),
            format!(
                "l.sort_by(|v1, v2| {}{list_order});",
                fmt_join(foreign_model.primaries(), &tmpl3, ".then").unwrap()
            ),
            format!(
                "cache.{rel_name}.sort_by(|v1, v2| {}{list_order});",
                fmt_join(foreign_model.primaries(), &tmpl2, ".then").unwrap()
            ),
        )
    };
    let (limit, cache_list_limit, order_and_limit) = if let Some(limit) = rel.limit {
        (
            format!(".limit({limit})"),
            format!("cache.{rel_name}.truncate({limit});"),
            format!(".order_by(vec![{order}]).limit({limit})"),
        )
    } else {
        ("".to_string(), "".to_string(), "".to_string())
    };
    let with_trashed = if rel.with_trashed {
        "_with_trashed"
    } else {
        ""
    };
    let soft_delete_filter = foreign_model.soft_delete_tpl(
        "",
        ".filter(|data| data.--1--.deleted_at.is_none())",
        ".filter(|data| data.--1--.deleted == 0)",
    );
    let rel_hash =
        crate::common::rel_hash(format!("{}::{}::{}", &model.group_name, &model.name, name));
    f.replace("{rel_name}", &_to_var_name(name))
        .replace("{raw_rel_name}", name)
        .replace("{rel_name_pascal}", &name.to_case(Case::Pascal))
        .replace("{rel_name_camel}", &name.to_case(Case::Camel))
        .replace("{rel_hash}", &rel_hash.to_string())
        .replace("{class}", &rel.get_foreign_class_name())
        .replace("{class_mod}", &rel.get_group_mod_name())
        .replace("{group_var}", &rel.get_group_var())
        .replace("{class_mod_var}", &rel.get_group_mod_var())
        .replace("{base_class_mod_var}", &rel.get_base_group_mod_var())
        .replace("{mod_name}", &rel.get_mod_name())
        .replace("{mod_var}", &_to_var_name(&rel.get_mod_name()))
        .replace("{local_table}", &model.table_name())
        // .replace("{table}", &_to_db_col(&rel.get_foreign_table_name(), true))
        .replace("{raw_table}", &rel.get_foreign_table_name())
        .replace("{index}", &index.to_string())
        .replace(
            "{label}",
            &label4(if_then_else!(
                SHOW_LABEL.load(Ordering::Relaxed),
                &rel.label,
                &None
            ))
            .unwrap(),
        )
        .replace(
            "{label_wo_hash}",
            &label4_wo_hash(if_then_else!(
                SHOW_LABEL.load(Ordering::Relaxed),
                &rel.label,
                &None
            ))
            .unwrap(),
        )
        .replace(
            "{comment}",
            &comment4(if_then_else!(
                SHOW_COMMNET.load(Ordering::Relaxed),
                &rel.comment,
                &None
            ))
            .unwrap(),
        )
        .replace("{additional_filter}", &additional_filter)
        .replace("{order}", &order)
        .replace("{limit}", &limit)
        .replace("{order_and_limit}", &order_and_limit)
        .replace("{list_sort}", &list_sort)
        .replace("{list_sort_for_update}", &list_sort_for_update)
        .replace("{cache_list_sort}", &cache_list_sort)
        .replace("{cache_list_limit}", &cache_list_limit)
        .replace("{pascal_name}", &model.name.to_case(Case::Pascal))
        .replace("{id_name}", &to_id_name(&model.name))
        .replace("{with_trashed}", with_trashed)
        .replace("{soft_delete_filter}", &soft_delete_filter)
        .replace("{local_keys}", &local_keys)
}

pub fn fmt_rel_outer_db_join(
    v: Vec<(&ModelDef, &String, &RelDef)>,
    f: &str,
    sep: &str,
) -> ::askama::Result<String> {
    let mut index = -1;
    Ok(v.iter()
        .map(|(model, name, rel)| {
            index += 1;
            _fmt_rel_outer_db(f, rel, name, model, index)
        })
        .collect::<Vec<_>>()
        .join(sep))
}

fn _fmt_rel_outer_db(
    f: &str,
    rel: &&RelDef,
    name: &&String,
    model: &&ModelDef,
    index: i32,
) -> String {
    let mut local_keys: Vec<_> = rel
        .get_local_cols(name, model)
        .iter()
        .map(|(k, v)| {
            let name = _to_var_name(k);
            if v.not_null {
                format!("self.{name}()")
            } else {
                format!("self.{name}()?")
            }
        })
        .collect();
    let local_keys = if local_keys.len() == 1 {
        local_keys.pop().unwrap()
    } else {
        format!("({})", local_keys.join(", "))
    };
    let with_trashed = if rel.with_trashed {
        "_with_trashed"
    } else {
        ""
    };
    let rel_hash =
        crate::common::rel_hash(format!("{}::{}::{}", &model.group_name, &model.name, name));
    f.replace("{rel_name}", &_to_var_name(name))
        .replace("{raw_rel_name}", name)
        .replace("{raw_db}", rel.db())
        .replace("{db_mod_var}", &_to_var_name(rel.db()))
        .replace("{rel_name_pascal}", &name.to_case(Case::Pascal))
        .replace("{rel_name_camel}", &name.to_case(Case::Camel))
        .replace("{rel_hash}", &rel_hash.to_string())
        .replace("{class}", &rel.get_foreign_class_name())
        .replace("{class_mod}", &rel.get_group_mod_name())
        .replace("{group_var}", &rel.get_group_var())
        .replace("{class_mod_var}", &rel.get_group_mod_var())
        .replace("{base_class_mod_var}", &rel.get_base_group_mod_var())
        .replace("{mod_name}", &rel.get_mod_name())
        .replace("{mod_var}", &_to_var_name(&rel.get_mod_name()))
        .replace("{local_table}", &model.table_name())
        .replace("{index}", &index.to_string())
        .replace(
            "{label}",
            &label4(if_then_else!(
                SHOW_LABEL.load(Ordering::Relaxed),
                &rel.label,
                &None
            ))
            .unwrap(),
        )
        .replace(
            "{label_wo_hash}",
            &label4_wo_hash(if_then_else!(
                SHOW_LABEL.load(Ordering::Relaxed),
                &rel.label,
                &None
            ))
            .unwrap(),
        )
        .replace(
            "{comment}",
            &comment4(if_then_else!(
                SHOW_COMMNET.load(Ordering::Relaxed),
                &rel.comment,
                &None
            ))
            .unwrap(),
        )
        .replace("{pascal_name}", &model.name.to_case(Case::Pascal))
        .replace("{id_name}", &to_id_name(&model.name))
        .replace("{with_trashed}", with_trashed)
        // .replace("{soft_delete_filter}", &soft_delete_filter)
        .replace("{local_keys}", &local_keys)
}

pub fn fmt_index_col(v: Vec<(&String, &FieldDef)>, f: &str, sep: &str) -> ::askama::Result<String> {
    let mut index = 0;
    Ok(v.iter()
        .map(|(name, col)| {
            index += 1;
            _fmt_index_col(name, col, f, index)
        })
        .collect::<Vec<_>>()
        .join(sep))
}

pub fn fmt_index_col_not_null_or_null(
    v: Vec<(&String, &FieldDef)>,
    not_null_case: &str,
    null_case: &str,
    sep: &str,
) -> ::askama::Result<String> {
    let mut index = 0;
    Ok(v.iter()
        .map(|(name, col)| {
            index += 1;
            let f = if col.not_null {
                not_null_case
            } else {
                null_case
            };
            _fmt_index_col(name, col, f, index)
        })
        .collect::<Vec<_>>()
        .join(sep))
}

fn _fmt_index_col(name: &&String, col: &&FieldDef, f: &str, index: usize) -> String {
    f.replace("{name}", name)
        .replace("{col_name}", &col.get_col_name(name))
        .replace("{col_esc}", &_to_db_col(&col.get_col_name(name), true))
        .replace("{col_pascal}", &name.to_case(Case::Pascal))
        .replace("{var}", &_to_var_name(name))
        .replace("{raw_var}", name)
        .replace("{bind_as_for_filter}", col.get_bind_as_for_filter())
        .replace("{filter_type}", &col.get_filter_type(domain_mode()))
        .replace("{index}", &index.to_string())
        .replace("{inner_to_raw}", col.get_inner_to_raw())
        .replace("{filter_check_eq}", &col.get_filter_eq(Some(index), true))
}

pub fn fmt_cache_owners(v: &[(String, String, String, u64)], f: &str) -> ::askama::Result<String> {
    Ok(v.iter()
        .map(|(mod_name, model_name, name, rel_hash)| {
            f.replace("{mod}", mod_name)
                .replace("{model_name}", &model_name.to_case(Case::Pascal))
                .replace("{rel_name_pascal}", &name.to_case(Case::Pascal))
                .replace("{rel_hash}", &rel_hash.to_string())
        })
        .collect::<Vec<_>>()
        .join(""))
}

#[allow(dead_code)]
pub fn check_lifetime(s: &str) -> ::askama::Result<String> {
    Ok(if s.contains("&'a") { "<'a>" } else { "" }.to_string())
}
pub fn label0(s: &Option<String>) -> ::askama::Result<String> {
    match s {
        None => Ok("".to_owned()),
        Some(s) => {
            let s = s
                .trim()
                .to_string()
                .replace("\r\n", "\n")
                .replace('\r', "\n")
                .replace('\n', "\n/// ");
            Ok(format!("/// ### {}\n", s))
        }
    }
}
pub fn comment0(s: &Option<String>) -> ::askama::Result<String> {
    match s {
        None => Ok("".to_owned()),
        Some(s) => {
            let s = s
                .trim()
                .to_string()
                .replace("\r\n", "\n")
                .replace('\r', "\n")
                .replace('\n', "\n/// ");
            Ok(format!("/// {}\n", s))
        }
    }
}
pub fn label4(s: &Option<String>) -> ::askama::Result<String> {
    match s {
        None => Ok("".to_owned()),
        Some(s) => {
            let s = s
                .trim()
                .to_string()
                .replace("\r\n", "\n")
                .replace('\r', "\n")
                .replace('\n', "\n    /// ");
            Ok(format!("    /// ### {}\n", s))
        }
    }
}
pub fn label4_wo_hash(s: &Option<String>) -> ::askama::Result<String> {
    match s {
        None => Ok("".to_owned()),
        Some(s) => {
            let s = s
                .trim()
                .to_string()
                .replace("\r\n", "\n")
                .replace('\r', "\n")
                .replace('\n', "\n    /// ");
            Ok(format!("    /// {}\n", s))
        }
    }
}
pub fn comment4(s: &Option<String>) -> ::askama::Result<String> {
    match s {
        None => Ok("".to_owned()),
        Some(s) => {
            let s = s
                .trim()
                .to_string()
                .replace("\r\n", "\n")
                .replace('\r', "\n")
                .replace('\n', "\n    /// ");
            Ok(format!("    /// {}\n", s))
        }
    }
}
pub fn strum_message4(s: &Option<String>) -> ::askama::Result<String> {
    match s {
        None => Ok("".to_owned()),
        Some(s) => Ok(format!("    #[strum(message = {:?})]\n", s.trim())),
    }
}
pub fn strum_detailed4(s: &Option<String>) -> ::askama::Result<String> {
    match s {
        None => Ok("".to_owned()),
        Some(s) => Ok(format!("    #[strum(detailed_message = {:?})]\n", s.trim())),
    }
}
pub fn strum_props4(def: &FieldDef) -> ::askama::Result<String> {
    let mut def = def.clone();
    def._name = None;
    Ok(format!(
        "    #[strum(props(def = {:?}))]\n",
        serde_json::to_string(&def).unwrap()
    ))
}
#[allow(dead_code)]
pub fn auto_opt<T: std::fmt::Display>(s: &Option<T>) -> ::askama::Result<String> {
    match s {
        None => Ok("".to_owned()),
        Some(s) => Ok(format!("{}", s)),
    }
}
pub fn disp_opt<T: std::fmt::Debug>(s: &Option<T>) -> ::askama::Result<String> {
    match s {
        None => Ok("None".to_owned()),
        Some(s) => Ok(format!("Some({:?})", s)),
    }
}
// pub fn if_then_else<T: std::fmt::Display>(wh: bool, th: T, el: T) -> ::askama::Result<T> {
//     Ok(if_then_else!(wh, th, el))
// }
pub fn if_then_else_ref<T: std::fmt::Display>(wh: &bool, th: T, el: T) -> ::askama::Result<T> {
    Ok(if_then_else!(*wh, th, el))
}
#[allow(dead_code)]
pub fn is_true(b: &Option<bool>) -> ::askama::Result<bool> {
    Ok(*b == Some(true))
}
pub fn replace1<C: AsRef<str>, S: AsRef<str>>(content: C, r1: S) -> ::askama::Result<String> {
    Ok(content.as_ref().replace("--1--", r1.as_ref()))
}
// pub fn replace3(content: &str, r1: &str, r2: &str, r3: &str) -> ::askama::Result<String> {
//     Ok(content
//         .replace("--1--", r1)
//         .replace("--2--", r2)
//         .replace("--3--", r3))
// }

pub fn senax_version(_s: &str) -> ::askama::Result<String> {
    Ok(crate::VERSION.to_string())
}

pub fn to_gql_guard(vec: Vec<String>) -> ::askama::Result<String> {
    use std::fmt::Write;
    let guard = vec.iter().fold(String::new(), |mut acc, v| {
        if acc.is_empty() {
            write!(&mut acc, "RoleGuard(Role::{v})").unwrap();
        } else {
            write!(&mut acc, "\n        .or(RoleGuard(Role::{v}))").unwrap();
        }
        acc
    });
    if !guard.is_empty() {
        Ok(guard)
    } else {
        Ok("NoGuard".to_string())
    }
}

pub fn to_api_guard(v: Vec<String>) -> ::askama::Result<String> {
    Ok(v.iter()
        .map(|v| format!("Role::{v}"))
        .collect::<Vec<_>>()
        .join(", "))
}
