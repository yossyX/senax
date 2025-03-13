use anyhow::{ensure, Context, Result};
use askama::Template;
use convert_case::{Case, Casing};
use indexmap::IndexMap;
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use crate::schema::{ConfigDef, StringOrArray, VALUE_OBJECTS};
use crate::{
    common::fs_write,
    schema::{self, set_domain_mode, to_id_name, ModelDef, CONFIG, GROUPS, MODELS},
    DB_PATH, DOMAIN_PATH,
};

pub mod template;

pub fn generate(db: &str, force: bool, clean: bool, skip_version_check: bool) -> Result<()> {
    if !skip_version_check {
        check_version(db)?;
    }
    let non_snake_case = crate::common::check_non_snake_case()?;
    schema::parse(db, false, false)?;

    let config = CONFIG.read().unwrap().as_ref().unwrap().clone();
    let exclude_domain = config.excluded_from_domain;
    let groups = GROUPS.read().unwrap().as_ref().unwrap().clone();
    let model_dir = Path::new(DB_PATH).join(db.to_case(Case::Snake));
    fs::create_dir_all(&model_dir)?;
    let domain_src_dir = Path::new(DOMAIN_PATH).join("src");
    fs::create_dir_all(&domain_src_dir)?;

    let file_path = model_dir.join("Cargo.toml");
    if force || !file_path.exists() {
        let tpl = template::CargoTemplate {
            db,
            config: &config,
        };
        fs_write(file_path, tpl.render()?)?;
    }

    let file_path = model_dir.join("build.rs");
    if force || !file_path.exists() {
        let tpl = template::BuildTemplate {};
        fs_write(file_path, tpl.render()?)?;
    }

    let model_src_dir = model_dir.join("src");
    fs::create_dir_all(&model_src_dir)?;

    let file_path = model_src_dir.join("lib.rs");
    if force || !file_path.exists() {
        let tpl = template::LibTemplate {
            db,
            config: &config,
            non_snake_case,
        };
        fs_write(file_path, tpl.render()?)?;
    }

    let file_path = model_src_dir.join("models.rs");
    let tpl = template::ModelsTemplate {
        groups: &groups,
        config: &config,
    };
    fs_write(file_path, tpl.render()?)?;

    let mod_names: BTreeMap<_, _> = VALUE_OBJECTS
        .read()
        .unwrap()
        .as_ref()
        .unwrap()
        .iter()
        .map(|(name, _def)| (name.to_case(Case::Snake), name.to_case(Case::Pascal)))
        .collect();
    let file_path = domain_src_dir.join("value_objects.rs");
    write_value_objects_rs(&file_path, &mod_names)?;

    let value_objects_dir = domain_src_dir.join("value_objects");
    let value_objects_base_dir = value_objects_dir.join("_base");
    let mut remove_files = HashSet::new();
    if clean && value_objects_dir.exists() {
        for entry in glob::glob(&format!("{}/**/*.rs", value_objects_dir.display()))? {
            match entry {
                Ok(path) => remove_files.insert(path.as_os_str().to_owned()),
                Err(_) => false,
            };
        }
    }
    fs::create_dir_all(&value_objects_base_dir)?;
    for (name, def) in VALUE_OBJECTS.read().unwrap().as_ref().unwrap() {
        let mod_name = name.to_case(Case::Snake);
        let mod_name = &mod_name;
        let file_path = value_objects_base_dir.join(format!("_{}.rs", mod_name));
        remove_files.remove(file_path.as_os_str());
        let tpl = template::DomainValueObjectBaseTemplate {
            mod_name,
            pascal_name: &name.to_case(Case::Pascal),
            def,
        }
        .render()?;
        fs_write(file_path, tpl)?;

        let file_path = value_objects_dir.join(format!("{}.rs", mod_name));
        remove_files.remove(file_path.as_os_str());
        let tpl = template::DomainValueObjectWrapperTemplate {
            mod_name,
            pascal_name: &name.to_case(Case::Pascal),
        }
        .render()?;
        if force || !file_path.exists() {
            fs_write(file_path, tpl)?;
        }
    }

    if !exclude_domain {
        write_impl_domain_db_rs(&model_src_dir, db, &groups, force)?;
    }
    let domain_models_dir = domain_src_dir.join("models");
    let impl_domain_dir = model_src_dir.join("impl_domain");
    if !exclude_domain {
        fs::create_dir_all(&domain_models_dir)?;
        if clean && impl_domain_dir.exists() {
            for entry in glob::glob(&format!("{}/**/*.rs", impl_domain_dir.display()))? {
                match entry {
                    Ok(path) => remove_files.insert(path.as_os_str().to_owned()),
                    Err(_) => false,
                };
            }
        }
        fs::create_dir_all(&impl_domain_dir)?;
        write_domain_db_rs(&domain_models_dir, db, &groups, force)?;
    }

    let file_path = model_src_dir.join("main.rs");
    if force || !file_path.exists() {
        let tpl = template::MainTemplate { db };
        fs_write(file_path, tpl.render()?)?;
    }

    let file_path = model_src_dir.join("seeder.rs");
    let tpl = template::SeederTemplate { groups: &groups };
    fs_write(file_path, tpl.render()?)?;

    let file_path = model_src_dir.join("accessor.rs");
    let tpl = template::AccessorTemplate {};
    fs_write(file_path, tpl.render()?)?;

    let file_path = model_src_dir.join("cache.rs");
    if !config.force_disable_cache {
        let tpl = template::CacheTemplate {};
        fs_write(file_path, tpl.render()?)?;
    } else if file_path.exists() {
        fs::remove_file(&file_path)?;
    }

    let file_path = model_src_dir.join("misc.rs");
    let tpl = template::MiscTemplate { config: &config };
    fs_write(file_path, tpl.render()?)?;

    let file_path = model_src_dir.join("connection.rs");
    let tpl = template::ConnectionTemplate {
        db,
        config: &config,
        tx_isolation: config.tx_isolation.map(|v| v.as_str()),
        read_tx_isolation: config.read_tx_isolation.map(|v| v.as_str()),
    };
    fs_write(file_path, tpl.render()?)?;

    let path = model_dir.join("migrations");
    if !path.exists() {
        fs::create_dir_all(&path)?;
        let file_path = path.join(".gitkeep");
        fs_write(file_path, "")?;
    }

    let path = model_dir.join("seeds");
    if !path.exists() {
        fs::create_dir_all(&path)?;
        let file_path = path.join(".gitkeep");
        fs_write(file_path, "")?;
    }

    let model_models_dir = model_src_dir.join("models");
    if clean && model_models_dir.exists() {
        for entry in glob::glob(&format!("{}/**/*.rs", model_models_dir.display()))? {
            match entry {
                Ok(path) => remove_files.insert(path.as_os_str().to_owned()),
                Err(_) => false,
            };
        }
    }
    fs::create_dir_all(&model_models_dir)?;

    let domain_db_dir = domain_models_dir.join(db.to_case(Case::Snake));
    if !exclude_domain {
        if clean && domain_db_dir.exists() {
            for entry in glob::glob(&format!("{}/**/*.rs", domain_db_dir.display()))? {
                match entry {
                    Ok(path) => remove_files.insert(path.as_os_str().to_owned()),
                    Err(_) => false,
                };
            }
        }
        fs::create_dir_all(&domain_db_dir)?;
    }

    for (group_name, defs) in &groups {
        let group_name = group_name.to_case(Case::Snake);
        let group_name = &group_name;
        MODELS.write().unwrap().replace(defs.clone());
        let mod_names: BTreeSet<String> = defs.iter().map(|(_, d)| d.mod_name()).collect();
        let entities_mod_names: BTreeSet<(String, &String)> = defs
            .iter()
            .filter(|(_, d)| !d.abstract_mode)
            .map(|(model_name, def)| (def.mod_name(), model_name))
            .collect();

        let model_group_dir = model_models_dir.join(group_name);
        fs::create_dir_all(&model_group_dir)?;

        let model_group_base_dir = model_group_dir.join("_base");
        fs::create_dir_all(&model_group_base_dir)?;

        let file_path = model_models_dir.join(format!("{}.rs", group_name));
        remove_files.remove(file_path.as_os_str());
        let concrete_models = defs.iter().filter(|(_k, v)| !v.abstract_mode).collect();
        let tpl = template::GroupTemplate {
            group_name,
            mod_names: &mod_names,
            models: concrete_models,
            config: &config,
        };
        fs_write(file_path, tpl.render()?)?;

        if !exclude_domain {
            write_domain_group_rs(
                &domain_db_dir,
                group_name,
                &entities_mod_names,
                &mod_names,
                force,
                &mut remove_files,
            )?;
            write_impl_domain_group_rs(
                &impl_domain_dir,
                db,
                group_name,
                &entities_mod_names,
                force,
                &mut remove_files,
            )?;
        }

        let domain_group_dir = domain_db_dir.join(group_name);
        if !exclude_domain {
            fs::create_dir_all(&domain_group_dir)?;
        }
        let visibility = if config.export_db_layer {
            ""
        } else {
            "(crate)"
        };

        for (model_name, def) in defs {
            let table_name = def.table_name();
            let mod_name = def.mod_name();
            let mod_name = &mod_name;
            if def.abstract_mode {
                let file_path = model_group_dir.join(format!("{}.rs", mod_name));
                remove_files.remove(file_path.as_os_str());
                if force || !file_path.exists() {
                    let tpl = template::GroupAbstractTemplate {
                        db,
                        group_name,
                        mod_name,
                        name: model_name,
                        pascal_name: &model_name.to_case(Case::Pascal),
                        def,
                        config: &config,
                        visibility,
                    };
                    fs_write(file_path, tpl.render()?)?;
                }

                let file_path = model_group_base_dir.join(format!("_{}.rs", mod_name));
                remove_files.remove(file_path.as_os_str());
                let tpl = template::GroupBaseAbstractTemplate {
                    db,
                    group_name,
                    mod_name,
                    name: model_name,
                    pascal_name: &model_name.to_case(Case::Pascal),
                    id_name: &to_id_name(model_name),
                    table_name: &table_name,
                    def,
                    config: &config,
                    visibility,
                };
                fs_write(file_path, tpl.render()?)?;

                if !exclude_domain {
                    write_domain_abstract(
                        &domain_db_dir,
                        db,
                        group_name,
                        mod_name,
                        force,
                        model_name,
                        def,
                        &mut remove_files,
                    )?;
                }
            } else {
                let file_path = model_group_dir.join(format!("{}.rs", mod_name));
                remove_files.remove(file_path.as_os_str());
                if force || !file_path.exists() {
                    let tpl = template::GroupTableTemplate {
                        db,
                        group_name,
                        mod_name,
                        model_name,
                        pascal_name: &model_name.to_case(Case::Pascal),
                        id_name: &to_id_name(model_name),
                        def,
                        config: &config,
                        visibility,
                    };
                    fs_write(file_path, tpl.render()?)?;
                }

                let file_path = model_group_base_dir.join(format!("_{}.rs", mod_name));
                remove_files.remove(file_path.as_os_str());
                let mut force_indexes = Vec::new();
                let (_, _, idx_map) = crate::migration_generator::make_table_def(def, &config)?;
                for (index_name, index_def) in &def.merged_indexes {
                    for (force_index_name, force_index_def) in &index_def.force_index_on {
                        let force_index_def = force_index_def.clone().unwrap_or_default();
                        let includes = force_index_def
                            .includes
                            .unwrap_or(StringOrArray::One(force_index_name.clone()));
                        let mut cond: Vec<_> = includes
                            .to_vec()
                            .iter()
                            .map(|v| format!("filter_digest.contains({:?})", v))
                            .collect();
                        let excludes = force_index_def
                            .excludes
                            .unwrap_or(StringOrArray::Many(vec![]));
                        for v in excludes.to_vec() {
                            cond.push(format!("!filter_digest.contains({:?})", v));
                        }
                        let idx = idx_map.get(index_name).unwrap();
                        let idx = format!("{:?}", template::filters::_to_db_col(idx, true));
                        force_indexes.push((cond.join(" && "), idx));
                    }
                }
                let tpl = template::GroupBaseTableTemplate {
                    db,
                    group_name,
                    mod_name,
                    model_name,
                    pascal_name: &model_name.to_case(Case::Pascal),
                    id_name: &to_id_name(model_name),
                    table_name: &table_name,
                    def,
                    force_indexes,
                    config: &config,
                    version_col: schema::ConfigDef::version(),
                    visibility,
                };
                fs_write(file_path, tpl.render()?)?;

                if !exclude_domain {
                    write_domain_entity(
                        &domain_db_dir,
                        db,
                        &config,
                        group_name,
                        mod_name,
                        force,
                        model_name,
                        def,
                        &mut remove_files,
                    )?;
                    write_impl_domain_entity(
                        &impl_domain_dir,
                        db,
                        &config,
                        group_name,
                        mod_name,
                        force,
                        model_name,
                        def,
                        &mut remove_files,
                    )?;
                }
            }
        }
    }
    if !exclude_domain {
        write_domain_models_rs(&domain_src_dir, db)?;
    }
    for file in &remove_files {
        println!("REMOVE:{}", file.to_string_lossy());
        fs::remove_file(file)?;
        let ancestors = Path::new(file).ancestors();
        for ancestor in ancestors {
            if let Ok(dir) = ancestor.read_dir() {
                if dir.count() == 0 {
                    fs::remove_dir(ancestor)?;
                } else {
                    break;
                }
            }
        }
    }
    Ok(())
}

fn write_value_objects_rs(file_path: &Path, mod_names: &BTreeMap<String, String>) -> Result<()> {
    let tpl = template::DomainValueObjectModsTemplate { mod_names }.render()?;
    let tpl = tpl.trim_start();
    if file_path.exists() {
        let content = fs::read_to_string(file_path)?;
        let re = Regex::new(r"(?s)// Do not modify below this line. \(ModStart\).+// Do not modify up to this line. \(ModEnd\)").unwrap();
        ensure!(
            re.is_match(&content),
            "File contents are invalid.: {:?}",
            &file_path
        );
        let content = re.replace(&content, tpl);
        fs_write(file_path, &*content)?;
    } else {
        fs_write(file_path, tpl)?;
    }
    Ok(())
}

fn write_domain_models_rs(domain_src_dir: &Path, db: &str) -> Result<()> {
    let file_path = domain_src_dir.join("models.rs");
    let mut content = if !file_path.exists() {
        template::DomainModelsTemplate.render()?
    } else {
        fs::read_to_string(&file_path)?
    };
    let re = Regex::new(r"// Do not modify this line\. \(Mod:([_a-zA-Z0-9,]*)\)").unwrap();
    let caps = re
        .captures(&content)
        .with_context(|| format!("Illegal file content:{}", &file_path.to_string_lossy()))?;
    let mut all: BTreeSet<String> = caps
        .get(1)
        .unwrap()
        .as_str()
        .split(',')
        .filter(|v| !v.is_empty())
        .map(|v| v.to_string())
        .collect();
    if !all.contains(db) {
        all.insert(db.to_string());
        let all = all.iter().cloned().collect::<Vec<_>>().join(",");
        let tpl = template::DomainModelsModTemplate { all, db }.render()?;
        content = re.replace(&content, tpl.trim_start()).to_string();

        let re = Regex::new(r"// Do not modify this line\. \(UseRepo\)").unwrap();
        let tpl = template::DomainModelsUseRepoTemplate { db }.render()?;
        content = re.replace(&content, tpl.trim_start()).to_string();

        let re = Regex::new(r"// Do not modify this line\. \(Repo\)").unwrap();
        let tpl = template::DomainModelsRepoTemplate { db }.render()?;
        content = re.replace(&content, tpl.trim_start()).to_string();

        let re = Regex::new(r"// Do not modify this line\. \(EmuRepo\)").unwrap();
        let tpl = template::DomainModelsEmuRepoTemplate { db }.render()?;
        content = re.replace(&content, tpl.trim_start()).to_string();

        let re = Regex::new(r"// Do not modify this line\. \(EmuImpl\)").unwrap();
        let tpl = template::DomainModelsEmuImplTemplate { db }.render()?;
        content = re.replace(&content, tpl.trim_start()).to_string();

        let re = Regex::new(r"// Do not modify this line\. \(EmuImplStart\)").unwrap();
        let tpl = template::DomainModelsEmuImplStartTemplate { db }.render()?;
        content = re.replace(&content, tpl.trim_start()).to_string();

        let re = Regex::new(r"// Do not modify this line\. \(EmuImplCommit\)").unwrap();
        let tpl = template::DomainModelsEmuImplCommitTemplate { db }.render()?;
        content = re.replace(&content, tpl.trim_start()).to_string();

        let re = Regex::new(r"// Do not modify this line\. \(EmuImplRollback\)").unwrap();
        let tpl = template::DomainModelsEmuImplRollbackTemplate { db }.render()?;
        content = re.replace(&content, tpl.trim_start()).to_string();
    }

    fs_write(&file_path, &*content)?;
    Ok(())
}

fn write_domain_db_rs(
    domain_models_dir: &Path,
    db: &str,
    groups: &IndexMap<String, IndexMap<String, Arc<ModelDef>>>,
    force: bool,
) -> Result<()> {
    let file_path = domain_models_dir.join(format!("{}.rs", &db.to_case(Case::Snake)));
    let content = if force || !file_path.exists() {
        template::DomainDbTemplate { db }.render()?
    } else {
        fs::read_to_string(&file_path)?
    };

    let re = Regex::new(r"(?s)// Do not modify below this line. \(ModStart\).+// Do not modify up to this line. \(ModEnd\)").unwrap();
    ensure!(
        re.is_match(&content),
        "File contents are invalid.: {:?}",
        &file_path
    );
    let tpl = template::DomainDbModTemplate { groups }.render()?;
    let tpl = tpl.trim_start();
    let content = re.replace(&content, tpl);

    let re = Regex::new(r"(?s)// Do not modify below this line. \(RepoStart\).+// Do not modify up to this line. \(RepoEnd\)").unwrap();
    ensure!(
        re.is_match(&content),
        "File contents are invalid.: {:?}",
        &file_path
    );
    let tpl = template::DomainDbRepoTemplate { groups }.render()?;
    let tpl = tpl.trim_start();
    let content = re.replace(&content, tpl);

    let re = Regex::new(r"(?s)// Do not modify below this line. \(QueriesStart\).+// Do not modify up to this line. \(QueriesEnd\)").unwrap();
    ensure!(
        re.is_match(&content),
        "File contents are invalid.: {:?}",
        &file_path
    );
    let tpl = template::QueriesDbQueriesTemplate { groups }.render()?;
    let tpl = tpl.trim_start();
    let content = re.replace(&content, tpl);

    let re = Regex::new(r"(?s)// Do not modify below this line. \(EmuRepoStart\).+// Do not modify up to this line. \(EmuRepoEnd\)").unwrap();
    ensure!(
        re.is_match(&content),
        "File contents are invalid.: {:?}",
        &file_path
    );
    let tpl = template::DomainDbEmuRepoTemplate { groups }.render()?;
    let tpl = tpl.trim_start();
    let content = re.replace(&content, tpl);

    let re = Regex::new(r"(?s)// Do not modify below this line. \(EmuQueriesStart\).+// Do not modify up to this line. \(EmuQueriesEnd\)").unwrap();
    ensure!(
        re.is_match(&content),
        "File contents are invalid.: {:?}",
        &file_path
    );
    let tpl = template::DomainDbEmuQueriesTemplate { groups }.render()?;
    let tpl = tpl.trim_start();
    let content = re.replace(&content, tpl);

    fs_write(file_path, &*content)?;
    Ok(())
}

fn write_impl_domain_db_rs(
    model_src_dir: &Path,
    db: &str,
    groups: &IndexMap<String, IndexMap<String, Arc<ModelDef>>>,
    force: bool,
) -> Result<()> {
    let file_path = model_src_dir.join("impl_domain.rs");
    let content = if force || !file_path.exists() {
        template::ImplDomainDbTemplate { db }.render()?
    } else {
        fs::read_to_string(&file_path)?
    };

    let re = Regex::new(r"(?s)// Do not modify below this line. \(ModStart\).+// Do not modify up to this line. \(ModEnd\)").unwrap();
    ensure!(
        re.is_match(&content),
        "File contents are invalid.: {:?}",
        &file_path
    );
    let tpl = template::DomainDbModTemplate { groups }.render()?;
    let tpl = tpl.trim_start();
    let content = re.replace(&content, tpl);

    let re = Regex::new(r"(?s)// Do not modify below this line. \(RepoStart\).+// Do not modify up to this line. \(RepoEnd\)").unwrap();
    ensure!(
        re.is_match(&content),
        "File contents are invalid.: {:?}",
        &file_path
    );
    let tpl = template::ImplDomainDbRepoTemplate { groups }.render()?;
    let tpl = tpl.trim_start();
    let content = re.replace(&content, tpl);

    let re = Regex::new(r"(?s)// Do not modify below this line. \(QueriesStart\).+// Do not modify up to this line. \(QueriesEnd\)").unwrap();
    ensure!(
        re.is_match(&content),
        "File contents are invalid.: {:?}",
        &file_path
    );
    let tpl = template::ImplDomainDbQueriesTemplate { groups }.render()?;
    let tpl = tpl.trim_start();
    let content = re.replace(&content, tpl);

    fs_write(file_path, &*content)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn write_domain_abstract(
    domain_db_dir: &Path,
    db: &str,
    group_name: &String,
    mod_name: &str,
    force: bool,
    model_name: &String,
    def: &Arc<ModelDef>,
    remove_files: &mut HashSet<OsString>,
) -> Result<(), anyhow::Error> {
    set_domain_mode(true);
    let domain_group_dir = domain_db_dir.join(group_name);
    fs::create_dir_all(&domain_group_dir)?;
    let file_path = domain_group_dir.join(format!("{}.rs", mod_name));
    remove_files.remove(file_path.as_os_str());
    let pascal_name = &model_name.to_case(Case::Pascal);
    if force || !file_path.exists() {
        let tpl = template::DomainAbstractTemplate {
            mod_name,
            pascal_name,
            def,
        };
        fs_write(file_path, tpl.render()?)?;
    }
    let domain_group_base_dir = domain_group_dir.join("_base");
    fs::create_dir_all(&domain_group_base_dir)?;
    let file_path = domain_group_base_dir.join(format!("_{}.rs", mod_name));
    remove_files.remove(file_path.as_os_str());
    let tpl = template::DomainBaseAbstractTemplate {
        db,
        group_name,
        mod_name,
        pascal_name,
        def,
    };
    fs_write(file_path, tpl.render()?)?;
    set_domain_mode(false);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn write_domain_entity(
    domain_db_dir: &Path,
    db: &str,
    config: &ConfigDef,
    group_name: &String,
    mod_name: &str,
    force: bool,
    model_name: &String,
    def: &Arc<ModelDef>,
    remove_files: &mut HashSet<OsString>,
) -> Result<(), anyhow::Error> {
    set_domain_mode(true);
    let domain_group_dir = domain_db_dir.join(group_name);
    fs::create_dir_all(&domain_group_dir)?;
    let file_path = domain_group_dir.join(format!("{}.rs", mod_name));
    remove_files.remove(file_path.as_os_str());
    let pascal_name = &model_name.to_case(Case::Pascal);
    let id_name = &to_id_name(model_name);
    let model_id: u64 = if let Some(model_id) = def.model_id {
        model_id
    } else {
        use crc::{Crc, CRC_64_ECMA_182};
        pub const CRC64: Crc<u64> = Crc::<u64>::new(&CRC_64_ECMA_182);
        CRC64.checksum(format!("{db}:{group_name}:{mod_name}").as_bytes())
    };
    if force || !file_path.exists() {
        let tpl = template::DomainEntityTemplate {
            db,
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
    fs::create_dir_all(&domain_group_base_dir)?;
    let file_path = domain_group_base_dir.join(format!("_{}.rs", mod_name));
    remove_files.remove(file_path.as_os_str());
    let tpl = template::DomainBaseEntityTemplate {
        db,
        config,
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

fn write_domain_group_rs(
    domain_db_dir: &Path,
    group_name: &String,
    entities_mod_names: &BTreeSet<(String, &String)>,
    mod_names: &BTreeSet<String>,
    force: bool,
    remove_files: &mut HashSet<OsString>,
) -> Result<()> {
    let file_path = domain_db_dir.join(format!("{}.rs", group_name));
    remove_files.remove(file_path.as_os_str());
    let content = if force || !file_path.exists() {
        template::DomainGroupTemplate { group_name }.render()?
    } else {
        fs::read_to_string(&file_path)?
    };

    let re = Regex::new(r"(?s)// Do not modify below this line. \(ModStart\).+// Do not modify up to this line. \(ModEnd\)").unwrap();
    ensure!(
        re.is_match(&content),
        "File contents are invalid.: {:?}",
        &file_path
    );
    let tpl = template::DomainGroupModTemplate { mod_names }.render()?;
    let tpl = tpl.trim_start();
    let content = re.replace(&content, tpl);

    let re = Regex::new(r"(?s)// Do not modify below this line. \(RepoStart\).+// Do not modify up to this line. \(RepoEnd\)").unwrap();
    ensure!(
        re.is_match(&content),
        "File contents are invalid.: {:?}",
        &file_path
    );
    let tpl = template::DomainGroupRepoTemplate {
        mod_names: entities_mod_names,
    }
    .render()?;
    let tpl = tpl.trim_start();
    let content = re.replace(&content, tpl);

    let re = Regex::new(r"(?s)// Do not modify below this line. \(QueriesStart\).+// Do not modify up to this line. \(QueriesEnd\)").unwrap();
    ensure!(
        re.is_match(&content),
        "File contents are invalid.: {:?}",
        &file_path
    );
    let tpl = template::DomainGroupQueriesTemplate {
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
    let tpl = template::DomainGroupEmuRepoTemplate {
        mod_names: entities_mod_names,
    }
    .render()?;
    let tpl = tpl.trim_start();
    let content = re.replace(&content, tpl);

    let re = Regex::new(r"(?s)// Do not modify below this line. \(EmuQueriesStart\).+// Do not modify up to this line. \(EmuQueriesEnd\)").unwrap();
    ensure!(
        re.is_match(&content),
        "File contents are invalid.: {:?}",
        &file_path
    );
    let tpl = template::DomainGroupEmuQueriesTemplate {
        mod_names: entities_mod_names,
    }
    .render()?;
    let tpl = tpl.trim_start();
    let content = re.replace(&content, tpl);

    fs_write(file_path, &*content)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn write_impl_domain_entity(
    impl_domain_dir: &Path,
    db: &str,
    config: &ConfigDef,
    group_name: &String,
    mod_name: &str,
    force: bool,
    model_name: &String,
    def: &Arc<ModelDef>,
    remove_files: &mut HashSet<OsString>,
) -> Result<(), anyhow::Error> {
    set_domain_mode(true);
    let impl_domain_group_dir = impl_domain_dir.join(group_name);
    fs::create_dir_all(&impl_domain_group_dir)?;
    let file_path = impl_domain_group_dir.join(format!("{}.rs", mod_name));
    remove_files.remove(file_path.as_os_str());
    let pascal_name = &model_name.to_case(Case::Pascal);
    let id_name = &to_id_name(model_name);
    if force || !file_path.exists() {
        let tpl = template::ImplDomainEntityTemplate {
            db,
            group_name,
            mod_name,
            pascal_name,
            id_name,
            def,
        };
        fs_write(file_path, tpl.render()?)?;
    }
    let path = impl_domain_group_dir.join("_base");
    fs::create_dir_all(&path)?;
    let file_path = path.join(format!("_{}.rs", mod_name));
    remove_files.remove(file_path.as_os_str());
    let tpl = template::ImplDomainBaseEntityTemplate {
        db,
        config,
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

fn write_impl_domain_group_rs(
    impl_domain_dir: &Path,
    db: &str,
    group_name: &String,
    entities_mod_names: &BTreeSet<(String, &String)>,
    force: bool,
    remove_files: &mut HashSet<OsString>,
) -> Result<()> {
    let file_path = impl_domain_dir.join(format!("{}.rs", group_name));
    remove_files.remove(file_path.as_os_str());
    let content = if force || !file_path.exists() {
        template::ImplDomainGroupTemplate { db, group_name }.render()?
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
    let tpl = template::DomainGroupModTemplate {
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
    let tpl = template::ImplDomainGroupRepoTemplate {
        mod_names: entities_mod_names,
    }
    .render()?;
    let tpl = tpl.trim_start();
    let content = re.replace(&content, tpl);

    let re = Regex::new(r"(?s)// Do not modify below this line. \(QueriesStart\).+// Do not modify up to this line. \(QueriesEnd\)").unwrap();
    ensure!(
        re.is_match(&content),
        "File contents are invalid.: {:?}",
        &file_path
    );
    let tpl = template::ImplDomainGroupQueriesTemplate {
        mod_names: entities_mod_names,
    }
    .render()?;
    let tpl = tpl.trim_start();
    let content = re.replace(&content, tpl);

    fs_write(file_path, &*content)?;
    Ok(())
}

pub fn check_version(db: &str) -> Result<()> {
    let model_dir = Path::new(DB_PATH).join(db.to_case(Case::Snake));
    let model_src_dir = model_dir.join("src");
    let file_path = model_src_dir.join("models.rs");
    if file_path.exists() {
        let content = fs::read_to_string(&file_path)?;
        let re = Regex::new(r"(?m)^// Senax v(.+)$").unwrap();
        let caps = re
            .captures(&content)
            .with_context(|| format!("Illegal file content:{}", &file_path.to_string_lossy()))?;
        let ver = caps.get(1).unwrap().as_str();
        let req = semver::VersionReq::parse(ver)?;
        let version = semver::Version::parse(crate::VERSION)?;
        ensure!(req.matches(&version), "Use {} version of Senax.", ver);
    }
    Ok(())
}
