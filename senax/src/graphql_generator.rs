use anyhow::{bail, ensure, Context, Result};
use askama::Template;
use convert_case::{Case, Casing};
use indexmap::IndexSet;
use regex::{Captures, Regex};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use crate::schema::{self, to_id_name, ModelDef, RelDef, _to_var_name, GROUPS, MODEL};

pub mod template;

pub fn generate(
    crate_path: &Path,
    db: &str,
    group: &Option<String>,
    model: &Option<String>,
    camel_case: bool,
    force: bool,
) -> Result<()> {
    schema::parse(db, true)?;

    let groups = unsafe { GROUPS.get().unwrap() }.clone();
    ensure!(
        crate_path.exists() && crate_path.is_dir(),
        "The crate path does not exist."
    );
    let base_path = crate_path.join("src").join("graphql");

    let group_names = if let Some(group) = group {
        ensure!(
            groups.contains_key(group),
            format!("The {db} db does not have {group} group.")
        );
        vec![group.clone()]
    } else {
        groups.iter().map(|(v, _)| v.clone()).collect()
    };
    let mut db_file_group_names = Vec::new();
    let db_path = base_path.join(db);
    fs::create_dir_all(&db_path)?;
    for group_name in &group_names {
        let group = groups.get(group_name).unwrap();
        let model_names = if let Some(model) = model {
            if let Some(def) = group.get(model) {
                write_model_file(
                    &db_path.join(group_name),
                    db,
                    group_name,
                    model,
                    def,
                    camel_case,
                    force,
                )?;
                vec![def.mod_name()]
            } else if let Some(tuple) = group.iter().find(|(_, v)| v.mod_name() == model) {
                write_model_file(
                    &db_path.join(group_name),
                    db,
                    group_name,
                    tuple.0,
                    tuple.1,
                    camel_case,
                    force,
                )?;
                vec![tuple.1.mod_name()]
            } else {
                bail!(format!(
                    "The {group_name} group does not have {model} model."
                ));
            }
        } else {
            let model_list: Vec<_> = group.iter().filter(|v| !v.1.abstract_mode).collect();
            for (name, def) in &model_list {
                write_model_file(
                    &db_path.join(group_name),
                    db,
                    group_name,
                    name,
                    def,
                    camel_case,
                    force,
                )?;
            }
            model_list.iter().map(|(_, v)| v.mod_name()).collect()
        };
        if !model_names.is_empty() {
            write_group_file(&db_path, db, group_name, &model_names)?;
            db_file_group_names.push(group_name.clone());
        }
    }
    write_db_file(&base_path, db, &db_file_group_names)?;
    Ok(())
}

fn write_db_file(path: &Path, db: &str, group_names: &[String]) -> Result<(), anyhow::Error> {
    let file_path = path.join(&format!("{}.rs", db));
    if !file_path.exists() {
        let tpl = template::DbTemplate { db };
        fs_write(&file_path, tpl.render()?)?;
    }
    let content = fs::read_to_string(&file_path)?;
    let re = Regex::new(r"\n// Do not modify this line\. \(GqiMod:([_a-zA-Z0-9,]*)\)").unwrap();
    let caps = re
        .captures(&content)
        .with_context(|| format!("Illegal file content:{}", &file_path.to_string_lossy()))?;
    let mut all: BTreeSet<String> = caps
        .get(1)
        .unwrap()
        .as_str()
        .split(',')
        .filter(|v| !v.is_empty())
        .map(|v| v.to_string())
        .collect();
    let add_groups: BTreeSet<String> = group_names
        .iter()
        .filter(|v| !all.contains(*v))
        .map(|v| v.to_string())
        .collect();
    if !add_groups.is_empty() {
        let add_groups = &add_groups;
        add_groups.iter().for_each(|v| {
            all.insert(v.clone());
        });
        let all = all.iter().cloned().collect::<Vec<_>>().join(",");
        let tpl = template::DbModTemplate { all, add_groups };
        let content = re.replace(&content, tpl.render()?);
        let tpl = template::DbQueryTemplate { db, add_groups };
        let content = content.replace(
            "\n    // Do not modify this line. (GqiQuery)",
            &tpl.render()?,
        );
        let tpl = template::DbMutationTemplate { db, add_groups };
        let content = content.replace(
            "\n    // Do not modify this line. (GqiMutation)",
            &tpl.render()?,
        );
        println!("{}", file_path.display());
        fs_write(file_path, &*content)?;
    }
    Ok(())
}

fn write_group_file(
    path: &Path,
    db: &str,
    group: &str,
    model_names: &[&str],
) -> Result<(), anyhow::Error> {
    let file_path = path.join(&format!("{}.rs", group));
    if !file_path.exists() {
        let tpl = template::GroupTemplate { db, group };
        fs_write(&file_path, tpl.render()?)?;
    }
    let content = fs::read_to_string(&file_path)?;
    let re = Regex::new(r"\n// Do not modify this line\. \(GqiMod:([_a-zA-Z0-9,]*)\)").unwrap();
    let caps = re
        .captures(&content)
        .with_context(|| format!("Illegal file content:{}", &file_path.to_string_lossy()))?;
    let mut all: BTreeSet<String> = caps
        .get(1)
        .unwrap()
        .as_str()
        .split(',')
        .filter(|v| !v.is_empty())
        .map(|v| v.to_string())
        .collect();
    let add_models: BTreeSet<String> = model_names
        .iter()
        .filter(|v| !all.contains(**v))
        .map(|v| v.to_string())
        .collect();
    if !add_models.is_empty() {
        let add_models = &add_models;
        add_models.iter().for_each(|v| {
            all.insert(v.clone());
        });
        let all = all.iter().cloned().collect::<Vec<_>>().join(",");
        let tpl = template::GroupModTemplate { all, add_models };
        let content = re.replace(&content, tpl.render()?);
        let tpl = template::GroupImplTemplate {
            db,
            group,
            add_models,
            mode: "Query",
        };
        let content = content.replace(
            "\n    // Do not modify this line. (GqiQuery)",
            &tpl.render()?,
        );
        let tpl = template::GroupImplTemplate {
            db,
            group,
            add_models,
            mode: "Mutation",
        };
        let content = content.replace(
            "\n    // Do not modify this line. (GqiMutation)",
            &tpl.render()?,
        );
        println!("{}", file_path.display());
        fs_write(file_path, &*content)?;
    }
    Ok(())
}

fn write_model_file(
    path: &Path,
    db: &str,
    group: &str,
    model_name: &str,
    def: &Arc<ModelDef>,
    camel_case: bool,
    force: bool,
) -> Result<(), anyhow::Error> {
    unsafe {
        MODEL.take();
        MODEL.set(def.clone()).unwrap();
    }
    let mod_name = def.mod_name();
    let pascal_name = &model_name.to_case(Case::Pascal);
    fs::create_dir_all(path)?;
    let file_path = path.join(&format!("{}.rs", mod_name));
    if force || !file_path.exists() {
        let tpl = template::ModelTemplate {
            db,
            group,
            mod_name,
            name: model_name,
            pascal_name: &model_name.to_case(Case::Pascal),
            id_name: &to_id_name(model_name),
            def,
        };
        fs_write(&file_path, tpl.render()?)?;
    }
    let content = fs::read_to_string(&file_path)?;
    let re = Regex::new(r"(?s)(// Do not modify this line\. \(GqiModelBegin\)).+(// Do not modify this line\. \(GqiModelEnd\))").unwrap();
    println!("{}", file_path.display());
    ensure!(re.is_match(&content), "File contents are invalid.");
    let mut relation_mods: IndexSet<String> = IndexSet::new();
    relation_mods.insert(format!(
        "\n#[allow(unused_imports)]\nuse db_{}::{}::{}::{{self as rel_{}_{}, *}};",
        db,
        _to_var_name(group),
        _to_var_name(mod_name),
        group,
        mod_name
    ));
    for mod_name in &def.relation_mods() {
        relation_mods.insert(format!(
            "\n#[allow(unused_imports)]\nuse db_{}::{}::{}::{{self as rel_{}_{}, *}};",
            db,
            _to_var_name(&mod_name[0]),
            _to_var_name(&mod_name[1]),
            mod_name[0],
            mod_name[1]
        ));
    }
    let mut buf = template::BaseModelTemplate {
        db,
        group,
        mod_name,
        pascal_name,
        def,
        camel_case,
    }
    .render()?;
    for (_model, rel_name, rel) in def.relations_one_cache() {
        let rel_model = RelDef::get_foreign_model(rel, rel_name);
        unsafe {
            MODEL.take();
            MODEL.set(rel_model.clone()).unwrap();
        }
        for mod_name in &rel_model.relation_mods() {
            relation_mods.insert(format!(
                "\n#[allow(unused_imports)]\nuse db_{}::{}::{}::{{self as rel_{}_{}, *}};",
                db, mod_name[0], mod_name[1], mod_name[0], mod_name[1]
            ));
        }
        let pascal_name = &rel_model.name.to_case(Case::Pascal);
        let class_mod = &RelDef::get_group_mod_name(rel, rel_name);
        let rel_id = &RelDef::get_foreign_id(rel, def, &rel_model);
        buf.push_str(
            &template::RelationTemplate {
                db,
                group,
                mod_name,
                rel_name,
                rel_id,
                pascal_name,
                class_mod,
                def: &rel_model,
            }
            .render()?,
        );
    }
    for (_model, rel_name, rel) in def.relations_many_cache() {
        let rel_model = RelDef::get_foreign_model(rel, rel_name);
        unsafe {
            MODEL.take();
            MODEL.set(rel_model.clone()).unwrap();
        }
        for mod_name in &rel_model.relation_mods() {
            relation_mods.insert(format!(
                "\n#[allow(unused_imports)]\nuse db_{}::{}::{}::{{self as rel_{}_{}, *}};",
                db, mod_name[0], mod_name[1], mod_name[0], mod_name[1]
            ));
        }
        let pascal_name = &rel_model.name.to_case(Case::Pascal);
        let class_mod = &RelDef::get_group_mod_name(rel, rel_name);
        let rel_id = &RelDef::get_foreign_id(rel, def, &rel_model);
        buf.push_str(
            &template::RelationTemplate {
                db,
                group,
                mod_name,
                rel_name,
                rel_id,
                pascal_name,
                class_mod,
                def: &rel_model,
            }
            .render()?,
        );
    }
    for (_model, rel_name, rel) in def.relations_one_only_cache() {
        let rel_model = RelDef::get_foreign_model(rel, rel_name);
        unsafe {
            MODEL.take();
            MODEL.set(rel_model.clone()).unwrap();
        }
        for mod_name in &rel_model.relation_mods() {
            relation_mods.insert(format!(
                "\n#[allow(unused_imports)]\nuse db_{}::{}::{}::{{self as rel_{}_{}, *}};",
                db, mod_name[0], mod_name[1], mod_name[0], mod_name[1]
            ));
        }
        let pascal_name = &rel_model.name.to_case(Case::Pascal);
        let class_mod = &RelDef::get_group_mod_name(rel, rel_name);
        buf.push_str(
            &template::ReferenceTemplate {
                db,
                group,
                mod_name,
                rel_name,
                pascal_name,
                class_mod,
                def: &rel_model,
            }
            .render()?,
        );
    }
    let relation_mods = relation_mods
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join("");
    let msg = "\n// Internal can be modified. However, it will be overwritten by auto-generation.";
    let content = re.replace(&content, |caps: &Captures| {
        format!(
            "{}{}{}\n{}{}",
            &caps[1], msg, &relation_mods, &buf, &caps[2]
        )
    });
    fs_write(file_path, &*content)?;
    Ok(())
}

fn fs_write<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> Result<()> {
    fn inner(path: &Path, contents: &[u8]) -> Result<()> {
        if let Ok(buf) = fs::read(path) {
            if !buf.eq(contents) {
                fs::write(path, contents)?;
            }
        } else {
            fs::write(path, contents)?;
        }
        Ok(())
    }
    inner(path.as_ref(), contents.as_ref())
}
