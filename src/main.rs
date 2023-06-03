use clap::Parser;
use json::*;
use mongodb::{options::ClientOptions, Client as MongoClient};
use mpd::Client as MPDClient;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use mpdtrackr::*;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about)]
/// Program that tracks mpd listening time on a per artist and per song basis
/// Note: running with more than one argument at a time will result in only one being executed
pub struct Args {
    /// Import collections from files (ex: artists.json will be imported into into the "artists"
    /// collection)
    /// Files must be in .json format and have the .json extension to be properly imported.
    #[arg(short, long, num_args = 0..)]
    import: Option<Vec<String>>,

    /// Run the tracker while printing logs to stdout
    #[arg(short, long, default_value_t = false)]
    logging: bool,

    /// Print the database to stdout
    #[arg(short, long, default_value_t = false)]
    print: bool,

    /// Directories to output the database to. Output files will be in .json format
    #[arg(short, long, num_args = 0..)]
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
            create_config();
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
            create_config();
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
    let mpd_client = MPDClient::connect(format!("{}{}", "localhost:", config["mpd_port"])).expect("Could not connect to mpd.\nDo you have an active mpd connection?");
    match args {
        Args {
            import: Some(files),
            logging: _,
            output: _,
            print: _,
        } => import(mongo_client, files).await,
        Args {
            import: _,
            logging: _,
            output: Some(files),
            print: _,
        } => output(mongo_client, files).await,
        Args {
            import: _,
            logging: _,
            output: _,
            print: true,
        } => print(mongo_client).await,
        _ => run(mongo_client, mpd_client, config, args.logging).await,
    }
}
