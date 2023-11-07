use std::{
    cmp::Ordering,
    collections::HashMap,
    fmt::format,
    fs::{create_dir_all, File},
    path::PathBuf,
    time::Duration,
};

use anyhow::{anyhow, Result};
use clap::Parser;
use fs2::FileExt;
use futures::{StreamExt, TryStreamExt};
use mpd::{Client, State};
use mpdtrackr::structs::{Args, DataRow, GroupBy, PrintArgs, SubCommand};
use sqlx::{Execute, QueryBuilder, Sqlite, SqlitePool};
use tokio::time::Instant;

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
        SubCommand::Print(args) => print(&pool, args).await?,
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
                let album = tags.get("Album");
                let genre = tags.get("Genre");
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
                                sqlx::query!("UPDATE listening_times SET playback_time = playback_time + 1 WHERE song_id = $1 AND date = $2", song_id, date)
                                    .execute(pool)
                                    .await?;
                                old_time = current_time
                            }
                            Ordering::Greater => old_time = current_time,
                            Ordering::Equal => (),
                        }

                        // Checked subtraction since it might panic in cases where looping
                        // again takes longer than one second
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

async fn print(pool: &sqlx::SqlitePool, command: PrintArgs) -> Result<()> {
    let sort_sequence = command
        .sort
        .iter()
        .map(|x| x.to_string())
        .reduce(|acc, x| acc + "," + &x)
        .unwrap_or_default();
    let mut builder: QueryBuilder<Sqlite> = sqlx::QueryBuilder::new(
        "
SELECT
    songs.title as title,
    songs.album as album,
    songs.genre as genre,
    songs.id as song_id,
    artists.name as artist,
    artists.id as artist_id,
    MIN(listening_times.date) AS first_listened,
    MAX(listening_times.date) AS last_listened,
    SUM(listening_times.playback_time) as time,
",
    );
    let new_str = match command.group.as_ref().expect("Default value set by clap") {
        GroupBy::AllTime => builder.push(
            "
listening_times.date as date
FROM songs
INNER JOIN listening_times
ON songs.id = listening_times.song_id 
INNER JOIN artists 
ON artists.id = songs.artist_id
",
        ),
        group => builder.push(format!(
            "
strftime('{group}', listening_times.date) AS date
FROM songs
INNER JOIN listening_times
ON songs.id = listening_times.song_id 
INNER JOIN artists 
ON artists.id = songs.artist_id
"
        )),
    };
    // query_str += &new_str;

    builder.push(match (command.after, command.before, command.between) {
        (Some(after), _, _) => format!("WHERE date > '{}' ", after),
        (_, Some(before), _) => format!("WHERE date < '{}' ", before),
        (_, _, Some(between)) => {
            format!("WHERE date BETWEEN '{}' and '{}' ", between[0], between[1])
        }
        (None, None, None) => String::new(),
    });

    // query_str += &range;

    builder.push(
        match command.group.as_ref().expect("Default value set by clap") {
            GroupBy::AllTime => format!("GROUP BY song_id ORDER BY {sort_sequence}"),
            group => format!(
                "GROUP BY song_id, strftime('{}', date) ORDER BY {sort_sequence}",
                group.format_time()
            ),
        },
    );

    // query_str += &new_str;

    // println!("{}", &query_str);
    let mut query = sqlx::query_as::<_, DataRow>(builder.sql()).fetch(pool);
    while let Some(entry) = query.next().await {
        let entry = entry?;
        if command.json {
            println!("{}", serde_json::to_string(&entry)?);
        } else {
            println!("{}", &entry);
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
