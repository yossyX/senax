use anyhow::Result;
use askama::Template;
use rand::distributions::{Alphanumeric, DistString};
use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{common::fs_write, DOMAIN_PATH, SCHEMA_PATH, SIMPLE_VALUE_OBJECTS_FILE};

pub fn generate(name: &Option<String>, non_snake_case: bool) -> Result<()> {
    let base_path: PathBuf = if let Some(name) = name {
        crate::common::check_ascii_name(name);
        fs::create_dir_all(name)?;
        name.parse()?
    } else {
        ".".parse()?
    };

    let file_path = base_path.join("Cargo.toml");
    let tpl = CargoTemplate;
    fs_write(file_path, tpl.render()?)?;

    let file_path = base_path.join(".env.example");
    let mut rng = rand::thread_rng();
    let tpl = EnvTemplate {
        tz: std::env::var("TZ").unwrap_or_default(),
        secret_key: Alphanumeric.sample_string(&mut rng, 40),
    };
    fs_write(file_path, tpl.render()?)?;

    let file_path = base_path.join(".env");
    if !file_path.exists() {
        fs_write(file_path, tpl.render()?)?;
    }

    let file_path = base_path.join(".gitignore");
    let tpl = GitignoreTemplate;
    fs_write(file_path, tpl.render()?)?;

    let file_path = base_path.join("build.sh");
    let tpl = BuildShTemplate;
    fs_write(&file_path, tpl.render()?)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = file_path.metadata()?;
        let mut permissions = metadata.permissions();
        permissions.set_mode(0o100 | permissions.mode());
        fs::set_permissions(file_path, permissions)?;
    }

    crate::schema::json_schema::write_schema(&base_path)?;

    let schema_path = base_path.join(SCHEMA_PATH);
    fs::create_dir_all(&schema_path)?;

    let file_path = schema_path.join(SIMPLE_VALUE_OBJECTS_FILE);
    let tpl = SimpleValueObjectsTemplate;
    fs_write(file_path, tpl.render()?)?;

    let domain_path = base_path.join(DOMAIN_PATH);
    let file_path = domain_path.join("src");
    fs::create_dir_all(file_path)?;

    let file_path = domain_path.join("Cargo.toml");
    let tpl = DomainCargoTemplate;
    fs_write(file_path, tpl.render()?)?;

    let file_path = domain_path.join("src/lib.rs");
    let tpl = DomainLibTemplate { non_snake_case };
    fs_write(file_path, tpl.render()?)?;

    let file_path = domain_path.join("src/models.rs");
    let tpl = DomainModelsTemplate;
    fs_write(file_path, tpl.render()?)?;

    let file_path = domain_path.join("src/use_cases.rs");
    let tpl = DomainUseCasesTemplate;
    fs_write(file_path, tpl.render()?)?;

    let file_path = domain_path.join("src/services.rs");
    let tpl = DomainServicesTemplate.render()?;
    fs_write(file_path, tpl)?;

    let file_path = domain_path.join("src/events.rs");
    let tpl = DomainEventsTemplate.render()?;
    fs_write(file_path, tpl)?;

    let file_path = domain_path.join("src/value_objects.rs");
    let tpl = DomainValueObjectsTemplate;
    fs_write(file_path, tpl.render()?)?;

    Ok(())
}

pub fn session() -> Result<()> {
    anyhow::ensure!(Path::new("Cargo.toml").exists(), "Incorrect directory.");
    let schema_path = Path::new(SCHEMA_PATH);
    fs::create_dir_all(schema_path)?;

    let file_path = schema_path.join("session.yml");
    let tpl = SessionTemplate {
        db_id: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64,
    };
    fs_write(file_path, tpl.render()?)?;
    Ok(())
}

#[derive(Template)]
#[template(path = "init/_Cargo.toml", escape = "none")]
pub struct CargoTemplate;

#[derive(Template)]
#[template(path = "init/.env.example", escape = "none")]
pub struct EnvTemplate {
    pub tz: String,
    pub secret_key: String,
}

#[derive(Template)]
#[template(path = "init/.gitignore", escape = "none")]
pub struct GitignoreTemplate;

#[derive(Template)]
#[template(path = "init/build.sh", escape = "none")]
pub struct BuildShTemplate;

#[derive(Template)]
#[template(path = "init/schema/session.yml", escape = "none")]
pub struct SessionTemplate {
    pub db_id: u64,
}

#[derive(Template)]
#[template(path = "init/schema/_simple_value_objects.yml", escape = "none")]
pub struct SimpleValueObjectsTemplate;

#[derive(Template)]
#[template(path = "init/domain/_Cargo.toml", escape = "none")]
pub struct DomainCargoTemplate;

#[derive(Template)]
#[template(path = "init/domain/src/lib.rs", escape = "none")]
pub struct DomainLibTemplate {
    pub non_snake_case: bool,
}

#[derive(Template)]
#[template(path = "init/domain/src/models.rs", escape = "none")]
pub struct DomainModelsTemplate;

#[derive(Template)]
#[template(path = "init/domain/src/use_cases.rs", escape = "none")]
pub struct DomainUseCasesTemplate;

#[derive(Template)]
#[template(path = "init/domain/src/services.rs", escape = "none")]
pub struct DomainServicesTemplate;

#[derive(Template)]
#[template(path = "init/domain/src/events.rs", escape = "none")]
pub struct DomainEventsTemplate;

#[derive(Template)]
#[template(path = "init/domain/src/value_objects.rs", escape = "none")]
pub struct DomainValueObjectsTemplate;
