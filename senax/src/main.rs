use anyhow::{ensure, Context as _, Result};
use chrono::Utc;
use clap::{Parser, Subcommand};
use dotenvy::dotenv;
use regex::Regex;
use schemars::gen::SchemaSettings;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::{env, path::PathBuf};

use crate::{schema::SchemaDef, schema_md::gen_schema_md};

#[macro_export]
macro_rules! error_exit {
    ($($arg:tt)*) => {{
        if cfg!(debug_assertions) {
            panic!($($arg)*);
        } else {
            use std::io::Write;
            use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
            let mut stderr = StandardStream::stderr(ColorChoice::Auto);
            let _ = stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red)));
            let _ = writeln!(&mut stderr, $($arg)*);
            let _ = stderr.reset();
            std::process::exit(1);
        }
    }};
}

mod api_document;
pub(crate) mod common;
mod db_document;
pub(crate) mod ddl {
    pub mod table;
}
mod actix_generator;
mod api_generator;
#[cfg(feature = "config")]
mod config_server;
mod db_generator;
mod init_generator;
mod migration_generator;
mod model_generator;
pub(crate) mod schema;
mod schema_md;

pub const SCHEMA_PATH: &str = "0_schema";
pub const DOMAIN_PATH: &str = "1_domain";
pub const DB_PATH: &str = "2_db";
pub const SIMPLE_VALUE_OBJECTS_FILE: &str = "_simple_value_objects.yml";
pub const DEFAULT_CONFIG_PORT: u16 = 9100;
pub const API_SCHEMA_PATH: &str = "api_schema";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

include!(concat!(env!("OUT_DIR"), "/templates.rs"));
#[cfg(feature = "config")]
include!(concat!(env!("OUT_DIR"), "/config_app.rs"));

#[derive(Parser)]
#[clap(name = "Senax code generator", author, version, about, long_about = None)]
#[clap(propagate_version = true)]
struct Cli {
    #[clap(long)]
    cwd: Option<PathBuf>,
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a workspace
    Init {
        /// Generated directory name
        name: Option<String>,
        #[clap(long)]
        non_snake_case: bool,
    },
    #[cfg(feature = "config")]
    Config {
        /// open port
        #[clap(short, long)]
        port: Option<u16>,
        /// open browser
        #[clap(short, long)]
        open: bool,
        #[clap(long)]
        backup: Option<PathBuf>,
        #[clap(long)]
        read_only: bool,
    },
    /// Generate an actix server
    NewActix {
        /// package name
        name: String,
        /// DB names
        #[clap(long)]
        db: String,
        /// Force overwrite
        #[clap(short, long)]
        force: bool,
    },
    /// Prepare to use DB
    InitDb {
        /// DB name
        db: String,
    },
    /// generate models
    Model {
        /// Specify the DB
        db: String,
        /// Force overwrite
        #[clap(short, long)]
        force: bool,
        /// Delete files under the directory before generating
        #[clap(short, long)]
        clean: bool,
        /// Skip Senax version check
        #[clap(long)]
        skip_version_check: bool,
    },
    /// generate api
    GenApi {
        /// Specify the server path
        path: PathBuf,
        /// Specify the DB
        db: String,
        /// Specify the group
        group: Option<String>,
        /// Specify the model
        model: Option<String>,
        #[clap(long)]
        ts_dir: Option<PathBuf>,
        /// Inquire about adding a model
        #[clap(short, long)]
        inquiry: bool,
        /// Force overwrite
        #[clap(short, long)]
        force: bool,
        /// Delete files under the directory before generating
        #[clap(short, long)]
        clean: bool,
    },
    /// generate migration ddl
    GenMigrate {
        /// Specify the DB
        db: String,
        /// Specify description and generate a file
        description: Option<String>,
        #[clap(long)]
        skip_empty: bool,
        #[clap(long)]
        use_test_db: bool,
    },
    /// Reflect the name change in the schema after generating the migration.
    ReflectMigrationChanges,
    /// generate a import data file
    GenSeed {
        /// Specify the db
        db: String,
        /// Specify description and output to file
        description: String,
    },
    /// generate a DB document file
    DbDoc {
        /// Specify the db
        db: String,
        /// Specify the group
        group: Option<String>,
        /// Include ER diagram
        #[clap(short, long)]
        er: bool,
        /// History length
        #[clap(short('H'), long)]
        history: Option<usize>,
        /// Output file
        #[clap(short, long)]
        output: Option<PathBuf>,
        /// Template file
        #[clap(short, long)]
        template: Option<PathBuf>,
    },
    /// generate a API document file
    ApiDoc {
        /// Specify the server path
        path: PathBuf,
        /// Specify the db
        db: String,
        /// Specify the group
        group: Option<String>,
        /// Output file
        #[clap(short, long)]
        output: Option<PathBuf>,
        /// Template file
        #[clap(short, long)]
        template: Option<PathBuf>,
    },
    SenaxSchema {
        #[clap(short, long)]
        doc: bool,
    },
    StreamId,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    if let Some(ref cwd) = cli.cwd {
        env::set_current_dir(cwd)
            .with_context(|| format!("directory error!: {}", cwd.to_string_lossy()))?;
    }
    dotenv().ok();

    let result = exec(cli).await;
    if let Err(err) = result {
        use std::io::Write;
        use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
        let mut stderr = StandardStream::stderr(ColorChoice::Auto);
        let _ = stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red)));
        if cfg!(debug_assertions) {
            let _ = write!(&mut stderr, "{:?}", err);
        } else {
            let _ = write!(&mut stderr, "{}", err);
        }
        let _ = stderr.reset();
        let _ = writeln!(&mut stderr);
        std::process::exit(1);
    }
    Ok(())
}

async fn exec(cli: Cli) -> Result<()> {
    let db_re = Regex::new(r"^[a-zA-Z][_a-zA-Z0-9]*$").unwrap();
    match &cli.command {
        Commands::Init {
            name,
            non_snake_case,
        } => {
            init_generator::generate(name, *non_snake_case)?;
        }
        #[cfg(feature = "config")]
        Commands::Config {
            port,
            open,
            backup,
            read_only,
        } => {
            config_server::start(*port, *open, backup, *read_only).await?;
        }
        Commands::NewActix { name, db, force } => {
            let db_list = db.split(',').map(|v| v.trim()).collect();
            actix_generator::generate(name, db_list, *force)?;
        }
        Commands::InitDb { db } => {
            ensure!(db_re.is_match(db), "bad db name!");
            db_generator::generate(db)?;
        }
        Commands::Model {
            db,
            force,
            clean,
            skip_version_check,
        } => {
            ensure!(db_re.is_match(db), "bad db name!");
            model_generator::generate(db, *force, *clean, *skip_version_check)?;
        }
        Commands::GenApi {
            path,
            db,
            group,
            model,
            ts_dir,
            inquiry,
            force,
            clean,
        } => {
            ensure!(db_re.is_match(db), "bad db name!");
            api_generator::generate(path, db, group, model, ts_dir, *inquiry, *force, *clean)?;
        }
        Commands::GenMigrate {
            db,
            description,
            skip_empty,
            use_test_db,
        } => {
            ensure!(db_re.is_match(db), "bad db name!");
            migration_generator::generate(db, description, *skip_empty, *use_test_db).await?;
        }
        Commands::ReflectMigrationChanges => {
            for db in crate::db_generator::list()? {
                common::reflect_migration_changes(&db)?;
            }
        }
        Commands::GenSeed { db, description } => {
            ensure!(db_re.is_match(db), "bad db name!");
            generate_seed_file(db, description)?;
        }
        Commands::DbDoc {
            db,
            group,
            er,
            history,
            output,
            template,
        } => {
            ensure!(db_re.is_match(db), "bad db name!");
            db_document::generate(db, group, *er, history, output, template)?;
        }
        Commands::ApiDoc {
            path,
            db,
            group,
            output,
            template,
        } => {
            ensure!(db_re.is_match(db), "bad db name!");
            let def = api_generator::serialize::generate(path, db, group)?;
            api_document::generate(def, output, template)?;
        }
        Commands::SenaxSchema { doc } => {
            if *doc {
                let settings = SchemaSettings::draft07().with(|s| {
                    s.option_nullable = true;
                    s.option_add_null_type = false;
                });
                let gen = settings.into_generator();
                let schema = gen.into_root_schema_for::<SchemaDef>();
                let schema = serde_json::to_string_pretty(&schema)?;
                let schema: Value = serde_json::from_str(&schema)?;
                let md = gen_schema_md(schema)?;
                println!("{}", md);
            } else {
                schema::json_schema::write_schema(std::path::Path::new("."))?;
            }
        }
        Commands::StreamId => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros() as u64;
            println!("{}", now);
        }
    }
    Ok(())
}

fn generate_seed_file(db: &str, description: &str) -> Result<()> {
    let description: String = description
        .chars()
        .map(|c| {
            if c.is_control() || c.is_whitespace() {
                '_'
            } else {
                c
            }
        })
        .collect();
    let path = Path::new(DB_PATH).join(db).join("seeds");
    fs::create_dir_all(&path)?;
    let dt = Utc::now();
    let file_prefix = dt.format("%Y%m%d%H%M%S").to_string();
    let file_path = path.join(format!("{}_{}.yml", file_prefix, description));
    common::fs_write(
        file_path,
        "# yaml-language-server: $schema=../seed-schema.json\n\n",
    )?;
    Ok(())
}
