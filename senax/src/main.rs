use anyhow::{ensure, Context as _, Result};
use chrono::Utc;
use clap::{Parser, Subcommand};
use dotenv::dotenv;
use once_cell::sync::OnceCell;
use regex::Regex;
use schemars::{gen::SchemaSettings, schema::RootSchema};
use serde_json::Value;
use std::fs;
use std::{env, path::PathBuf};

use crate::{schema::SchemaDef, schema_md::gen_schema_md};

pub(crate) mod common;
mod db_document;
pub(crate) mod ddl {
    pub mod parser;
    pub mod table;
}
mod graphql_generator;
mod migration_generator;
mod model_generator;
pub(crate) mod schema;
mod schema_generator;
mod schema_md;

pub static MODELS_PATH: OnceCell<PathBuf> = OnceCell::new();

include!(concat!(env!("OUT_DIR"), "/templates.rs"));

#[derive(Parser)]
#[clap(name = "senax code generator", author, version, about, long_about = None)]
#[clap(propagate_version = true)]
struct Cli {
    #[clap(long, value_parser)]
    dir: Option<PathBuf>,
    #[clap(long, value_parser)]
    base_dir: Option<PathBuf>,
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// generate models
    Model {
        /// Specify the DB
        #[clap(value_parser)]
        db: String,
        /// Force overwrite
        #[clap(short, long)]
        force: bool,
    },
    /// generate graphql api
    Graphql {
        /// Specify the crate path
        #[clap(value_parser)]
        path: PathBuf,
        /// Specify the DB
        #[clap(value_parser)]
        db: String,
        /// Specify the group
        #[clap(value_parser)]
        group: Option<String>,
        /// Specify the model
        #[clap(value_parser)]
        model: Option<String>,
        /// Use camel case
        #[clap(long)]
        camel_case: bool,
        /// With column title
        #[clap(long)]
        with_title: bool,
        /// With column comment
        #[clap(long)]
        with_comment: bool,
        /// Force overwrite
        #[clap(short, long)]
        force: bool,
    },
    /// generate migration ddl
    GenMigrate {
        /// Specify the DB
        #[clap(value_parser)]
        db: String,
        /// Specify description and generate a file
        #[clap(value_parser)]
        description: Option<String>,
        /// If true, creates a pair of up and down migration files
        #[clap(short, long)]
        revert: bool,
    },
    /// generate a import data file
    GenSeed {
        /// Specify the db
        #[clap(value_parser)]
        db: String,
        /// Specify description and output to file
        #[clap(value_parser)]
        description: String,
    },
    /// generate a DB document file
    GenDbDoc {
        /// Specify the db
        #[clap(value_parser)]
        db: String,
        /// Specify the group
        #[clap(value_parser)]
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
    /// generate a schema yml file from DB
    GenSchema {
        /// Specify the DB uri
        #[clap(long, value_parser, value_name = "DB_URI")]
        uri: String,
    },
    GenConfSchema {
        #[clap(short, long)]
        doc: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    if let Some(ref base_dir) = cli.base_dir {
        env::set_current_dir(base_dir)
            .with_context(|| format!("directory error!: {}", base_dir.to_string_lossy()))?;
    }
    MODELS_PATH
        .set(cli.dir.as_ref().cloned().unwrap_or("./db".parse()?))
        .unwrap();
    dotenv().ok();

    let result = exec(cli).await;
    if let Err(err) = result {
        if cfg!(debug_assertions) {
            eprintln!("{:?}", err);
        } else {
            eprintln!("{}", err);
        }
        std::process::exit(1);
    }
    Ok(())
}

async fn exec(cli: Cli) -> Result<(), anyhow::Error> {
    match &cli.command {
        Commands::Model { db, force } => {
            let re = Regex::new(r"^[_a-zA-Z0-9]+$").unwrap();
            ensure!(re.is_match(db), "bad db name!");
            model_generator::generate(db, *force)?;
        }
        Commands::Graphql {
            path,
            db,
            group,
            model,
            camel_case,
            force,
            with_title,
            with_comment,
        } => {
            let re = Regex::new(r"^[_a-zA-Z0-9]+$").unwrap();
            ensure!(re.is_match(db), "bad db name!");
            model_generator::template::filters::SHOW_TITLE
                .store(*with_title, std::sync::atomic::Ordering::SeqCst);
            model_generator::template::filters::SHOW_COMMNET
                .store(*with_comment, std::sync::atomic::Ordering::SeqCst);
            graphql_generator::generate(path, db, group, model, *camel_case, *force)?;
        }
        Commands::GenMigrate {
            db,
            description,
            revert,
        } => {
            let re = Regex::new(r"^[_a-zA-Z0-9]+$").unwrap();
            ensure!(re.is_match(db), "bad db name!");
            migration_generator::generate(db, description, *revert).await?;
        }
        Commands::GenSeed { db, description } => {
            let re = Regex::new(r"^[_a-zA-Z0-9]+$").unwrap();
            ensure!(re.is_match(db), "bad db name!");
            generate_seed_file(db, description)?;
        }
        Commands::GenDbDoc {
            db,
            group,
            er,
            history,
            output,
            template,
        } => {
            let re = Regex::new(r"^[_a-zA-Z0-9]+$").unwrap();
            ensure!(re.is_match(db), "bad db name!");
            db_document::generate(db, group, *er, history, output, template)?;
        }
        Commands::GenSchema { uri } => {
            schema_generator::generate(uri).await?;
        }
        Commands::GenConfSchema { doc } => {
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
                let settings = SchemaSettings::draft07().with(|s| {
                    s.option_nullable = false;
                    s.option_add_null_type = true;
                });
                let gen = settings.into_generator();
                let schema = gen.into_root_schema_for::<SchemaDef>();
                let schema = serde_json::to_string(&schema)?;
                let schema = schema.replace(r#""additionalProperties":{"#,
                    r#""propertyNames":{"pattern":"^\\p{XID_Start}\\p{XID_Continue}*$"},"additionalProperties":{"#);
                let schema = schema.replace(r#""conf":{"default":{},"type":"object","propertyNames":{"pattern":"^\\p{XID_Start}\\p{XID_Continue}*$"}"#,
                    r#""conf":{"default":{},"type":"object","propertyNames":{"pattern":"^[A-Za-z][0-9A-Z_a-z]*$"}"#);
                let schema = schema.replace(r#""groups":{"type":"object","propertyNames":{"pattern":"^\\p{XID_Start}\\p{XID_Continue}*$"}"#,
                    r#""groups":{"type":"object","propertyNames":{"pattern":"^[A-Za-z][0-9A-Z_a-z]*$"}"#);
                let schema: RootSchema = serde_json::from_str(&schema)?;
                let schema = serde_json::to_string_pretty(&schema)?;
                println!("{}", schema);
            }
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
    let path = MODELS_PATH.get().unwrap().join(db).join("seeds");
    fs::create_dir_all(&path)?;
    let dt = Utc::now();
    let file_prefix = dt.format("%Y%m%d%H%M%S").to_string();
    let file_path = path.join(format!("{}_{}.yml", file_prefix, description));
    println!("{}", file_path.display());
    fs::write(
        file_path,
        "# yaml-language-server: $schema=../seed-schema.json\n\n",
    )?;
    Ok(())
}
