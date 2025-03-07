use anyhow::{ensure, Context, Result};
use askama::Template;
use convert_case::{Case, Casing};
use indexmap::IndexMap;
use regex::{Captures, Regex};
use std::collections::{BTreeSet, HashSet};
use std::ffi::OsString;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Arc;

use crate::api_generator::schema::{ApiConfigDef, ApiDbDef, ApiFieldDef, ApiModelDef, API_CONFIG};
use crate::api_generator::template::{
    ConfigTemplate, DbConfigTemplate, MutationRootTemplate, QueryRootTemplate,
};
use crate::common::{fs_write, parse_yml_file, simplify_yml};
use crate::schema::{to_id_name, ModelDef, _to_var_name, GROUPS};
use crate::{model_generator, API_SCHEMA_PATH};

use self::schema::{ApiRelationDef, RelationVisibility};

pub mod schema;
pub mod serialize;
pub mod template;

#[allow(clippy::too_many_arguments)]
pub fn generate(
    server_path: &Path,
    db_path: &str,
    group_path: &Option<String>,
    model_path: &Option<String>,
    ts_dir: &Option<PathBuf>,
    inquiry: bool,
    force: bool,
    clean: bool,
) -> Result<()> {
    ensure!(
        server_path.exists() && server_path.is_dir(),
        "The crate path does not exist."
    );

    let schema_dir = server_path.join(API_SCHEMA_PATH);
    let db_config_path = schema_dir.join(format!("{db_path}.yml"));
    let db = if db_config_path.exists() {
        let db_config: ApiDbDef = parse_yml_file(&db_config_path)?;
        db_config.db.clone().unwrap_or(db_path.to_string())
    } else {
        db_path.to_string()
    };

    model_generator::check_version(&db)?;
    crate::schema::parse(&db, true, false)?;
    crate::schema::set_domain_mode(true);
    let groups = GROUPS.read().unwrap().as_ref().unwrap().clone();

    fs::create_dir_all(&schema_dir)?;

    let config_path = schema_dir.join("_config.yml");
    if !config_path.exists() {
        let tpl = ConfigTemplate;
        fs_write(&config_path, tpl.render()?)?;
    }
    let config: ApiConfigDef = parse_yml_file(&config_path)?;
    API_CONFIG.write().unwrap().replace(config.clone());

    if !db_config_path.exists() {
        let tpl = DbConfigTemplate;
        fs_write(&db_config_path, tpl.render()?)?;
    }
    let mut db_config: ApiDbDef = parse_yml_file(&db_config_path)?;
    db_config.fix();
    model_generator::template::filters::SHOW_LABEL.store(db_config.with_label(), Ordering::SeqCst);
    model_generator::template::filters::SHOW_COMMNET
        .store(db_config.with_comment(), Ordering::SeqCst);

    let src_path = server_path.join("src");
    let file_path = src_path.join("auto_api.rs");
    let mut content = fs::read_to_string(&file_path)
        .with_context(|| format!("Cannot read file: {:?}", &file_path))?;
    let db_var_name = _to_var_name(&db_path.to_case(Case::Snake));
    let reg = Regex::new(&format!(r"pub mod {};", db_var_name))?;
    if !reg.is_match(&content) {
        content = content.replace(
            "// Do not modify this line. (ApiDbMod)",
            &format!(
                "pub mod {};\n// Do not modify this line. (ApiDbMod)",
                db_var_name
            ),
        );
        content = content.replace(
            "// Do not modify this line. (ApiRouteConfig)",
            &format!(
                "cfg.service(scope(\"/{}\").configure({}::route_config));\n    // Do not modify this line. (ApiRouteConfig)",
                &db_path,
                db_var_name,
            ),
        );
        content = content.replace(
            "    // Do not modify this line. (ApiJsonSchema)",
            &format!("    {}::gen_json_schema(&dir.join(\"{}\"))?;\n    // Do not modify this line. (ApiJsonSchema)", db_var_name, &db_path.to_case(Case::Snake)),
        );
        let tpl = QueryRootTemplate {
            db_path,
            camel_case: db_config.camel_case(),
        };
        content = content.replace("impl QueryRoot {", tpl.render()?.trim_start());
        let tpl = MutationRootTemplate {
            db_path,
            camel_case: db_config.camel_case(),
        };
        content = content.replace("impl MutationRoot {", tpl.render()?.trim_start());
        fs_write(file_path, &*content)?;
    }

    let file_path = src_path.join("auth.rs");
    let content = fs::read_to_string(&file_path)
        .with_context(|| format!("Cannot read file: {:?}", &file_path))?;
    let re = Regex::new(r"(?s)// Do not modify below this line. \(RoleStart\).+// Do not modify up to this line. \(RoleEnd\)").unwrap();
    ensure!(
        re.is_match(&content),
        "File contents are invalid.: {:?}",
        &file_path
    );
    let buf = if config.default_role.is_none() {
        String::from("    #[display(fmt = \"\")]\n    #[default]\n    _None,\n")
    } else {
        String::new()
    };
    let roles = config.roles.iter().fold(buf, |mut buf, role| {
        if let Some(dflt) = &config.default_role {
            if dflt == role.0 {
                buf.push_str("    #[default]\n");
            }
        }
        if let Some(def) = role.1 {
            if let Some(alias) = &def.alias {
                writeln!(&mut buf, "    #[display(fmt = {:?})]", alias).unwrap();
                writeln!(&mut buf, "    #[serde(rename = {:?})]", alias).unwrap();
            }
        }
        writeln!(&mut buf, "    {},", _to_var_name(role.0)).unwrap();
        buf
    });
    let tpl = format!("// Do not modify below this line. (RoleStart)\n{roles}    // Do not modify up to this line. (RoleEnd)");
    let content = re.replace(&content, tpl);

    let re = Regex::new(r"(?s)// Do not modify below this line. \(ImplRoleStart\).+// Do not modify up to this line. \(ImplRoleEnd\)").unwrap();
    ensure!(
        re.is_match(&content),
        "File contents are invalid.: {:?}",
        &file_path
    );
    let roles = config.roles.iter().fold(String::new(), |mut buf, role| {
        write!(
            &mut buf,
            "    pub fn is_{}(&self) -> bool {{\n        self == &Self::{}\n    }}\n",
            role.0,
            _to_var_name(role.0)
        )
        .unwrap();
        buf
    });
    let tpl = format!("// Do not modify below this line. (ImplRoleStart)\n{roles}    // Do not modify up to this line. (ImplRoleEnd)");
    let content = re.replace(&content, tpl);

    fs_write(file_path, &*content)?;

    let schema_dir = schema_dir.join(db_path);
    fs::create_dir_all(&schema_dir)?;

    let ts_dir = if let Some(ts_dir) = ts_dir {
        if ts_dir.is_dir() {
            Some(
                ts_dir
                    .join("src")
                    .join("gql_query")
                    .join(db_path.to_case(Case::Snake)),
            )
        } else {
            eprintln!("The ts-dir directory does not exist.: {}", ts_dir.display());
            None
        }
    } else {
        None
    };
    if let Some(ts_dir) = &ts_dir {
        if ts_dir.exists() {
            fs::remove_dir_all(ts_dir)?;
        }
    }

    let group_paths = if let Some(group) = group_path {
        vec![group.clone()]
    } else if db_config.groups.is_empty() {
        groups
            .iter()
            .filter(|(v, _)| inquiry || (schema_dir.join(format!("{}.yml", v)).exists()))
            .map(|(v, _)| v.clone())
            .collect()
    } else {
        db_config.groups.iter().map(|(v, _)| v.clone()).collect()
    };
    let base_path = src_path.join("auto_api");
    let mut db_file_group_names = Vec::new();
    let db_base_path = base_path.join(db_path.to_case(Case::Snake));
    let mut remove_files = HashSet::new();
    if clean && db_base_path.exists() {
        for entry in glob::glob(&format!("{}/**/*.rs", db_base_path.display()))? {
            match entry {
                Ok(path) => remove_files.insert(path.as_os_str().to_owned()),
                Err(_) => false,
            };
        }
    }
    fs::create_dir_all(&db_base_path)?;
    for group_path in &group_paths {
        let schema_path = schema_dir.join(&format!("{group_path}.yml"));
        let mut api_model_map: IndexMap<String, Option<ApiModelDef>> = if schema_path.exists() {
            parse_yml_file(&schema_path)?
        } else {
            IndexMap::default()
        };
        for (_, def) in api_model_map.iter_mut() {
            if let Some(v) = def.as_mut() {
                v.fix()
            }
        }
        let mut update_group_def = false;

        let group_name = if let Some(Some(api_group_def)) = db_config.groups.get(group_path) {
            api_group_def.group.as_ref().unwrap_or(group_path)
        } else {
            group_path
        };
        let group = groups
            .get(group_name)
            .unwrap_or_else(|| panic!("The {db} DB does not have {group_name} group."));
        let group_path_mod_name = group_path.to_case(Case::Snake);
        let group_mod_name = group_name.to_case(Case::Snake);

        let model_paths = if let Some(model_path) = model_path {
            vec![model_path.clone()]
        } else if inquiry {
            group
                .iter()
                .filter(|(_, def)| !def.abstract_mode)
                .map(|(v, _)| v.clone())
                .collect()
        } else {
            api_model_map.iter().map(|(v, _)| v.clone()).collect()
        };
        // let mut model_path_names = Vec::new();

        for model_path in &model_paths {
            if api_model_map.get(model_path).is_none()
                && inquiry
                && !dialoguer::Confirm::new()
                    .with_prompt(format!("Add an API for the {} model?", model_path))
                    .default(true)
                    .interact()?
            {
                continue;
            }
            let model_name = if let Some(Some(api_model)) = api_model_map.get(model_path) {
                api_model.model.as_ref().unwrap_or(model_path)
            } else {
                model_path
            };
            let def = group.get(model_name).unwrap_or_else(|| {
                panic!("The {group_name} group does not have {model_name} model.")
            });

            let api_def = write_model_file(
                &db_base_path.join(&group_path_mod_name),
                &db,
                db_path,
                &group_mod_name,
                &group_path_mod_name,
                model_name,
                model_path,
                def,
                api_model_map
                    .get(model_path)
                    .cloned()
                    .map(|v| v.unwrap_or_default()),
                &db_config,
                inquiry,
                force,
                &ts_dir,
                &mut remove_files,
            )?;
            // model_path_names.push(&def.name);
            if !api_model_map.contains_key(model_path) {
                if api_def == ApiModelDef::default() {
                    api_model_map.insert(model_path.clone(), None);
                } else {
                    api_model_map.insert(model_path.clone(), Some(api_def));
                }
                update_group_def = true;
            }
        }
        if !model_paths.is_empty() {
            write_group_file(
                &db_base_path,
                db_path,
                &group_path_mod_name,
                &model_paths,
                db_config.camel_case(),
                force || clean,
                &mut remove_files,
            )?;
            db_file_group_names.push(group_path.clone());
        }
        if !schema_path.exists() || update_group_def {
            let mut buf = "# yaml-language-server: $schema=../../../senax-schema.json#properties/api_model\n\n".to_string();
            buf.push_str(&simplify_yml(serde_yaml::to_string(&api_model_map)?)?);
            fs_write(schema_path, &buf)?;
        }
        if !db_config.groups.contains_key(group_path) {
            db_config.groups.insert(group_path.to_string(), None);
            let mut buf =
                "# yaml-language-server: $schema=../../senax-schema.json#definitions/ApiDbDef\n\n"
                    .to_string();
            buf.push_str(&simplify_yml(serde_yaml::to_string(&db_config)?)?);
            fs_write(&db_config_path, &buf)?;
        }
    }
    write_db_file(
        &base_path,
        &db,
        db_path,
        &db_file_group_names,
        force || clean,
        &db_config,
    )?;
    for file in &remove_files {
        println!("REMOVE:{}", file.to_string_lossy());
        fs::remove_file(file)?;
    }
    Ok(())
}

fn write_db_file(
    path: &Path,
    db: &str,
    db_path: &str,
    group_names: &[String],
    force: bool,
    config: &ApiDbDef,
) -> Result<()> {
    let camel_case = config.camel_case();
    let file_path = path.join(format!("{}.rs", &db_path.to_case(Case::Snake)));
    let content = if force || !file_path.exists() {
        template::DbTemplate { db, db_path }.render()?
    } else {
        fs::read_to_string(&file_path)?
    };
    let re = Regex::new(r"\n// Do not modify this line\. \(GqlMod:([_a-zA-Z0-9,]*)\)").unwrap();
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
    let add_groups = &add_groups;
    add_groups.iter().for_each(|v| {
        all.insert(v.clone());
    });
    let all = all.iter().cloned().collect::<Vec<_>>().join(",");
    let tpl = template::DbModTemplate { all, add_groups };
    let content = re.replace(&content, tpl.render()?);
    let tpl = template::DbQueryTemplate {
        db_path,
        add_groups,
        camel_case,
    };
    let mut content = content.replace(
        "\n    // Do not modify this line. (GqlQuery)",
        &tpl.render()?,
    );
    for group in add_groups {
        let tpl = template::DbMutationTemplate {
            db_path,
            name: group,
            camel_case,
        };
        content = content.replace(
            "\n    // Do not modify this line. (GqlMutation)",
            &tpl.render()?,
        );
        content = content.replace(
            "// Do not modify this line. (ApiRouteConfig)",
            &format!(
                "cfg.service(scope(\"/{}\").configure({}::route_config));\n    // Do not modify this line. (ApiRouteConfig)",
                &group,
                _to_var_name(&group.to_case(Case::Snake)),
            ),
        );
    }
    let tpl = template::DbJsonSchemaTemplate { add_groups };
    let content = content.replace(
        "\n    // Do not modify this line. (JsonSchema)",
        &tpl.render()?,
    );
    fs_write(file_path, &*content)?;
    Ok(())
}

fn write_group_file(
    path: &Path,
    db: &str,
    group: &str,
    model_paths: &[String],
    camel_case: bool,
    force: bool,
    remove_files: &mut HashSet<OsString>,
) -> Result<()> {
    let file_path = path.join(format!("{}.rs", group));
    remove_files.remove(file_path.as_os_str());
    let content = if force || !file_path.exists() {
        template::GroupTemplate { db, group }.render()?
    } else {
        fs::read_to_string(&file_path)?
    };
    let re = Regex::new(r"\n// Do not modify this line\. \(GqlMod:([_a-zA-Z0-9,]*)\)").unwrap();
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
    let add_models: BTreeSet<String> = model_paths
        .iter()
        .filter(|v| !all.contains(*v))
        .map(|v| v.to_string())
        .collect();
    let add_models = &add_models;
    add_models.iter().for_each(|v| {
        all.insert(v.clone());
    });
    let all = all.iter().cloned().collect::<Vec<_>>().join(",");
    let tpl = template::GroupModTemplate { all, add_models };
    let mut content = re.replace(&content, tpl.render()?).to_string();
    let tpl = template::GroupImplTemplate {
        db,
        group,
        add_models,
        mode: "Query",
        camel_case,
    };
    content = content.replace(
        "\n    // Do not modify this line. (GqlQuery)",
        &tpl.render()?,
    );
    let tpl = template::GroupImplTemplate {
        db,
        group,
        add_models,
        mode: "Mutation",
        camel_case,
    };
    content = content.replace(
        "\n    // Do not modify this line. (GqlMutation)",
        &tpl.render()?,
    );
    for model in add_models.iter() {
        content = content.replace(
            "// Do not modify this line. (ApiRouteConfig)",
            &format!(
                "cfg.service(scope(\"/{}\").configure({}::_route_config));\n    // Do not modify this line. (ApiRouteConfig)",
                &model,
                _to_var_name(&model.to_case(Case::Snake)),
            ),
        );
    }
    let tpl = template::GroupJsonSchemaTemplate { add_models };
    content = content.replace(
        "\n    // Do not modify this line. (JsonSchema)",
        &tpl.render()?,
    );
    fs_write(file_path, &*content)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn write_model_file(
    path: &Path,
    db: &str,
    db_path: &str,
    group: &str,
    group_path: &str,
    model_name: &str,
    model_path: &str,
    def: &Arc<ModelDef>,
    api_def: Option<ApiModelDef>,
    config: &ApiDbDef,
    inquiry: bool,
    force: bool,
    ts_dir: &Option<PathBuf>,
    remove_files: &mut HashSet<OsString>,
) -> Result<ApiModelDef> {
    let api_def = if let Some(api_def) = api_def {
        api_def.clone()
    } else {
        let mut rel_list = Vec::new();
        ApiModelDef {
            relations: if inquiry {
                inquire_relation(model_name, def, &mut rel_list)?
            } else {
                Default::default()
            },
            ..Default::default()
        }
    };

    let mod_name = def.mod_name();
    let mod_name = &mod_name;
    let model_path_mod_name = model_path.to_case(Case::Snake);
    let pascal_name = &model_name.to_case(Case::Pascal);
    let graphql_name = &format!(
        "{}{}{}",
        db_path.to_case(Case::Pascal),
        group_path.to_case(Case::Pascal),
        model_path.to_case(Case::Pascal)
    );
    fs::create_dir_all(path)?;
    let file_path = path.join(format!("{}.rs", &model_path_mod_name));
    remove_files.remove(file_path.as_os_str());
    let content = if force || !file_path.exists() {
        template::ModelTemplate {
            db,
            db_path,
            group,
            group_path,
            mod_name,
            pascal_name: &model_name.to_case(Case::Pascal),
            graphql_name,
            id_name: &to_id_name(model_name),
            def,
            camel_case: config.camel_case(),
            api_def: &api_def,
        }
        .render()?
    } else {
        fs::read_to_string(&file_path)?
    };
    let re = Regex::new(r"(?s)(// Do not modify below this line. \(GqlModelStart\)).+(// Do not modify up to this line. \(GqlModelEnd\))").unwrap();
    ensure!(
        re.is_match(&content),
        "File contents are invalid.: {:?}",
        &file_path
    );

    ApiRelationDef::push(api_def.relations(def)?);
    ApiFieldDef::push(api_def.fields(def, config)?);

    let mut gql_fields = make_gql_fields(def, config.camel_case());
    let mut buf = template::BaseModelTemplate {
        db,
        group,
        mod_name,
        model_name,
        pascal_name,
        graphql_name,
        config,
        def,
        camel_case: config.camel_case(),
        api_def: &api_def,
    }
    .render()?;
    write_relation(
        def,
        &mut buf,
        db,
        graphql_name,
        config.camel_case(),
        0,
        false,
        false,
        &mut gql_fields,
        &api_def,
    )?;
    ApiRelationDef::pop();
    ApiFieldDef::pop();
    let msg = "\n// From here to the GqlModelEnd line is overwritten by automatic generation.\n";
    let content = re.replace_all(&content, |caps: &Captures| {
        format!("{}{}{}\n{}", &caps[1], msg, &buf, &caps[2])
    });

    fs_write(file_path, &*content)?;

    if let Some(ts_dir) = ts_dir {
        let ts_dir = ts_dir.join(group_path);
        fs::create_dir_all(&ts_dir)?;
        let file_path = ts_dir.join(format!("{}.tsx", model_name));
        use inflector::Inflector;
        let db_case = if config.camel_case() {
            db_path.to_camel_case()
        } else {
            db_path.to_string()
        };
        let group_case = if config.camel_case() {
            group_path.to_camel_case()
        } else {
            group_path.to_string()
        };
        let model_case = if config.camel_case() {
            model_path.to_camel_case()
        } else {
            model_path.to_string()
        };
        let tpl = template::ModelTsTemplate {
            path: format!("{}_{}_{}", db_path, group_path, model_path),
            model_path,
            curly_begin: format!("{}{{{}{{{}", db_case, group_case, model_case),
            curly_end: "}}",
            pascal_name: format!("{}{}{}", &db.to_case(Case::Pascal), &group.to_case(Case::Pascal), &model_name.to_case(Case::Pascal)),
            graphql_name,
            id_name: &to_id_name(model_name),
            def,
            gql_fields: gql_fields.join(","),
            api_def: &api_def,
        };
        fs_write(file_path, tpl.render()?)?;
    }
    Ok(api_def)
}

fn make_gql_fields(def: &ModelDef, camel_case: bool) -> Vec<String> {
    let mut gql_fields = vec!["_id".to_string()];
    let conv_case = if camel_case {
        |v: &str| v.to_case(Case::Camel)
    } else {
        |v: &str| v.to_string()
    };
    for (name, col) in def.for_api_response() {
        gql_fields.push(format!("{}{}", conv_case(name), col.gql_type()));
    }
    gql_fields
}

fn inquire_relation(
    model_name: &str,
    def: &Arc<ModelDef>,
    rel_list: &mut Vec<String>,
) -> Result<schema::Relations> {
    let mut items = Vec::new();
    for (_, rel_name, _) in def.relations_one(false) {
        items.push(rel_name);
    }
    for (_, rel_name, _) in def.relations_many(false) {
        items.push(rel_name);
    }
    for (_, rel_name, _) in def.relations_belonging(false) {
        items.push(rel_name);
    }
    if items.is_empty() {
        return Ok(IndexMap::default());
    }
    let prompt = if rel_list.is_empty() {
        format!("Select the {} model relations", model_name)
    } else {
        format!(
            "Select the {}({}) model relations",
            model_name,
            rel_list.join("->")
        )
    };
    let selections: Vec<usize> = dialoguer::MultiSelect::new()
        .with_prompt(&prompt)
        .items(&items)
        .interact()?;
    let mut selected = HashSet::new();
    for i in selections {
        selected.insert(items[i].clone());
    }

    let mut relations = IndexMap::default();
    let mut closure =
        |rel_name: &String, rel: &crate::schema::RelDef| -> Result<Option<ApiRelationDef>> {
            let rel_model = rel.get_foreign_model();
            rel_list.push(rel_name.clone());
            let api_def = ApiRelationDef {
                relations: inquire_relation(model_name, &rel_model, rel_list)?,
                ..Default::default()
            };
            rel_list.pop();
            if api_def == ApiRelationDef::default() {
                Ok(None)
            } else {
                Ok(Some(api_def))
            }
        };
    for (_, rel_name, rel) in def.relations_one(false) {
        if !selected.contains(rel_name) {
            continue;
        }
        relations.insert(rel_name.clone(), closure(rel_name, rel)?);
    }
    for (_, rel_name, rel) in def.relations_many(false) {
        if !selected.contains(rel_name) {
            continue;
        }
        relations.insert(rel_name.clone(), closure(rel_name, rel)?);
    }
    for (_, rel_name, rel) in def.relations_belonging(false) {
        if !selected.contains(rel_name) {
            continue;
        }
        relations.insert(rel_name.clone(), closure(rel_name, rel)?);
    }
    Ok(relations)
}

#[allow(clippy::too_many_arguments)]
fn write_relation(
    def: &Arc<ModelDef>,
    buf: &mut String,
    db: &str,
    graphql_name: &str,
    camel_case: bool,
    indent: usize,
    no_read: bool,
    no_update: bool,
    gql_fields: &mut Vec<String>,
    api_def: &ApiModelDef,
) -> Result<()> {
    let mut relation_buf = String::new();
    for (_model, rel_name, rel) in def.relations_one(false) {
        let rel_model = rel.get_foreign_model();
        let api_relation = ApiRelationDef::get(rel_name).unwrap();
        let rel_id = &rel.get_foreign_id(def);
        ApiRelationDef::push(api_relation.relations(&rel_model)?);
        ApiFieldDef::push(api_relation.fields(&rel_model, rel_id)?);
        let pascal_name = &rel_model.name.to_case(Case::Pascal);
        let graphql_name = &format!("{}{}", graphql_name, rel_name.to_case(Case::Pascal));
        relation_buf.push_str(&format!("\n#[rustfmt::skip]\nmod _{} {{\n    ", rel_name));
        relation_buf.push_str(
            &template::RelationTemplate {
                db,
                graphql_name,
                rel_name,
                rel_id,
                pascal_name,
                def: &rel_model,
                camel_case,
                rel_mod: rel.get_group_mod_var(),
                has_many: false,
                no_read: no_read || api_relation.visibility == Some(RelationVisibility::WriteOnly),
                no_update: no_update
                    || api_relation.visibility == Some(RelationVisibility::ReadOnly),
                replace: api_relation.use_replace,
                api_def,
            }
            .render()?
            .replace('\n', "\n    "),
        );
        let mut rel_fields = make_gql_fields(&rel_model, camel_case);
        write_relation(
            &rel_model,
            &mut relation_buf,
            db,
            graphql_name,
            camel_case,
            4,
            no_read,
            no_update,
            &mut rel_fields,
            api_def,
        )?;
        if !(no_read || api_relation.visibility == Some(RelationVisibility::WriteOnly)) {
            gql_fields.push(format!("{}{{{}}}", rel_name, rel_fields.join(",")));
        }
        ApiRelationDef::pop();
        ApiFieldDef::pop();
        relation_buf.push_str("\n}");
    }
    for (_model, rel_name, rel) in def.relations_many(false) {
        let rel_model = rel.get_foreign_model();
        let api_relation = ApiRelationDef::get(rel_name).unwrap();
        let rel_id = &rel.get_foreign_id(def);
        ApiRelationDef::push(api_relation.relations(&rel_model)?);
        ApiFieldDef::push(api_relation.fields(&rel_model, rel_id)?);
        let pascal_name = &rel_model.name.to_case(Case::Pascal);
        let graphql_name = &format!("{}{}", graphql_name, rel_name.to_case(Case::Pascal));
        relation_buf.push_str(&format!("\n#[rustfmt::skip]\nmod _{} {{\n    ", rel_name));
        relation_buf.push_str(
            &template::RelationTemplate {
                db,
                graphql_name,
                rel_name,
                rel_id,
                pascal_name,
                def: &rel_model,
                camel_case,
                rel_mod: rel.get_group_mod_var(),
                has_many: true,
                no_read: no_read || api_relation.visibility == Some(RelationVisibility::WriteOnly),
                no_update: no_update
                    || api_relation.visibility == Some(RelationVisibility::ReadOnly),
                replace: false,
                api_def,
            }
            .render()?
            .replace('\n', "\n    "),
        );
        let mut rel_fields = make_gql_fields(&rel_model, camel_case);
        write_relation(
            &rel_model,
            &mut relation_buf,
            db,
            graphql_name,
            camel_case,
            4,
            no_read,
            no_update,
            &mut rel_fields,
            api_def,
        )?;
        if !(no_read || api_relation.visibility == Some(RelationVisibility::WriteOnly)) {
            gql_fields.push(format!("{}{{{}}}", rel_name, rel_fields.join(",")));
        }
        ApiRelationDef::pop();
        ApiFieldDef::pop();
        relation_buf.push_str("\n}");
    }
    for (_model, rel_name, rel) in def.relations_belonging(false) {
        let rel_model = rel.get_foreign_model();
        let api_relation = ApiRelationDef::get(rel_name).unwrap();
        ApiRelationDef::push(api_relation.relations(&rel_model)?);
        ApiFieldDef::push(api_relation.fields(&rel_model, &[])?);
        let pascal_name = &rel_model.name.to_case(Case::Pascal);
        let graphql_name = &format!("{}{}", graphql_name, rel_name.to_case(Case::Pascal));
        relation_buf.push_str(&format!("\n#[rustfmt::skip]\nmod _{} {{\n    ", rel_name));
        relation_buf.push_str(
            &template::ReferenceTemplate {
                db,
                graphql_name,
                rel_name,
                pascal_name,
                def: &rel_model,
                camel_case,
                rel_mod: rel.get_group_mod_var(),
            }
            .render()?
            .replace('\n', "\n    "),
        );
        let mut rel_fields = make_gql_fields(&rel_model, camel_case);
        write_relation(
            &rel_model,
            &mut relation_buf,
            db,
            graphql_name,
            camel_case,
            4,
            false,
            true,
            &mut rel_fields,
            api_def,
        )?;
        gql_fields.push(format!("{}{{{}}}", rel_name, rel_fields.join(",")));
        ApiRelationDef::pop();
        ApiFieldDef::pop();
        relation_buf.push_str("\n}");
    }
    buf.push_str(&relation_buf.replace('\n', &format!("\n{}", " ".repeat(indent))));
    Ok(())
}
