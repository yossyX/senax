use anyhow::{bail, Context as _, Result};
use askama::Template;
use chrono::Local;
use chrono::NaiveDate;
use indexmap::IndexMap;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Serialize;
use std::env;
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::Arc;
use tera::{Context, Tera};

use crate::schema::ColumnDef;
use crate::schema::IndexDef;
use crate::schema::RelDef;
use crate::schema::RelationsType;
use crate::schema::MODEL;
use crate::schema::MODELS;
use crate::schema::{self, EnumDef, GroupDef, ModelDef, CONFIG, ENUM_GROUPS, GROUPS, HISTORY};

#[derive(Debug, Serialize, Clone)]
struct Group<'a> {
    group_name: &'a String,
    group_def: &'a GroupDef,
    history: Option<&'a Vec<serde_yaml::Value>>,
    models: Option<IndexMap<&'a String, DocModel>>,
    enums: Option<&'a IndexMap<String, EnumDef>>,
    last_update_at: Option<NaiveDate>,
    er: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
struct DocModel {
    title: Option<String>,
    comment: Option<String>,
    table_name: String,
    columns: IndexMap<String, ColumnDef>,
    relations: IndexMap<String, Option<RelDef>>,
    indexes: IndexMap<String, IndexDef>,
}
impl From<&Arc<ModelDef>> for DocModel {
    fn from(m: &Arc<ModelDef>) -> Self {
        DocModel {
            title: m.title.clone(),
            comment: m.comment.clone(),
            table_name: m.table_name(),
            columns: m.merged_columns.clone(),
            relations: m.merged_relations.clone(),
            indexes: m.merged_indexes.clone(),
        }
    }
}

pub fn generate(db: &str, group_name: &Option<String>, er: bool) -> Result<()> {
    schema::parse(db)?;

    let config = unsafe { CONFIG.get().unwrap() }.clone();
    let history = unsafe { HISTORY.get().unwrap() }.clone();
    let groups = unsafe { GROUPS.get().unwrap() }.clone();
    let enum_groups = unsafe { ENUM_GROUPS.get().unwrap() }.clone();
    let locale = env::var("LC_ALL").unwrap_or_else(|_| {
        env::var("LC_TIME").unwrap_or_else(|_| env::var("LANG").unwrap_or_default())
    });
    let locale = locale.split('.').collect::<Vec<_>>()[0];

    let mut context = Context::new();
    context.insert("config", &config);
    context.insert("locale", locale);
    context.insert("date", &Local::now().to_rfc3339());
    let mut group_list = Vec::new();
    if let Some(group_name) = group_name {
        let models = groups.get(group_name);
        if let Some(models) = models {
            unsafe {
                MODELS.take();
                MODELS.set(models.clone()).unwrap();
            }
        }
        group_list.push(Group {
            group_name,
            group_def: config
                .groups
                .get(group_name)
                .context("The specified group was not found.")?,
            history: history.get(group_name),
            models: models.map(|i| {
                i.iter().fold(IndexMap::new(), |mut acc, (k, v)| {
                    if v.has_table() {
                        acc.insert(k, v.into());
                    }
                    acc
                })
            }),
            enums: enum_groups.get(group_name),
            last_update_at: history.get(group_name).map(|v| {
                v.iter().fold(NaiveDate::MIN, |acc, h| {
                    let k: serde_yaml::Value = "date".to_string().into();
                    let v = h.as_mapping().unwrap().get(&k).unwrap();
                    acc.max(v.as_str().unwrap().parse().unwrap())
                })
            }),
            er: gen_er(group_name, &models, er)?,
        });
        context.insert("group_list", &group_list);
    } else {
        for (group_name, group_def) in &config.groups {
            let models = groups.get(group_name);
            if let Some(models) = models {
                unsafe {
                    MODELS.take();
                    MODELS.set(models.clone()).unwrap();
                }
            }
            group_list.push(Group {
                group_name,
                group_def,
                history: history.get(group_name),
                models: models.map(|i| {
                    i.iter().fold(IndexMap::new(), |mut acc, (k, v)| {
                        if v.has_table() {
                            acc.insert(k, v.into());
                        }
                        acc
                    })
                }),
                enums: enum_groups.get(group_name),
                last_update_at: history.get(group_name).map(|v| {
                    v.iter().fold(NaiveDate::MIN, |acc, h| {
                        let k: serde_yaml::Value = "date".to_string().into();
                        let v = h.as_mapping().unwrap().get(&k).unwrap();
                        acc.max(v.as_str().unwrap().parse().unwrap())
                    })
                }),
                er: gen_er(group_name, &models, er)?,
            });
        }
        context.insert("group_list", &group_list);
    }

    let filename = if locale == "ja_JP" {
        "templates/db-document-jp.html"
    } else {
        "templates/db-document.html"
    };
    let tpl = crate::TEMPLATES.get(filename)?;
    let tpl = std::str::from_utf8(tpl.as_ref())?;
    let mut tera = Tera::default();
    tera.add_raw_template("db-document.html", tpl)?;
    let result = tera.render("db-document.html", &context)?;
    println!("{}", result);
    Ok(())
}

fn gen_er(
    group_name: &str,
    models: &Option<&IndexMap<String, Arc<ModelDef>>>,
    use_er: bool,
) -> Result<Option<String>> {
    let models = match models {
        Some(v) if !v.is_empty() && use_er => v,
        _ => {
            return Ok(None);
        }
    };
    let mut _models: IndexMap<String, Model> = IndexMap::new();
    let mut another_models: IndexMap<String, AnotherModel> = IndexMap::new();
    let mut relations: IndexMap<String, Relation> = IndexMap::new();
    for (model_name, model) in models.iter() {
        unsafe {
            MODEL.take();
            MODEL.set(model.clone()).unwrap();
        }

        if !model.has_table() {
            continue;
        }
        let title = if let Some(ref title) = model.title {
            title.clone()
        } else {
            model_name.clone()
        };
        let mut columns: IndexMap<String, Column> = IndexMap::new();
        for (col_name, relation) in model.merged_relations.iter() {
            let foreign = RelDef::get_foreign_model(relation, col_name);
            let foreign_name = if foreign.group_name == group_name {
                foreign.name.clone()
            } else {
                let foreign_name = format!("{}__{}", foreign.group_name, foreign.name);
                let title = if let Some(ref title) = foreign.title {
                    title.clone()
                } else {
                    foreign.name.clone()
                };
                let link = format!("{}::{}", foreign.group_name, foreign.name);
                another_models.insert(foreign_name.clone(), AnotherModel { link, title });
                foreign_name
            };
            match relation {
                Some(rel) => {
                    if rel.type_def.is_none() || rel.type_def == Some(RelationsType::One) {
                        let local = if let Some(ref local) = rel.local {
                            local.clone()
                        } else {
                            format!("{}_id", col_name)
                        };
                        if let Some(col) = model.merged_columns.get(&local) {
                            relations.insert(
                                format!("{model_name}:{local}"),
                                to_rel(&foreign_name, col, model),
                            );
                            columns.insert(local.clone(), to_column(&local, col));
                        }
                    }
                }
                None => {
                    if let Some(col) = model.merged_columns.get(col_name) {
                        relations.insert(
                            format!("{model_name}:{col_name}"),
                            to_rel(&foreign_name, col, model),
                        );
                        columns.insert(col_name.clone(), to_column(col_name, col));
                    }
                }
            }
        }
        _models.insert(model_name.clone(), Model { title, columns });
    }

    let tpl = ErDotTemplate {
        group_name,
        models: _models,
        another_models,
        relations,
    };
    let output = dot(&tpl.render()?)?;
    static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?s)<svg .+</svg>").unwrap());
    if let Some(caps) = RE.captures(&output) {
        let mut svg = String::from("<svg ");
        svg.push_str(caps.get(0).unwrap().as_str());
        return Ok(Some(svg));
    }
    eprintln!("{}", &tpl.render()?);
    bail!("dot failed!");
}

#[derive(Template)]
#[template(
    source = r###"
digraph "" {
    splines=true
    graph [layout = fdp]
    node [shape="box" fontname="times" fontsize="10" margin="0" penwidth="0.5"];
    edge [dir="both" arrowsize=0.5 arrowhead="none" arrowtail="dot" penwidth="0.5"];
    @%- for (name, model) in models %@
    @{ name }@[shape="plain" href="#model_@{ group_name }@::@{ name }@" label = <
    <table cellspacing="0" cellpadding="2" cellborder="0" border="1" color="black">
    <tr><td port="_">@{ model.title|e }@</td></tr>
    @%- if !model.columns.is_empty() %@
    <hr />
    @%- for (name, column) in model.columns %@
    <tr><td align="left" port="@{ name|e }@">@{ column.title|e }@</td></tr>
    @%- endfor %@
    @%- endif %@
    </table>>]
    @%- endfor %@

    @%- for (name, model) in another_models %@
    @{ name }@[shape="plain" href="#model_@{ model.link }@" label = <
    <table cellspacing="0" cellpadding="2" cellborder="0" border="1" color="black">
    <tr><td port="_">@{ model.title|e }@</td></tr>
    </table>>]
    @%- endfor %@

    @%- for (from, rel) in relations %@
    @{ from }@ -> @{ rel.to }@[@% if rel.null %@arrowhead="odiamond" style="dashed"@% endif %@@% if rel.one %@ arrowtail="none"@% endif %@]
    @%- endfor %@
}
"###,
    ext = "txt"
)]
struct ErDotTemplate<'a> {
    group_name: &'a str,
    models: IndexMap<String, Model>,
    another_models: IndexMap<String, AnotherModel>,
    relations: IndexMap<String, Relation>,
}

struct Model {
    title: String,
    columns: IndexMap<String, Column>,
}
struct Column {
    title: String,
}
fn to_column(name: &str, col: &ColumnDef) -> Column {
    Column {
        title: col
            .title
            .clone()
            .unwrap_or_else(|| col.get_col_name(name).to_string()),
    }
}
struct AnotherModel {
    link: String,
    title: String,
}
struct Relation {
    to: String,
    null: bool,
    one: bool,
}
fn to_rel(foreign_name: &String, col: &ColumnDef, model: &ModelDef) -> Relation {
    Relation {
        to: format!("{foreign_name}:_"),
        null: !col.not_null,
        one: col.primary && model.primaries().len() == 1,
    }
}
fn dot(str: &str) -> Result<String> {
    let mut child = Command::new("dot")
        .arg("-Tsvg")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    {
        let child_stdin = child.stdin.as_mut().unwrap();
        child_stdin.write_all(str.as_bytes())?;
    }
    let output = child.wait_with_output()?;
    let output = std::str::from_utf8(&output.stdout)?.to_string();
    Ok(output)
}
