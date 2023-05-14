use clap::Parser;
use json::*;
use mongodb::{options::ClientOptions, Client as MongoClient};
use mpd::Client as MPDClient;
use std::fs::File;
use std::io::Read;
use std::path::Path;
mod lib;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about)]
/// Program that tracks mpd listening time on a per artist and per song basis
pub struct Args {
    /// Import database from files
    #[arg(short, long)]
    import: Option<Vec<String>>,

    /// Print the database to stdout
    #[arg(short, long, default_value_t = false)]
    print: bool,

    /// Files to output the database to
    #[arg(short, long)]
    output: Option<Vec<String>>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let config_file_dir_literal =
        dirs::config_dir().unwrap().to_str().unwrap().to_string() + "/mpdtrackr/config.json";
    let config_file_dir = Path::new(&config_file_dir_literal);
    let mut config_file = match File::open(&config_file_dir) {
        Ok(k) => k,
        Err(_) => {
            lib::create_config();
            File::open(&config_file_dir).unwrap()
        }
    };
    let mut config_file_contents = String::new();
    config_file
        .read_to_string(&mut config_file_contents)
        .unwrap();
    let config = match parse(config_file_contents.as_str()) {
        Ok(k) => k,
        Err(_) => {
            lib::create_config();
            match parse(config_file_contents.as_str()) {
                Ok(k) => k,
                Err(e) => {
                    println!("{}", e);
                    panic!("Could not parse config file.")
                }
            }
        }
    };

    let mongo_client_options = ClientOptions::parse(format!(
        "{}{}",
        "mongodb://localhost:", config["mongo_port"]
    ))
    .await
    .unwrap();
    let mongo_client = MongoClient::with_options(mongo_client_options).unwrap();
    let mpd_client = MPDClient::connect(format!("{}{}", "localhost:", config["mpd_port"])).unwrap();
    match args {
        Args {
            import: Some(files),
            output: _,
            print: _,
        } => lib::import(mongo_client, files),
        Args {
            import: _,
            output: Some(files),
            print: _,
        } => lib::output(mongo_client, files),
        Args {
            import: _,
            output: _,
            print: true,
        } => lib::print(mongo_client).await,
        _ => lib::run(mongo_client, mpd_client, config).await,
    }
}
