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
    #[arg(short, long)]
    pub days: Option<usize>,
    #[arg(short, long)]
    pub weeks: Option<usize>,
    #[arg(short, long)]
    pub months: Option<usize>,
    #[arg(short, long)]
    pub years: Option<usize>,
    #[arg(short, long)]
    /// Print extra information
    pub verbose: bool,
    #[arg(short, long)]
    /// Output data in json format
    pub json: bool,
    #[arg(short, long)]
    /// Group listening times by given time frame
    pub group: Option<GroupBy>,
    #[arg(short, long, default_value = "time")]
    /// Sort entries by given option
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

// impl Display for GroupDurations{
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self{
//             Self::Day =>
//         }
//     }
// }
//

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

// impl From<String> for GroupDurations {
//     fn from(value: String) -> Self {
//         match value.to_lowercase().as_str() {
//             "-d" | "--day" | "days" => Self::Day,
//             "-w" | "--week" | "weeks" => Self::Week,
//             "-m" | "--month" | "months" => Self::Month,
//             "-y" | "--year" | "years" => Self::Year,
//             _ => Self::AllTime,
//         }
//     }
// }
