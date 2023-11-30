use std::{
    fmt::Display,
    fs::File,
    io::{self, Write},
};

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};
use serde_json::ser::Formatter;
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
    // /// Export data from the database
    // Export {
    //     #[arg(short, long)]
    //     files: Vec<String>,
    // },
    // /// Import data into the database
    // Import {
    //     #[arg(short, long)]
    //     files: Vec<String>,
    // },
    /// Print listening stats to stdout with formatting options
    Print(PrintArgs),
}

#[derive(Debug, ValueEnum, Clone)]
pub enum TimeGroup {
    Day,
    Week,
    Month,
    Year,
    AllTime,
}

#[derive(Debug, ValueEnum, Clone)]
pub enum FieldGroup {
    Album,
    Artist,
    Genre,
    Title,
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
    #[arg(short = 'g', long, default_value = "all-time")]
    pub time_group: Option<TimeGroup>,
    /// Group listening times given field
    #[arg(short = 'G', long, default_value = "title")]
    pub field_group: Option<FieldGroup>,
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

impl Display for FieldGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                FieldGroup::Album => "songs.album",
                FieldGroup::Artist => "artists.name",
                FieldGroup::Title => "songs.title",
                FieldGroup::Genre => "songs.genre",
            }
        )
    }
}

impl Default for FieldGroup {
    fn default() -> Self {
        Self::Title
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
    pub mpd_port: u16,
}

impl Config {
    pub fn from_config_file() -> Result<Self> {
        let config = dirs::config_dir()
            .ok_or(anyhow!("No config directory found!"))?
            .join(env!("CARGO_PKG_NAME"))
            .join(concat!(env!("CARGO_PKG_NAME"), "-config.json"));
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
        // default settings for most mpd users
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artist_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub song_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artist: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub album: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub genre: Option<String>,
    pub time: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<u32>,
    pub first_listened: chrono::NaiveDate,
    pub last_listened: chrono::NaiveDate,
    #[serde(skip_serializing_if = "Option::is_none")]
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

        // Don't display nullable fields if they are null
        write!(
            f,
            r#"{}{}{}{}{}Listening Time: {}, Date: {}, {}First Listened: {}, Last Listened: {}"#,
            match &self.artist {
                Some(k) => format!(r#"Artist: "{}", "#, k),
                None => String::new(),
            },
            match &self.title {
                Some(k) => format!(r#"Title: "{}", "#, k),
                None => String::new(),
            },
            match &self.duration {
                Some(k) => format!(r#"Duration: "{}", "#, k),
                None => String::new(),
            },
            match &self.album {
                Some(k) => format!(r#"Album: "{}", "#, k),
                None => String::new(),
            },
            match &self.genre {
                Some(k) => format!(r#"Genre: "{}", "#, k),
                None => String::new(),
            },
            time,
            self.date,
            match &self.times_listened {
                Some(k) => format!(r#"Times Listened: {}, "#, k),
                None => String::new(),
            },
            self.first_listened,
            self.last_listened
        )
    }
}

impl TimeGroup {
    pub fn format_time(&self) -> String {
        match self {
            TimeGroup::Day => "%Y-%m-%d",
            TimeGroup::Week => "%W",
            TimeGroup::Month => "%m",
            TimeGroup::Year => "%Y",
            TimeGroup::AllTime => unreachable!(),
        }
        .to_string()
    }
}

impl Display for TimeGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                TimeGroup::Day => "%Y-%m-%d",
                TimeGroup::Week => "%Y-%W",
                TimeGroup::Month => "%Y-%m",
                TimeGroup::Year => "%Y",
                TimeGroup::AllTime => unreachable!(),
            }
        )
    }
}

impl Default for TimeGroup {
    fn default() -> Self {
        Self::AllTime
    }
}

#[derive(Clone, Debug)]
pub struct NewlineFormatter;

impl Formatter for NewlineFormatter {
    /// Called before every array value.  Writes a `,` if needed to
    /// the specified writer.
    #[inline]
    fn begin_array_value<W>(&mut self, writer: &mut W, first: bool) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        if first {
            writer.write_all(b"\n")
        } else {
            writer.write_all(b",\n")
        }
    }

    #[inline]
    fn end_array<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        writer.write_all(b"\n]")
    }
}
