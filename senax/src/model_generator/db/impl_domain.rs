use anyhow::{Result, ensure};
use askama::Template;
use convert_case::{Case, Casing};
use regex::Regex;
use std::collections::{BTreeSet, HashSet};
use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use crate::common::{OVERWRITTEN_MSG, fs_write};
use crate::schema::{_to_var_name, ConfigDef, GroupsDef, ModelDef, set_domain_mode};
use crate::{SEPARATED_BASE_FILES, filters};

pub fn write_impl_domain_rs(
    model_src_dir: &Path,
    db: &str,
    groups: &GroupsDef,
    force: bool,
) -> Result<()> {
    let file_path = model_src_dir.join("impl_domain.rs");
    let content = if force || !file_path.exists() {
        #[derive(Template)]
        #[template(path = "db/src/impl_domain.rs", escape = "none")]
        pub struct ImplDomainDbTemplate<'a> {
            pub db: &'a str,
        }

        ImplDomainDbTemplate { db }.render()?
    } else {
        fs::read_to_string(&file_path)?.replace("\r\n", "\n")
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
@%- for (name, (_, defs)) in groups %@
pub use _base::impl_domain::@{ name|snake|to_var_name }@;
@%- endfor %@
// Do not modify up to this line. (ModEnd)"###,
        ext = "txt",
        escape = "none"
    )]
    pub struct ModTemplate<'a> {
        pub groups: &'a GroupsDef,
    }

    let tpl = ModTemplate { groups }.render()?;
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
    @%- for (name, (_, defs)) in groups %@
    get_repo!(@{ name|snake|to_var_name }@, dyn _repository::@{ name|snake|to_var_name }@::@{ name|pascal }@Repository, _repo_@{ name|snake }@::impl_domain::@{ name|snake|to_var_name }@::@{ name|pascal }@RepositoryImpl);
    @%- endfor %@
    // Do not modify up to this line. (RepoEnd)"###,
        ext = "txt",
        escape = "none"
    )]
    pub struct ImplDomainDbRepoTemplate<'a> {
        pub groups: &'a GroupsDef,
    }

    let tpl = ImplDomainDbRepoTemplate { groups }.render()?;
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
    @%- for (name, (_, defs)) in groups %@
    get_repo!(@{ name|snake|to_var_name }@, dyn _repository::@{ name|snake|to_var_name }@::@{ name|pascal }@QueryService, _repo_@{ name|snake }@::impl_domain::@{ name|snake|to_var_name }@::@{ name|pascal }@QueryServiceImpl);
    @%- endfor %@
    // Do not modify up to this line. (QueryServiceEnd)"###,
        ext = "txt",
        escape = "none"
    )]
    pub struct ImplDomainDbQueryServiceTemplate<'a> {
        pub groups: &'a GroupsDef,
    }

    let tpl = ImplDomainDbQueryServiceTemplate { groups }.render()?;
    let tpl = tpl.trim_start();
    let content = re.replace(&content, tpl);

    fs_write(file_path, &*content)?;
    Ok(())
}

pub fn write_group_rs(
    impl_domain_dir: &Path,
    group_name: &String,
    entities_mod_names: &BTreeSet<(String, &String)>,
    impl_domain_output: String,
    remove_files: &mut HashSet<OsString>,
) -> Result<()> {
    let file_path = impl_domain_dir.join(format!("{}.rs", group_name.to_case(Case::Snake)));
    remove_files.remove(file_path.as_os_str());
    #[derive(Template)]
    #[template(path = "db/base/src/impl_domain/group.rs", escape = "none")]
    struct ImplDomainGroupTemplate {
        pub mod_names: BTreeSet<String>,
        pub impl_domain_output: String,
    }

    let mod_names: BTreeSet<String> = entities_mod_names.iter().map(|v| v.0.clone()).collect();
    let content = ImplDomainGroupTemplate {
        mod_names,
        impl_domain_output,
    }
    .render()?;

    fs_write(file_path, &*content)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn write_entity(
    impl_domain_dir: &Path,
    db: &str,
    config: &ConfigDef,
    group_name: &str,
    mod_name: &str,
    model_name: &str,
    def: &Arc<ModelDef>,
    remove_files: &mut HashSet<OsString>,
) -> Result<String, anyhow::Error> {
    set_domain_mode(true);
    let pascal_name = &model_name.to_case(Case::Pascal);
    #[derive(Template)]
    #[template(path = "db/base/src/impl_domain/entities/entity.rs", escape = "none")]
    pub struct ImplDomainEntityTemplate<'a> {
        pub db: &'a str,
        pub config: &'a ConfigDef,
        pub group_name: &'a str,
        pub mod_name: &'a str,
        pub pascal_name: &'a str,
        pub def: &'a Arc<ModelDef>,
    }

    let tpl = ImplDomainEntityTemplate {
        db,
        config,
        group_name,
        mod_name,
        pascal_name,
        def,
    };
    let ret = tpl.render()?;
    set_domain_mode(false);
    if SEPARATED_BASE_FILES {
        let impl_domain_group_dir = impl_domain_dir.join(group_name.to_case(Case::Snake));
        let file_path = impl_domain_group_dir.join(format!("{}.rs", mod_name));
        remove_files.remove(file_path.as_os_str());
        fs_write(file_path, &format!("{}{}", OVERWRITTEN_MSG, ret))?;
        Ok("".to_string())
    } else {
        Ok(format!(
            "pub mod {} {{\n{}}}\n",
            _to_var_name(mod_name),
            ret
        ))
    }
}
