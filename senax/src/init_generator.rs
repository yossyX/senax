use anyhow::Result;
use askama::Template;
use rand::distr::{Alphanumeric, SampleString};
use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{DOMAIN_PATH, SCHEMA_PATH, SIMPLE_VALUE_OBJECTS_FILE, common::fs_write};

pub fn generate(name: &Option<String>, non_snake_case: bool) -> Result<()> {
    let base_path: PathBuf = if let Some(name) = name {
        crate::common::check_ascii_name(name);
        fs::create_dir_all(name)?;
        name.parse()?
    } else {
        ".".parse()?
    };

    #[derive(Template)]
    #[template(path = "init/_Cargo.toml", escape = "none")]
    struct CargoTemplate;

    let file_path = base_path.join("Cargo.toml");
    let tpl = CargoTemplate;
    fs_write(file_path, tpl.render()?)?;

    #[derive(Template)]
    #[template(path = "init/.env.example", escape = "none")]
    struct EnvTemplate {
        pub tz: String,
        pub secret_key: String,
    }

    let file_path = base_path.join(".env.example");
    let mut rng = rand::rng();
    let tpl = EnvTemplate {
        tz: std::env::var("TZ").unwrap_or_default(),
        secret_key: Alphanumeric.sample_string(&mut rng, 40),
    };
    fs_write(file_path, tpl.render()?)?;

    let file_path = base_path.join(".env");
    if !file_path.exists() {
        fs_write(file_path, tpl.render()?)?;
    }

    #[derive(Template)]
    #[template(path = "init/.gitignore", escape = "none")]
    struct GitignoreTemplate;

    let file_path = base_path.join(".gitignore");
    let tpl = GitignoreTemplate;
    fs_write(file_path, tpl.render()?)?;

    #[derive(Template)]
    #[template(path = "init/build.sh", escape = "none")]
    struct BuildShTemplate;

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

    #[derive(Template)]
    #[template(path = "init/schema/_simple_value_objects.yml", escape = "none")]
    struct SimpleValueObjectsTemplate;

    let file_path = schema_path.join(SIMPLE_VALUE_OBJECTS_FILE);
    let tpl = SimpleValueObjectsTemplate;
    fs_write(file_path, tpl.render()?)?;

    let domain_path = base_path.join(DOMAIN_PATH);

    base_domain(&domain_path.join("base_domain"), non_snake_case)?;

    #[derive(Template)]
    #[template(path = "domain/_Cargo.toml", escape = "none")]
    struct DomainCargoTemplate;

    let file_path = domain_path.join("Cargo.toml");
    let tpl = DomainCargoTemplate;
    fs_write(file_path, tpl.render()?)?;

    #[derive(Template)]
    #[template(path = "domain/src/lib.rs", escape = "none")]
    struct DomainLibTemplate {
        pub non_snake_case: bool,
    }

    let file_path = domain_path.join("src/lib.rs");
    let tpl = DomainLibTemplate { non_snake_case };
    fs_write(file_path, tpl.render()?)?;

    #[derive(Template)]
    #[template(path = "domain/src/repository.rs", escape = "none")]
    struct DomainRepositoryTemplate;

    let file_path = domain_path.join("src/repository.rs");
    let tpl = DomainRepositoryTemplate;
    fs_write(file_path, tpl.render()?)?;

    #[derive(Template)]
    #[template(path = "domain/src/use_cases.rs", escape = "none")]
    struct DomainUseCasesTemplate;

    let file_path = domain_path.join("src/use_cases.rs");
    let tpl = DomainUseCasesTemplate;
    fs_write(file_path, tpl.render()?)?;

    #[derive(Template)]
    #[template(path = "domain/src/services.rs", escape = "none")]
    struct DomainServicesTemplate;

    let file_path = domain_path.join("src/services.rs");
    let tpl = DomainServicesTemplate.render()?;
    fs_write(file_path, tpl)?;

    #[derive(Template)]
    #[template(path = "domain/src/events.rs", escape = "none")]
    struct DomainEventsTemplate;

    let file_path = domain_path.join("src/events.rs");
    let tpl = DomainEventsTemplate.render()?;
    fs_write(file_path, tpl)?;

    Ok(())
}

fn base_domain(path: &Path, non_snake_case: bool) -> Result<()> {
    #[derive(Template)]
    #[template(path = "domain/base_domain/_Cargo.toml", escape = "none")]
    struct DomainCargoTemplate;

    let file_path = path.join("Cargo.toml");
    let tpl = DomainCargoTemplate;
    fs_write(file_path, tpl.render()?)?;

    #[derive(Template)]
    #[template(path = "domain/base_domain/src/lib.rs", escape = "none")]
    struct DomainLibTemplate {
        pub non_snake_case: bool,
    }

    let file_path = path.join("src/lib.rs");
    let tpl = DomainLibTemplate { non_snake_case };
    fs_write(file_path, tpl.render()?)?;

    #[derive(Template)]
    #[template(path = "domain/base_domain/src/models.rs", escape = "none")]
    struct DomainModelsTemplate;

    let file_path = path.join("src/models.rs");
    let tpl = DomainModelsTemplate;
    fs_write(file_path, tpl.render()?)?;

    #[derive(Template)]
    #[template(path = "domain/base_domain/src/value_objects.rs", escape = "none")]
    struct DomainValueObjectsTemplate;

    let file_path = path.join("src/value_objects.rs");
    let tpl = DomainValueObjectsTemplate;
    fs_write(file_path, tpl.render()?)?;

    Ok(())
}

pub fn session() -> Result<()> {
    anyhow::ensure!(Path::new("Cargo.toml").exists(), "Incorrect directory.");
    let schema_path = Path::new(SCHEMA_PATH);

    #[derive(Template)]
    #[template(path = "init/schema/session.yml", escape = "none")]
    struct SessionTemplate {
        pub db_id: u64,
    }

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
