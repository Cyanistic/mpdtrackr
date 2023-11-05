use std::fmt::Display;

use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;
use sqlx::FromRow;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub subcommand: SubCommand,
}

#[derive(Subcommand, Debug)]
pub enum SubCommand {
    /// Run the mpdtrackr daemon.
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
                SortBy::Time => "listening_times.playback_time",
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

#[derive(FromRow, Debug, Serialize)]
pub struct DataRow {
    pub artist_id: u32,
    pub song_id: u32,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub time: u32,
    pub first_listened: chrono::NaiveDate,
    pub last_listened: chrono::NaiveDate,
    pub date: String,
}

impl Display for DataRow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            r#"Artist: "{}", Title: "{}", Time: {}, Album: "{}", Genre: "{}", Date: {}, First Listened: {}, Last Listened: {}"#,
            self.artist,
            self.title,
            self.time,
            self.album.as_ref().unwrap_or(&String::new()),
            self.album.as_ref().unwrap_or(&String::new()),
            self.date,
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
