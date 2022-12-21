use anyhow::{bail, Context as _, Result};
use askama::Template;
use chrono::DateTime;
use chrono::Local;
use chrono::NaiveDateTime;
use chrono::Utc;
use indexmap::IndexMap;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::env;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;
use tera::Filter;
use tera::{Context, Tera};

use crate::schema::ColumnDef;
use crate::schema::IndexDef;
use crate::schema::RelDef;
use crate::schema::RelationsType;
use crate::schema::MODEL;
use crate::schema::MODELS;
use crate::schema::{self, EnumDef, GroupDef, ModelDef, CONFIG, ENUM_GROUPS, GROUPS};
use crate::MODELS_PATH;

#[derive(Debug, Serialize, Clone)]
struct Group<'a> {
    group_name: &'a String,
    group_def: &'a GroupDef,
    models: Option<IndexMap<&'a String, DocModel>>,
    enums: Option<&'a IndexMap<String, EnumDef>>,
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

#[derive(Serialize)]
pub struct History {
    pub date: DateTime<Local>,
    pub description: String,
}

pub fn generate(
    db: &str,
    group_name: &Option<String>,
    er: bool,
    history_max: &Option<usize>,
    output: &Option<PathBuf>,
    template: &Option<PathBuf>,
) -> Result<()> {
    schema::parse(db)?;

    let config = unsafe { CONFIG.get().unwrap() }.clone();
    let groups = unsafe { GROUPS.get().unwrap() }.clone();
    let enum_groups = unsafe { ENUM_GROUPS.get().unwrap() }.clone();
    let locale = env::var("LC_ALL").unwrap_or_else(|_| {
        env::var("LC_TIME").unwrap_or_else(|_| env::var("LANG").unwrap_or_default())
    });
    let locale = locale.split('.').collect::<Vec<_>>()[0];

    let base_path = MODELS_PATH.get().unwrap().join(&db);
    let ddl_path = base_path.join("migrations");
    fn file_read(path: &PathBuf) -> Result<String> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("file cannot read: {:?}", path))?;
        let mut result = String::new();
        for line in content.split('\n') {
            match line.strip_prefix("-- ") {
                Some(line) => {
                    result.push_str(line);
                    result.push('\n');
                }
                None => break,
            }
        }
        Ok(result)
    }
    let re = Regex::new(r"^(\d{14})_(.+)\.sql$").unwrap();
    let mut files = BTreeMap::new();
    for entry in ddl_path.read_dir()?.flatten() {
        if entry.file_type()?.is_file() {
            let path = entry.path();
            let file_name = path
                .file_name()
                .map(|v| v.to_string_lossy())
                .unwrap_or_default();
            let caps = re.captures(&file_name);
            if let Some(caps) = caps {
                if caps.get(2).unwrap().as_str().ends_with(".down") {
                    continue;
                }
                files.insert(
                    caps.get(1).unwrap().as_str().to_string(),
                    entry.path().clone(),
                );
            }
        }
    }
    let mut history = Vec::new();
    if let Some(max) = history_max {
        for (date, file) in files.into_iter() {
            let utc = NaiveDateTime::parse_from_str(&date, "%Y%m%d%H%M%S")
                .map(|ndt| DateTime::<Utc>::from_utc(ndt, Utc))?;
            let date: DateTime<Local> = DateTime::from(utc);
            let description = file_read(&file)?;
            if description.is_empty() {
                continue;
            }
            history.push(History { date, description });
        }
        history.reverse();
        history.truncate(*max);
    }

    let mut context = Context::new();
    context.insert("config", &config);
    context.insert("locale", locale);
    context.insert("date", &Local::now().to_rfc3339());
    context.insert("history", &history);
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
            models: models.map(|i| {
                i.iter().fold(IndexMap::new(), |mut acc, (k, v)| {
                    if v.has_table() {
                        acc.insert(k, v.into());
                    }
                    acc
                })
            }),
            enums: enum_groups.get(group_name),
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
                models: models.map(|i| {
                    i.iter().fold(IndexMap::new(), |mut acc, (k, v)| {
                        if v.has_table() {
                            acc.insert(k, v.into());
                        }
                        acc
                    })
                }),
                enums: enum_groups.get(group_name),
                er: gen_er(group_name, &models, er)?,
            });
        }
        context.insert("group_list", &group_list);
    }

    let tpl = if let Some(template) = template {
        std::fs::read_to_string(template)?
    } else {
        let filename = if locale == "ja_JP" {
            "templates/db-document-jp.html"
        } else {
            "templates/db-document.html"
        };
        let tpl = crate::TEMPLATES.get(filename)?;
        std::str::from_utf8(tpl.as_ref())?.to_string()
    };
    let mut tera = Tera::default();
    tera.add_raw_template("db-document.html", &tpl)?;
    tera.register_filter("history_description", ConvHistory(true));
    tera.register_filter("history_description_wo_esc", ConvHistory(false));
    let result = tera.render("db-document.html", &context)?;
    if let Some(output) = output {
        std::fs::write(output, result)?;
    } else {
        println!("{}", result);
    }
    Ok(())
}

struct ConvHistory(bool);
impl Filter for ConvHistory {
    fn filter(&self, value: &Value, args: &HashMap<String, Value>) -> tera::Result<Value> {
        static RE: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"^(.*)\[([A-Za-z0-9]+):([^,\]]*):([^\]]*)\](.*)$").unwrap());
        let v = value.as_str().unwrap_or_default();
        let mut result = String::new();
        for line in v.split('\n') {
            let line = line.trim_end();
            if RE.is_match(line) {
                let after = RE.replace(line, |c: &regex::Captures<'_>| {
                    let table = c.get(3).unwrap().as_str();
                    let column = c.get(4).unwrap().as_str();
                    let typ = c.get(2).unwrap().as_str();
                    let replace = if column.contains(',') {
                        let mut key = typ.to_string();
                        key.push_str("_Plural");
                        args.get(&key).or_else(|| args.get(typ))
                    } else {
                        args.get(typ)
                    };
                    if let Some(replace) = replace {
                        let mut after = String::new();
                        let comment = c.get(1).unwrap().as_str();
                        if self.0 {
                            after.push_str(&tera::escape_html(comment));
                        } else {
                            after.push_str(comment);
                        }
                        after.push_str(
                            &replace
                                .as_str()
                                .unwrap()
                                .replace("{T}", table)
                                .replace("{C}", column),
                        );
                        let comment = c.get(5).unwrap().as_str();
                        if self.0 {
                            after.push_str(&tera::escape_html(comment));
                        } else {
                            after.push_str(comment);
                        }
                        after
                    } else {
                        "".to_owned()
                    }
                });
                if !after.is_empty() {
                    result.push_str(&after);
                    result.push('\n');
                }
            } else {
                if self.0 {
                    result.push_str(&tera::escape_html(line));
                } else {
                    result.push_str(line);
                }
                result.push('\n');
            }
        }
        Ok(result.trim_end().into())
    }

    fn is_safe(&self) -> bool {
        true
    }
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
