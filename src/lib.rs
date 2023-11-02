use futures::stream::TryStreamExt;
use json::*;
use mongodb::bson::{doc, Document};
use mongodb::{options::FindOptions, Client as MongoClient};
use mpd::Client as MPDClient;
use std::fs::{File, create_dir_all};
use std::io::Read;
use std::path::Path;
use std::thread::sleep;
use std::{
    io::Write,
    time::{Duration, Instant},
};
use anyhow::{Result, anyhow};

pub fn create_config() -> Result<()>{
    let config_dir = dirs::config_dir().unwrap().join("mpdtrackr");
    let config_file_path = config_dir.join("config.json");
    if !config_dir.is_dir() {
        create_dir_all(&config_dir)?;
    }
    let mut config = match config_file_path.is_file() {
        false => File::create(config_file_path)?,
        true => File::options()
            .read(true)
            .write(true)
            .open(config_file_path)
            ?,
    };
    config
        .write_all("{\n \"mongo_port\": 27017,\n \"mpd_port\": 6600\n}".as_bytes()).map_err(|e| anyhow!(e))
}

pub async fn import(mongo_client: MongoClient, files: Vec<String>) {
    let db = mongo_client.database("mpdtrackr");
    for i in files {
        let path = Path::new(&i);
        let contents = std::fs::read_to_string(&path).unwrap();
        let json: JsonValue = json::parse(&contents).unwrap();
        let collection_name = path
            .file_name()
            .expect("Could not get file name")
            .to_str()
            .expect("Could not convert file name to string")
            .replace(".json", "");
        let collection = db.collection::<Document>(&collection_name);
        for i in json.members() {
            if collection_name == "artists" {
                match collection
                    .find_one(doc! { "artist": i["artist"].as_str() }, None)
                    .await
                    .unwrap()
                {
                    Some(_) => {
                        collection
                            .update_one(
                                doc! {"artist": i["artist"].as_str().unwrap()},
                                doc! { "$inc": { "time": i["time"].as_i32().unwrap()  }},
                                None,
                            )
                            .await
                            .unwrap();
                    }
                    None => {
                        collection.insert_one(doc! {"artist": i["artist"].as_str().unwrap(), "artist": i["artist"].as_str().unwrap(), "time": i["time"].as_i32().unwrap()}, None).await.unwrap();
                    }
                }
            } else {
                match collection
                    .find_one(doc! { "title": i["title"].as_str() }, None)
                    .await
                    .unwrap()
                {
                    Some(_) => {
                        collection
                            .update_one(
                                doc! {"title": i["title"].as_str().unwrap()},
                                doc! { "$inc": { "time": i["time"].as_i32().unwrap() }},
                                None,
                            )
                            .await
                            .unwrap();
                    }
                    None => {
                        collection.insert_one(doc! {"title": i["title"].as_str().unwrap(), "artist": i["artist"].as_str().unwrap(), "time": i["time"].as_i32().unwrap()}, None).await.unwrap();
                    }
                }
            }
        }
    }
}

pub async fn output(mongo_client: MongoClient, dirs: Vec<String>) {
    let db = mongo_client.database("mpdtrackr");
    let find_options = FindOptions::builder()
        .sort(doc! {"time": -1})
        .projection(doc! {"_id": 0})
        .build();
    for i in dirs {
        for collection_name in db.list_collection_names(None).await.unwrap() {
            let mut cursor = db
                .collection::<Document>(collection_name.as_str())
                .find(None, find_options.clone())
                .await
                .unwrap();
            let mut file = File::create(Path::new(&format!("{i}/{collection_name}.json"))).unwrap();
            file.write_all("[\n".as_bytes()).unwrap();
            file.write_all(
                cursor
                    .try_next()
                    .await
                    .unwrap()
                    .unwrap()
                    .to_string()
                    .as_bytes(),
            )
            .unwrap();
            while let Some(item) = cursor.try_next().await.unwrap() {
                file.write_all((",\n".to_string() + item.to_string().as_str()).as_bytes())
                    .unwrap();
            }
            file.write_all("]".as_bytes()).unwrap();
        }
    }
}

fn parse_artist(file_name: &str) -> &str {
    let mut start = 0;
    if let Some(k) = file_name.rfind('/') {
        start = k + 1;
    } else if let Some(k) = file_name.rfind('\\') {
        start = k + 1;
    }
    
    match file_name[start..].chars().position(|x| x == '-') {
        Some(k) => file_name[start..k].trim(),
        None => file_name[start..].trim(),
    }
}

fn parse_title(file_name: &str) -> &str {
    let end = match file_name.rfind('.') {
        Some(k) => k,
        None => file_name.len(),
    };
    match file_name.find('-') {
        Some(k) => &file_name[k + 1..end].trim(),
        None => &file_name[..end],
    }
}

pub async fn print(mongo_client: MongoClient) {
    let db = mongo_client.database("mpdtrackr");
    let find_options = FindOptions::builder()
        .sort(doc! {"time": -1})
        .projection(doc! {"_id": 0})
        .build();
    for collection_name in db.list_collection_names(None).await.unwrap() {
        let mut cursor = db
            .collection::<Document>(collection_name.as_str())
            .find(None, find_options.clone())
            .await
            .unwrap();
        while let Some(item) = cursor.try_next().await.unwrap() {
            println!("{}", item);
        }
    }
}

pub async fn run(
    mongo_client: MongoClient,
    mut mpd_client: MPDClient,
    config: JsonValue,
    logging: bool,
) {
    let db = mongo_client.database("mpdtrackr");
    let mongo_artists = db.collection::<Document>("artists");
    let mongo_songs = db.collection::<Document>("songs");
    if logging {
        println!("mptrackr started");
    }
    loop {
        while mpd_client.status().unwrap().time.is_none(){
        }
        let mut current_time = mpd_client.status().unwrap().time.unwrap().0.num_seconds();
        let song = mpd_client.currentsong().unwrap().unwrap();
        let artist = match song.tags.get("Artist") {
            Some(k) => k,
            None => {
                if logging {
                    println!("Could not get artrsist name from mpd, parsing artist name from filename...");
                }
                parse_artist(&song.file)
            }
        };
        let title = match &song.title {
            Some(k) => k,
            None => {
                if logging {
                    println!(
                        "Could not get song name from mpd, parsing song name from filename..."
                    );
                }
                parse_title(&song.file)
            }
        };
        if mongo_artists
            .find_one(doc! {"artist": artist}, None)
            .await
            .unwrap()
            .is_none()
        {
            mongo_artists
                .insert_one(doc! {"artist": artist, "time": 0}, None)
                .await
                .unwrap();
            if logging {
                println!(
                    "New artist: \"{}\" inserted into artists collection",
                    artist
                );
            }
        }
        if mongo_songs
            .find_one(doc! {"title": &title}, None)
            .await
            .unwrap()
            .is_none()
        {
            mongo_songs
                .insert_one(doc! {"title": &title, "artist": artist, "time": 0}, None)
                .await
                .unwrap();
            if logging {
                println!(
                    "New song: \"{} - {}\" inserted into songs collection",
                    artist, title,
                );
            }
        }
        let mut old_time = current_time;
        let mut start_time = Instant::now();
        while title
            == match mpd_client.currentsong().unwrap().unwrap().title {
                Some(k) => k,
                None => parse_title(&mpd_client.currentsong().unwrap().unwrap().file).to_string(),
            }
        {
            current_time = match mpd_client.status().unwrap().time {
                Some(k) => k.0.num_seconds(),
                None => break,
            };
            let elapsed = start_time.elapsed();
            let remaining = Duration::from_secs(1) - elapsed;
            sleep(remaining);
            if old_time < current_time {
                mongo_artists
                    .update_one(doc! {"artist": artist}, doc! {"$inc": {"time": 1}}, None)
                    .await
                    .unwrap();
                if logging {
                    println!("Artist: \"{}\" time incremented by 1", artist);
                }
                mongo_songs
                    .update_one(doc! {"title": &title}, doc! {"$inc": {"time": 1}}, None)
                    .await
                    .unwrap();
                if logging {
                    println!("Song: \"{} - {}\" time incremented by 1", artist, title);
                }
                old_time = current_time;
            } else if old_time > current_time {
                old_time = current_time;
            }
            start_time = Instant::now();
        }
    }
}
