use anyhow::{ensure, Result};
use clap::{Parser, Subcommand};
use dotenv::dotenv;
use std::path::PathBuf;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct AppArg {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand, PartialEq, Clone, Debug)]
enum Command {
    Migrate {
        /// Drop DB before migrating
        #[clap(short, long)]
        clean: bool,
        /// Use test environment
        #[clap(short, long)]
        test: bool,
    },
    GenSeedSchema,
    Seed {
        /// Seed file name or path
        #[clap(value_parser)]
        file_name: Option<PathBuf>,
        /// Use clean migration
        #[clap(short, long)]
        clean: bool,
        /// Use test environment
        #[clap(short, long)]
        test: bool,
    },
    Check {
        /// Use test environment
        #[clap(short, long)]
        test: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let arg: AppArg = AppArg::parse();
    match arg.command {
        Command::Migrate { clean, test } => {
            if clean {
                ensure!(
                    cfg!(debug_assertions),
                    "clean migrate is debug environment only"
                );
            }
            migrate(test, clean).await?;
        }
        Command::GenSeedSchema => {
            gen_seed_schema()?;
        }
        Command::Seed {
            file_name,
            clean,
            test,
        } => {
            if clean {
                ensure!(
                    cfg!(debug_assertions),
                    "clean migrate is debug environment only"
                );
                migrate(test, clean).await?;
            }
            seed(test, file_name).await?;
        }
        Command::Check { test } => {
            check(test).await?;
        }
    }
    Ok(())
}

#[rustfmt::skip]
pub async fn migrate(use_test: bool, clean: bool) -> Result<()> {
    tokio::try_join!(
        db_@{ db }@::migrate(use_test, clean),
    )?;
    Ok(())
}

pub fn gen_seed_schema() -> Result<()> {
    let schema = db_@{ db }@::loader::gen_seed_schema()?;
    println!("{}", schema);
    Ok(())
}

pub async fn seed(use_test: bool, file_name: Option<PathBuf>) -> Result<()> {
    db_@{ db }@::loader::seed(use_test, file_name).await
}

#[rustfmt::skip]
pub async fn check(use_test: bool) -> Result<()> {
    tokio::try_join!(
        db_@{ db }@::check(use_test),
    )?;
    Ok(())
}
