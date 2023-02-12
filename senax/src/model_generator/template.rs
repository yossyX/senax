use crate::model_generator::schema::*;
use askama::Template;
use indexmap::IndexMap;
use std::{collections::BTreeSet, sync::Arc};

pub mod filters;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionType {
    None,
    Actix,
}

#[derive(Template)]
#[template(path = "model/_Cargo.toml", escape = "none")]
pub struct CargoTemplate<'a> {
    pub db: &'a str,
    pub as_session: SessionType,
}

#[derive(Template)]
#[template(path = "model/build.rs", escape = "none")]
pub struct BuildTemplate {}

#[derive(Template)]
#[template(path = "model/src/lib.rs", escape = "none")]
pub struct LibTemplate<'a> {
    pub db: &'a str,
    pub groups: &'a IndexMap<String, IndexMap<String, Arc<ModelDef>>>,
    pub config: &'a ConfigDef,
}

#[derive(Template)]
#[template(path = "model/src/main.rs", escape = "none")]
pub struct MainTemplate<'a> {
    pub db: &'a str,
}

#[derive(Template)]
#[template(path = "model/src/seeder.rs", escape = "none")]
pub struct SeederTemplate<'a> {
    pub groups: &'a IndexMap<String, IndexMap<String, Arc<ModelDef>>>,
}

#[derive(Template)]
#[template(path = "model/src/group.rs", escape = "none")]
pub struct GroupTemplate<'a> {
    pub group_name: &'a str,
    pub mod_names: &'a BTreeSet<&'a str>,
    pub tables: IndexMap<&'a String, &'a Arc<ModelDef>>,
    pub config: &'a ConfigDef,
}

#[derive(Template)]
#[template(path = "model/src/accessor.rs", escape = "none")]
pub struct AccessorTemplate {}

#[derive(Template)]
#[template(path = "model/src/cache.rs", escape = "none")]
pub struct CacheTemplate {}

#[derive(Template)]
#[template(path = "model/src/misc.rs", escape = "none")]
pub struct MiscTemplate<'a> {
    pub db: &'a str,
    pub config: &'a ConfigDef,
}

#[derive(Template)]
#[template(path = "model/src/connection.rs", escape = "none")]
pub struct ConnectionTemplate<'a> {
    pub db: &'a str,
    pub config: &'a ConfigDef,
    pub tx_isolation: Option<&'a str>,
    pub read_tx_isolation: Option<&'a str>,
}

#[derive(Template)]
#[template(path = "model/src/group/enum.rs", escape = "none")]
pub struct GroupEnumTemplate<'a> {
    pub db: &'a str,
    pub group_name: &'a str,
    pub mod_name: &'a str,
    pub name: &'a str,
    pub def: &'a EnumDef,
    pub config: &'a ConfigDef,
}

#[derive(Template)]
#[template(path = "model/src/group/base/_enum.rs", escape = "none")]
pub struct GroupBaseEnumTemplate<'a> {
    pub db: &'a str,
    pub group_name: &'a str,
    pub mod_name: &'a str,
    pub name: &'a str,
    pub pascal_name: &'a str,
    pub def: &'a EnumDef,
    pub config: &'a ConfigDef,
}

#[derive(Template)]
#[template(path = "model/src/group/table.rs", escape = "none")]
pub struct GroupTableTemplate<'a> {
    pub db: &'a str,
    pub group_name: &'a str,
    pub mod_name: &'a str,
    pub name: &'a str,
    pub id_name: &'a str,
    pub def: &'a Arc<ModelDef>,
    pub config: &'a ConfigDef,
}

#[derive(Template)]
#[template(path = "model/src/group/base/_table.rs", escape = "none")]
pub struct GroupBaseTableTemplate<'a> {
    pub db: &'a str,
    pub group_name: &'a str,
    pub mod_name: &'a str,
    pub name: &'a str,
    pub pascal_name: &'a str,
    pub id_name: &'a str,
    pub table_name: &'a str,
    pub def: &'a Arc<ModelDef>,
    pub config: &'a ConfigDef,
    pub version_col: &'a str,
}

#[derive(Template)]
#[template(path = "model/src/group/abstract.rs", escape = "none")]
pub struct GroupAbstractTemplate<'a> {
    pub db: &'a str,
    pub group_name: &'a str,
    pub mod_name: &'a str,
    pub name: &'a str,
    pub def: &'a Arc<ModelDef>,
    pub config: &'a ConfigDef,
}

#[derive(Template)]
#[template(path = "model/src/group/base/_abstract.rs", escape = "none")]
pub struct GroupBaseAbstractTemplate<'a> {
    pub db: &'a str,
    pub group_name: &'a str,
    pub mod_name: &'a str,
    pub name: &'a str,
    pub pascal_name: &'a str,
    pub id_name: &'a str,
    pub table_name: &'a str,
    pub def: &'a Arc<ModelDef>,
    pub config: &'a ConfigDef,
}
