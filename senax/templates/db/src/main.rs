use anyhow::{ensure, Result};
use clap::{Parser, Subcommand};
use dotenvy::dotenv;
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
        /// Drop DB before migrating in release environment
        #[clap(long)]
        force_delete_all_db: bool,
        /// Use test environment
        #[clap(short, long)]
        test: bool,
    },
    GenSeedSchema,
    Seed {
        /// Seed file name or path
        file_name: Option<PathBuf>,
        /// Use clean migration
        #[clap(short, long)]
        clean: bool,
        /// Drop DB before migrating in release environment
        #[clap(long)]
        force_delete_all_db: bool,
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

    #[cfg(feature = "etcd")]
    senax_common::etcd::init().await?;

    let arg: AppArg = AppArg::parse();
    match arg.command {
        Command::Migrate {
            clean,
            test,
            force_delete_all_db,
        } => {
            if clean {
                ensure!(
                    force_delete_all_db || cfg!(debug_assertions),
                    "clean migrate is debug environment only"
                );
            }
            db_@{ db|snake }@::migrate(test, clean || force_delete_all_db, false).await?;
        }
        Command::GenSeedSchema => {
            gen_seed_schema()?;
        }
        Command::Seed {
            file_name,
            clean,
            test,
            force_delete_all_db,
        } => {
            if clean {
                ensure!(
                    force_delete_all_db || cfg!(debug_assertions),
                    "clean migrate is debug environment only"
                );
                db_@{ db|snake }@::migrate(test, clean || force_delete_all_db, false).await?;
            }
            db_@{ db|snake }@::seeder::seed(test, file_name).await?;
        }
        Command::Check { test } => {
            db_@{ db|snake }@::check(test).await?;
        }
    }
    Ok(())
}

pub fn gen_seed_schema() -> Result<()> {
    db_@{ db|snake }@::seeder::gen_seed_schema()?;
    Ok(())
}
@{-"\n"}@