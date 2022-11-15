use crate::{common::if_then_else, model_generator::schema::*};
use convert_case::{Case, Casing};
use std::fmt::Write;

fn _to_db_col(s: &str, esc: bool) -> String {
    if esc {
        format!("`{}`", s)
    } else {
        s.to_owned()
    }
}

fn _raw_var_name(s: &str) -> String {
    s.to_owned()
}

pub fn to_var_name(s: &str) -> ::askama::Result<String> {
    Ok(_to_var_name(s))
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
// pub fn upper(s: &str) -> ::askama::Result<String> {
//     Ok(s.to_case(Case::UpperSnake))
// }
pub fn db_esc(s: &str) -> ::askama::Result<String> {
    Ok(_to_db_col(s, true))
}

pub fn fmt_join_with_paren(
    v: &Vec<(&String, &ColumnDef)>,
    f: &str,
    sep: &str,
) -> ::askama::Result<String> {
    if v.len() > 1 {
        Ok(format!("({})", fmt_join(v, f, sep)?))
    } else {
        fmt_join(v, f, sep)
    }
}

pub fn fmt_join(v: &[(&String, &ColumnDef)], f: &str, sep: &str) -> ::askama::Result<String> {
    let mut index = -1;
    Ok(v.iter()
        .map(|(name, col)| {
            index += 1;
            _fmt_join(f, name, col, index)
        })
        .collect::<Vec<_>>()
        .join(sep))
}

pub fn fmt_join_cache_or_not(
    v: &[(&String, &ColumnDef)],
    th: &str,
    el: &str,
    sep: &str,
) -> ::askama::Result<String> {
    let mut index = -1;
    Ok(v.iter()
        .map(|(name, col)| {
            index += 1;
            if !col.exclude_from_cache {
                _fmt_join(th, name, col, index)
            } else {
                _fmt_join(el, name, col, index)
            }
        })
        .collect::<Vec<_>>()
        .join(sep))
}

fn _fmt_join(f: &str, name: &&String, col: &&ColumnDef, index: i32) -> String {
    f.replace("{col}", &_to_db_col(name, false))
        .replace("{col_esc}", &_to_db_col(&col.get_col_name(name), true))
        .replace("{var}", &_to_var_name(name))
        .replace("{raw_var}", &_raw_var_name(name))
        .replace("{var_pascal}", &name.to_case(Case::Pascal))
        .replace("{upper}", &name.to_case(Case::UpperSnake))
        .replace("{inner}", &col.get_inner_type(&false))
        .replace("{inner_without_option}", &col.get_inner_type(&true))
        .replace("{may_null}", col.get_may_null())
        .replace("{default}", &col.get_serde_default())
        .replace("{rename}", &col.get_rename(name))
        .replace("{validate}", &col.get_validate())
        .replace("{outer}", &col.get_outer_type())
        .replace("{outer_ref}", &col.get_outer_ref_type())
        .replace("{outer_owned}", &col.get_outer_owned_type())
        .replace("{outer_for_update}", &col.get_outer_for_update_type())
        .replace("{accessor}", &col.accessor(false, ""))
        .replace("{accessor_with_type}", &col.accessor(true, ""))
        .replace("{accessor_with_sep_type}", &col.accessor(true, "::"))
        .replace("{convert_inner}", &col.convert_inner_type())
        .replace("{convert_outer}", col.convert_outer_type())
        .replace("{convert_serialize}", col.convert_serialize())
        .replace("{factory}", &col.get_factory_type())
        .replace("{factory_default}", col.get_factory_default())
        .replace("{convert_factory}", &col.convert_factory_type())
        .replace("{cond_type}", &col.get_cond_type())
        .replace("{bind_as}", col.get_bind_as())
        .replace("{index}", &index.to_string())
        .replace("{clone}", col.clone_str())
        .replace("{placeholder}", &col.placeholder())
        .replace("{title}", &comment4(&col.title).unwrap())
        .replace("{comment}", &comment4(&col.comment).unwrap())
}

pub fn fmt_rel_join_foreign_is_not_null_or_null(
    v: &[(&ModelDef, &String, &Option<RelDef>)],
    not_null_case: &str,
    null_case: &str,
    sep: &str,
) -> ::askama::Result<String> {
    let mut index = -1;
    Ok(v.iter()
        .map(|(model, name, rel)| {
            let f = if RelDef::foreign_is_not_null(rel, name, model) {
                not_null_case
            } else {
                null_case
            };
            index += 1;
            _fmt_rel(f, rel, name, model, index)
        })
        .collect::<Vec<_>>()
        .join(sep))
}

pub fn fmt_rel_join(
    v: &[(&ModelDef, &String, &Option<RelDef>)],
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

fn _fmt_rel(
    f: &str,
    rel: &&Option<RelDef>,
    name: &&String,
    model: &&ModelDef,
    index: i32,
) -> String {
    let local = RelDef::get_local_id(rel, name, &model.id_name());
    let local_col_name = if let Some(local_col) = model.merged_columns.get(&local) {
        local_col.get_col_name(&local).to_string()
    } else {
        local.clone()
    };
    let local_id = model
        .id()
        .iter()
        .map(|(name, _a)| name.as_str())
        .last()
        .unwrap_or("id");
    let foreign_model = RelDef::get_foreign_model(rel, name);
    let foreign = RelDef::get_foreign_id(rel, model, &foreign_model);
    let primaries = fmt_join(&foreign_model.primaries(), "{col_esc}", ",").unwrap();
    let asc = if_then_else!(rel.as_ref().map(|v| v.desc).unwrap_or(false), "Desc", "Asc");
    let list_order = if_then_else!(
        rel.as_ref().map(|v| v.desc).unwrap_or(false),
        ".reverse()",
        ""
    );
    let class_mod = RelDef::get_group_mod_name(rel, name);
    let alias = _to_var_name(name);
    let and_cond = if let Some(raw_cond) = rel.as_ref().and_then(|v| v.raw_cond.as_ref()) {
        format!(".and({})", raw_cond)
    } else {
        "".to_string()
    };
    let (order_by, list_sort) = if let Some(col) = rel.as_ref().and_then(|v| v.order_by.as_ref()) {
        let col = if let Some(local_col) = model.merged_columns.get(col) {
            local_col.get_col_name(col)
        } else {
            col.into()
        };
        let col = _to_var_name(&col);
        (
                format!("rel_{class_mod}::OrderBy::{asc}(rel_{class_mod}::Col::{col})"),
                format!("cache.{alias}.sort_by(|v1, v2| v1._inner.{col}.cmp(&v2._inner.{col}){list_order});"),
            )
    } else {
        let tmpl = format!("rel_{class_mod}::OrderBy::{asc}(rel_{class_mod}::Col::{{var}})");
        (
            fmt_join(&foreign_model.primaries(), &tmpl, ",").unwrap(),
            "".to_string(),
        )
    };
    let (limit, list_limit) = if let Some(limit) = rel.as_ref().and_then(|v| v.limit) {
        (
            format!(".limit({limit})"),
            format!("cache.{alias}.truncate({limit});"),
        )
    } else {
        ("".to_string(), "".to_string())
    };
    let (title, comment) = if let Some(ref v) = rel {
        (&v.title, &v.comment)
    } else {
        (&None, &None)
    };
    let mut constraint = String::new();
    let mut with_trashed = "";
    if let Some(ref v) = rel {
        constraint.push_str(match v.on_delete {
            Some(ReferenceOption::Restrict) => " ON DELETE RESTRICT",
            Some(ReferenceOption::Cascade) => " ON DELETE CASCADE",
            Some(ReferenceOption::SetNull) => " ON DELETE SET NULL",
            //                Some(ReferenceOption::NoAction) => " ON DELETE NO ACTION",
            Some(ReferenceOption::SetZero) => "",
            None => "",
        });
        constraint.push_str(match v.on_update {
            Some(ReferenceOption::Restrict) => " ON UPDATE RESTRICT",
            Some(ReferenceOption::Cascade) => " ON UPDATE CASCADE",
            Some(ReferenceOption::SetNull) => " ON UPDATE SET NULL",
            //                Some(ReferenceOption::NoAction) => " ON UPDATE NO ACTION",
            Some(ReferenceOption::SetZero) => "",
            None => "",
        });
        if v.use_cache_with_trashed {
            with_trashed = "_with_trashed";
        }
    }
    f.replace("{col}", &_to_db_col(&local, false))
        .replace("{col_esc}", &_to_db_col(&local_col_name, true))
        .replace("{var}", &_to_var_name(&local))
        .replace("{alias}", &_to_var_name(name))
        .replace("{raw_alias}", name)
        .replace("{alias_pascal}", &name.to_case(Case::Pascal))
        .replace("{class}", &RelDef::get_foreign_class_name(rel, name))
        .replace("{class_mod}", &RelDef::get_group_mod_name(rel, name))
        .replace("{mod_name}", &RelDef::get_mod_name(rel, name))
        .replace("{local_id}", local_id)
        .replace("{foreign}", &foreign)
        .replace("{foreign_esc}", &_to_db_col(&foreign, true))
        .replace("{foreign_var}", &_to_var_name(&foreign))
        .replace("{foreign_pascal}", &foreign.to_case(Case::Pascal))
        .replace("{local_table}", &model.table_name())
        .replace(
            "{table}",
            &_to_db_col(&RelDef::get_foreign_table_name(rel, name), true),
        )
        .replace("{raw_table}", &RelDef::get_foreign_table_name(rel, name))
        .replace("{index}", &index.to_string())
        .replace("{title}", &comment4(title).unwrap())
        .replace("{comment}", &comment4(comment).unwrap())
        .replace("{constraint}", &constraint)
        .replace("{primaries}", &primaries)
        .replace("{and_cond}", &and_cond)
        .replace("{order_by}", &order_by)
        .replace("{limit}", &limit)
        .replace("{list_sort}", &list_sort)
        .replace("{list_limit}", &list_limit)
        .replace("{pascal_name}", &model.name.to_case(Case::Pascal))
        .replace("{id_name}", &to_id_name(&model.name))
        .replace("{with_trashed}", with_trashed)
}

pub fn fmt_rel_join_not_null_or_null(
    v: &[(&ModelDef, &String, &Option<RelDef>)],
    not_null_case: &str,
    null_case: &str,
    sep: &str,
) -> ::askama::Result<String> {
    let mut index = -1;
    Ok(v.iter()
        .map(|(model, name, rel)| {
            let local = RelDef::get_local_id(rel, name, &model.id_name());
            let col = model
                .merged_columns
                .get(&local)
                .unwrap_or_else(|| panic!("{} column is not defined in {}", local, model.name));
            let f = if col.not_null {
                not_null_case
            } else {
                null_case
            };
            index += 1;
            _fmt_rel(f, rel, name, model, index)
        })
        .collect::<Vec<_>>()
        .join(sep))
}

#[allow(dead_code)]
pub fn fmt_index(v: &[(&ModelDef, &String, &IndexDef)], f: &str) -> ::askama::Result<String> {
    Ok(v.iter()
        .map(|(model, name, index)| _fmt_index(name, index, model, f))
        .collect::<Vec<_>>()
        .join(""))
}

#[allow(dead_code)]
pub fn fmt_index_not_null_or_null(
    v: &[(&ModelDef, &String, &IndexDef)],
    not_null_case: &str,
    null_case: &str,
) -> ::askama::Result<String> {
    Ok(v.iter()
        .map(|(model, name, index)| {
            let mut col_name = name.to_string();
            if !index.fields.is_empty() {
                for row in &index.fields {
                    col_name = row.0.clone();
                }
            }
            let col = model
                .merged_columns
                .get(&col_name)
                .unwrap_or_else(|| panic!("{} column is not defined in {}", col_name, model.name));
            if col.exclude_from_cache {
                panic!(
                    "Unique column cannot be excluded from cache: {} in {}",
                    col_name, model.name
                );
            }
            let f = if col.not_null {
                not_null_case
            } else {
                null_case
            };
            _fmt_index(name, index, model, f)
        })
        .collect::<Vec<_>>()
        .join(""))
}

fn _fmt_index(name: &&String, index: &&IndexDef, model: &&ModelDef, f: &str) -> String {
    let mut name = name.to_string();
    let mut prop_name = name.clone();
    let mut col_name = String::new();
    let mut v = Vec::new();
    let mut length = 0;
    let mut first = true;
    if !index.fields.is_empty() {
        for row in &index.fields {
            let col = model
                .merged_columns
                .get(row.0)
                .unwrap_or_else(|| panic!("{} index is not in columns", row.0));
            let col = col.get_col_name(row.0).to_string();
            let mut s = format!("`{}`", &col);
            if first {
                col_name = col;
                prop_name = row.0.clone();
                first = false;
            }
            if let Some(def) = row.1 {
                if let Some(len) = def.length {
                    length = len;
                    let _ = write!(s, "({})", len);
                }
                if let Some(sorting) = def.sorting {
                    match sorting {
                        SortType::Asc => s.push_str(" ASC"),
                        SortType::Desc => s.push_str(" DESC"),
                    }
                }
            }
            v.push(s);
        }
    } else {
        let col = model
            .merged_columns
            .get(&name)
            .unwrap_or_else(|| panic!("{} index is not in columns", name));
        col_name = col.get_col_name(&name).to_string();
        v.push(format!("`{}`", &col_name));
    }
    name = match index.type_def {
        Some(IndexType::Index) => format!("IDX_{}", name),
        Some(IndexType::Unique) => format!("UQ_{}", name),
        Some(IndexType::Fulltext) => format!("FT_{}", name),
        Some(IndexType::Spatial) => format!("SP_{}", name),
        None => format!("IDX_{}", name),
    };
    let index_type = match index.type_def {
        Some(IndexType::Index) => "INDEX",
        Some(IndexType::Unique) => "UNIQUE",
        Some(IndexType::Fulltext) => "FULLTEXT",
        Some(IndexType::Spatial) => "SPATIAL",
        None => "INDEX",
    };
    let mut cols = v.join(", ");
    let col = model.merged_columns.get(&prop_name);
    if col.unwrap().type_def == ColumnType::ArrayInt {
        cols = format!("(CAST(`{}` AS UNSIGNED ARRAY))", col_name);
    }
    if col.unwrap().type_def == ColumnType::ArrayString {
        cols = format!("(CAST(`{}` AS CHAR({}) ARRAY))", col_name, length);
    }
    let parser = if let Some(parser) = index.parser {
        format!(" WITH PARSER {}", parser)
    } else {
        "".to_string()
    };
    f.replace("{name}", &name)
        .replace("{index_type}", index_type)
        .replace("{cols}", &cols)
        .replace("{col_esc}", &_to_db_col(&col_name, true))
        .replace("{table_name}", &model.table_name())
        .replace("{pascal_name}", &model.name.to_case(Case::Pascal))
        .replace("{col_name}", &prop_name)
        .replace("{col_pascal}", &prop_name.to_case(Case::Pascal))
        .replace("{var}", &_to_var_name(&prop_name))
        .replace("{bind_as}", col.unwrap().get_bind_as())
        .replace(
            "{cond_type}",
            &col.map(|col| col.get_cond_type()).unwrap_or_default(),
        )
        .replace("{parser}", &parser)
}

pub fn fmt_index_col(v: &[(&String, &ColumnDef)], f: &str, sep: &str) -> ::askama::Result<String> {
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
    v: &[(&String, &ColumnDef)],
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
fn _fmt_index_col(name: &&String, col: &&ColumnDef, f: &str, index: i32) -> String {
    f.replace("{name}", name)
        .replace("{col_name}", &col.get_col_name(name))
        .replace("{col_esc}", &_to_db_col(&col.get_col_name(name), true))
        .replace("{col_pascal}", &name.to_case(Case::Pascal))
        .replace("{var}", &_to_var_name(name))
        .replace("{bind_as}", col.get_bind_as())
        .replace("{cond_type}", &col.get_cond_type())
        .replace("{index}", &index.to_string())
}

#[allow(dead_code)]
pub fn check_lifetime(s: &str) -> ::askama::Result<String> {
    Ok(if s.contains("&'a") { "<'a>" } else { "" }.to_string())
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
pub fn strum_props4(def: &ColumnDef) -> ::askama::Result<String> {
    Ok(format!(
        "    #[strum(props(def = {:?}))]\n",
        serde_json::to_string(def).unwrap()
    ))
}
#[allow(dead_code)]
pub fn auto_opt<T: std::fmt::Display>(s: &Option<T>) -> ::askama::Result<String> {
    match s {
        None => Ok("".to_owned()),
        Some(ref s) => Ok(format!("{}", s)),
    }
}
pub fn disp_opt<T: std::fmt::Debug>(s: &Option<T>) -> ::askama::Result<String> {
    match s {
        None => Ok("None".to_owned()),
        Some(ref s) => Ok(format!("Some({:?})", s)),
    }
}
pub fn if_then_else<T: std::fmt::Display>(wh: &bool, th: T, el: T) -> ::askama::Result<T> {
    Ok(if_then_else!(*wh, th, el))
}
#[allow(dead_code)]
pub fn is_true(b: &Option<bool>) -> ::askama::Result<bool> {
    Ok(*b == Some(true))
}
