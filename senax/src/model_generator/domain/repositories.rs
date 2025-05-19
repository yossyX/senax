use crate::common::{AtomicLoad as _, OVERWRITTEN_MSG};
use crate::model_generator::REL_START;
use crate::schema::{GroupsDef, IS_MAIN_GROUP};
use crate::{SEPARATED_BASE_FILES, filters};
use crate::{
    common::fs_write,
    schema::{ModelDef, set_domain_mode, to_id_name},
};
use anyhow::{Result, ensure};
use askama::Template;
use convert_case::{Case, Casing as _};
use regex::Regex;
use std::{
    collections::{BTreeSet, HashSet},
    ffi::OsString,
    fs,
    path::Path,
    sync::Arc,
};

pub fn write_group_files(
    domain_repositories_dir: &Path,
    db: &str,
    group_name: &str,
    groups: &GroupsDef,
    ref_groups: &[String],
    force: bool,
    remove_files: &mut HashSet<OsString>,
) -> Result<()> {
    let base_dir = domain_repositories_dir.join(group_name.to_case(Case::Snake));
    let file_path = base_dir.join("Cargo.toml");
    remove_files.remove(file_path.as_os_str());
    let mut content = if force || !file_path.exists() {
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
    let reg = Regex::new(r"(?m)^repository_\w+\s*=.+\n")?;
    content = reg.replace_all(&content, "").into_owned();
    for group in ref_groups {
        let db = &db.to_case(Case::Snake);
        let group = &group.to_case(Case::Snake);
        content = content.replace(
            "[dependencies]",
            &format!(
                "[dependencies]\nrepository_{}_{} = {{ path = \"../{}\" }}",
                db, group, group
            ),
        );
    }
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
@%- for (name, (_, defs)) in groups %@
pub mod @{ name|snake|to_var_name }@;
@%- endfor %@
@%- for name in ref_groups %@
pub use repository_@{ db|snake }@_@{ name|snake }@::repositories::@{ name|snake|to_var_name }@;
@%- endfor %@
// Do not modify up to this line. (ModEnd)"###,
            ext = "txt",
            escape = "none"
        )]
        struct ModTemplate<'a> {
            pub db: &'a str,
            pub groups: &'a GroupsDef,
            pub ref_groups: &'a [String],
        }

        let re = Regex::new(r"(?s)// Do not modify below this line. \(ModStart\).+// Do not modify up to this line. \(ModEnd\)").unwrap();
        ensure!(
            re.is_match(&content),
            "File contents are invalid.: {:?}",
            &file_path
        );
        let tpl = ModTemplate {
            db,
            groups,
            ref_groups,
        }
        .render()?;
        let tpl = tpl.trim_start();
        let content = re.replace(&content, tpl);

        #[derive(Template)]
        #[template(
            source = r###"
    // Do not modify below this line. (RepoStart)
    @%- for (name, (_, defs)) in groups %@
    fn @{ name|snake|to_var_name }@(&self) -> Box<dyn @{ name|snake|to_var_name }@::@{ name|pascal }@Repository>;
    @%- endfor %@
    // Do not modify up to this line. (RepoEnd)"###,
            ext = "txt",
            escape = "none"
        )]
        struct RepoTemplate<'a> {
            pub groups: &'a GroupsDef,
        }

        let re = Regex::new(r"(?s)// Do not modify below this line. \(RepoStart\).+// Do not modify up to this line. \(RepoEnd\)").unwrap();
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
    @%- for (name, (_, defs)) in groups %@
    fn @{ name|snake|to_var_name }@(&self) -> Box<dyn @{ name|snake|to_var_name }@::@{ name|pascal }@QueryService>;
    @%- endfor %@
    // Do not modify up to this line. (QueryServiceEnd)"###,
            ext = "txt",
            escape = "none"
        )]
        struct QueryServiceTemplate<'a> {
            pub groups: &'a GroupsDef,
        }

        let re = Regex::new(r"(?s)// Do not modify below this line. \(QueryServiceStart\).+// Do not modify up to this line. \(QueryServiceEnd\)").unwrap();
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
    @%- for (name, (_, defs)) in groups %@
    get_emu_repo!(@{ name|snake|to_var_name }@, dyn @{ name|snake|to_var_name }@::@{ name|pascal }@Repository, @{ name|snake|to_var_name }@::Emu@{ name|pascal }@Repository);
    @%- endfor %@
    // Do not modify up to this line. (EmuRepoEnd)"###,
            ext = "txt",
            escape = "none"
        )]
        struct EmuRepoTemplate<'a> {
            pub groups: &'a GroupsDef,
        }

        let re = Regex::new(r"(?s)// Do not modify below this line. \(EmuRepoStart\).+// Do not modify up to this line. \(EmuRepoEnd\)").unwrap();
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
    @%- for (name, (_, defs)) in groups %@
    get_emu_repo!(@{ name|snake|to_var_name }@, dyn @{ name|snake|to_var_name }@::@{ name|pascal }@QueryService, @{ name|snake|to_var_name }@::Emu@{ name|pascal }@QueryService);
    @%- endfor %@
    // Do not modify up to this line. (EmuQueryServiceEnd)"###,
            ext = "txt",
            escape = "none"
        )]
        struct EmuQueryServiceTemplate<'a> {
            pub groups: &'a GroupsDef,
        }

        let re = Regex::new(r"(?s)// Do not modify below this line. \(EmuQueryServiceStart\).+// Do not modify up to this line. \(EmuQueryServiceEnd\)").unwrap();
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
    for (name, (f, defs)) in groups {
        let is_main_group = f.relaxed_load() == REL_START;
        IS_MAIN_GROUP.relaxed_store(is_main_group);
        let mod_names: BTreeSet<String> = defs
            .iter()
            .filter(|(_k, (_, v))| !v.abstract_mode)
            .map(|(_, (_, d))| d.mod_name())
            .collect();
        let mod_names = &mod_names;
        let entities_mod_names: BTreeSet<(String, &String)> = defs
            .iter()
            .filter(|(_, (_, d))| !d.abstract_mode)
            .map(|(model_name, (_, def))| (def.mod_name(), model_name))
            .collect();
        let entities_mod_names = &entities_mod_names;
        let file_path = repositories_dir.join(&format!("{}.rs", name.to_case(Case::Snake)));
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
            Template { group_name: &name }.render()?
        } else {
            fs::read_to_string(&file_path)?.replace("\r\n", "\n")
        };
        {
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
pub mod @{ mod_name|to_var_name }@;
@%- endfor %@
// Do not modify up to this line. (ModEnd)"###,
                ext = "txt",
                escape = "none"
            )]
            pub struct DomainGroupModTemplate<'a> {
                pub mod_names: &'a BTreeSet<String>,
            }

            let tpl = DomainGroupModTemplate { mod_names }.render()?;
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
    fn @{ mod_name|to_var_name }@(&self) -> Box<dyn @{ mod_name|to_var_name }@::@{ model_name|pascal }@Repository>;
    @%- endfor %@
    // Do not modify up to this line. (RepoEnd)"###,
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
    fn @{ mod_name|to_var_name }@(&self) -> Box<dyn @{ mod_name|to_var_name }@::@{ model_name|pascal }@QueryService>;
    @%- endfor %@
    // Do not modify up to this line. (QueryServiceEnd)"###,
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

            let re = Regex::new(r"(?s)// Do not modify below this line. \(EmuRepoStart\).+// Do not modify up to this line. \(EmuRepoEnd\)").unwrap();
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
    get_emu_table!(@{ mod_name|to_var_name }@, dyn @{ mod_name|to_var_name }@::@{ model_name|pascal }@Repository, @{ mod_name|to_var_name }@::Emu@{ model_name|pascal }@Repository);
    @%- endfor %@
    // Do not modify up to this line. (EmuRepoEnd)"###,
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

            let re = Regex::new(r"(?s)// Do not modify below this line. \(EmuQueryServiceStart\).+// Do not modify up to this line. \(EmuQueryServiceEnd\)").unwrap();
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
    get_emu_table!(@{ mod_name|to_var_name }@, dyn @{ mod_name|to_var_name }@::@{ model_name|pascal }@QueryService, @{ mod_name|to_var_name }@::Emu@{ model_name|pascal }@Repository);
    @%- endfor %@
    // Do not modify up to this line. (EmuQueryServiceEnd)"###,
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
        for (model_name, (_, def)) in defs {
            let group_name = name;
            let mod_name = def.mod_name();
            let mod_name = &mod_name;
            if !def.abstract_mode {
                output.push_str(&write_entity(
                    &repositories_dir,
                    db,
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
            let group_dir = repositories_dir.join(name.to_case(Case::Snake));
            let file_path = group_dir.join("_base.rs");
            remove_files.remove(file_path.as_os_str());
            fs_write(file_path, output)?;
        }
    }
    IS_MAIN_GROUP.relaxed_store(true);
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
@%- for (name, (_, defs)) in groups %@
pub use repository_@{ db|snake }@_@{ name|snake }@::repositories::@{ name|snake|to_var_name }@ as @{ name|snake|to_var_name }@;
@%- endfor %@
// Do not modify up to this line. (ModEnd)"###,
        ext = "txt",
        escape = "none"
    )]
    pub struct ModTemplate<'a> {
        pub db: &'a str,
        pub groups: &'a GroupsDef,
    }

    let re = Regex::new(r"(?s)// Do not modify below this line. \(ModStart\).+// Do not modify up to this line. \(ModEnd\)").unwrap();
    ensure!(
        re.is_match(&content),
        "File contents are invalid.: {:?}",
        &file_path
    );
    let tpl = ModTemplate { db, groups }.render()?;
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
    fn @{ name|snake|to_var_name }@(&self) -> Box<dyn @{ name|snake|to_var_name }@::@{ name|pascal }@Repository>;
    @%- endfor %@
    // Do not modify up to this line. (RepoEnd)"###,
        ext = "txt",
        escape = "none"
    )]
    pub struct DomainDbRepoTemplate<'a> {
        pub groups: &'a GroupsDef,
    }

    let tpl = DomainDbRepoTemplate { groups }.render()?;
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
    fn @{ name|snake|to_var_name }@(&self) -> Box<dyn @{ name|snake|to_var_name }@::@{ name|pascal }@QueryService>;
    @%- endfor %@
    // Do not modify up to this line. (QueryServiceEnd)"###,
        ext = "txt",
        escape = "none"
    )]
    pub struct QueryServiceDbQueryServiceTemplate<'a> {
        pub groups: &'a GroupsDef,
    }

    let tpl = QueryServiceDbQueryServiceTemplate { groups }.render()?;
    let tpl = tpl.trim_start();
    let content = re.replace(&content, tpl);

    let re = Regex::new(r"(?s)// Do not modify below this line. \(EmuRepoStart\).+// Do not modify up to this line. \(EmuRepoEnd\)").unwrap();
    ensure!(
        re.is_match(&content),
        "File contents are invalid.: {:?}",
        &file_path
    );

    #[derive(Template)]
    #[template(
        source = r###"
    // Do not modify below this line. (EmuRepoStart)
    @%- for (name, (_, defs)) in groups %@
    get_emu_group!(@{ name|snake|to_var_name }@, dyn @{ name|snake|to_var_name }@::@{ name|pascal }@Repository, @{ name|snake|to_var_name }@::Emu@{ name|pascal }@Repository);
    @%- endfor %@
    // Do not modify up to this line. (EmuRepoEnd)"###,
        ext = "txt",
        escape = "none"
    )]
    pub struct DomainDbEmuRepoTemplate<'a> {
        pub groups: &'a GroupsDef,
    }

    let tpl = DomainDbEmuRepoTemplate { groups }.render()?;
    let tpl = tpl.trim_start();
    let content = re.replace(&content, tpl);

    let re = Regex::new(r"(?s)// Do not modify below this line. \(EmuQueryServiceStart\).+// Do not modify up to this line. \(EmuQueryServiceEnd\)").unwrap();
    ensure!(
        re.is_match(&content),
        "File contents are invalid.: {:?}",
        &file_path
    );

    #[derive(Template)]
    #[template(
        source = r###"
    // Do not modify below this line. (EmuQueryServiceStart)
    @%- for (name, (_, defs)) in groups %@
    get_emu_group!(@{ name|snake|to_var_name }@, dyn @{ name|snake|to_var_name }@::@{ name|pascal }@QueryService, @{ name|snake|to_var_name }@::Emu@{ name|pascal }@QueryService);
    @%- endfor %@
    // Do not modify up to this line. (EmuQueryServiceEnd)"###,
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
    for (group, (_, _)) in groups.iter().rev() {
        let db = &db.to_case(Case::Snake);
        let group = &group.to_case(Case::Snake);
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
pub fn write_entity(
    repositories_dir: &Path,
    db: &str,
    group_name: &str,
    mod_name: &str,
    force: bool,
    model_name: &str,
    def: &Arc<ModelDef>,
    remove_files: &mut HashSet<OsString>,
) -> Result<String, anyhow::Error> {
    set_domain_mode(true);
    let domain_group_dir = repositories_dir.join(group_name.to_case(Case::Snake));
    let file_path = domain_group_dir.join(format!("{}.rs", mod_name));
    remove_files.remove(file_path.as_os_str());
    let pascal_name = &model_name.to_case(Case::Pascal);
    let id_name = &to_id_name(model_name);

    #[derive(Template)]
    #[template(
        path = "domain/group_repositories/src/repositories/entities/entity.rs",
        escape = "none"
    )]
    pub struct DomainEntityTemplate<'a> {
        pub db: &'a str,
        pub group_name: &'a str,
        pub mod_name: &'a str,
        pub pascal_name: &'a str,
        pub id_name: &'a str,
        pub def: &'a Arc<ModelDef>,
    }

    if force || !file_path.exists() {
        let tpl = DomainEntityTemplate {
            db,
            group_name,
            mod_name,
            pascal_name,
            id_name,
            def,
        };
        fs_write(file_path, tpl.render()?)?;
    }

    #[derive(Template)]
    #[template(
        path = "domain/group_repositories/src/repositories/entities/base/_entity.rs",
        escape = "none"
    )]
    pub struct DomainBaseEntityTemplate<'a> {
        pub db: &'a str,
        pub group_name: &'a str,
        pub mod_name: &'a str,
        pub model_name: &'a str,
        pub pascal_name: &'a str,
        pub def: &'a Arc<ModelDef>,
    }

    let tpl = DomainBaseEntityTemplate {
        db,
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
        fs_write(file_path, &format!("{}{}", OVERWRITTEN_MSG, ret))?;
        Ok("".to_string())
    } else {
        Ok(format!("pub mod _{} {{\n{}}}\n", mod_name, ret))
    }
}
