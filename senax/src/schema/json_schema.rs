use std::path::Path;

use schemars::{gen::SchemaSettings, schema::RootSchema};
use serde_json::Value;

use crate::{
    api_generator::schema::{ApiConfigJson, ApiDbJson, ApiModelJson},
    schema::SchemaDef,
};

use super::{ConfigJson, ModelJson, ValueObjectJson};

pub fn write_schema(base_path: &Path) -> Result<(), anyhow::Error> {
    let schema = whole_schema()?;
    let path = base_path.join("senax-schema.json");
    println!("{}", path.display());
    std::fs::write(path, schema)?;
    Ok(())
}

pub fn whole_schema() -> Result<String, anyhow::Error> {
    let settings = SchemaSettings::draft07().with(|s| {
        s.option_nullable = false;
        s.option_add_null_type = true;
    });
    let gen = settings.into_generator();
    let schema = gen.into_root_schema_for::<SchemaDef>();
    let schema = serde_json::to_string(&schema)?;
    let schema = schema.replace(r#""additionalProperties":{"#,
        r#""propertyNames":{"pattern":"^\\p{XID_Start}\\p{XID_Continue}*(?<!_)$"},"additionalProperties":{"#);
    let mut schema: Value = serde_json::from_str(&schema)?;
    schema
        .as_object_mut()
        .unwrap()
        .get_mut("properties")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .get_mut("conf")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .insert(
            "propertyNames".to_string(),
            serde_json::json!({"pattern":"^[A-Za-z][_0-9A-Za-z]*(?<!_)$"}),
        );
    schema
        .as_object_mut()
        .unwrap()
        .get_mut("properties")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .get_mut("model")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .insert(
            "propertyNames".to_string(),
            serde_json::json!({"pattern":"^[A-Za-z][_0-9A-Za-z]*(?<!_)$"}),
        );
    schema
        .as_object_mut()
        .unwrap()
        .get_mut("properties")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .get_mut("simple_value_object")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .insert(
            "propertyNames".to_string(),
            serde_json::json!({"pattern":"^[A-Za-z][_0-9A-Za-z]*(?<!_)$"}),
        );
    schema
        .as_object_mut()
        .unwrap()
        .get_mut("definitions")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .get_mut("ConfigDef")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .get_mut("properties")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .get_mut("groups")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .insert(
            "propertyNames".to_string(),
            serde_json::json!({"pattern":"^[A-Za-z][_0-9A-Za-z]*(?<!_)$"}),
        );
    schema
        .as_object_mut()
        .unwrap()
        .get_mut("definitions")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .get_mut("GroupDef")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .get_mut("properties")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .get_mut("models")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .insert(
            "propertyNames".to_string(),
            serde_json::json!({"pattern":"^[A-Za-z][_0-9A-Za-z]*(?<!_)$"}),
        );
    schema
        .as_object_mut()
        .unwrap()
        .get_mut("definitions")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .get_mut("ModelDef")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .get_mut("properties")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .get_mut("fields")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .insert(
            "propertyNames".to_string(),
            serde_json::json!({"pattern":"^\\p{XID_Start}\\p{XID_Continue}*(?<!_)$"}),
        );
    schema
        .as_object_mut()
        .unwrap()
        .get_mut("definitions")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .get_mut("ApiModelDef")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .get_mut("properties")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .get_mut("fields")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .insert(
            "propertyNames".to_string(),
            serde_json::json!({"pattern":"^\\p{XID_Start}\\p{XID_Continue}*(?<!_)$"}),
        );
    schema
        .as_object_mut()
        .unwrap()
        .get_mut("definitions")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .get_mut("ApiRelationDef")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .get_mut("properties")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .get_mut("fields")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .insert(
            "propertyNames".to_string(),
            serde_json::json!({"pattern":"^\\p{XID_Start}\\p{XID_Continue}*(?<!_)$"}),
        );
    let schema = serde_json::to_string_pretty(&schema)?;
    Ok(schema)
}

pub fn json_config_schema() -> Result<RootSchema, anyhow::Error> {
    let settings = SchemaSettings::draft07().with(|s| {
        s.option_nullable = true;
        s.option_add_null_type = false;
    });
    let gen = settings.into_generator();
    let schema = gen.into_root_schema_for::<ConfigJson>();
    Ok(schema)
}

pub fn json_model_schema() -> Result<RootSchema, anyhow::Error> {
    let settings = SchemaSettings::draft07().with(|s| {
        s.option_nullable = true;
        s.option_add_null_type = false;
    });
    let gen = settings.into_generator();
    let schema = gen.into_root_schema_for::<ModelJson>();
    Ok(schema)
}

pub fn json_simple_vo_schema() -> Result<RootSchema, anyhow::Error> {
    let settings = SchemaSettings::draft07().with(|s| {
        s.option_nullable = true;
        s.option_add_null_type = false;
    });
    let gen = settings.into_generator();
    let schema = gen.into_root_schema_for::<ValueObjectJson>();
    Ok(schema)
}

pub fn json_api_config_schema() -> Result<RootSchema, anyhow::Error> {
    let settings = SchemaSettings::draft07().with(|s| {
        s.option_nullable = true;
        s.option_add_null_type = false;
    });
    let gen = settings.into_generator();
    let schema = gen.into_root_schema_for::<ApiConfigJson>();
    Ok(schema)
}

pub fn json_api_db_schema() -> Result<RootSchema, anyhow::Error> {
    let settings = SchemaSettings::draft07().with(|s| {
        s.option_nullable = true;
        s.option_add_null_type = false;
    });
    let gen = settings.into_generator();
    let schema = gen.into_root_schema_for::<ApiDbJson>();
    Ok(schema)
}

pub fn json_api_schema() -> Result<RootSchema, anyhow::Error> {
    let settings = SchemaSettings::draft07().with(|s| {
        s.option_nullable = true;
        s.option_add_null_type = false;
    });
    let gen = settings.into_generator();
    let schema = gen.into_root_schema_for::<ApiModelJson>();
    Ok(schema)
}
