use std::{
    cmp::Ordering,
    collections::HashMap,
    fs::File,
    path::PathBuf,
    time::Duration,
};

use crate::structs::{Config, DataRow, FieldGroup, NewlineFormatter, PrintArgs, TimeGroup};
use anyhow::{anyhow, Result};
use fs2::FileExt;
use log::{info, warn};
use mpd::{Client, State};
use serde::Serialize;
use serde_json::Serializer;
use sqlx::{QueryBuilder, Sqlite};
use tokio::time::Instant;

pub async fn run(pool: &sqlx::SqlitePool, config: &Config) -> Result<()> {
    // Use file locks to prevent multiple instances running at once since data will be written
    // twice to the database (probably not what you want if you're looking for accurate statistics)
    let lock_file =
        File::create(std::env::temp_dir().join(concat!(env!("CARGO_PKG_NAME"), ".lock")))?;
    lock_file
        .try_lock_exclusive()
        .map_err(|_| {
            concat!(
                "An instance of ",
                env!("CARGO_PKG_NAME"),
                " is already running!"
            )
        })
        .unwrap();
    let mut mpd = Client::connect(format!("{}:{}", config.mpd_url, config.mpd_port))?;

    // Infinite loop to update everything once the playing song changes
    'outer: loop {
        let outer_song = mpd
            .currentsong()?
            .ok_or(anyhow!("Error: no song playing"))?;
        let tags = HashMap::<String, String>::from_iter(outer_song.tags.clone());
        let artist = match outer_song.artist {
            Some(k) => Some(k),
            None => {
                let path = PathBuf::from(&outer_song.file);
                warn!(
                    "No artist found for '{}'. Attempting to parse artist from file name...",
                    path.display()
                );
                // The first part of the file name before the " - " should be the artist name
                // so attempt to use that as the artist name
                path.file_stem()
                    .and_then(|x| x.to_str())
                    .and_then(|x| x.find(" - ").map(|ind| x[..ind].trim().to_string()))
            }
        };
        let duration = outer_song.duration.map(|x| x.as_secs() as u32);
        let artist_id = match sqlx::query!("SELECT * FROM artists WHERE name = $1", artist)
            .fetch_optional(pool)
            .await?
        {
            Some(k) => k.id,
            None => {
                sqlx::query!("INSERT INTO artists VALUES ($1, $2)", None::<u8>, artist)
                    .execute(pool)
                    .await?;
                info!(
                    "Inserting new artist into database: '{}'",
                    artist.unwrap_or_default()
                );
                continue;
            }
        };
        let title = match outer_song.title.clone() {
            Some(k) => Some(k),
            None => {
                let path = PathBuf::from(&outer_song.file);
                warn!(
                    "No title found for '{}'. Attempting to parse title from file name...",
                    path.display()
                );
                // The last part of the file name after the '-' should be the song title
                // so attempt to use that as the title
                path.file_stem()
                    .and_then(|x| x.to_str())
                    .and_then(|x| x.find(" - ").map(|ind| x[ind + 1..].trim().to_string()))
            }
        };

        info!(
            "Tracking stats for: '{} - {}'",
            artist.as_deref().unwrap_or_default(),
            title.as_deref().unwrap_or_default()
        );

        let song_id = match sqlx::query!("SELECT id, title FROM songs WHERE title = $1", title)
            .fetch_optional(pool)
            .await?
        {
            Some(k) => k.id,
            None => {
                let album = tags.get("Album");
                let genre = tags.get("Genre");
                sqlx::query!(
                    "INSERT INTO songs VALUES ($1, $2, $3, $4, $5, $6)",
                    None::<u8>,
                    title,
                    artist_id,
                    album,
                    genre,
                    duration
                )
                .execute(pool)
                .await?;
                info!(
                    "Inserting new song into database: '{}'",
                    title.unwrap_or_default()
                );
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
                // Pause the tracker until music is playing
                State::Pause | State::Stop => tokio::time::sleep(Duration::from_millis(10)).await,
                State::Play => {
                    // Switch songs if the currently playing song is different from the one we have
                    // stats on
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

                        // Sleep to prevent wasted resources and utilize checked
                        // subtraction since it might panic in cases where looping
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

pub async fn print(pool: &sqlx::SqlitePool, command: PrintArgs) -> Result<()> {
    // Convert the Vec of enums into comma separated strings to feed them into the sql query
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
    songs.duration as duration,
    artists.name as artist,
    artists.id as artist_id,
    MIN(listening_times.date) AS first_listened,
    MAX(listening_times.date) AS last_listened,
    SUM(listening_times.playback_time) as time,
    SUM(listening_times.playback_time / songs.duration) as times_listened,
",
    );
    match command
        .time_group
        .as_ref()
        .expect("Default value set by clap")
    {
        TimeGroup::AllTime => builder.push(
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

    // Use one match statement to determine which where clause to use since only one can be used at
    // a time
    builder.push(match (command.after, command.before, command.between) {
        (Some(after), _, _) => format!("WHERE date > '{}' ", after),
        (_, Some(before), _) => format!("WHERE date < '{}' ", before),
        (_, _, Some(between)) => {
            format!("WHERE date BETWEEN '{}' and '{}' ", between[0], between[1])
        }
        (None, None, None) => String::new(),
    });

    builder.push(format!(
        "GROUP BY {} ",
        command
            .field_group
            .as_ref()
            .expect("Default value set by clap")
    ));

    match command
        .time_group
        .as_ref()
        .expect("Default value set by clap")
    {
        TimeGroup::AllTime => (),
        group => {
            builder.push(format!(", strftime('{}', date)", group.format_time()));
        }
    }

    builder.push(format!("ORDER BY {sort_sequence}"));

    // Fetch each entry from the database using the provided query and print to stdout
    let mut query = sqlx::query_as::<_, DataRow>(builder.sql())
        .fetch_all(pool)
        .await?;

    // Hide fields that don't make sense for specific groupings
    // Had to resort to this since the alternative would be to make 4 separate structs
    // with slightly different fields
    match command
        .field_group
        .as_ref()
        .expect("Default value set by clap")
    {
        FieldGroup::Album => {
            for entry in query.iter_mut() {
                entry.duration = None;
                entry.title = None;
                entry.song_id = None;
                entry.times_listened = None;
            }
        }
        FieldGroup::Artist => {
            for entry in query.iter_mut() {
                entry.duration = None;
                entry.title = None;
                entry.song_id = None;
                entry.album = None;
                entry.genre = None;
                entry.times_listened = None;
            }
        }
        FieldGroup::Genre => {
            for entry in query.iter_mut() {
                entry.duration = None;
                entry.title = None;
                entry.song_id = None;
                entry.album = None;
                entry.artist = None;
                entry.artist_id = None;
                entry.times_listened = None;
            }
        }
        FieldGroup::Title => (),
    }

    if command.json {
        // This is safe because I copied most of the logic from serde_json
        println!("{}", unsafe {
            String::from_utf8_unchecked({
                let mut buf = Vec::with_capacity(128);
                let mut ser = Serializer::with_formatter(&mut buf, NewlineFormatter);
                query.serialize(&mut ser)?;
                buf
            })
        });
    } else {
        print!(
            "{}",
            query.iter().fold(String::new(), |acc, x| {
                let mut acc = acc + &x.to_string();
                acc.push('\n');
                acc
            })
        );
    }
    Ok(())
}

pub async fn import(_files: Vec<String>) {
    todo!()
}

pub async fn export(_files: Vec<String>) {
    todo!()
}
