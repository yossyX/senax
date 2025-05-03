use crate::filters;
use crate::schema::GroupsDef;
use crate::{
    common::fs_write,
    schema::{ModelDef, set_domain_mode, to_id_name},
};
use anyhow::{Context, Result, ensure};
use askama::Template;
use convert_case::{Case, Casing as _};
use indexmap::IndexMap;
use regex::Regex;
use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
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
        fs::read_to_string(&file_path)?
    };
    for group in ref_groups {
        let reg = Regex::new(&format!(r"(?m)^repository_{}_{}\s*=", db, group))?;
        if !reg.is_match(&content) {
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
        struct Template<'a> {
            group_name: &'a str,
        }
        Template { group_name }.render()?
    } else {
        fs::read_to_string(&file_path)?
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
    let base_group_name = group_name;
    for (name, (_, defs)) in groups {
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
            };
            Template { group_name: &name }.render()?
        } else {
            fs::read_to_string(&file_path)?
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
        for (model_name, (_, def)) in defs {
            let group_name = name;
            let mod_name = def.mod_name();
            let mod_name = &mod_name;
            if !def.abstract_mode {
                write_entity(
                    &repositories_dir,
                    db,
                    base_group_name,
                    group_name,
                    mod_name,
                    force,
                    model_name,
                    def,
                    remove_files,
                )?;
            }
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
        fs::read_to_string(&file_path)?
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
        #[derive(Template)]
        #[template(path = "domain/db_repositories/_Cargo.toml", escape = "none")]
        struct DomainCargoTemplate<'a> {
            db: &'a str,
        }
        DomainCargoTemplate { db }.render()?
    } else {
        fs::read_to_string(&file_path)?
    };
    for (group, _) in groups {
        let reg = Regex::new(&format!(r"(?m)^repository_{}_{}\s*=", db, group))?;
        if !reg.is_match(&content) {
            let db = &db.to_case(Case::Snake);
            let group = &group.to_case(Case::Snake);
            content = content.replace(
                "\"mockall\"",
                &format!("\"mockall\",\"repository_{}_{}/mock\"", db, group),
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
    }
    fs_write(file_path, &*content)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn write_entity(
    repositories_dir: &Path,
    db: &str,
    base_group_name: &str,
    group_name: &str,
    mod_name: &str,
    force: bool,
    model_name: &str,
    def: &Arc<ModelDef>,
    remove_files: &mut HashSet<OsString>,
) -> Result<(), anyhow::Error> {
    set_domain_mode(true);
    let domain_group_dir = repositories_dir.join(group_name.to_case(Case::Snake));
    let file_path = domain_group_dir.join(format!("{}.rs", mod_name));
    remove_files.remove(file_path.as_os_str());
    let pascal_name = &model_name.to_case(Case::Pascal);
    let id_name = &to_id_name(model_name);
    let model_id: u64 = if let Some(model_id) = def.model_id {
        model_id
    } else {
        use crc::{CRC_64_ECMA_182, Crc};
        pub const CRC64: Crc<u64> = Crc::<u64>::new(&CRC_64_ECMA_182);
        CRC64.checksum(format!("{db}:{group_name}:{mod_name}").as_bytes())
    };

    #[derive(Template)]
    #[template(
        path = "domain/group_repositories/src/repositories/entities/entity.rs",
        escape = "none"
    )]
    pub struct DomainEntityTemplate<'a> {
        pub db: &'a str,
        pub base_group_name: &'a str,
        pub group_name: &'a str,
        pub mod_name: &'a str,
        pub pascal_name: &'a str,
        pub id_name: &'a str,
        pub def: &'a Arc<ModelDef>,
        pub model_id: u64,
    }

    if force || !file_path.exists() {
        let tpl = DomainEntityTemplate {
            db,
            base_group_name,
            group_name,
            mod_name,
            pascal_name,
            id_name,
            def,
            model_id,
        };
        fs_write(file_path, tpl.render()?)?;
    }
    let domain_group_base_dir = domain_group_dir.join("_base");
    let file_path = domain_group_base_dir.join(format!("_{}.rs", mod_name));
    remove_files.remove(file_path.as_os_str());

    #[derive(Template)]
    #[template(
        path = "domain/group_repositories/src/repositories/entities/base/_entity.rs",
        escape = "none"
    )]
    pub struct DomainBaseEntityTemplate<'a> {
        pub db: &'a str,
        pub base_group_name: &'a str,
        pub group_name: &'a str,
        pub mod_name: &'a str,
        pub model_name: &'a str,
        pub pascal_name: &'a str,
        pub id_name: &'a str,
        pub def: &'a Arc<ModelDef>,
    }

    let tpl = DomainBaseEntityTemplate {
        db,
        base_group_name,
        group_name,
        mod_name,
        model_name,
        pascal_name,
        id_name,
        def,
    };
    fs_write(file_path, tpl.render()?)?;
    set_domain_mode(false);
    Ok(())
}
