use std::{
    env,
    fs::{create_dir_all, File},
    time::Duration,
};

use anyhow::Result;
use clap::Parser;
use log::error;
use mpdtrackr::{
    structs::{Args, Config, SubCommand},
    utils::*,
};

const PKG_NAME: &str = env!("CARGO_PKG_NAME");

#[tokio::main]
async fn main() -> Result<()> {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info")
    }
    pretty_env_logger::init();
    let db_file = dirs::data_dir()
        .unwrap()
        .join(PKG_NAME)
        .join(concat!(env!("CARGO_PKG_NAME"), ".db"));
    if !db_file.is_file() {
        create_dir_all(db_file.parent().expect("DB file should have parent dir"))?;
        File::create(&db_file)?;
    }

    let args = Args::parse();
    let pool = sqlx::sqlite::SqlitePool::connect_lazy(&format!("sqlite://{}", db_file.display()))?;
    let config: Config = Config::from_config_file().unwrap_or_else(|e| {
        error!(
            "Error parsing config file: '{}'. Falling back to default config",
            e
        );
        Config::new()
    });

    // create sqlite tables
    sqlx::query(
        r#"
CREATE TABLE IF NOT EXISTS artists (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 name TEXT NOT NULL COLLATE NOCASE
);
CREATE TABLE IF NOT EXISTS songs (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 title TEXT NOT NULL COLLATE NOCASE,
 artist_id INTEGER,
 album TEXT COLLATE NOCASE,
 genre TEXT COLLATE NOCASE,
 duration INTEGER,
 FOREIGN KEY (artist_id)
    REFERENCES artists (id) 
    ON UPDATE CASCADE
    ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS listening_times (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 date DATE,
 song_id INTEGER,
 playback_time INTEGER NOT NULL,
 FOREIGN KEY (song_id)
    REFERENCES songs (id) 
    ON UPDATE SET NULL
    ON DELETE SET NULL
);"#,
    )
    .execute(&pool)
    .await?;

    match args.subcommand {
        SubCommand::Run => loop {
            error!("{:?}", run(&pool, &config).await);
            std::thread::sleep(Duration::from_secs(1));
        },
        SubCommand::Print(args) => print(&pool, args).await?,
        // SubCommand::Export { files } => export(files).await,
        // SubCommand::Import { files } => import(files).await,
    }
    Ok(())
}
