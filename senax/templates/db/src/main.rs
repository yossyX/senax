use anyhow::{ensure, Result};
use clap::{Parser, Subcommand};
use dotenvy::dotenv;

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
    #[cfg(feature = "seeder")]
    GenSeedSchema,
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
        #[cfg(feature = "seeder")]
        Command::GenSeedSchema => {
            gen_seed_schema()?;
        }
    }
    Ok(())
}

#[cfg(feature = "seeder")]
pub fn gen_seed_schema() -> Result<()> {
    db_@{ db|snake }@::seeder::gen_seed_schema()?;
    Ok(())
}
@{-"\n"}@