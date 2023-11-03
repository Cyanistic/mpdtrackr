use std::{
    cmp::Ordering,
    collections::HashMap,
    fs::{create_dir_all, File},
    path::PathBuf,
    sync::Mutex,
    time::Duration,
};

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use fs2::FileExt;
use futures::StreamExt;
use mpd::{Client, State};
use tokio::time::Instant;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    subcommand: SubCommand,
}

#[derive(Subcommand, Debug)]
enum SubCommand {
    Run,
    Export {
        #[arg(short, long)]
        files: Vec<String>,
    },
    Import {
        #[arg(short, long)]
        files: Vec<String>,
    },
    Print {
        #[arg(short, long)]
        days: Option<usize>,
        #[arg(short, long)]
        weeks: Option<usize>,
        #[arg(short, long)]
        months: Option<usize>,
        #[arg(short, long)]
        years: Option<usize>,
        #[arg(short, long)]
        verbose: bool,
        #[arg(short, long)]
        json: bool,
    },
}

const PKG_NAME: &str = env!("CARGO_PKG_NAME");

#[tokio::main]
async fn main() -> Result<()> {
    std::fs::create_dir_all(dirs::config_dir().unwrap().join(PKG_NAME))?;
    let db_file = dirs::data_dir()
        .unwrap()
        .join(PKG_NAME)
        .join(concat!(env!("CARGO_PKG_NAME"), ".db"));
    if !db_file.exists() {
        create_dir_all(db_file.parent().unwrap())?;
        File::create(&db_file)?;
    }
    let args = Args::parse();
    let pool = sqlx::sqlite::SqlitePool::connect_lazy(&format!("sqlite://{}", db_file.display()))?;

    // create sqlite tables
    sqlx::query(
        r#"
CREATE TABLE IF NOT EXISTS artists (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 name TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS songs (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 title TEXT NOT NULL,
 artist_id INTEGER,
 album TEXT,
 genre TEXT,
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
            let _ = run(&pool).await.map_err(|e| eprintln!("Error: {}", e));
            std::thread::sleep(Duration::from_secs(1));
        },
        SubCommand::Print {
            days,
            weeks,
            months,
            years,
            verbose,
            json,
        } => {
            print(
                &pool,
                [days, weeks, months, years]
                    .into_iter()
                    .enumerate()
                    .map(|(k, v)| {
                        v.unwrap_or(0)
                            * match k {
                                0 => 1,
                                1 => 7,
                                2 => 30,
                                3 => 365,
                                _ => 0,
                            }
                    })
                    .sum(),
                verbose,
                json,
            )
            .await?
        }
        SubCommand::Export { files } => export(files).await,
        SubCommand::Import { files } => import(files).await,
    }
    Ok(())
}

async fn run(pool: &sqlx::SqlitePool) -> Result<()> {
    let lock_file =
        File::create(std::env::temp_dir().join(concat!(env!("CARGO_PKG_NAME"), ".lock")))?;
    lock_file.try_lock_exclusive().expect(concat!(
        "An instance of ",
        env!("CARGO_PKG_NAME"),
        " is already running!"
    ));
    let mut mpd = Client::connect(format!("127.0.0.1:{}", 6600))?;
    'outer: loop {
        let outer_song = mpd
            .currentsong()?
            .ok_or(anyhow!("Error: no song playing"))?;
        let tags = HashMap::<String, String>::from_iter(outer_song.tags.clone());
        let artist = match outer_song.artist {
            Some(k) => Some(k),
            None => {
                let path = PathBuf::from(&outer_song.file);
                path.file_stem()
                    .and_then(|x| x.to_str())
                    .and_then(|x| x.find('-').map(|ind| x[..ind].trim().to_string()))
            }
        };
        let artist_id = match sqlx::query!("SELECT * FROM artists WHERE name = $1", artist)
            .fetch_optional(pool)
            .await?
        {
            Some(k) => k.id,
            None => {
                sqlx::query!("INSERT INTO artists VALUES ($1, $2)", None::<u8>, artist)
                    .execute(pool)
                    .await?;
                continue;
            }
        };
        let title = match outer_song.title.clone() {
            Some(k) => Some(k),
            None => {
                let path = PathBuf::from(&outer_song.file);
                path.file_stem()
                    .and_then(|x| x.to_str())
                    .and_then(|x| x.find('-').map(|ind| x[ind + 1..].trim().to_string()))
            }
        };
        let song_id = match sqlx::query!("SELECT id, title FROM songs WHERE title = $1", title)
            .fetch_optional(pool)
            .await?
        {
            Some(k) => k.id,
            None => {
                let album = tags.get("Album").unwrap_or(&String::new()).clone();
                let genre = tags.get("Genre").unwrap_or(&String::new()).clone();
                sqlx::query!(
                    "INSERT INTO songs VALUES ($1, $2, $3, $4, $5)",
                    None::<u8>,
                    title,
                    artist_id,
                    album,
                    genre
                )
                .execute(pool)
                .await?;
                continue;
            }
        };
        let date = chrono::Local::now().date_naive();
        if sqlx::query!(
            "SELECT * from listening_times where date = $1 and song_id = $2",
            date,
            song_id
        )
        .fetch_optional(pool)
        .await?
        .is_none()
        {
            sqlx::query!(
                "INSERT INTO listening_times VALUES ($1, $2, $3, $4)",
                None::<u8>,
                date,
                song_id,
                0
            )
            .execute(pool)
            .await?;
        }

        let mut current_time = mpd
            .status()?
            .time
            .ok_or(anyhow!("No time found on playing song"))?
            .0;
        let mut old_time = current_time;
        'inner: loop {
            let now = Instant::now();
            match mpd.status()?.state {
                State::Pause | State::Stop => tokio::time::sleep(Duration::from_millis(10)).await,
                State::Play => {
                    let inner_song = match mpd.currentsong()? {
                        Some(k) => k,
                        None => continue 'inner,
                    };
                    if outer_song.title == inner_song.title {
                        current_time = mpd
                            .status()?
                            .time
                            .ok_or(anyhow!("No time found on playing song"))?
                            .0;
                        match old_time.cmp(&current_time) {
                            Ordering::Less => {
                                sqlx::query!("UPDATE listening_times SET playback_time = playback_time + 1 WHERE song_id = $1 AND date = $2", song_id, date).execute(pool).await?;
                                old_time = current_time
                            }
                            Ordering::Greater => old_time = current_time,
                            Ordering::Equal => (),
                        }
                        tokio::time::sleep(
                            Duration::from_secs(1)
                                .checked_sub(now.elapsed())
                                .unwrap_or_default(),
                        )
                        .await
                    } else {
                        continue 'outer;
                    }
                }
            }
        }
    }
}

async fn print(pool: &sqlx::SqlitePool, days: usize, verbose: bool, json: bool) -> Result<()> {
    match verbose {
        true => {
            let mut query = sqlx::query!("SELECT songs.*, artists.name AS artist, SUM(listening_times.playback_time) AS playback_time FROM songs INNER JOIN listening_times ON songs.id = listening_times.song_id INNER JOIN artists ON artists.id = songs.artist_id GROUP BY songs.id;").fetch(pool);
            while let Some(entry) = query.next().await {
                let entry = entry?;
                let playback_time = format!(
                    "{}h{}m{}s",
                    &entry.playback_time / 3600,
                    (&entry.playback_time % 3600) / 60,
                    (&entry.playback_time % 60)
                );
                println!("{:?} {}", entry, playback_time);
            }
        }
        false => {
            let mut query = sqlx::query!("SELECT songs.title, artists.name AS artist, SUM(listening_times.playback_time) AS playback_time FROM songs INNER JOIN listening_times ON songs.id = listening_times.song_id INNER JOIN artists ON artists.id = songs.artist_id GROUP BY songs.id;").fetch(pool);
            while let Some(entry) = query.next().await {
                let entry = entry?;
                let playback_time = format!(
                    "{}h{}m{}s",
                    &entry.playback_time / 3600,
                    (&entry.playback_time % 3600) / 60,
                    (&entry.playback_time % 60)
                );
                println!("{:?} {}", entry, playback_time);
            }
        }
    }
    Ok(())
}

async fn import(files: Vec<String>) {
    todo!()
}

async fn export(files: Vec<String>) {
    todo!()
}
