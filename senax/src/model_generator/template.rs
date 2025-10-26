use crate::model_generator::schema::*;
use askama::Template;
use compact_str::CompactString;
use indexmap::IndexMap;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

pub mod filters;

#[derive(Template)]
#[template(path = "model/_Cargo.toml", escape = "none")]
pub struct CargoTemplate<'a> {
    pub db: &'a str,
    pub config: &'a ConfigDef,
}

#[derive(Template)]
#[template(path = "model/build.rs", escape = "none")]
pub struct BuildTemplate {}

#[derive(Template)]
#[template(path = "model/src/lib.rs", escape = "none")]
pub struct LibTemplate<'a> {
    pub db: &'a str,
    pub config: &'a ConfigDef,
    pub non_snake_case: bool,
}

#[derive(Template)]
#[template(path = "model/src/models.rs", escape = "none")]
pub struct ModelsTemplate<'a> {
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
    pub mod_names: &'a BTreeSet<String>,
    pub models: IndexMap<&'a String, &'a Arc<ModelDef>>,
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
    pub config: &'a ConfigDef,
}

#[derive(Template)]
#[template(path = "model/src/connection.rs", escape = "none")]
pub struct ConnectionTemplate<'a> {
    pub db: &'a str,
    pub config: &'a ConfigDef,
    pub tx_isolation: Option<&'a str>,
    pub read_tx_isolation: Option<&'a str>,
    pub groups: &'a IndexMap<String, IndexMap<String, Arc<ModelDef>>>,
}

#[allow(dead_code)]
#[derive(Template)]
#[template(path = "model/src/group/table.rs", escape = "none")]
pub struct GroupTableTemplate<'a> {
    pub db: &'a str,
    pub group_name: &'a str,
    pub mod_name: &'a str,
    pub model_name: &'a str,
    pub pascal_name: &'a str,
    pub id_name: &'a str,
    pub def: &'a Arc<ModelDef>,
    pub config: &'a ConfigDef,
    pub visibility: &'a str,
}

#[derive(Template)]
#[template(path = "model/src/group/base/_table.rs", escape = "none")]
pub struct GroupBaseTableTemplate<'a> {
    pub db: &'a str,
    pub group_name: &'a str,
    pub mod_name: &'a str,
    pub model_name: &'a str,
    pub pascal_name: &'a str,
    pub id_name: &'a str,
    pub table_name: &'a str,
    pub def: &'a Arc<ModelDef>,
    pub force_indexes: Vec<(String, String)>,
    pub config: &'a ConfigDef,
    pub version_col: CompactString,
    pub visibility: &'a str,
}

#[allow(dead_code)]
#[derive(Template)]
#[template(path = "model/src/group/abstract.rs", escape = "none")]
pub struct GroupAbstractTemplate<'a> {
    pub db: &'a str,
    pub group_name: &'a str,
    pub mod_name: &'a str,
    pub name: &'a str,
    pub pascal_name: &'a str,
    pub def: &'a Arc<ModelDef>,
    pub config: &'a ConfigDef,
    pub visibility: &'a str,
}

#[allow(dead_code)]
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
    pub visibility: &'a str,
}

#[derive(Template)]
#[template(path = "init/domain/src/models.rs", escape = "none")]
pub struct DomainModelsTemplate;

#[derive(Template)]
#[template(
    source = r###"
pub mod @{ db|snake|to_var_name }@;
// Do not modify this line. (Mod:@{ all }@)"###,
    ext = "txt",
    escape = "none"
)]
pub struct DomainModelsModTemplate<'a> {
    pub all: String,
    pub db: &'a str,
}

#[derive(Template)]
#[template(
    source = r###"
#[cfg(any(feature = "mock", test))]
use self::@{ db|snake|to_var_name }@::@{ db|pascal }@Repositories as _;
// Do not modify this line. (UseRepo)"###,
    ext = "txt",
    escape = "none"
)]
pub struct DomainModelsUseRepoTemplate<'a> {
    pub db: &'a str,
}

#[derive(Template)]
#[template(
    source = r###"
    fn @{ db|snake }@_repository(&self) -> Box<dyn @{ db|snake|to_var_name }@::@{ db|pascal }@Repositories>;
    fn @{ db|snake }@_query(&self) -> Box<dyn @{ db|snake|to_var_name }@::@{ db|pascal }@Queries>;
    // Do not modify this line. (Repo)"###,
    ext = "txt",
    escape = "none"
)]
pub struct DomainModelsRepoTemplate<'a> {
    pub db: &'a str,
}

#[derive(Template)]
#[template(
    source = r###"
    pub @{ db|snake|to_var_name }@: @{ db|snake|to_var_name }@::Emu@{ db|pascal }@Repositories,
    // Do not modify this line. (EmuRepo)"###,
    ext = "txt",
    escape = "none"
)]
pub struct DomainModelsEmuRepoTemplate<'a> {
    pub db: &'a str,
}

#[derive(Template)]
#[template(
    source = r###"
    fn @{ db|snake|to_var_name }@_repository(&self) -> Box<dyn @{ db|snake|to_var_name }@::@{ db|pascal }@Repositories> {
        Box::new(self.@{ db|snake|to_var_name }@.clone())
    }
    fn @{ db|snake|to_var_name }@_query(&self) -> Box<dyn @{ db|snake|to_var_name }@::@{ db|pascal }@Queries> {
        Box::new(self.@{ db|snake|to_var_name }@.clone())
    }
    // Do not modify this line. (EmuImpl)"###,
    ext = "txt",
    escape = "none"
)]
pub struct DomainModelsEmuImplTemplate<'a> {
    pub db: &'a str,
}

#[derive(Template)]
#[template(
    source = r###"
        self.@{ db|snake|to_var_name }@.begin().await?;
        // Do not modify this line. (EmuImplStart)"###,
    ext = "txt",
    escape = "none"
)]
pub struct DomainModelsEmuImplStartTemplate<'a> {
    pub db: &'a str,
}

#[derive(Template)]
#[template(
    source = r###"
        self.@{ db|snake|to_var_name }@.commit().await?;
        // Do not modify this line. (EmuImplCommit)"###,
    ext = "txt",
    escape = "none"
)]
pub struct DomainModelsEmuImplCommitTemplate<'a> {
    pub db: &'a str,
}

#[derive(Template)]
#[template(
    source = r###"
        self.@{ db|snake|to_var_name }@.rollback().await?;
        // Do not modify this line. (EmuImplRollback)"###,
    ext = "txt",
    escape = "none"
)]
pub struct DomainModelsEmuImplRollbackTemplate<'a> {
    pub db: &'a str,
}

#[allow(dead_code)]
#[derive(Template)]
#[template(path = "init/domain/src/value_objects/base.rs", escape = "none")]
pub struct DomainValueObjectBaseTemplate<'a> {
    pub mod_name: &'a str,
    pub pascal_name: &'a str,
    pub def: &'a FieldDef,
}

#[derive(Template)]
#[template(path = "init/domain/src/value_objects/wrapper.rs", escape = "none")]
pub struct DomainValueObjectWrapperTemplate<'a> {
    pub mod_name: &'a str,
    pub pascal_name: &'a str,
}

#[derive(Template)]
#[template(path = "init/domain/src/models/db.rs", escape = "none")]
pub struct DomainDbTemplate<'a> {
    pub db: &'a str,
}

#[derive(Template)]
#[template(path = "init/impl_domain/db.rs", escape = "none")]
pub struct ImplDomainDbTemplate<'a> {
    pub db: &'a str,
}

#[derive(Template)]
#[template(
    source = r###"
// Do not modify below this line. (ModStart)
@%- for (name, defs) in groups %@
pub mod @{ name|snake|to_var_name }@;
@%- endfor %@
// Do not modify up to this line. (ModEnd)"###,
    ext = "txt",
    escape = "none"
)]
pub struct DomainDbModTemplate<'a> {
    pub groups: &'a IndexMap<String, IndexMap<String, Arc<ModelDef>>>,
}

#[derive(Template)]
#[template(
    source = r###"
    // Do not modify below this line. (RepoStart)
    @%- for (name, defs) in groups %@
    fn @{ name|snake|to_var_name }@(&self) -> Box<dyn @{ name|snake|to_var_name }@::@{ name|pascal }@Repositories>;
    @%- endfor %@
    // Do not modify up to this line. (RepoEnd)"###,
    ext = "txt",
    escape = "none"
)]
pub struct DomainDbRepoTemplate<'a> {
    pub groups: &'a IndexMap<String, IndexMap<String, Arc<ModelDef>>>,
}

#[derive(Template)]
#[template(
    source = r###"
    // Do not modify below this line. (QueriesStart)
    @%- for (name, defs) in groups %@
    fn @{ name|snake|to_var_name }@(&self) -> Box<dyn @{ name|snake|to_var_name }@::@{ name|pascal }@Queries>;
    @%- endfor %@
    // Do not modify up to this line. (QueriesEnd)"###,
    ext = "txt",
    escape = "none"
)]
pub struct QueriesDbQueriesTemplate<'a> {
    pub groups: &'a IndexMap<String, IndexMap<String, Arc<ModelDef>>>,
}

#[derive(Template)]
#[template(
    source = r###"
    // Do not modify below this line. (EmuRepoStart)
    @%- for (name, defs) in groups %@
    get_emu_group!(@{ name|snake|to_var_name }@, dyn @{ name|snake|to_var_name }@::@{ name|pascal }@Repositories, @{ name|snake|to_var_name }@::Emu@{ name|pascal }@Repositories);
    @%- endfor %@
    // Do not modify up to this line. (EmuRepoEnd)"###,
    ext = "txt",
    escape = "none"
)]
pub struct DomainDbEmuRepoTemplate<'a> {
    pub groups: &'a IndexMap<String, IndexMap<String, Arc<ModelDef>>>,
}

#[derive(Template)]
#[template(
    source = r###"
    // Do not modify below this line. (EmuQueriesStart)
    @%- for (name, defs) in groups %@
    get_emu_group!(@{ name|snake|to_var_name }@, dyn @{ name|snake|to_var_name }@::@{ name|pascal }@Queries, @{ name|snake|to_var_name }@::Emu@{ name|pascal }@Queries);
    @%- endfor %@
    // Do not modify up to this line. (EmuQueriesEnd)"###,
    ext = "txt",
    escape = "none"
)]
pub struct DomainDbEmuQueriesTemplate<'a> {
    pub groups: &'a IndexMap<String, IndexMap<String, Arc<ModelDef>>>,
}

#[derive(Template)]
#[template(
    source = r###"
    // Do not modify below this line. (RepoStart)
    @%- for (name, defs) in groups %@
    get_repo!(@{ name|snake|to_var_name }@, dyn _domain::@{ name|snake|to_var_name }@::@{ name|pascal }@Repositories, @{ name|snake|to_var_name }@::@{ name|pascal }@RepositoriesImpl);
    @%- endfor %@
    // Do not modify up to this line. (RepoEnd)"###,
    ext = "txt",
    escape = "none"
)]
pub struct ImplDomainDbRepoTemplate<'a> {
    pub groups: &'a IndexMap<String, IndexMap<String, Arc<ModelDef>>>,
}

#[derive(Template)]
#[template(
    source = r###"
    // Do not modify below this line. (QueriesStart)
    @%- for (name, defs) in groups %@
    get_repo!(@{ name|snake|to_var_name }@, dyn _domain::@{ name|snake|to_var_name }@::@{ name|pascal }@Queries, @{ name|snake|to_var_name }@::@{ name|pascal }@QueriesImpl);
    @%- endfor %@
    // Do not modify up to this line. (QueriesEnd)"###,
    ext = "txt",
    escape = "none"
)]
pub struct ImplDomainDbQueriesTemplate<'a> {
    pub groups: &'a IndexMap<String, IndexMap<String, Arc<ModelDef>>>,
}

#[derive(Template)]
#[template(path = "init/domain/src/models/group.rs", escape = "none")]
pub struct DomainGroupTemplate<'a> {
    pub group_name: &'a str,
}

#[derive(Template)]
#[template(
    source = r###"
// Do not modify below this line. (ModStart)
pub mod _base {
@%- for mod_name in mod_names %@
    pub mod _@{ mod_name }@;
@%- endfor %@
}
@%- for mod_name in mod_names %@
pub mod @{ mod_name|to_var_name }@;
@%- endfor %@
// Do not modify up to this line. (ModEnd)"###,
    ext = "txt",
    escape = "none"
)]
pub struct DomainGroupModTemplate<'a> {
    pub mod_names: &'a BTreeSet<String>,
}

#[derive(Template)]
#[template(
    source = r###"
    // Do not modify below this line. (RepoStart)
    @%- for (mod_name, model_name) in mod_names %@
    fn @{ mod_name|to_var_name }@(&self) -> Box<dyn @{ mod_name|to_var_name }@::@{ model_name|pascal }@Repository>;
    @%- endfor %@
    // Do not modify up to this line. (RepoEnd)"###,
    ext = "txt",
    escape = "none"
)]
pub struct DomainGroupRepoTemplate<'a> {
    pub mod_names: &'a BTreeSet<(String, &'a String)>,
}

#[derive(Template)]
#[template(
    source = r###"
    // Do not modify below this line. (QueriesStart)
    @%- for (mod_name, model_name) in mod_names %@
    fn @{ mod_name|to_var_name }@(&self) -> Box<dyn @{ mod_name|to_var_name }@::@{ model_name|pascal }@Query>;
    @%- endfor %@
    // Do not modify up to this line. (QueriesEnd)"###,
    ext = "txt",
    escape = "none"
)]
pub struct DomainGroupQueriesTemplate<'a> {
    pub mod_names: &'a BTreeSet<(String, &'a String)>,
}

#[derive(Template)]
#[template(
    source = r###"
    // Do not modify below this line. (EmuRepoStart)
    @%- for (mod_name, model_name) in mod_names %@
    get_emu_repo!(@{ mod_name|to_var_name }@, dyn @{ mod_name|to_var_name }@::@{ model_name|pascal }@Repository, @{ mod_name|to_var_name }@::Emu@{ model_name|pascal }@Repository);
    @%- endfor %@
    // Do not modify up to this line. (EmuRepoEnd)"###,
    ext = "txt",
    escape = "none"
)]
pub struct DomainGroupEmuRepoTemplate<'a> {
    pub mod_names: &'a BTreeSet<(String, &'a String)>,
}

#[derive(Template)]
#[template(
    source = r###"
    // Do not modify below this line. (EmuQueriesStart)
    @%- for (mod_name, model_name) in mod_names %@
    get_emu_repo!(@{ mod_name|to_var_name }@, dyn @{ mod_name|to_var_name }@::@{ model_name|pascal }@Query, @{ mod_name|to_var_name }@::Emu@{ model_name|pascal }@Repository);
    @%- endfor %@
    // Do not modify up to this line. (EmuQueriesEnd)"###,
    ext = "txt",
    escape = "none"
)]
pub struct DomainGroupEmuQueriesTemplate<'a> {
    pub mod_names: &'a BTreeSet<(String, &'a String)>,
}

#[derive(Template)]
#[template(path = "init/impl_domain/group.rs", escape = "none")]
pub struct ImplDomainGroupTemplate<'a> {
    pub db: &'a str,
    pub group_name: &'a str,
}

#[derive(Template)]
#[template(
    source = r###"
    // Do not modify below this line. (RepoStart)
    @%- for (mod_name, model_name) in mod_names %@
    get_repo!(@{ mod_name|to_var_name }@, dyn _domain::@{ mod_name|to_var_name }@::@{ model_name|pascal }@Repository, @{ mod_name|to_var_name }@::@{ model_name|pascal }@RepositoryImpl);
    @%- endfor %@
    // Do not modify up to this line. (RepoEnd)"###,
    ext = "txt",
    escape = "none"
)]
pub struct ImplDomainGroupRepoTemplate<'a> {
    pub mod_names: &'a BTreeSet<(String, &'a String)>,
}

#[derive(Template)]
#[template(
    source = r###"
    // Do not modify below this line. (QueriesStart)
    @%- for (mod_name, model_name) in mod_names %@
    get_repo!(@{ mod_name|to_var_name }@, dyn _domain::@{ mod_name|to_var_name }@::@{ model_name|pascal }@Query, @{ mod_name|to_var_name }@::@{ model_name|pascal }@RepositoryImpl);
    @%- endfor %@
    // Do not modify up to this line. (QueriesEnd)"###,
    ext = "txt",
    escape = "none"
)]
pub struct ImplDomainGroupQueriesTemplate<'a> {
    pub mod_names: &'a BTreeSet<(String, &'a String)>,
}

#[derive(Template)]
#[template(
    source = r###"
// Do not modify below this line. (ModStart)
mod _base {
@%- for (mod_name, _) in mod_names %@
    pub mod _@{ mod_name }@;
@%- endfor %@
}
@%- for (mod_name, _) in mod_names %@
mod @{ mod_name|to_var_name }@;
@%- endfor %@
@%- for (mod_name, name) in mod_names %@
pub use @{ mod_name|to_var_name }@::@{ name }@;
@%- endfor %@
// Do not modify up to this line. (ModEnd)"###,
    ext = "txt",
    escape = "none"
)]
pub struct DomainValueObjectModsTemplate<'a> {
    pub mod_names: &'a BTreeMap<String, String>,
}

#[derive(Template)]
#[template(path = "init/domain/src/models/entities/entity.rs", escape = "none")]
pub struct DomainEntityTemplate<'a> {
    pub db: &'a str,
    pub group_name: &'a str,
    pub mod_name: &'a str,
    pub pascal_name: &'a str,
    pub id_name: &'a str,
    pub def: &'a Arc<ModelDef>,
    pub model_id: u64,
}

#[derive(Template)]
#[template(
    path = "init/domain/src/models/entities/base/_entity.rs",
    escape = "none"
)]
pub struct DomainBaseEntityTemplate<'a> {
    pub db: &'a str,
    pub config: &'a ConfigDef,
    pub group_name: &'a str,
    pub mod_name: &'a str,
    pub model_name: &'a str,
    pub pascal_name: &'a str,
    pub id_name: &'a str,
    pub def: &'a Arc<ModelDef>,
}

#[derive(Template)]
#[template(path = "init/domain/src/models/entities/abstract.rs", escape = "none")]
pub struct DomainAbstractTemplate<'a> {
    pub mod_name: &'a str,
    pub pascal_name: &'a str,
    pub def: &'a Arc<ModelDef>,
}

#[derive(Template)]
#[template(
    path = "init/domain/src/models/entities/base/_abstract.rs",
    escape = "none"
)]
pub struct DomainBaseAbstractTemplate<'a> {
    pub db: &'a str,
    pub group_name: &'a str,
    pub mod_name: &'a str,
    pub pascal_name: &'a str,
    pub def: &'a Arc<ModelDef>,
}

#[allow(dead_code)]
#[derive(Template)]
#[template(path = "init/impl_domain/entities/entity.rs", escape = "none")]
pub struct ImplDomainEntityTemplate<'a> {
    pub db: &'a str,
    pub group_name: &'a str,
    pub mod_name: &'a str,
    pub pascal_name: &'a str,
    pub id_name: &'a str,
    pub def: &'a Arc<ModelDef>,
}

#[allow(dead_code)]
#[derive(Template)]
#[template(path = "init/impl_domain/entities/base/_entity.rs", escape = "none")]
pub struct ImplDomainBaseEntityTemplate<'a> {
    pub db: &'a str,
    pub config: &'a ConfigDef,
    pub group_name: &'a str,
    pub mod_name: &'a str,
    pub pascal_name: &'a str,
    pub id_name: &'a str,
    pub def: &'a Arc<ModelDef>,
}
