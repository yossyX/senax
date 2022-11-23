use anyhow::Result;
use askama::Template;
use convert_case::{Case, Casing};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use crate::{
    schema::{self, to_id_name, CONFIG, ENUM_GROUPS, GROUPS, MODEL, MODELS},
    MODELS_PATH,
};

pub mod template;

pub fn generate(db: &str, force: bool) -> Result<()> {
    schema::parse(db)?;

    let config = unsafe { CONFIG.get().unwrap() }.clone();
    let groups = unsafe { GROUPS.get().unwrap() }.clone();
    let enum_groups = unsafe { ENUM_GROUPS.get().unwrap() }.clone();
    let base_path = MODELS_PATH.get().unwrap().join(&db);
    fs::create_dir_all(&base_path)?;

    let file_path = base_path.join("Cargo.toml");
    if force || !file_path.exists() {
        let tpl = template::CargoTemplate { db };
        println!("{}", file_path.display());
        fs_write(file_path, tpl.render()?)?;
    }

    let file_path = base_path.join("build.rs");
    if force || !file_path.exists() {
        let tpl = template::BuildTemplate {};
        println!("{}", file_path.display());
        fs_write(file_path, tpl.render()?)?;
    }

    let src_path = base_path.join("src");
    fs::create_dir_all(&src_path)?;

    let file_path = src_path.join("lib.rs");
    if force || !file_path.exists() {
        let tpl = template::LibTemplate {
            db,
            groups: &groups,
            config: &config,
        };
        println!("{}", file_path.display());
        fs_write(file_path, tpl.render()?)?;
    }

    let file_path = src_path.join("main.rs");
    if force || !file_path.exists() {
        let tpl = template::MainTemplate { db };
        println!("{}", file_path.display());
        fs_write(file_path, tpl.render()?)?;
    }

    let file_path = src_path.join("seeder.rs");
    let tpl = template::SeederTemplate { groups: &groups };
    println!("{}", file_path.display());
    fs_write(file_path, tpl.render()?)?;

    let path = base_path.join("migrations");
    if !path.exists() {
        fs::create_dir_all(&path)?;
        let file_path = path.join(".gitkeep");
        println!("{}", file_path.display());
        fs_write(file_path, "")?;
    }

    let path = base_path.join("seeds");
    if !path.exists() {
        fs::create_dir_all(&path)?;
        let file_path = path.join(".gitkeep");
        println!("{}", file_path.display());
        fs_write(file_path, "")?;
    }

    for (group_name, defs) in &groups {
        unsafe {
            MODELS.take();
            MODELS.set(defs.clone()).unwrap();
        }
        let mut mod_names: BTreeSet<&str> = defs.iter().map(|(_, d)| d.mod_name()).collect();
        let mut enum_names: BTreeSet<&str> = enum_groups
            .get(group_name)
            .unwrap()
            .iter()
            .map(|(_, d)| d.mod_name())
            .collect();
        mod_names.append(&mut enum_names);

        let path = src_path.join(group_name);
        fs::create_dir_all(&path)?;

        let base_path = path.join("base");
        fs::create_dir_all(&base_path)?;

        let file_path = src_path.join(format!("{}.rs", group_name));
        let concrete_tables = defs.iter().filter(|(_k, v)| !v.abstract_mode).collect();
        let tpl = template::GroupTemplate {
            group_name,
            mod_names: &mod_names,
            tables: concrete_tables,
            config: &config,
        };
        println!("{}", file_path.display());
        fs_write(file_path, tpl.render()?)?;

        let file_path = src_path.join("accessor.rs");
        let tpl = template::AccessorTemplate {};
        println!("{}", file_path.display());
        fs_write(file_path, tpl.render()?)?;

        let file_path = src_path.join("cache.rs");
        let tpl = template::CacheTemplate {};
        println!("{}", file_path.display());
        fs_write(file_path, tpl.render()?)?;

        let file_path = src_path.join("misc.rs");
        let tpl = template::MiscTemplate {
            db,
            config: &config,
        };
        println!("{}", file_path.display());
        fs_write(file_path, tpl.render()?)?;

        let file_path = src_path.join("connection.rs");
        let tpl = template::ConnectionTemplate {
            db,
            config: &config,
            tx_isolation: config.tx_isolation.map(|v| v.as_str()),
            read_tx_isolation: config.read_tx_isolation.map(|v| v.as_str()),
        };
        println!("{}", file_path.display());
        fs_write(file_path, tpl.render()?)?;

        for (model_name, def) in defs {
            unsafe {
                MODEL.take();
                MODEL.set(def.clone()).unwrap();
            }
            let table_name = def.table_name();
            let mod_name = def.mod_name();
            if def.abstract_mode {
                let file_path = path.join(format!("{}.rs", mod_name));
                if force || !file_path.exists() {
                    let tpl = template::GroupAbstractTemplate {
                        db,
                        group_name,
                        mod_name,
                        name: model_name,
                        def,
                        config: &config,
                    };
                    println!("{}", file_path.display());
                    fs_write(file_path, tpl.render()?)?;
                }

                let file_path = base_path.join(format!("_{}.rs", mod_name));
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
                };
                println!("{}", file_path.display());
                fs_write(file_path, tpl.render()?)?;
            } else {
                let file_path = path.join(format!("{}.rs", mod_name));
                if force || !file_path.exists() {
                    let tpl = template::GroupTableTemplate {
                        db,
                        group_name,
                        mod_name,
                        name: model_name,
                        id_name: &to_id_name(model_name),
                        def,
                        config: &config,
                    };
                    println!("{}", file_path.display());
                    fs_write(file_path, tpl.render()?)?;
                }

                let file_path = base_path.join(format!("_{}.rs", mod_name));
                let tpl = template::GroupBaseTableTemplate {
                    db,
                    group_name,
                    mod_name,
                    name: model_name,
                    pascal_name: &model_name.to_case(Case::Pascal),
                    id_name: &to_id_name(model_name),
                    table_name: &table_name,
                    def,
                    config: &config,
                    version_col: schema::VERSIONED,
                };
                println!("{}", file_path.display());
                fs_write(file_path, tpl.render()?)?;
            }
        }
    }
    for (group_name, defs) in &enum_groups {
        for (model_name, def) in defs {
            let path = src_path.join(group_name);
            let base_path = path.join("base");
            let mod_name = def.mod_name();
            let file_path = path.join(format!("{}.rs", mod_name));
            if force || !file_path.exists() {
                let tpl = template::GroupEnumTemplate {
                    db,
                    group_name,
                    mod_name,
                    name: model_name,
                    def,
                    config: &config,
                };
                println!("{}", file_path.display());
                fs_write(file_path, tpl.render()?)?;
            }

            let file_path = base_path.join(format!("_{}.rs", mod_name));
            let tpl = template::GroupBaseEnumTemplate {
                db,
                group_name,
                mod_name,
                name: model_name,
                pascal_name: &format!("_{}", model_name.to_case(Case::Pascal)),
                def,
                config: &config,
            };
            println!("{}", file_path.display());
            fs_write(file_path, tpl.render()?)?;
        }
    }
    Ok(())
}

fn fs_write<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> Result<()> {
    fn inner(path: &Path, contents: &[u8]) -> Result<()> {
        if let Ok(buf) = fs::read(path) {
            if !buf.eq(contents) {
                fs::write(path, contents)?;
            }
        } else {
            fs::write(path, contents)?;
        }
        Ok(())
    }
    inner(path.as_ref(), contents.as_ref())
}
