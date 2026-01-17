use anyhow::{Result, ensure};
use clap::{Parser, Subcommand};
use dotenvy::dotenv;
use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct AppArg {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand, PartialEq, Clone, Debug)]
enum Command {
    /// Execute database migration
    Migrate {
        #[clap(long)]
        db: Option<String>,
        /// Drop database before migrating
        #[clap(short, long)]
        clean: bool,
        /// Drop database before migrating in release environment
        #[clap(long)]
        force_drop_db: bool,
        /// ignore missing migration error
        #[clap(long)]
        ignore_missing: bool,
        /// Removes entries from _sqlx_migrations for applied migrations missing in the current set.
        /// Note: Does not revert any applied DDL changes.
        #[clap(long)]
        remove_missing: bool,
        /// Use test environment
        #[clap(short, long)]
        test: bool,
    },
    /// Generate a schema for the seed
    /// The seed_schema feature is required.
    GenSeedSchema,
    /// Import the database seed.
    Seed {
        #[clap(long)]
        db: Option<String>,
        /// Use clean migration
        #[clap(short, long)]
        clean: bool,
        /// Drop database before migrating in release environment
        #[clap(long)]
        force_drop_db: bool,
        /// ignore missing migration error
        #[clap(long)]
        ignore_missing: bool,
        #[clap(long)]
        remove_missing: bool,
        /// Use test environment
        #[clap(short, long)]
        test: bool,
    },
    /// Check database tables
    Check {
        /// Use test environment
        #[clap(short, long)]
        test: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    init();

    let arg: AppArg = AppArg::parse();
    match arg.command {
        Command::Migrate {
            db,
            clean,
            force_drop_db,
            ignore_missing,
            remove_missing,
            test,
        } => {
            if clean {
                ensure!(
                    force_drop_db || cfg!(debug_assertions),
                    "clean migrate is debug environment only"
                );
            }
            let local = tokio::task::LocalSet::new();
            local
                .run_until(async move {
                    migrate(
                        db.as_deref(),
                        test,
                        clean || force_drop_db,
                        ignore_missing,
                        remove_missing,
                    )
                    .await
                })
                .await?;
        }
        Command::GenSeedSchema => {
            ensure!(
                cfg!(feature = "seed_schema"),
                "\"--features=seed_schema\" required."
            );
            gen_seed_schema()?;
        }
        Command::Seed {
            db,
            clean,
            force_drop_db,
            ignore_missing,
            remove_missing,
            test,
        } => {
            if clean {
                ensure!(
                    force_drop_db || cfg!(debug_assertions),
                    "clean migrate is debug environment only"
                );
                let local = tokio::task::LocalSet::new();
                let _db = db.clone();
                local
                    .run_until(async move {
                        migrate(
                            _db.as_deref(),
                            test,
                            clean || force_drop_db,
                            ignore_missing,
                            remove_missing,
                        )
                        .await
                    })
                    .await?;
            }
            seed(db.as_deref(), test).await?;
        }
        Command::Check { test } => {
            check(test).await?;
        }
    }
    Ok(())
}

pub fn init() {
    // Do not modify this line. (DbInit)
}

#[rustfmt::skip]
pub async fn migrate(db: Option<&str>, use_test: bool, clean: bool, ignore_missing: bool, remove_missing: bool) -> Result<()> {
    let mut join_set = tokio::task::JoinSet::new();
    // Do not modify this line. (migrate)
    let mut error = None;
    while let Some(res) = join_set.join_next().await {
        if let Err(e) = res? 
            && let Some(e) = error.replace(e) {
                log::error!("{}", e);
            }
    }
    if let Some(e) = error {
        return Err(e);
    }
    Ok(())
}

pub fn gen_seed_schema() -> Result<()> {
    // Do not modify this line. (gen_seed_schema)
    Ok(())
}

pub async fn seed(_db: Option<&str>, _use_test: bool) -> Result<()> {
    // Do not modify this line. (seed)
    Ok(())
}

#[rustfmt::skip]
pub async fn check(use_test: bool) -> Result<()> {
    tokio::try_join!(
        // Do not modify this line. (check)
    )?;
    Ok(())
}
@{-"\n"}@