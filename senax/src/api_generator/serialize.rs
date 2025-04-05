use anyhow::{Result, ensure};
use convert_case::{Case, Casing};
use indexmap::IndexMap;
use inflector::Inflector;
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use crate::API_SCHEMA_PATH;
use crate::api_generator::schema::{API_CONFIG, ApiConfigDef, ApiDbDef, ApiFieldDef, ApiModelDef};
use crate::common::parse_yml_file;
use crate::schema::{
    CONFIG, DataType, FilterDef, FilterSortDirection, FilterType, GROUPS, ModelDef,
};

use super::schema::{ApiRelationDef, ApiRoleDef, JsUpdaterDef, RelationVisibility};

#[derive(Debug, Serialize, Clone, Default)]
pub struct ApiDef {
    pub cased_db_name: String,
    pub db_name: String,
    pub roles: IndexMap<String, ApiRoleDef>,
    pub default_role: Option<String>,
    pub groups: Vec<Group>,
}

#[derive(Debug, Serialize, Clone)]
pub struct Group {
    pub name: String,
    pub cased_name: String,
    pub label: Option<String>,
    pub models: Vec<DocModel>,
}

#[derive(Debug, Serialize, Clone)]
pub struct Selector {
    pub name: String,
    pub js_updater: IndexMap<String, JsUpdaterDef>,
    pub use_for_update_by_operator: bool,
    pub use_for_delete: bool,
    pub filters: Vec<Filter>,
    pub orders: IndexMap<String, Order>,
}

#[derive(Debug, Serialize, Clone)]
pub struct Filter {
    pub name: String,
    pub indent: usize,
    #[serde(rename = "type")]
    pub _type: FilterType,
    pub fields: Vec<String>,
    pub required: bool,
    pub relation: Option<String>,
    pub relation_fields: IndexMap<String, FilterDef>,
    pub json_path: Option<String>,
    pub query: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct Order {
    pub fields: Vec<String>,
    pub direction: Option<FilterSortDirection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direct_sql: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct DocModel {
    pub name: String,
    pub cased_name: String,
    pub gql_name: String,
    pub label: Option<String>,
    pub pk: String,
    pub has_all_query: bool,
    pub use_find_by_pk: bool,
    pub use_delete_by_pk: bool,
    pub use_import: bool,
    pub disable_mutation: bool,
    pub disable_update: bool,
    pub readable_roles: Vec<String>,
    pub creatable_roles: Vec<String>,
    pub importable_roles: Vec<String>,
    pub updatable_roles: Vec<String>,
    pub deletable_roles: Vec<String>,
    pub readable_filter: Option<String>,
    pub updatable_filter: Option<String>,
    pub deletable_filter: Option<String>,
    pub selectors: Vec<Selector>,
    pub fields: Vec<Field>,
    pub relations: Vec<Arc<Relation>>,
    pub all_fields: Vec<Field>,
    pub all_relations: Vec<Arc<Relation>>,
}

#[derive(Debug, Serialize, Clone, Default)]
pub struct Field {
    pub name: String,
    pub cased_name: String,
    pub indent: usize,
    pub label: Option<String>,
    #[serde(rename = "type")]
    pub data_type: Option<DataType>,
    pub signed: bool,
    pub gql_type: Option<String>,
    pub no_read: bool,
    pub no_update: bool,
    pub required: bool,
    pub disable_update: bool,
    pub has_many: bool,
    pub replace: bool,
    pub validator: Option<String>,
    pub default: Option<serde_yaml::Value>,
    pub on_insert_formula: Option<String>,
    pub on_update_formula: Option<String>,
}

#[derive(Debug, Serialize, Clone, Default)]
pub struct Relation {
    pub name: String,
    pub label: Option<String>,
    pub gql_name: String,
    pub has_many: bool,
    pub no_read: bool,
    pub no_update: bool,
    pub replace: bool,
    pub fields: Vec<Field>,
    pub relations: Vec<Arc<Relation>>,
}

pub fn generate(server_path: &Path, db: &str, group: &Option<String>) -> Result<ApiDef> {
    crate::schema::parse(db, true, false)?;
    crate::schema::set_domain_mode(true);

    let groups = GROUPS.read().unwrap().as_ref().unwrap().clone();
    ensure!(
        server_path.exists() && server_path.is_dir(),
        "The crate path does not exist."
    );
    let config = CONFIG.read().unwrap().as_ref().unwrap().clone();

    let schema_dir = server_path.join(API_SCHEMA_PATH);
    let config_path = schema_dir.join("_config.yml");
    let api_config: ApiConfigDef = parse_yml_file(&config_path)?;
    API_CONFIG.write().unwrap().replace(api_config.clone());

    let db_config_path = schema_dir.join(format!("{db}.yml"));
    let mut db_config: ApiDbDef = parse_yml_file(&db_config_path)?;
    db_config.fix();

    let mut api_def = ApiDef {
        cased_db_name: if db_config.camel_case() {
            db.to_case(Case::Camel)
        } else {
            db.to_string()
        },
        db_name: db.to_string(),
        roles: api_config
            .roles
            .iter()
            .map(|(k, v)| (k.clone(), v.clone().unwrap_or_default()))
            .collect(),
        default_role: api_config.default_role.clone(),
        ..Default::default()
    };

    let schema_dir = schema_dir.join(db);
    let group_names = if let Some(group) = group {
        ensure!(
            groups.contains_key(group),
            "The {} db does not have {} group.",
            db,
            group
        );
        vec![group.clone()]
    } else {
        groups
            .iter()
            .filter(|(v, _)| {
                schema_dir.join(format!("{}.yml", v)).exists() && db_config.groups.contains_key(*v)
            })
            .map(|(v, _)| v.clone())
            .collect()
    };
    let mut api_groups: Vec<Group> = Vec::new();

    for org_group_name in &group_names {
        let schema_path = schema_dir.join(format!("{org_group_name}.yml"));
        let mut api_group_def: IndexMap<String, Option<ApiModelDef>> = if schema_path.exists() {
            parse_yml_file(&schema_path)?
        } else {
            IndexMap::default()
        };
        for (_, def) in api_group_def.iter_mut() {
            if let Some(v) = def.as_mut() {
                v.fix()
            }
        }

        let group = groups.get(org_group_name).unwrap();
        for (k, _) in &api_group_def {
            if !group.contains_key(k) {
                eprintln!("There is no {} model in the {} group.", k, org_group_name)
            }
        }
        let group_name = org_group_name.to_case(Case::Snake);
        let group_name = &group_name;
        let mut api_models: Vec<DocModel> = Vec::new();

        let model_list: Vec<_> = group
            .iter()
            .filter(|(name, _)| api_group_def.contains_key(*name))
            .filter(|(_, def)| !def.abstract_mode)
            .collect();
        for (name, def) in &model_list {
            if api_group_def.get(*name).is_some() {
                api_models.push(make_model(
                    db,
                    group_name,
                    name,
                    def,
                    api_group_def
                        .get(*name)
                        .cloned()
                        .map(|v| v.unwrap_or_default()),
                    &db_config,
                )?);
            }
        }
        let label = config
            .groups
            .get(org_group_name)
            .and_then(|v| v.as_ref().and_then(|v| v.label.clone()));
        api_groups.push(Group {
            name: org_group_name.clone(),
            cased_name: if db_config.camel_case() {
                org_group_name.to_camel_case()
            } else {
                org_group_name.to_string()
            },
            label,
            models: api_models,
        });
    }
    api_def.groups = api_groups;
    Ok(api_def)
}

#[allow(clippy::too_many_arguments)]
fn make_model(
    db: &str,
    group: &str,
    _model_name: &str,
    def: &Arc<ModelDef>,
    api_def: Option<ApiModelDef>,
    config: &ApiDbDef,
) -> Result<DocModel> {
    let api_def = if let Some(api_def) = api_def {
        api_def.clone()
    } else {
        ApiModelDef::default()
    };
    let mod_name = def.mod_name();

    ApiRelationDef::push(api_def.relations(def)?);
    ApiFieldDef::push(api_def.fields(def, config)?);

    let gql_name = format!(
        "{}{}{}",
        db.to_case(Case::Pascal),
        group.to_case(Case::Pascal),
        mod_name.to_case(Case::Pascal)
    );

    let mut all_fields = Vec::new();
    let mut all_relations = Vec::new();
    all_fields.append(&mut fields(def, 0, config.camel_case()));

    let relations = make_relation(
        def,
        0,
        &mut all_fields,
        &mut all_relations,
        &gql_name,
        config.camel_case(),
        false,
        false,
    )?;
    ApiRelationDef::pop();
    ApiFieldDef::pop();

    Ok(DocModel {
        name: def.name.clone(),
        cased_name: if config.camel_case() {
            def.name.to_camel_case()
        } else {
            def.name.to_string()
        },
        gql_name,
        label: def.label.clone(),
        pk: crate::model_generator::template::filters::fmt_join(
            def.primaries(),
            "{var}: {gql_type}",
            ",",
        )?,
        has_all_query: def.use_all_rows_cache() && !def.use_filtered_row_cache(),
        use_find_by_pk: api_def.use_find_by_pk,
        use_delete_by_pk: api_def.use_delete_by_pk,
        use_import: api_def.use_import,
        disable_mutation: api_def.disable_mutation,
        disable_update: def.disable_update(),
        readable_roles: api_def.readable_roles(config, group),
        creatable_roles: api_def.creatable_roles(config, group),
        importable_roles: api_def.importable_roles(config, group),
        updatable_roles: api_def.updatable_roles(config, group),
        deletable_roles: api_def.deletable_roles(config, group),
        readable_filter: api_def.readable_filter.clone(),
        updatable_filter: api_def.updatable_filter.clone(),
        deletable_filter: api_def.deletable_filter.clone(),
        selectors: def
            .selectors
            .iter()
            .filter_map(|(n, def)| {
                api_def.selector(n).pop().map(|v| {
                    let mut filters = Vec::new();
                    make_filters(&mut filters, 0, &def.filters);
                    Selector {
                        name: n.to_string(),
                        js_updater: v.js_updater.clone(),
                        use_for_delete: v.use_for_delete,
                        use_for_update_by_operator: v.use_for_update_by_operator,
                        filters,
                        orders: def
                            .orders
                            .iter()
                            .map(|(n, v)| {
                                (
                                    n.to_string(),
                                    Order {
                                        fields: v
                                            .fields
                                            .iter()
                                            .map(|(m, _)| m.to_string())
                                            .collect(),
                                        direction: v.direction,
                                        direct_sql: v.direct_sql.clone(),
                                    },
                                )
                            })
                            .collect(),
                    }
                })
            })
            .collect(),
        fields: fields(def, 0, config.camel_case()),
        relations,
        all_fields,
        all_relations,
    })
}

fn make_filters(buf: &mut Vec<Filter>, indent: usize, filters: &IndexMap<String, FilterDef>) {
    for (name, filter) in filters {
        buf.push(Filter {
            name: name.to_string(),
            indent,
            _type: filter._type,
            fields: filter.fields.iter().map(|(n, _)| n.clone()).collect(),
            required: filter.required,
            relation: filter.relation.clone(),
            relation_fields: filter.relation_fields.clone(),
            json_path: filter.json_path.clone(),
            query: filter.query.clone(),
        });
        make_filters(buf, indent + 1, &filter.relation_fields);
    }
}

#[allow(clippy::too_many_arguments)]
fn make_relation(
    def: &Arc<ModelDef>,
    indent: usize,
    all_fields: &mut Vec<Field>,
    all_relations: &mut Vec<Arc<Relation>>,
    gql_name: &str,
    camel_case: bool,
    no_read: bool,
    no_update: bool,
) -> Result<Vec<Arc<Relation>>> {
    let mut relations = Vec::new();
    for (_model, rel_name, rel) in def.relations_one(false) {
        let rel_model = rel.get_foreign_model();
        let api_relation = ApiRelationDef::get(rel_name).unwrap();
        let rel_id = &rel.get_foreign_id(def);
        ApiRelationDef::push(api_relation.relations(&rel_model)?);
        ApiFieldDef::push(api_relation.fields(&rel_model, rel_id)?);
        let gql_name = format!("{}{}", gql_name, rel_name.to_case(Case::Pascal));
        let index = all_relations.len();
        all_fields.push(Field {
            name: rel_name.to_string(),
            cased_name: if camel_case {
                rel_name.to_camel_case()
            } else {
                rel_name.to_string()
            },
            indent,
            label: rel.label.clone(),
            data_type: None,
            signed: false,
            gql_type: None,
            no_read: no_read || api_relation.visibility == Some(RelationVisibility::WriteOnly),
            no_update: no_update || api_relation.visibility == Some(RelationVisibility::ReadOnly),
            required: false,
            disable_update: false,
            has_many: false,
            replace: api_relation.use_replace,
            validator: None,
            default: None,
            on_insert_formula: None,
            on_update_formula: None,
        });
        all_fields.append(&mut fields(&rel_model, indent + 1, camel_case));
        let _relations = make_relation(
            &rel_model,
            indent + 1,
            all_fields,
            all_relations,
            &gql_name,
            camel_case,
            no_read,
            no_update,
        )?;
        let relation = Arc::new(Relation {
            name: rel_name.to_string(),
            label: rel.label.clone(),
            gql_name,
            has_many: false,
            no_read: no_read || api_relation.visibility == Some(RelationVisibility::WriteOnly),
            no_update: no_update || api_relation.visibility == Some(RelationVisibility::ReadOnly),
            replace: api_relation.use_replace,
            fields: fields(&rel_model, 0, camel_case),
            relations: _relations,
        });
        relations.push(relation.clone());
        all_relations.insert(index, relation);
        ApiRelationDef::pop();
        ApiFieldDef::pop();
    }
    for (_model, rel_name, rel) in def.relations_many(false) {
        let rel_model = rel.get_foreign_model();
        let api_relation = ApiRelationDef::get(rel_name).unwrap();
        let rel_id = &rel.get_foreign_id(def);
        ApiRelationDef::push(api_relation.relations(&rel_model)?);
        ApiFieldDef::push(api_relation.fields(&rel_model, rel_id)?);
        let gql_name = format!("{}{}", gql_name, rel_name.to_case(Case::Pascal));
        let index = all_relations.len();
        all_fields.push(Field {
            name: rel_name.to_string(),
            cased_name: if camel_case {
                rel_name.to_camel_case()
            } else {
                rel_name.to_string()
            },
            indent,
            label: rel.label.clone(),
            data_type: None,
            signed: false,
            gql_type: None,
            no_read: no_read || api_relation.visibility == Some(RelationVisibility::WriteOnly),
            no_update: no_update || api_relation.visibility == Some(RelationVisibility::ReadOnly),
            required: false,
            disable_update: false,
            has_many: true,
            replace: false,
            validator: None,
            default: None,
            on_insert_formula: None,
            on_update_formula: None,
        });
        all_fields.append(&mut fields(&rel_model, indent + 1, camel_case));
        let _relations = make_relation(
            &rel_model,
            indent + 1,
            all_fields,
            all_relations,
            &gql_name,
            camel_case,
            no_read,
            no_update,
        )?;
        let relation = Arc::new(Relation {
            name: rel_name.to_string(),
            label: rel.label.clone(),
            gql_name,
            has_many: true,
            no_read: no_read || api_relation.visibility == Some(RelationVisibility::WriteOnly),
            no_update: no_update || api_relation.visibility == Some(RelationVisibility::ReadOnly),
            replace: false,
            fields: fields(&rel_model, 0, camel_case),
            relations: _relations,
        });
        relations.push(relation.clone());
        all_relations.insert(index, relation);
        ApiRelationDef::pop();
        ApiFieldDef::pop();
    }
    for (_model, rel_name, rel) in def.relations_belonging(false) {
        let rel_model = rel.get_foreign_model();
        let api_relation = ApiRelationDef::get(rel_name).unwrap();
        ApiRelationDef::push(api_relation.relations(&rel_model)?);
        ApiFieldDef::push(api_relation.fields(&rel_model, &[])?);
        let gql_name = format!("{}{}", gql_name, rel_name.to_case(Case::Pascal));
        let index = all_relations.len();
        all_fields.push(Field {
            name: rel_name.to_string(),
            cased_name: if camel_case {
                rel_name.to_camel_case()
            } else {
                rel_name.to_string()
            },
            indent,
            label: rel.label.clone(),
            data_type: None,
            signed: false,
            gql_type: None,
            no_read: false,
            no_update: true,
            required: false,
            disable_update: false,
            has_many: false,
            replace: false,
            validator: None,
            default: None,
            on_insert_formula: None,
            on_update_formula: None,
        });
        all_fields.append(&mut fields(&rel_model, indent + 1, camel_case));
        let _relations = make_relation(
            &rel_model,
            indent + 1,
            all_fields,
            all_relations,
            &gql_name,
            camel_case,
            false,
            true,
        )?;
        let relation = Arc::new(Relation {
            name: rel_name.to_string(),
            label: rel.label.clone(),
            gql_name,
            has_many: false,
            no_read: false,
            no_update: true,
            replace: false,
            fields: fields(&rel_model, 0, camel_case),
            relations: _relations,
        });
        relations.push(relation.clone());
        all_relations.insert(index, relation);
        ApiRelationDef::pop();
        ApiFieldDef::pop();
    }
    Ok(relations)
}

fn fields(def: &Arc<ModelDef>, indent: usize, camel_case: bool) -> Vec<Field> {
    let mut fields = Vec::new();
    for (name, field) in def
        .merged_fields
        .iter()
        .filter(|(k, _v)| ApiFieldDef::has(k))
    {
        let response: HashMap<_, _> = def.for_api_response().into_iter().collect();
        let request: HashMap<_, _> = def.for_api_request().into_iter().collect();
        if !response.contains_key(name) && !request.contains_key(name) {
            continue;
        }
        fields.push(Field {
            name: name.to_string(),
            cased_name: if camel_case {
                name.to_camel_case()
            } else {
                name.to_string()
            },
            indent,
            label: field.label.clone(),
            data_type: Some(field.data_type),
            signed: field.signed,
            gql_type: Some(field.get_gql_type()),
            no_read: !response.contains_key(name),
            no_update: !request.contains_key(name),
            required: field.api_required(name) && request.contains_key(name),
            disable_update: ApiFieldDef::disable_update(name),
            has_many: false,
            replace: false,
            validator: ApiFieldDef::validator(name),
            default: ApiFieldDef::default(name),
            on_insert_formula: ApiFieldDef::on_insert_formula(name),
            on_update_formula: ApiFieldDef::on_update_formula(name),
        });
    }
    fields
}
