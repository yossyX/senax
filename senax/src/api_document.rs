use anyhow::Result;
use chrono::Local;
use convert_case::{Case, Casing};
use serde_json::Value;
use std::path::PathBuf;
use std::{collections::HashMap, env};
use tera::{Context, Filter, Tera};

pub fn generate(
    api_def: crate::api_generator::serialize::ApiDef,
    output: &Option<PathBuf>,
    template: &Option<PathBuf>,
) -> Result<()> {
    let locale = env::var("LC_ALL").unwrap_or_else(|_| {
        env::var("LC_TIME").unwrap_or_else(|_| env::var("LANG").unwrap_or_default())
    });
    let locale = locale.split('.').collect::<Vec<_>>()[0];

    let mut context = Context::new();
    context.insert("locale", locale);
    context.insert("date", &Local::now().to_rfc3339());
    context.insert("api_def", &api_def);
    context.insert("api_def_json", &serde_json::to_string(&api_def)?);

    let tpl = if let Some(template) = template {
        std::fs::read_to_string(template)?
    } else {
        let filename = if locale == "ja_JP" {
            "templates/api-document-jp.html"
        } else {
            "templates/api-document.html"
        };
        let tpl = crate::TEMPLATES.get(filename)?;
        std::str::from_utf8(tpl.as_ref())?.to_string()
    };
    let mut tera = Tera::default();
    tera.register_filter("title", Title);
    tera.register_filter("pascal", Pascal);
    tera.register_filter("camel", Camel);
    tera.register_filter("snake", Snake);
    tera.register_filter("upper_snake", UpperSnake);
    tera.register_filter("gql_pascal", GqlPascal);
    tera.register_filter("gql_camel", GqlCamel);
    tera.add_raw_template("template", &tpl)?;
    let result = tera.render("template", &context)?;
    if let Some(output) = output {
        std::fs::write(output, result)?;
    } else {
        println!("{}", result);
    }
    Ok(())
}

struct Title;
impl Filter for Title {
    fn filter(&self, value: &Value, _args: &HashMap<String, Value>) -> tera::Result<Value> {
        let v = value.as_str().unwrap_or_default();
        Ok(v.to_case(Case::Title).into())
    }
}
struct Pascal;
impl Filter for Pascal {
    fn filter(&self, value: &Value, _args: &HashMap<String, Value>) -> tera::Result<Value> {
        let v = value.as_str().unwrap_or_default();
        Ok(v.to_case(Case::Pascal).into())
    }
}
struct Camel;
impl Filter for Camel {
    fn filter(&self, value: &Value, _args: &HashMap<String, Value>) -> tera::Result<Value> {
        let v = value.as_str().unwrap_or_default();
        Ok(v.to_case(Case::Camel).into())
    }
}
struct Snake;
impl Filter for Snake {
    fn filter(&self, value: &Value, _args: &HashMap<String, Value>) -> tera::Result<Value> {
        let v = value.as_str().unwrap_or_default();
        Ok(v.to_case(Case::Snake).into())
    }
}
struct UpperSnake;
impl Filter for UpperSnake {
    fn filter(&self, value: &Value, _args: &HashMap<String, Value>) -> tera::Result<Value> {
        let v = value.as_str().unwrap_or_default();
        Ok(v.to_case(Case::UpperSnake).into())
    }
}
struct GqlPascal;
impl Filter for GqlPascal {
    fn filter(&self, value: &Value, _args: &HashMap<String, Value>) -> tera::Result<Value> {
        use inflector::Inflector;
        let v = value.as_str().unwrap_or_default();
        Ok(v.to_pascal_case().into())
    }
}
struct GqlCamel;
impl Filter for GqlCamel {
    fn filter(&self, value: &Value, _args: &HashMap<String, Value>) -> tera::Result<Value> {
        use inflector::Inflector;
        let v = value.as_str().unwrap_or_default();
        Ok(v.to_camel_case().into())
    }
}
