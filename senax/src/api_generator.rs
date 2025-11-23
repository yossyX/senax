use anyhow::{Context, Result, ensure};
use askama::Template;
use indexmap::IndexMap;
use regex::{Captures, Regex};
use std::collections::HashSet;
use std::ffi::OsString;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::Ordering;

use crate::api_generator::schema::{API_CONFIG, ApiConfigDef, ApiDbDef, ApiFieldDef, ApiModelDef};
use crate::api_generator::template::{DbConfigTemplate, MutationRootTemplate, QueryRootTemplate};
use crate::common::ToCase as _;
use crate::common::{fs_write, parse_yml_file, simplify_yml};
use crate::filters;
use crate::schema::{_to_var_name, GROUPS, ModelDef, to_id_name};
use crate::{API_SCHEMA_PATH, model_generator};

use self::schema::{ApiRelationDef, RelationVisibility};

pub mod schema;
pub mod serialize;
pub mod template;

#[allow(clippy::too_many_arguments)]
pub fn generate(
    server: &str,
    db_route: &str,
    group_route: &Option<String>,
    model_route: &Option<String>,
    ts_dir: &Option<PathBuf>,
    inquiry: bool,
    force: bool,
    clean: bool,
) -> Result<()> {
    let server_dir = Path::new(server);
    ensure!(
        server_dir.exists() && server_dir.is_dir(),
        "The crate path does not exist."
    );

    let schema_dir = server_dir.join(API_SCHEMA_PATH);
    let db_config_path = schema_dir.join(format!("{db_route}.yml"));
    let db = if db_config_path.exists() {
        let db_config: ApiDbDef = parse_yml_file(&db_config_path)?;
        db_config.db.clone().unwrap_or(db_route.to_string())
    } else {
        db_route.to_string()
    };

    model_generator::check_version(&db)?;
    crate::schema::parse(&db, true, false)?;
    crate::schema::set_domain_mode(true);
    let group_lock = GROUPS.read().unwrap();
    let groups = group_lock.as_ref().unwrap();

    let config_path = schema_dir.join("_config.yml");
    let config: ApiConfigDef = parse_yml_file(&config_path)?;
    API_CONFIG.write().unwrap().replace(config.clone());

    if !db_config_path.exists() {
        let tpl = DbConfigTemplate;
        fs_write(&db_config_path, tpl.render()?)?;
    }
    let mut db_config: ApiDbDef = parse_yml_file(&db_config_path)?;
    db_config.fix();
    filters::SHOW_LABEL.store(db_config.with_label(), Ordering::SeqCst);
    filters::SHOW_COMMNET.store(db_config.with_comment(), Ordering::SeqCst);

    let src_dir = server_dir.join("src");
    let base_src_dir = server_dir.join("base/src");
    let file_path = src_dir.join("auto_api.rs");
    let mut content = fs::read_to_string(&file_path)
        .with_context(|| format!("Cannot read file: {:?}", &file_path))?;
    let db_snake = db_route.to_snake();
    let db_var_name = _to_var_name(&db_snake);
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
                "let _flatten_{db_snake}_ = true;
    if _flatten_{db_snake}_ {{
        cfg.configure({db_var_name}::route_config);
    }} else {{
        cfg.service(scope(\"/{db_route}\").configure({db_var_name}::route_config));
    }}
    // Do not modify this line. (ApiRouteConfig)"
            ),
        );
        content = content.replace(
            "    // Do not modify this line. (ApiJsonSchema)",
            &format!("    {}::gen_json_schema(&dir.join(\"{}\"))?;\n    // Do not modify this line. (ApiJsonSchema)", db_var_name, &db_route.to_snake()),
        );
        let tpl = QueryRootTemplate { db_route };
        content = content.replace("impl QueryRoot {", tpl.render()?.trim_start());
        let tpl = MutationRootTemplate { db_route };
        content = content.replace("impl MutationRoot {", tpl.render()?.trim_start());
    }
    if db_config.promote_children {
        content = content.replace(
            &format!(r#"#[graphql(name = "{db_route}")]"#),
            &format!(r#"#[graphql(name = "{db_route}", flatten)]"#),
        );
        content = content.replace(
            &format!("let _flatten_{db_snake}_ = false;"),
            &format!("let _flatten_{db_snake}_ = true;"),
        );
    } else {
        content = content.replace(
            &format!(r#"#[graphql(name = "{db_route}", flatten)]"#),
            &format!(r#"#[graphql(name = "{db_route}")]"#),
        );
        content = content.replace(
            &format!("let _flatten_{db_snake}_ = true;"),
            &format!("let _flatten_{db_snake}_ = false;"),
        );
    }
    fs_write(file_path, &*content)?;

    let file_path = base_src_dir.join("auth.rs");
    let content = fs::read_to_string(&file_path)
        .with_context(|| format!("Cannot read file: {:?}", &file_path))?;
    let re = Regex::new(r"(?s)// Do not modify below this line. \(RoleStart\).+// Do not modify above this line. \(RoleEnd\)").unwrap();
    ensure!(
        re.is_match(&content),
        "File contents are invalid.: {:?}",
        &file_path
    );
    let buf = if config.default_role.is_none() {
        String::from("    #[display(\"\")]\n    #[default]\n    _None,\n")
    } else {
        String::new()
    };
    let roles = config.roles.iter().fold(buf, |mut buf, role| {
        if let Some(dflt) = &config.default_role
            && dflt == role.0
        {
            buf.push_str("    #[default]\n");
        }
        if let Some(def) = role.1
            && let Some(alias) = &def.alias
        {
            writeln!(&mut buf, "    #[display({:?})]", alias).unwrap();
            writeln!(&mut buf, "    #[serde(rename = {:?})]", alias).unwrap();
        }
        writeln!(&mut buf, "    {},", _to_var_name(role.0)).unwrap();
        buf
    });
    let tpl = format!(
        "// Do not modify below this line. (RoleStart)\n{roles}    // Do not modify above this line. (RoleEnd)"
    );
    let content = re.replace(&content, tpl);

    let re = Regex::new(r"(?s)// Do not modify below this line. \(ImplRoleStart\).+// Do not modify above this line. \(ImplRoleEnd\)").unwrap();
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
    let tpl = format!(
        "// Do not modify below this line. (ImplRoleStart)\n{roles}    // Do not modify above this line. (ImplRoleEnd)"
    );
    let content = re.replace(&content, tpl);

    fs_write(file_path, &*content)?;

    let schema_dir = schema_dir.join(db_route);
    let ts_dir = if let Some(ts_dir) = ts_dir {
        if ts_dir.is_dir() {
            Some(
                ts_dir
                    .join("src")
                    .join("gql_query")
                    .join(db_route.to_snake()),
            )
        } else {
            eprintln!("The ts-dir directory does not exist.: {}", ts_dir.display());
            None
        }
    } else {
        None
    };
    if let Some(ts_dir) = &ts_dir
        && ts_dir.exists()
    {
        fs::remove_dir_all(ts_dir)?;
    }

    let group_routes = if let Some(group) = group_route {
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
    let api_dir = server_dir.join("auto_api");
    let mut group_route_names = Vec::new();
    let api_db_dir = api_dir.join(db_route.to_snake());
    let mut remove_files = HashSet::new();
    if clean && api_db_dir.exists() {
        for entry in glob::glob(&format!("{}/**/*.*", api_db_dir.display()))? {
            match entry {
                Ok(path) => remove_files.insert(path.as_os_str().to_owned()),
                Err(_) => false,
            };
        }
    }

    let file_path = server_dir.join("Cargo.toml");
    let mut content = fs::read_to_string(&file_path)
        .with_context(|| format!("Cannot read file: {:?}", &file_path))?;
    let name = server.to_snake();
    let mut deps = IndexMap::new();
    for group_route in group_routes.iter().rev() {
        let db_route = db_route.to_snake();
        let group_route = group_route.to_snake();
        deps.insert(
            format!("_{}_{}_{}", name, db_route, group_route),
            format!(
                "_{}_{}_{} = {{ path = \"auto_api/{}/{}\" }}",
                name, db_route, group_route, db_route, group_route
            ),
        );
    }
    let reg = Regex::new(&format!(r"(?m)^(_{}_{}_\w+)\s*=.+\n", name, db_route))?;
    for (line, [dep]) in reg.captures_iter(&content.clone()).map(|c| c.extract()) {
        if deps.shift_remove(dep).is_none() {
            content = content.replace(line, "");
        }
    }
    for (_, dep) in deps {
        content = content.replace("[dependencies]", &format!("[dependencies]\n{}", dep));
    }
    fs_write(file_path, &*content)?;

    for group_route in group_routes.iter().rev() {
        let schema_path = schema_dir.join(format!("{group_route}.yml"));
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

        let group_name = if let Some(Some(api_group_def)) = db_config.groups.get(group_route) {
            api_group_def.group.as_ref().unwrap_or(group_route)
        } else {
            group_route
        };
        let (_, group) = groups
            .get(group_name)
            .unwrap_or_else(|| panic!("The {db} DB does not have {group_name} group."));
        let group_route_mod_name = group_route.to_snake();
        let group_mod_name = group_name.to_snake();

        let api_group_dir = api_db_dir.join(&group_route_mod_name);

        #[derive(Template)]
        #[template(path = "api/_Cargo.toml", escape = "none")]
        pub struct CargoTemplate<'a> {
            pub server: &'a str,
            pub db: &'a str,
            pub group_name: &'a str,
        }

        let file_path = api_group_dir.join("Cargo.toml");
        remove_files.remove(file_path.as_os_str());
        if force || !file_path.exists() {
            let content = CargoTemplate {
                server,
                db: &db,
                group_name,
            }
            .render()?;
            fs_write(file_path, &*content)?;
        }

        #[derive(Template)]
        #[template(path = "api/lib.rs", escape = "none")]
        pub struct LibTemplate<'a> {
            pub db: &'a str,
        }

        let file_path = api_group_dir.join("src/lib.rs");
        remove_files.remove(file_path.as_os_str());
        if force || !file_path.exists() {
            let content = LibTemplate { db: &db }.render()?;
            fs_write(file_path, &*content)?;
        }

        let model_routes = if let Some(route) = model_route {
            vec![route.clone()]
        } else if inquiry {
            group
                .iter()
                .filter(|(_, (_, def))| !def.abstract_mode)
                .map(|(v, _)| v.clone())
                .collect()
        } else {
            api_model_map.iter().map(|(v, _)| v.clone()).collect()
        };

        for model_route in &model_routes {
            if api_model_map.get(model_route).is_none()
                && inquiry
                && !dialoguer::Confirm::new()
                    .with_prompt(format!("Add an API for the {} model?", model_route))
                    .default(true)
                    .interact()?
            {
                continue;
            }
            let model_name = if let Some(Some(api_model)) = api_model_map.get(model_route) {
                api_model.model.as_ref().unwrap_or(model_route)
            } else {
                model_route
            };
            let (_, def) = group.get(model_name).unwrap_or_else(|| {
                panic!("The {group_name} group does not have {model_name} model.")
            });

            let api_def = write_model_file(
                server,
                &api_group_dir,
                &db,
                db_route,
                &group_mod_name,
                &group_route_mod_name,
                model_name,
                model_route,
                def,
                api_model_map
                    .get(model_route)
                    .cloned()
                    .map(|v| v.unwrap_or_default()),
                &db_config,
                inquiry,
                force,
                &ts_dir,
                &mut remove_files,
            )?;
            if !api_model_map.contains_key(model_route) {
                if api_def == ApiModelDef::default() {
                    api_model_map.insert(model_route.clone(), None);
                } else {
                    api_model_map.insert(model_route.clone(), Some(api_def));
                }
                update_group_def = true;
            }
        }
        if !model_routes.is_empty() {
            write_group_file(
                &api_group_dir,
                db_route,
                &group_route_mod_name,
                &model_routes,
                &db_config,
                force || clean,
                &mut remove_files,
            )?;
            group_route_names.push(group_route.clone());
        }
        if !schema_path.exists() || update_group_def {
            let mut buf = "# yaml-language-server: $schema=../../../senax-schema.json#properties/api_model\n\n".to_string();
            buf.push_str(&simplify_yml(serde_yaml::to_string(&api_model_map)?)?);
            fs_write(schema_path, &buf)?;
        }
        if !db_config.groups.contains_key(group_route) {
            db_config.groups.insert(group_route.to_string(), None);
            let mut buf =
                "# yaml-language-server: $schema=../../senax-schema.json#definitions/ApiDbDef\n\n"
                    .to_string();
            buf.push_str(&simplify_yml(serde_yaml::to_string(&db_config)?)?);
            fs_write(&db_config_path, &buf)?;
        }
    }
    write_db_file(
        &src_dir,
        server,
        &db,
        db_route,
        &group_route_names,
        force || clean,
        &db_config,
    )?;
    for file in &remove_files {
        println!("REMOVE:{}", file.to_string_lossy());
        fs::remove_file(file)?;
        let ancestors = Path::new(file).ancestors();
        for ancestor in ancestors {
            if let Ok(dir) = ancestor.read_dir() {
                if dir.count() == 0 {
                    fs::remove_dir(ancestor)?;
                } else {
                    break;
                }
            }
        }
    }
    Ok(())
}

fn write_db_file(
    path: &Path,
    server: &str,
    _db: &str,
    db_route: &str,
    group_route_names: &[String],
    force: bool,
    config: &ApiDbDef,
) -> Result<()> {
    let file_path = path
        .join("auto_api")
        .join(format!("{}.rs", &db_route.to_snake()));
    let mut content = if force || !file_path.exists() {
        #[derive(Template)]
        #[template(path = "api/db.rs", escape = "none")]
        pub struct DbTemplate<'a> {
            pub db_route: &'a str,
        }

        DbTemplate { db_route }.render()?
    } else {
        fs::read_to_string(&file_path)?.replace("\r\n", "\n")
    };
    for group_route in group_route_names.iter().rev() {
        let group_snake = group_route.to_snake();
        let group_var_name = _to_var_name(&group_snake);
        let chk = format!(
            "\npub use _{}_{}_{}::api as {};\n",
            server.to_snake(),
            db_route.to_snake(),
            group_snake,
            group_var_name
        );
        if !content.contains(&chk) {
            #[derive(Template)]
            #[template(
                source = r###"
pub use _@{ server|snake }@_@{ db_route|snake }@_@{ group_route|snake }@::api as @{ group_route|snake|to_var_name }@;
// Do not modify this line. (GqlMod)"###,
                ext = "txt",
                escape = "none"
            )]
            pub struct DbModTemplate<'a> {
                pub server: &'a str,
                pub db_route: &'a str,
                pub group_route: &'a str,
            }

            let tpl = DbModTemplate {
                server,
                db_route,
                group_route,
            };
            content = content.replace("\n// Do not modify this line. (GqlMod)", &tpl.render()?);

            #[derive(Template)]
            #[template(
                source = r###"
    #[graphql(name = "@{ group_route }@")]
    async fn @{ group_route|to_var_name }@(&self) -> @{ group_route|snake|to_var_name }@::GqlQuery@{ db_route|pascal }@@{ group_route|pascal }@ {
        @{ group_route|snake|to_var_name }@::GqlQuery@{ db_route|pascal }@@{ group_route|pascal }@
    }
    // Do not modify this line. (GqlQuery)"###,
                ext = "txt",
                escape = "none"
            )]
            pub struct DbQueryTemplate<'a> {
                pub db_route: &'a str,
                pub group_route: &'a str,
            }

            let tpl = DbQueryTemplate {
                db_route,
                group_route,
            };
            content = content.replace(
                "\n    // Do not modify this line. (GqlQuery)",
                &tpl.render()?,
            );

            #[derive(Template)]
            #[template(
                source = r###"
    #[graphql(name = "@{ group_route }@")]
    async fn @{ group_route|to_var_name }@(&self) -> @{ group_route|snake|to_var_name }@::GqlMutation@{ db_route|pascal }@@{ group_route|pascal }@ {
        @{ group_route|snake|to_var_name }@::GqlMutation@{ db_route|pascal }@@{ group_route|pascal }@
    }
    // Do not modify this line. (GqlMutation)"###,
                ext = "txt",
                escape = "none"
            )]
            pub struct DbMutationTemplate<'a> {
                pub db_route: &'a str,
                pub group_route: &'a str,
            }

            let tpl = DbMutationTemplate {
                db_route,
                group_route,
            };
            content = content.replace(
                "\n    // Do not modify this line. (GqlMutation)",
                &tpl.render()?,
            );
            content = content.replace(
                "// Do not modify this line. (ApiRouteConfig)",
                &format!(
                    "let _flatten_{group_snake}_ = true;
    if _flatten_{group_snake}_ {{
        cfg.configure({group_var_name}::route_config);
    }} else {{
        cfg.service(scope(\"/{group_route}\").configure({group_var_name}::route_config));
    }}
    // Do not modify this line. (ApiRouteConfig)"
                ),
            );

            #[derive(Template)]
            #[template(
                source = r###"
    @{ group_route|snake|to_var_name }@::gen_json_schema(&dir.join("@{ group_route }@"))?;
    // Do not modify this line. (JsonSchema)"###,
                ext = "txt",
                escape = "none"
            )]
            pub struct DbJsonSchemaTemplate<'a> {
                pub group_route: &'a str,
            }

            let tpl = DbJsonSchemaTemplate { group_route };
            content = content.replace(
                "\n    // Do not modify this line. (JsonSchema)",
                &tpl.render()?,
            );
        }
        if config.promote_group_children(group_route) {
            content = content.replace(
                &format!(r#"#[graphql(name = "{group_route}")]"#),
                &format!(r#"#[graphql(name = "{group_route}", flatten)]"#),
            );
            content = content.replace(
                &format!("let _flatten_{group_snake}_ = false;"),
                &format!("let _flatten_{group_snake}_ = true;"),
            );
        } else {
            content = content.replace(
                &format!(r#"#[graphql(name = "{group_route}", flatten)]"#),
                &format!(r#"#[graphql(name = "{group_route}")]"#),
            );
            content = content.replace(
                &format!("let _flatten_{group_snake}_ = true;"),
                &format!("let _flatten_{group_snake}_ = false;"),
            );
        }
    }
    fs_write(file_path, &*content)?;
    Ok(())
}

fn write_group_file(
    path: &Path,
    db_route: &str,
    group_route: &str,
    model_routes: &[String],
    db_config: &ApiDbDef,
    force: bool,
    remove_files: &mut HashSet<OsString>,
) -> Result<()> {
    let camel_case = db_config.camel_case();
    let file_path = path.join("src/api.rs");
    remove_files.remove(file_path.as_os_str());
    let mut content = if force || !file_path.exists() {
        template::GroupTemplate {
            db: db_route,
            group: group_route,
        }
        .render()?
    } else {
        fs::read_to_string(&file_path)?.replace("\r\n", "\n")
    };
    for model_route in model_routes.iter() {
        let chk = format!("\npub mod {};\n", _to_var_name(&model_route.to_snake()));
        if !content.contains(&chk) {
            #[derive(Template)]
            #[template(
                source = r###"
pub mod @{ model_route|snake|to_var_name }@;
// Do not modify this line. (GqlMod)"###,
                ext = "txt",
                escape = "none"
            )]
            pub struct GroupModTemplate<'a> {
                pub model_route: &'a str,
            }

            let tpl = GroupModTemplate { model_route };
            content = content.replace("\n// Do not modify this line. (GqlMod)", &tpl.render()?);

            #[derive(Template)]
            #[template(
                source = r###"
    @%- if !camel_case %@
    #[graphql(name = "@{ model_route }@")]
    @%- endif %@
    async fn @{ model_route|to_var_name }@(&self) -> @{ model_route|snake|to_var_name }@::Gql@{ mode }@@{ graphql_name }@ {
        @{ model_route|snake|to_var_name }@::Gql@{ mode }@@{ graphql_name }@
    }
    // Do not modify this line. (Gql@{ mode }@)"###,
                ext = "txt",
                escape = "none"
            )]
            pub struct GroupImplTemplate<'a> {
                pub mode: &'a str,
                pub model_route: &'a str,
                pub graphql_name: &'a str,
                pub camel_case: bool,
            }

            let graphql_name = &db_config.graphql_name(db_route, group_route, model_route);
            let tpl = GroupImplTemplate {
                mode: "Query",
                model_route,
                graphql_name,
                camel_case,
            };
            content = content.replace(
                "\n    // Do not modify this line. (GqlQuery)",
                &tpl.render()?,
            );
            let tpl = GroupImplTemplate {
                mode: "Mutation",
                model_route,
                graphql_name,
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
                &model_route,
                _to_var_name(&model_route.to_snake()),
                ),
            );

            #[derive(Template)]
            #[template(
                source = r###"
    @{ model_route|snake|to_var_name }@::gen_json_schema(dir)?;
    // Do not modify this line. (JsonSchema)"###,
                ext = "txt",
                escape = "none"
            )]
            pub struct GroupJsonSchemaTemplate<'a> {
                pub model_route: &'a str,
            }

            let tpl = GroupJsonSchemaTemplate { model_route };
            content = content.replace(
                "\n    // Do not modify this line. (JsonSchema)",
                &tpl.render()?,
            );
        }
    }
    fs_write(file_path, &*content)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn write_model_file(
    server_name: &str,
    path: &Path,
    db: &str,
    db_route: &str,
    group: &str,
    group_route: &str,
    model_name: &str,
    model_route: &str,
    def: &Arc<ModelDef>,
    api_def: Option<ApiModelDef>,
    config: &ApiDbDef,
    inquiry: bool,
    force: bool,
    ts_dir: &Option<PathBuf>,
    remove_files: &mut HashSet<OsString>,
) -> Result<ApiModelDef> {
    let path = path.join("src/api");
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
    let model_route_mod_name = model_route.to_snake();
    let pascal_name = &model_name.to_pascal();
    let graphql_name = &config.graphql_name(db_route, group_route, model_route);
    let file_path = path.join(format!("{}.rs", &model_route_mod_name));
    remove_files.remove(file_path.as_os_str());
    let content = if force || !file_path.exists() {
        template::ModelTemplate {
            server_name,
            db,
            db_route,
            group,
            group_route,
            mod_name,
            pascal_name: &model_name.to_pascal(),
            graphql_name,
            id_name: &to_id_name(model_name),
            def,
            camel_case: config.camel_case(),
            api_def: &api_def,
        }
        .render()?
    } else {
        fs::read_to_string(&file_path)?.replace("\r\n", "\n")
    };
    let re = Regex::new(r"(?s)(// Do not modify below this line. \(GqlModelStart\)).+(// Do not modify above this line. \(GqlModelEnd\))").unwrap();
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
        server_name,
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
        let ts_dir = ts_dir.join(group_route);
        let file_path = ts_dir.join(format!("{}.tsx", model_name));
        use inflector::Inflector;
        let model_case = if config.camel_case() {
            model_route.to_camel_case()
        } else {
            model_route.to_string()
        };
        let tpl = template::ModelTsTemplate {
            path: format!(
                "{}{}{}",
                if config.promote_children {
                    String::new()
                } else {
                    format!("{}_", db_route.to_snake())
                },
                if config.promote_group_children(group_route) {
                    String::new()
                } else {
                    format!("{}_", group_route.to_snake())
                },
                model_route.to_snake()
            ),
            model_route,
            curly_begin: format!(
                "{}{}{}",
                if config.promote_children {
                    String::new()
                } else {
                    format!("{db_route}{{")
                },
                if config.promote_group_children(group_route) {
                    String::new()
                } else {
                    format!("{group_route}{{")
                },
                model_case
            ),
            curly_end: format!(
                "{}{}",
                if config.promote_children { "" } else { "}" },
                if config.promote_group_children(group_route) {
                    ""
                } else {
                    "}"
                },
            ),
            pascal_name: format!(
                "{}{}{}",
                if config.promote_children {
                    String::new()
                } else {
                    db.to_pascal()
                },
                if config.promote_group_children(group_route) {
                    String::new()
                } else {
                    group.to_pascal()
                },
                model_name.to_pascal()
            ),
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
        |v: &str| v.to_camel()
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
    server_name: &str,
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
        let pascal_name = &rel_model.name.to_pascal();
        let graphql_name = &format!("{}{}", graphql_name, rel_name.to_pascal());
        relation_buf.push_str(&format!("\n#[rustfmt::skip]\nmod _{} {{\n    ", rel_name));
        relation_buf.push_str(
            &template::RelationTemplate {
                server_name,
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
            server_name,
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
        let pascal_name = &rel_model.name.to_pascal();
        let graphql_name = &format!("{}{}", graphql_name, rel_name.to_pascal());
        relation_buf.push_str(&format!("\n#[rustfmt::skip]\nmod _{} {{\n    ", rel_name));
        relation_buf.push_str(
            &template::RelationTemplate {
                server_name,
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
            server_name,
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
        let pascal_name = &rel_model.name.to_pascal();
        let graphql_name = &format!("{}{}", graphql_name, rel_name.to_pascal());
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
            server_name,
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

pub fn api_db_list(server: &Path) -> Result<Vec<String>> {
    let mut list = Vec::new();
    for entry in fs::read_dir(server.join(API_SCHEMA_PATH))? {
        let entry = entry?;
        let path = entry.path();
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_str()
            .unwrap_or_default();
        if path.is_file() && !name.eq_ignore_ascii_case("_config.yml") && name.ends_with(".yml") {
            list.push(name.trim_end_matches(".yml").to_string());
        }
    }
    Ok(list)
}
