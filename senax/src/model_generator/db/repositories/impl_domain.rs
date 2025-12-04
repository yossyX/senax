use anyhow::{Result, ensure};
use askama::Template;
use regex::Regex;
use std::collections::{BTreeSet, HashSet};
use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use crate::common::ToCase as _;
use crate::common::{OVERWRITTEN_MSG, fs_write};
use crate::schema::{ConfigDef, GroupsDef, ModelDef, set_domain_mode};
use crate::{SEPARATED_BASE_FILES, filters};

pub fn write_impl_domain_rs(
    src_dir: &Path,
    db: &str,
    group_name: &str,
    groups: &GroupsDef,
    force: bool,
    remove_files: &mut HashSet<OsString>,
) -> Result<()> {
    let file_path = src_dir.join("impl_domain.rs");
    remove_files.remove(file_path.as_os_str());
    let content = if force || !file_path.exists() {
        #[derive(Template)]
        #[template(path = "db/repositories/src/impl_domain.rs", escape = "none")]
        struct ImplDomainDbTemplate<'a> {
            db: &'a str,
            group_name: &'a str,
        }

        ImplDomainDbTemplate { db, group_name }.render()?
    } else {
        fs::read_to_string(&file_path)?.replace("\r\n", "\n")
    };

    let re = Regex::new(r"(?s)// Do not modify below this line. \(ModStart\).+// Do not modify above this line. \(ModEnd\)").unwrap();
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
pub@% if !name.eq(group_name) %@(crate)@% endif %@ mod @{ name|snake|ident }@;
@%- endfor %@
// Do not modify above this line. (ModEnd)"###,
        ext = "txt",
        escape = "none"
    )]
    struct ModTemplate<'a> {
        group_name: &'a str,
        groups: &'a GroupsDef,
    }

    let tpl = ModTemplate { group_name, groups }.render()?;
    let tpl = tpl.trim_start();
    let content = re.replace(&content, tpl);

    let re = Regex::new(r"(?s)// Do not modify below this line. \(RepoStart\).+// Do not modify above this line. \(RepoEnd\)").unwrap();
    ensure!(
        re.is_match(&content),
        "File contents are invalid.: {:?}",
        &file_path
    );

    #[derive(Template)]
    #[template(
        source = r###"
    // Do not modify below this line. (RepoStart)
    @%- for (group, _) in groups %@
    get_repo!(@{ group|snake|ident }@, dyn _domain::@{ group|snake|ident }@::@{ group|pascal }@Repository, @{ group|snake|ident }@::@{ group|pascal }@RepositoryImpl);
    @%- endfor %@
    // Do not modify above this line. (RepoEnd)"###,
        ext = "txt",
        escape = "none"
    )]
    struct RepoTemplate<'a> {
        groups: &'a GroupsDef,
    }

    let tpl = RepoTemplate { groups }.render()?;
    let tpl = tpl.trim_start();
    let content = re.replace(&content, tpl);

    let re = Regex::new(r"(?s)// Do not modify below this line. \(QueryServiceStart\).+// Do not modify above this line. \(QueryServiceEnd\)").unwrap();
    ensure!(
        re.is_match(&content),
        "File contents are invalid.: {:?}",
        &file_path
    );

    #[derive(Template)]
    #[template(
        source = r###"
    // Do not modify below this line. (QueryServiceStart)
    @%- for (group, _) in groups %@
    get_repo!(@{ group|snake|ident }@, dyn _domain::@{ group|snake|ident }@::@{ group|pascal }@QueryService, @{ group|snake|ident }@::@{ group|pascal }@QueryServiceImpl);
    @%- endfor %@
    // Do not modify above this line. (QueryServiceEnd)"###,
        ext = "txt",
        escape = "none"
    )]
    struct QueryServiceTemplate<'a> {
        groups: &'a GroupsDef,
    }

    let tpl = QueryServiceTemplate { groups }.render()?;
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
    let file_path = impl_domain_dir.join(format!("{}.rs", group_name.to_snake()));
    remove_files.remove(file_path.as_os_str());
    let content = if force || !file_path.exists() {
        #[derive(Template)]
        #[template(path = "db/repositories/src/impl_domain/group.rs", escape = "none")]
        struct GroupTemplate<'a> {
            db: &'a str,
            base_group_name: &'a str,
            group_name: &'a str,
        }

        GroupTemplate {
            db,
            base_group_name,
            group_name,
        }
        .render()?
    } else {
        fs::read_to_string(&file_path)?.replace("\r\n", "\n")
    };

    let mod_names: BTreeSet<String> = entities_mod_names.iter().map(|v| v.0.clone()).collect();
    let re = Regex::new(r"(?s)// Do not modify below this line. \(ModStart\).+// Do not modify above this line. \(ModEnd\)").unwrap();
    ensure!(
        re.is_match(&content),
        "File contents are invalid.: {:?}",
        &file_path
    );

    #[derive(Template)]
    #[template(
        source = r###"
// Do not modify below this line. (ModStart)
@%- if SEPARATED_BASE_FILES %@
pub mod _base {
@%- for mod_name in mod_names %@
    pub mod _@{ mod_name }@;
@%- endfor %@
}
@%- else %@
pub mod _base;
@%- endif %@
@%- for mod_name in mod_names %@
pub mod @{ mod_name|ident }@;
@%- endfor %@
// Do not modify above this line. (ModEnd)"###,
        ext = "txt",
        escape = "none"
    )]
    struct ModTemplate<'a> {
        mod_names: &'a BTreeSet<String>,
    }

    let tpl = ModTemplate {
        mod_names: &mod_names,
    }
    .render()?;
    let tpl = tpl.trim_start();
    let content = re.replace(&content, tpl);

    let re = Regex::new(r"(?s)// Do not modify below this line. \(RepoStart\).+// Do not modify above this line. \(RepoEnd\)").unwrap();
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
    get_repo!(@{ mod_name|ident }@, dyn _domain::@{ mod_name|ident }@::@{ model_name|pascal }@Repository, @{ mod_name|ident }@::@{ model_name|pascal }@RepositoryImpl);
    @%- endfor %@
    // Do not modify above this line. (RepoEnd)"###,
        ext = "txt",
        escape = "none"
    )]
    struct RepoTemplate<'a> {
        mod_names: &'a BTreeSet<(String, &'a String)>,
    }

    let tpl = RepoTemplate {
        mod_names: entities_mod_names,
    }
    .render()?;
    let tpl = tpl.trim_start();
    let content = re.replace(&content, tpl);

    let re = Regex::new(r"(?s)// Do not modify below this line. \(QueryServiceStart\).+// Do not modify above this line. \(QueryServiceEnd\)").unwrap();
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
    get_repo!(@{ mod_name|ident }@, dyn _domain::@{ mod_name|ident }@::@{ model_name|pascal }@QueryService, @{ mod_name|ident }@::@{ model_name|pascal }@RepositoryImpl);
    @%- endfor %@
    // Do not modify above this line. (QueryServiceEnd)"###,
        ext = "txt",
        escape = "none"
    )]
    struct QueryServiceTemplate<'a> {
        mod_names: &'a BTreeSet<(String, &'a String)>,
    }

    let tpl = QueryServiceTemplate {
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
) -> Result<String, anyhow::Error> {
    set_domain_mode(true);
    let impl_domain_group_dir = impl_domain_dir.join(group_name.to_snake());
    let file_path = impl_domain_group_dir.join(format!("{}.rs", mod_name));
    remove_files.remove(file_path.as_os_str());
    let pascal_name = &model_name.to_pascal();
    if force || !file_path.exists() {
        #[derive(Template)]
        #[template(
            path = "db/repositories/src/impl_domain/entities/entity.rs",
            escape = "none"
        )]
        struct EntityTemplate<'a> {
            db: &'a str,
            base_group_name: &'a str,
            group_name: &'a str,
            mod_name: &'a str,
            pascal_name: &'a str,
        }

        let tpl = EntityTemplate {
            db,
            base_group_name,
            group_name,
            mod_name,
            pascal_name,
        };
        fs_write(file_path, tpl.render()?)?;
    }

    #[derive(Template)]
    #[template(
        path = "db/repositories/src/impl_domain/entities/base/_entity.rs",
        escape = "none"
    )]
    struct BaseEntityTemplate<'a> {
        db: &'a str,
        config: &'a ConfigDef,
        base_group_name: &'a str,
        group_name: &'a str,
        mod_name: &'a str,
        pascal_name: &'a str,
        def: &'a Arc<ModelDef>,
    }

    let tpl = BaseEntityTemplate {
        db,
        config,
        base_group_name,
        group_name,
        mod_name,
        pascal_name,
        def,
    };
    let ret = tpl.render()?;
    set_domain_mode(false);
    if SEPARATED_BASE_FILES {
        let path = impl_domain_group_dir.join("_base");
        let file_path = path.join(format!("_{}.rs", mod_name));
        remove_files.remove(file_path.as_os_str());
        fs_write(file_path, format!("{}{}", OVERWRITTEN_MSG, ret))?;
        Ok("".to_string())
    } else {
        Ok(format!("pub mod _{} {{\n{}}}\n", mod_name, ret))
    }
}
