use std::{fmt::Display, fs::File, io::Write};

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub subcommand: SubCommand,
}

#[derive(Subcommand, Debug)]
pub enum SubCommand {
    /// Run the daemon.
    Run,
    /// Export data from the database
    Export {
        #[arg(short, long)]
        files: Vec<String>,
    },
    /// Import data into the database
    Import {
        #[arg(short, long)]
        files: Vec<String>,
    },
    /// Print listening stats to stdout with formatting options
    Print(PrintArgs),
}

#[derive(Debug, ValueEnum, Clone)]
pub enum GroupBy {
    Day,
    Week,
    Month,
    Year,
    AllTime,
}

#[derive(Debug, ValueEnum, Clone)]
pub enum SortBy {
    /// Album name
    Album,
    /// Artist name
    Artist,
    /// Song title
    Title,
    /// Song genre
    Genre,
    /// Time listened
    Time,
    /// Most recently listened
    Recent,
}

#[derive(Debug, Parser)]
pub struct PrintArgs {
    /// Output data in json format
    #[arg(short, long)]
    pub json: bool,
    /// Only print stats between a start DATE and end DATE
    /// Dates should be in Y-M-D format
    // A tuple would be better for this but that doesn't work in clap yet
    #[arg(
        short = 'B',
        long,
        number_of_values = 2,
        group = "range",
        value_name("DATE")
    )]
    pub between: Option<Vec<chrono::NaiveDate>>,
    /// Only print stats before DATE
    #[arg(short = 'b', long, group = "range", value_name("DATE"))]
    pub before: Option<chrono::NaiveDate>,
    /// Only print stats after DATE
    #[arg(short = 'a', long, group = "range", value_name("DATE"))]
    pub after: Option<chrono::NaiveDate>,
    /// Group listening times by given time frame
    #[arg(short, long, default_value = "all-time")]
    pub group: Option<GroupBy>,
    /// Sort entries by given option
    #[arg(short, long, default_value = "time")]
    pub sort: Vec<SortBy>,
}

impl Display for SortBy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                SortBy::Album => "songs.album",
                SortBy::Artist => "artists.name",
                SortBy::Title => "songs.title",
                SortBy::Genre => "songs.genre",
                SortBy::Time => "time",
                SortBy::Recent => "listening_times.date",
            }
        )
    }
}

impl Default for SortBy {
    fn default() -> Self {
        Self::Time
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub mpd_url: Box<str>,
    pub mpd_port: usize,
}

impl Config {
    pub fn from_config_file() -> Result<Self> {
        let config = dirs::config_dir()
            .ok_or(anyhow!("No config directory found!"))?
            .join(env!("CARGO_PKG_NAME"))
            .join("mpdtrackr-config.json");
        if !config.is_file() {
            std::fs::create_dir_all(config.parent().expect("Config file should have parent dir"))?;
            write!(
                File::create(&config)?,
                "{}",
                serde_json::to_string_pretty(&Config::new())?
            )?;
        }
        Ok(serde_json::from_str::<Config>(&std::fs::read_to_string(
            config,
        )?)?)
    }

    pub fn new() -> Self {
        Config {
            mpd_url: "127.0.0.1".into(),
            mpd_port: 6600,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(FromRow, Debug, Serialize)]
pub struct DataRow {
    pub artist_id: u32,
    pub song_id: u32,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub time: u32,
    pub duration: Option<u32>,
    pub first_listened: chrono::NaiveDate,
    pub last_listened: chrono::NaiveDate,
    pub times_listened: Option<u32>,
    pub date: String,
}

impl Display for DataRow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let time = {
            format!(
                "{}h{}m{}s",
                (self.time / 3600),
                (self.time % 3600) / 60,
                self.time % 60
            )
        };
        write!(
            f,
            r#"Artist: "{}", Title: "{}", Duration: {}, Listening Time: {}, Album: "{}", Genre: "{}", Date: {}, Times Listened: {}, First Listened: {}, Last Listened: {}"#,
            self.artist,
            self.title,
            self.duration.unwrap_or_default(),
            time,
            self.album.as_deref().unwrap_or_default(),
            self.genre.as_deref().unwrap_or_default(),
            self.date,
            self.times_listened.unwrap_or_default(),
            self.first_listened,
            self.last_listened
        )
    }
}

impl GroupBy {
    pub fn format_time(&self) -> String {
        match self {
            GroupBy::Day => "%Y-%m-%d",
            GroupBy::Week => "%W",
            GroupBy::Month => "%m",
            GroupBy::Year => "%Y",
            GroupBy::AllTime => unreachable!(),
        }
        .to_string()
    }
}

impl Display for GroupBy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                GroupBy::Day => "%Y-%m-%d",
                GroupBy::Week => "%Y-%W",
                GroupBy::Month => "%Y-%m",
                GroupBy::Year => "%Y",
                GroupBy::AllTime => unreachable!(),
            }
        )
    }
}

impl Default for GroupBy {
    fn default() -> Self {
        Self::AllTime
    }
}
