use anyhow::{Result, ensure};
use askama::Template;
use convert_case::{Case, Casing};
use indexmap::IndexMap;
use regex::Regex;
use std::collections::{BTreeSet, HashSet};
use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use crate::common::fs_write;
use crate::schema::{ConfigDef, ModelDef, set_domain_mode, to_id_name};
use crate::filters;

pub fn write_impl_domain_rs(
    src_dir: &Path,
    db: &str,
    groups: &IndexMap<String, IndexMap<String, Arc<ModelDef>>>,
    force: bool,
) -> Result<()> {
    let file_path = src_dir.join("impl_domain.rs");
    let content = if force || !file_path.exists() {
        #[derive(Template)]
        #[template(path = "model/repositories/src/impl_domain.rs", escape = "none")]
        pub struct ImplDomainDbTemplate<'a> {
            pub db: &'a str,
        }

        ImplDomainDbTemplate { db }.render()?
    } else {
        fs::read_to_string(&file_path)?
    };

    let re = Regex::new(r"(?s)// Do not modify below this line. \(ModStart\).+// Do not modify up to this line. \(ModEnd\)").unwrap();
    ensure!(
        re.is_match(&content),
        "File contents are invalid.: {:?}",
        &file_path
    );
    
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
pub struct ModTemplate<'a> {
    pub groups: &'a IndexMap<String, IndexMap<String, Arc<ModelDef>>>,
}

    let tpl = ModTemplate { groups }.render()?;
    let tpl = tpl.trim_start();
    let content = re.replace(&content, tpl);

    fs_write(file_path, &*content)?;
    Ok(())
}

pub fn write_group_rs(
    impl_domain_dir: &Path,
    db: &str,
    base_group_name: &str,
    group_name: &str,
    entities_mod_names: &BTreeSet<(String, &String)>,
    force: bool,
    remove_files: &mut HashSet<OsString>,
) -> Result<()> {
    let file_path = impl_domain_dir.join(format!("{}.rs", group_name.to_case(Case::Snake)));
    remove_files.remove(file_path.as_os_str());
    let content = if force || !file_path.exists() {
        #[derive(Template)]
        #[template(path = "model/repositories/src/impl_domain/group.rs", escape = "none")]
        struct ImplDomainGroupTemplate<'a> {
            pub db: &'a str,
            pub base_group_name: &'a str,
            pub group_name: &'a str,
        }

        ImplDomainGroupTemplate { db, base_group_name, group_name }.render()?
    } else {
        fs::read_to_string(&file_path)?
    };

    let mod_names: BTreeSet<String> = entities_mod_names.iter().map(|v| v.0.clone()).collect();
    let re = Regex::new(r"(?s)// Do not modify below this line. \(ModStart\).+// Do not modify up to this line. \(ModEnd\)").unwrap();
    ensure!(
        re.is_match(&content),
        "File contents are invalid.: {:?}",
        &file_path
    );
    
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

    let tpl = DomainGroupModTemplate {
        mod_names: &mod_names,
    }
    .render()?;
    let tpl = tpl.trim_start();
    let content = re.replace(&content, tpl);

    let re = Regex::new(r"(?s)// Do not modify below this line. \(RepoStart\).+// Do not modify up to this line. \(RepoEnd\)").unwrap();
    ensure!(
        re.is_match(&content),
        "File contents are invalid.: {:?}",
        &file_path
    );
    
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

    let tpl = ImplDomainGroupRepoTemplate {
        mod_names: entities_mod_names,
    }
    .render()?;
    let tpl = tpl.trim_start();
    let content = re.replace(&content, tpl);

    let re = Regex::new(r"(?s)// Do not modify below this line. \(QueryServiceStart\).+// Do not modify up to this line. \(QueryServiceEnd\)").unwrap();
    ensure!(
        re.is_match(&content),
        "File contents are invalid.: {:?}",
        &file_path
    );
    
#[derive(Template)]
#[template(
    source = r###"
    // Do not modify below this line. (QueryServiceStart)
    @%- for (mod_name, model_name) in mod_names %@
    get_repo!(@{ mod_name|to_var_name }@, dyn _domain::@{ mod_name|to_var_name }@::@{ model_name|pascal }@QueryService, @{ mod_name|to_var_name }@::@{ model_name|pascal }@RepositoryImpl);
    @%- endfor %@
    // Do not modify up to this line. (QueryServiceEnd)"###,
    ext = "txt",
    escape = "none"
)]
pub struct ImplDomainGroupQueryServiceTemplate<'a> {
    pub mod_names: &'a BTreeSet<(String, &'a String)>,
}

    let tpl = ImplDomainGroupQueryServiceTemplate {
        mod_names: entities_mod_names,
    }
    .render()?;
    let tpl = tpl.trim_start();
    let content = re.replace(&content, tpl);

    fs_write(file_path, &*content)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn write_entity(
    impl_domain_dir: &Path,
    db: &str,
    config: &ConfigDef,
    base_group_name: &str,
    group_name: &str,
    mod_name: &str,
    force: bool,
    model_name: &str,
    def: &Arc<ModelDef>,
    remove_files: &mut HashSet<OsString>,
) -> Result<(), anyhow::Error> {
    set_domain_mode(true);
    let impl_domain_group_dir = impl_domain_dir.join(group_name.to_case(Case::Snake));
    let file_path = impl_domain_group_dir.join(format!("{}.rs", mod_name));
    remove_files.remove(file_path.as_os_str());
    let pascal_name = &model_name.to_case(Case::Pascal);
    let id_name = &to_id_name(model_name);
    if force || !file_path.exists() {
        #[derive(Template)]
        #[template(path = "model/repositories/src/impl_domain/entities/entity.rs", escape = "none")]
        pub struct ImplDomainEntityTemplate<'a> {
            pub db: &'a str,
            pub base_group_name: &'a str,
            pub group_name: &'a str,
            pub mod_name: &'a str,
            pub pascal_name: &'a str,
            pub id_name: &'a str,
            pub def: &'a Arc<ModelDef>,
        }

        let tpl = ImplDomainEntityTemplate {
            db,
            base_group_name,
            group_name,
            mod_name,
            pascal_name,
            id_name,
            def,
        };
        fs_write(file_path, tpl.render()?)?;
    }
    let path = impl_domain_group_dir.join("_base");
    let file_path = path.join(format!("_{}.rs", mod_name));
    remove_files.remove(file_path.as_os_str());

    #[derive(Template)]
    #[template(
        path = "model/repositories/src/impl_domain/entities/base/_entity.rs",
        escape = "none"
    )]
    pub struct ImplDomainBaseEntityTemplate<'a> {
        pub db: &'a str,
        pub config: &'a ConfigDef,
        pub base_group_name: &'a str,
        pub group_name: &'a str,
        pub mod_name: &'a str,
        pub pascal_name: &'a str,
        pub id_name: &'a str,
        pub def: &'a Arc<ModelDef>,
    }

    let tpl = ImplDomainBaseEntityTemplate {
        db,
        config,
        base_group_name,
        group_name,
        mod_name,
        pascal_name,
        id_name,
        def,
    };
    fs_write(file_path, tpl.render()?)?;
    set_domain_mode(false);
    Ok(())
}

