use crate::common::ToCase as _;
use crate::common::{AtomicLoad as _, OVERWRITTEN_MSG};
use crate::schema::{ConfigDef, GroupsDef};
use crate::{SEPARATED_BASE_FILES, filters};
use crate::{
    common::fs_write,
    schema::{ModelDef, set_domain_mode, to_id_name},
};
use crate::schema::Joinable;
use anyhow::{Result, ensure};
use askama::Template;
use regex::Regex;
use std::{
    collections::{BTreeSet, HashSet},
    ffi::OsString,
    fs,
    path::Path,
    sync::Arc,
};

#[allow(clippy::regex_creation_in_loops)]
#[allow(clippy::too_many_arguments)]
pub fn write_group_files(
    domain_repositories_dir: &Path,
    db: &str,
    config: &ConfigDef,
    group_name: &str,
    groups: &GroupsDef,
    force: bool,
    remove_files: &mut HashSet<OsString>,
) -> Result<()> {
    let base_dir = domain_repositories_dir.join(group_name.to_snake());
    let file_path = base_dir.join("Cargo.toml");
    remove_files.remove(file_path.as_os_str());
    let content = if force || !file_path.exists() {
        #[derive(Template)]
        #[template(path = "domain/group_repositories/_Cargo.toml", escape = "none")]
        struct Template<'a> {
            db: &'a str,
            group_name: &'a str,
        }
        Template { db, group_name }.render()?
    } else {
        fs::read_to_string(&file_path)?.replace("\r\n", "\n")
    };
    fs_write(file_path, &*content)?;

    let src_dir = base_dir.join("src");
    let file_path = src_dir.join("lib.rs");
    remove_files.remove(file_path.as_os_str());
    if force || !file_path.exists() {
        #[derive(Template)]
        #[template(path = "domain/group_repositories/src/lib.rs", escape = "none")]
        struct Template;
        let content = Template.render()?;
        fs_write(file_path, &*content)?;
    }

    let file_path = src_dir.join("repositories.rs");
    remove_files.remove(file_path.as_os_str());
    let content = if force || !file_path.exists() {
        #[derive(Template)]
        #[template(
            path = "domain/group_repositories/src/repositories.rs",
            escape = "none"
        )]
        struct Template;
        Template.render()?
    } else {
        fs::read_to_string(&file_path)?.replace("\r\n", "\n")
    };
    {
        #[derive(Template)]
        #[template(
            source = r###"
// Do not modify below this line. (ModStart)
@%- for (name, defs) in groups %@
pub mod @{ name|snake|ident }@;
@%- endfor %@
// Do not modify above this line. (ModEnd)"###,
            ext = "txt",
            escape = "none"
        )]
        struct ModTemplate<'a> {
            pub db: &'a str,
            pub groups: &'a GroupsDef,
        }

        let re = Regex::new(r"(?s)// Do not modify below this line. \(ModStart\).+// Do not modify above this line. \(ModEnd\)").unwrap();
        ensure!(
            re.is_match(&content),
            "File contents are invalid.: {:?}",
            &file_path
        );
        let tpl = ModTemplate { db, groups }.render()?;
        let tpl = tpl.trim_start();
        let content = re.replace(&content, tpl);

        #[derive(Template)]
        #[template(
            source = r###"
    // Do not modify below this line. (RepoStart)
    @%- for (name, defs) in groups %@
    fn @{ name|snake|ident }@(&self) -> Box<dyn @{ name|snake|ident }@::@{ name|pascal }@Repository>;
    @%- endfor %@
    // Do not modify above this line. (RepoEnd)"###,
            ext = "txt",
            escape = "none"
        )]
        struct RepoTemplate<'a> {
            pub groups: &'a GroupsDef,
        }

        let re = Regex::new(r"(?s)// Do not modify below this line. \(RepoStart\).+// Do not modify above this line. \(RepoEnd\)").unwrap();
        ensure!(
            re.is_match(&content),
            "File contents are invalid.: {:?}",
            &file_path
        );
        let tpl = RepoTemplate { groups }.render()?;
        let tpl = tpl.trim_start();
        let content = re.replace(&content, tpl);

        #[derive(Template)]
        #[template(
            source = r###"
    // Do not modify below this line. (QueryServiceStart)
    @%- for (name, defs) in groups %@
    fn @{ name|snake|ident }@(&self) -> Box<dyn @{ name|snake|ident }@::@{ name|pascal }@QueryService>;
    @%- endfor %@
    // Do not modify above this line. (QueryServiceEnd)"###,
            ext = "txt",
            escape = "none"
        )]
        struct QueryServiceTemplate<'a> {
            pub groups: &'a GroupsDef,
        }

        let re = Regex::new(r"(?s)// Do not modify below this line. \(QueryServiceStart\).+// Do not modify above this line. \(QueryServiceEnd\)").unwrap();
        ensure!(
            re.is_match(&content),
            "File contents are invalid.: {:?}",
            &file_path
        );
        let tpl = QueryServiceTemplate { groups }.render()?;
        let tpl = tpl.trim_start();
        let content = re.replace(&content, tpl);

        #[derive(Template)]
        #[template(
            source = r###"
    // Do not modify below this line. (EmuRepoStart)
    @%- for (name, defs) in groups %@
    get_emu_repo!(@{ name|snake|ident }@, dyn @{ name|snake|ident }@::@{ name|pascal }@Repository, @{ name|snake|ident }@::Emu@{ name|pascal }@Repository);
    @%- endfor %@
    // Do not modify above this line. (EmuRepoEnd)"###,
            ext = "txt",
            escape = "none"
        )]
        struct EmuRepoTemplate<'a> {
            pub groups: &'a GroupsDef,
        }

        let re = Regex::new(r"(?s)// Do not modify below this line. \(EmuRepoStart\).+// Do not modify above this line. \(EmuRepoEnd\)").unwrap();
        ensure!(
            re.is_match(&content),
            "File contents are invalid.: {:?}",
            &file_path
        );
        let tpl = EmuRepoTemplate { groups }.render()?;
        let tpl = tpl.trim_start();
        let content = re.replace(&content, tpl);

        #[derive(Template)]
        #[template(
            source = r###"
    // Do not modify below this line. (EmuQueryServiceStart)
    @%- for (name, defs) in groups %@
    get_emu_repo!(@{ name|snake|ident }@, dyn @{ name|snake|ident }@::@{ name|pascal }@QueryService, @{ name|snake|ident }@::Emu@{ name|pascal }@QueryService);
    @%- endfor %@
    // Do not modify above this line. (EmuQueryServiceEnd)"###,
            ext = "txt",
            escape = "none"
        )]
        struct EmuQueryServiceTemplate<'a> {
            pub groups: &'a GroupsDef,
        }

        let re = Regex::new(r"(?s)// Do not modify below this line. \(EmuQueryServiceStart\).+// Do not modify above this line. \(EmuQueryServiceEnd\)").unwrap();
        ensure!(
            re.is_match(&content),
            "File contents are invalid.: {:?}",
            &file_path
        );
        let tpl = EmuQueryServiceTemplate { groups }.render()?;
        let tpl = tpl.trim_start();
        let content = re.replace(&content, tpl);
        fs_write(file_path, &*content)?;
    }

    let repositories_dir = src_dir.join("repositories");
    for (name, defs) in groups {
        let mod_names: BTreeSet<String> = defs
            .iter()
            .filter(|(_k, v)| !v.abstract_mode)
            .map(|(_, d)| d.mod_name())
            .collect();
        let mod_names = &mod_names;
        let entities_mod_names: BTreeSet<(String, &String)> = defs
            .iter()
            .filter(|(_, d)| !d.abstract_mode)
            .map(|(model_name, def)| (def.mod_name(), model_name))
            .collect();
        let entities_mod_names = &entities_mod_names;
        let file_path = repositories_dir.join(format!("{}.rs", name.to_snake()));
        remove_files.remove(file_path.as_os_str());
        let content = if force || !file_path.exists() {
            #[derive(Template)]
            #[template(
                path = "domain/group_repositories/src/repositories/group.rs",
                escape = "none"
            )]
            struct Template<'a> {
                group_name: &'a str,
            }
            Template { group_name: name }.render()?
        } else {
            fs::read_to_string(&file_path)?.replace("\r\n", "\n")
        };
        {
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
            pub struct DomainGroupModTemplate<'a> {
                pub mod_names: &'a BTreeSet<String>,
            }

            let tpl = DomainGroupModTemplate { mod_names }.render()?;
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
    fn @{ mod_name|ident }@(&self) -> Box<dyn @{ mod_name|ident }@::@{ model_name|pascal }@Repository>;
    @%- endfor %@
    // Do not modify above this line. (RepoEnd)"###,
                ext = "txt",
                escape = "none"
            )]
            pub struct DomainGroupRepoTemplate<'a> {
                pub mod_names: &'a BTreeSet<(String, &'a String)>,
            }

            let tpl = DomainGroupRepoTemplate {
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
    fn @{ mod_name|ident }@(&self) -> Box<dyn @{ mod_name|ident }@::@{ model_name|pascal }@QueryService>;
    @%- endfor %@
    // Do not modify above this line. (QueryServiceEnd)"###,
                ext = "txt",
                escape = "none"
            )]
            pub struct DomainGroupQueryServiceTemplate<'a> {
                pub mod_names: &'a BTreeSet<(String, &'a String)>,
            }

            let tpl = DomainGroupQueryServiceTemplate {
                mod_names: entities_mod_names,
            }
            .render()?;
            let tpl = tpl.trim_start();
            let content = re.replace(&content, tpl);

            let re = Regex::new(r"(?s)// Do not modify below this line. \(EmuRepoStart\).+// Do not modify above this line. \(EmuRepoEnd\)").unwrap();
            ensure!(
                re.is_match(&content),
                "File contents are invalid.: {:?}",
                &file_path
            );

            #[derive(Template)]
            #[template(
                source = r###"
    // Do not modify below this line. (EmuRepoStart)
    @%- for (mod_name, model_name) in mod_names %@
    get_emu_table!(@{ mod_name|ident }@, dyn @{ mod_name|ident }@::@{ model_name|pascal }@Repository, @{ mod_name|ident }@::Emu@{ model_name|pascal }@Repository);
    @%- endfor %@
    // Do not modify above this line. (EmuRepoEnd)"###,
                ext = "txt",
                escape = "none"
            )]
            pub struct DomainGroupEmuRepoTemplate<'a> {
                pub mod_names: &'a BTreeSet<(String, &'a String)>,
            }

            let tpl = DomainGroupEmuRepoTemplate {
                mod_names: entities_mod_names,
            }
            .render()?;
            let tpl = tpl.trim_start();
            let content = re.replace(&content, tpl);

            let re = Regex::new(r"(?s)// Do not modify below this line. \(EmuQueryServiceStart\).+// Do not modify above this line. \(EmuQueryServiceEnd\)").unwrap();
            ensure!(
                re.is_match(&content),
                "File contents are invalid.: {:?}",
                &file_path
            );

            #[derive(Template)]
            #[template(
                source = r###"
    // Do not modify below this line. (EmuQueryServiceStart)
    @%- for (mod_name, model_name) in mod_names %@
    get_emu_table!(@{ mod_name|ident }@, dyn @{ mod_name|ident }@::@{ model_name|pascal }@QueryService, @{ mod_name|ident }@::Emu@{ model_name|pascal }@Repository);
    @%- endfor %@
    // Do not modify above this line. (EmuQueryServiceEnd)"###,
                ext = "txt",
                escape = "none"
            )]
            pub struct DomainGroupEmuQueryServiceTemplate<'a> {
                pub mod_names: &'a BTreeSet<(String, &'a String)>,
            }

            let tpl = DomainGroupEmuQueryServiceTemplate {
                mod_names: entities_mod_names,
            }
            .render()?;
            let tpl = tpl.trim_start();
            let content = re.replace(&content, tpl);

            fs_write(file_path, &*content)?;
        }
        let mut output = String::new();
        output.push_str(OVERWRITTEN_MSG);
        for (model_name, def) in defs {
            let group_name = name;
            let mod_name = def.mod_name();
            let mod_name = &mod_name;
            if !def.abstract_mode {
                output.push_str(&write_entity(
                    &repositories_dir,
                    db,
                    config,
                    group_name,
                    mod_name,
                    force,
                    model_name,
                    def,
                    remove_files,
                )?);
            }
        }
        if !SEPARATED_BASE_FILES {
            let group_dir = repositories_dir.join(name.to_snake());
            let file_path = group_dir.join("_base.rs");
            remove_files.remove(file_path.as_os_str());
            fs_write(file_path, output)?;
        }
    }
    Ok(())
}

pub fn write_lib_rs(
    domain_repositories_src_dir: &Path,
    db: &str,
    groups: &GroupsDef,
    force: bool,
) -> Result<()> {
    let file_path = domain_repositories_src_dir.join("lib.rs");
    let content = if force || !file_path.exists() {
        crate::db_generator::DomainDbLibTemplate { db }.render()?
    } else {
        fs::read_to_string(&file_path)?.replace("\r\n", "\n")
    };

    #[derive(Template)]
    #[template(
        source = r###"
// Do not modify below this line. (ModStart)
@%- for (name, defs) in groups %@
pub use repository_@{ db|snake }@_@{ name|snake }@::repositories::@{ name|snake|ident }@ as @{ name|snake|ident }@;
@%- endfor %@
// Do not modify above this line. (ModEnd)"###,
        ext = "txt",
        escape = "none"
    )]
    pub struct ModTemplate<'a> {
        pub db: &'a str,
        pub groups: &'a GroupsDef,
    }

    let re = Regex::new(r"(?s)// Do not modify below this line. \(ModStart\).+// Do not modify above this line. \(ModEnd\)").unwrap();
    ensure!(
        re.is_match(&content),
        "File contents are invalid.: {:?}",
        &file_path
    );
    let tpl = ModTemplate { db, groups }.render()?;
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
    @%- for (name, defs) in groups %@
    fn @{ name|snake|ident }@(&self) -> Box<dyn @{ name|snake|ident }@::@{ name|pascal }@Repository>;
    @%- endfor %@
    // Do not modify above this line. (RepoEnd)"###,
        ext = "txt",
        escape = "none"
    )]
    pub struct DomainDbRepoTemplate<'a> {
        pub groups: &'a GroupsDef,
    }

    let tpl = DomainDbRepoTemplate { groups }.render()?;
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
    @%- for (name, defs) in groups %@
    fn @{ name|snake|ident }@(&self) -> Box<dyn @{ name|snake|ident }@::@{ name|pascal }@QueryService>;
    @%- endfor %@
    // Do not modify above this line. (QueryServiceEnd)"###,
        ext = "txt",
        escape = "none"
    )]
    pub struct QueryServiceDbQueryServiceTemplate<'a> {
        pub groups: &'a GroupsDef,
    }

    let tpl = QueryServiceDbQueryServiceTemplate { groups }.render()?;
    let tpl = tpl.trim_start();
    let content = re.replace(&content, tpl);

    let re = Regex::new(r"(?s)// Do not modify below this line. \(EmuRepoStart\).+// Do not modify above this line. \(EmuRepoEnd\)").unwrap();
    ensure!(
        re.is_match(&content),
        "File contents are invalid.: {:?}",
        &file_path
    );

    #[derive(Template)]
    #[template(
        source = r###"
    // Do not modify below this line. (EmuRepoStart)
    @%- for (name, defs) in groups %@
    get_emu_group!(@{ name|snake|ident }@, dyn @{ name|snake|ident }@::@{ name|pascal }@Repository, @{ name|snake|ident }@::Emu@{ name|pascal }@Repository);
    @%- endfor %@
    // Do not modify above this line. (EmuRepoEnd)"###,
        ext = "txt",
        escape = "none"
    )]
    pub struct DomainDbEmuRepoTemplate<'a> {
        pub groups: &'a GroupsDef,
    }

    let tpl = DomainDbEmuRepoTemplate { groups }.render()?;
    let tpl = tpl.trim_start();
    let content = re.replace(&content, tpl);

    let re = Regex::new(r"(?s)// Do not modify below this line. \(EmuQueryServiceStart\).+// Do not modify above this line. \(EmuQueryServiceEnd\)").unwrap();
    ensure!(
        re.is_match(&content),
        "File contents are invalid.: {:?}",
        &file_path
    );

    #[derive(Template)]
    #[template(
        source = r###"
    // Do not modify below this line. (EmuQueryServiceStart)
    @%- for (name, defs) in groups %@
    get_emu_group!(@{ name|snake|ident }@, dyn @{ name|snake|ident }@::@{ name|pascal }@QueryService, @{ name|snake|ident }@::Emu@{ name|pascal }@QueryService);
    @%- endfor %@
    // Do not modify above this line. (EmuQueryServiceEnd)"###,
        ext = "txt",
        escape = "none"
    )]
    pub struct DomainDbEmuQueryServiceTemplate<'a> {
        pub groups: &'a GroupsDef,
    }

    let tpl = DomainDbEmuQueryServiceTemplate { groups }.render()?;
    let tpl = tpl.trim_start();
    let content = re.replace(&content, tpl);

    fs_write(file_path, &*content)?;
    Ok(())
}

pub fn write_cargo_toml(
    domain_repositories_dir: &Path,
    db: &str,
    groups: &GroupsDef,
    force: bool,
) -> Result<()> {
    let file_path = domain_repositories_dir.join("Cargo.toml");
    let mut content = if force || !file_path.exists() {
        crate::db_generator::DomainCargoTemplate { db }.render()?
    } else {
        fs::read_to_string(&file_path)?.replace("\r\n", "\n")
    };
    let reg = Regex::new(r"(?m)^repository_\w+\s*=.+\n")?;
    content = reg.replace_all(&content, "").into_owned();
    let reg = Regex::new(r#"[ \t]*"repository_\w+/mock"[ \t]*,?[ \t]*\n?"#)?;
    content = reg.replace_all(&content, "").into_owned();
    for (group, _) in groups.iter().rev() {
        let db = &db.to_snake();
        let group = &group.to_snake();
        content = content.replace(
            "\"mockall\"",
            &format!("\"mockall\",\n    \"repository_{}_{}/mock\"", db, group),
        );
        content = content.replace(
            "[dependencies]",
            &format!(
                "[dependencies]\nrepository_{}_{} = {{ path = \"groups/{}\" }}",
                db, group, group
            ),
        );
        content = content.replace(
            "[dev-dependencies]",
            &format!(
                "[dev-dependencies]\nrepository_{}_{} = {{ path = \"groups/{}\", features = [\"mock\"] }}",
                db, group, group
            ),
        );
    }
    fs_write(file_path, &*content)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn write_entity(
    repositories_dir: &Path,
    db: &str,
    config: &ConfigDef,
    group_name: &str,
    mod_name: &str,
    force: bool,
    model_name: &str,
    def: &Arc<ModelDef>,
    remove_files: &mut HashSet<OsString>,
) -> Result<String, anyhow::Error> {
    set_domain_mode(true);
    let domain_group_dir = repositories_dir.join(group_name.to_snake());
    let file_path = domain_group_dir.join(format!("{}.rs", mod_name));
    remove_files.remove(file_path.as_os_str());
    let pascal_name = &model_name.to_pascal();
    let id_name = &to_id_name(model_name);

    #[derive(Template)]
    #[template(
        path = "domain/group_repositories/src/repositories/entities/entity.rs",
        escape = "none"
    )]
    pub struct DomainEntityTemplate<'a> {
        pub group_name: &'a str,
        pub mod_name: &'a str,
        pub pascal_name: &'a str,
        pub def: &'a Arc<ModelDef>,
    }

    let content = if force || !file_path.exists() {
        DomainEntityTemplate {
            group_name,
            mod_name,
            pascal_name,
            def,
        }
        .render()?
    } else {
        fs::read_to_string(&file_path)?.replace("\r\n", "\n")
    };

    #[derive(Template)]
    #[template(
        source = r###"
// Do not modify below this line. (ModStart)
use async_trait::async_trait;
#[allow(unused_imports)]
use base_domain as domain;
#[allow(unused_imports)]
use base_domain::models::@{ db|snake|ident }@ as _model_;
@%- for (name, rel_def) in def.belongs_to_outer_db(Joinable::Join) %@
#[allow(unused_imports)]
pub use base_domain::models::@{ rel_def.db()|snake|ident }@ as _@{ rel_def.db()|snake }@_model_;
@%- endfor %@
@%- for (enum_name, column_def) in def.num_enums(true) %@
#[rustfmt::skip]
pub use base_domain::models::@{ db|snake|ident }@::@{ group_name|snake|ident }@::@{ mod_name|ident }@::@{ enum_name|pascal }@;
@%- endfor %@
@%- for (enum_name, column_def) in def.str_enums(true) %@
#[rustfmt::skip]
pub use base_domain::models::@{ db|snake|ident }@::@{ group_name|snake|ident }@::@{ mod_name|ident }@::@{ enum_name|pascal }@;
@%- endfor %@
#[rustfmt::skip]
pub use super::_base::_@{ mod_name }@::{join, Joiner_};
#[rustfmt::skip]
pub use super::_base::_@{ mod_name }@::{filter, order, Filter_};
pub use super::_base::_@{ mod_name }@::@{ pascal_name }@Factory;
use super::_base::_@{ mod_name }@::{_@{ pascal_name }@QueryService, _@{ pascal_name }@Repository};
pub use base_domain::models::@{ db|snake|ident }@::@{ group_name|snake|ident }@::@{ mod_name|ident }@::consts;
#[rustfmt::skip]
pub use base_domain::models::@{ db|snake|ident }@::@{ group_name|snake|ident }@::@{ mod_name|ident }@::{
    @{ pascal_name }@, @{ pascal_name }@Updater,
};
@%- for id in def.id() %@
#[rustfmt::skip]
pub use base_domain::models::@{ db|snake|ident }@::@{ group_name|snake|ident }@::@{ mod_name|ident }@::@{ id_name }@;
@%- endfor %@
#[rustfmt::skip]
pub use base_domain::models::@{ db|snake|ident }@::@{ group_name|snake|ident }@::@{ mod_name|ident }@::@{ pascal_name }@Primary;
#[rustfmt::skip]
pub use super::_base::_@{ mod_name }@::{_@{ pascal_name }@QueryFindBuilder, _@{ pascal_name }@RepositoryFindBuilder};
#[cfg(any(feature = "mock", test))]
pub use base_domain::models::@{ db|snake|ident }@::@{ group_name|snake|ident }@::@{ mod_name|ident }@::@{ pascal_name }@Entity;
@%- for (selector, selector_def) in def.selectors %@
#[rustfmt::skip]
pub use super::_base::_@{ mod_name }@::@{ pascal_name }@Repository@{ selector|pascal }@Builder;
@%- endfor %@
@%- for (selector, selector_def) in def.selectors %@
#[rustfmt::skip]
pub use super::_base::_@{ mod_name }@::{@{ pascal_name }@Query@{ selector|pascal }@Builder, @{ pascal_name }@Query@{ selector|pascal }@Cursor, @{ pascal_name }@Query@{ selector|pascal }@Filter, @{ pascal_name }@Query@{ selector|pascal }@Order};
@%- endfor %@
#[cfg(any(feature = "mock", test))]
pub use self::{MockQueryService_ as Mock@{ pascal_name }@QueryService, MockRepository_ as Mock@{ pascal_name }@Repository};
#[cfg(any(feature = "mock", test))]
pub use super::_base::_@{ mod_name }@::Emu@{ pascal_name }@Repository;
@{- def.relations(Joinable::Join)|fmt_rel_join("
// pub use base_domain::models::--1--::{class_mod_path} as _{raw_rel_name}_model_;", "")|replace1(db|snake|ident) }@
@{- def.relations_belonging_outer_db(Joinable::Join, false)|fmt_rel_outer_db_join("
// pub use base_domain::models::{db_mod_ident}::{class_mod_path} as _{raw_rel_name}_model_;", "") }@
// Do not modify above this line. (ModEnd)"###,
        ext = "txt",
        escape = "none"
    )]
    pub struct ModTemplate<'a> {
        pub db: &'a str,
        pub group_name: &'a str,
        pub mod_name: &'a str,
        pub pascal_name: &'a str,
        pub id_name: &'a str,
        pub def: &'a Arc<ModelDef>,
    }

    let re = Regex::new(r"(?s)// Do not modify below this line. \(ModStart\).+// Do not modify above this line. \(ModEnd\)").unwrap();
    ensure!(
        re.is_match(&content),
        "File contents are invalid.: {:?}",
        &file_path
    );
    let tpl = ModTemplate {
        db,
        group_name,
        mod_name,
        pascal_name,
        id_name,
        def,
    }
    .render()?;
    let tpl = tpl.trim_start();
    let content = re.replace(&content, tpl);

    #[derive(Template)]
    #[template(
        source = r###"
    // Do not modify below this line. (RepositoryMockStart)
    #[async_trait]
    impl _@{ pascal_name }@Repository for Repository_ {
        @%- if !def.disable_update() %@
        fn find(&self, id: @{ def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") }@) -> Box<dyn _@{ pascal_name }@RepositoryFindBuilder>;
        @%- endif %@
        fn convert_factory(&self, factory: @{ pascal_name }@Factory) -> Box<dyn @{ pascal_name }@Updater>;
        async fn save(&self, obj: Box<dyn @{ pascal_name }@Updater>) -> anyhow::Result<Option<Box<dyn @{ pascal_name }@>>>;
        @%- if !def.disable_update() %@
        async fn import(&self, list: Vec<Box<dyn @{ pascal_name }@Updater>>, option: Option<domain::models::ImportOption>) -> anyhow::Result<()>;
        @%- endif %@
        @%- if def.use_insert_delayed() %@
        async fn insert_delayed(&self, obj: Box<dyn @{ pascal_name }@Updater>) -> anyhow::Result<()>;
        @%- endif %@
        @%- if !def.disable_delete() %@
        async fn delete(&self, obj: Box<dyn @{ pascal_name }@Updater>) -> anyhow::Result<()>;
        @%- if def.primaries().len() == 1 %@
        async fn delete_by_ids(&self, ids: &[@{ def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") }@]) -> anyhow::Result<u64>;
        @%- endif %@
        async fn delete_all(&self) -> anyhow::Result<()>;
        @%- endif %@
        @%- if def.act_as_job_queue() %@
        async fn fetch(&self, limit: usize) -> anyhow::Result<Vec<Box<dyn @{ pascal_name }@Updater>>>;
        @%- endif %@
        @%- for (selector, selector_def) in def.selectors %@
        fn @{ selector|ident }@(&self) -> Box<dyn @{ pascal_name }@Repository@{ selector|pascal }@Builder>;
        @%- endfor %@
    }
    // Do not modify above this line. (RepositoryMockEnd)"###,
        ext = "txt",
        escape = "none"
    )]
    pub struct RepositoryMockTemplate<'a> {
        pub pascal_name: &'a str,
        pub def: &'a Arc<ModelDef>,
    }

    let re = Regex::new(r"(?s)// Do not modify below this line. \(RepositoryMockStart\).+// Do not modify above this line. \(RepositoryMockEnd\)").unwrap();
    ensure!(
        re.is_match(&content),
        "File contents are invalid.: {:?}",
        &file_path
    );
    let tpl = RepositoryMockTemplate { pascal_name, def }.render()?;
    let tpl = tpl.trim_start();
    let content = re.replace(&content, tpl);

    #[derive(Template)]
    #[template(
        source = r###"
    // Do not modify below this line. (QueryServiceMockStart)
    #[async_trait]
    impl _@{ pascal_name }@QueryService for QueryService_ {
        @%- if def.use_all_rows_cache() && !def.use_filtered_row_cache() %@
        async fn all(&self) -> anyhow::Result<Box<dyn base_domain::models::EntityIterator<dyn @{ pascal_name }@Cache>>>;
        @%- endif %@
        @%- for (selector, selector_def) in def.selectors %@
        fn @{ selector|ident }@(&self) -> Box<dyn @{ pascal_name }@Query@{ selector|pascal }@Builder>;
        @%- endfor %@
        fn find(&self, id: @{ def.primaries()|fmt_join_with_paren("{domain_outer_owned}", ", ") }@) -> Box<dyn _@{ pascal_name }@QueryFindBuilder>;
    }
    // Do not modify above this line. (QueryServiceMockEnd)"###,
        ext = "txt",
        escape = "none"
    )]
    pub struct QueryServiceMockTemplate<'a> {
        pub pascal_name: &'a str,
        pub def: &'a Arc<ModelDef>,
    }

    let re = Regex::new(r"(?s)// Do not modify below this line. \(QueryServiceMockStart\).+// Do not modify above this line. \(QueryServiceMockEnd\)").unwrap();
    ensure!(
        re.is_match(&content),
        "File contents are invalid.: {:?}",
        &file_path
    );
    let tpl = QueryServiceMockTemplate { pascal_name, def }.render()?;
    let tpl = tpl.trim_start();
    let content = re.replace(&content, tpl);

    fs_write(file_path, &*content)?;

    #[derive(Template)]
    #[template(
        path = "domain/group_repositories/src/repositories/entities/base/_entity.rs",
        escape = "none"
    )]
    pub struct DomainBaseEntityTemplate<'a> {
        pub db: &'a str,
        pub config: &'a ConfigDef,
        pub group_name: &'a str,
        pub mod_name: &'a str,
        pub model_name: &'a str,
        pub pascal_name: &'a str,
        pub def: &'a Arc<ModelDef>,
    }

    let tpl = DomainBaseEntityTemplate {
        db,
        config,
        group_name,
        mod_name,
        model_name,
        pascal_name,
        def,
    };
    let ret = tpl.render()?;
    set_domain_mode(false);
    if SEPARATED_BASE_FILES {
        let domain_group_base_dir = domain_group_dir.join("_base");
        let file_path = domain_group_base_dir.join(format!("_{}.rs", mod_name));
        remove_files.remove(file_path.as_os_str());
        fs_write(file_path, format!("{}{}", OVERWRITTEN_MSG, ret))?;
        Ok("".to_string())
    } else {
        Ok(format!("pub mod _{} {{\n{}}}\n", mod_name, ret))
    }
}
