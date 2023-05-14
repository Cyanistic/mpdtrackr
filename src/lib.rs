use futures::stream::TryStreamExt;
use json::*;
use mongodb::bson::{doc, Document};
use mongodb::{options::FindOptions, Client as MongoClient};
use mpd::Client as MPDClient;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::thread::sleep;
use std::{io::Write, time::Duration};

pub fn create_config_dir() {
    match std::fs::create_dir_all(Path::new(
        &(dirs::config_dir().unwrap().display().to_string() + "/mpdtrackr"),
    )) {
        Ok(k) => k,
        Err(e) => println!("Could not create config directory: {}", e),
    };
}

pub fn create_config() {
    let dir = dirs::config_dir().unwrap().display().to_string() + "/mpdtrackr";
    let config_file_dir =
        dirs::config_dir().unwrap().display().to_string() + "/mpdtrackr/config.json";
    let config_dir = Path::new(&dir);
    let config_file_path = Path::new(&config_file_dir);
    match config_dir.is_dir() {
        false => create_config_dir(),
        true => (),
    };
    let mut config = match config_file_path.is_file() {
        false => File::create(config_file_path).unwrap(),
        true => File::options()
            .read(true)
            .write(true)
            .open(config_file_path)
            .unwrap(),
    };
    match config.write_all("{\n \"mongo_port\": 27017,\n \"mpd_port\": 6600\n}".as_bytes()) {
        Ok(_) => (),
        Err(e) => println!("{}", e),
    };
}

pub async fn import(mongo_client: MongoClient, files: Vec<String>) {
    let db = mongo_client.database("mpdtrackr");
    for i in files {
        let path = Path::new(&i);
        let mut file = File::open(path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
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
                        ()
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
                        ()
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

pub async fn run(mongo_client: MongoClient, mut mpd_client: MPDClient, config: JsonValue) {
    let db = mongo_client.database("mpdtrackr");
    let mongo_artists = db.collection::<Document>("artists");
    let mongo_songs = db.collection::<Document>("songs");
    loop {
        let mut current_time = mpd_client.status().unwrap().time.unwrap().0.num_seconds();
        let song = mpd_client.currentsong().unwrap().unwrap();
        let artist = song.tags.get("Artist").unwrap();
        let title = song.title.clone().unwrap().clone();
        match mongo_artists
            .find_one(doc! {"artist": artist}, None)
            .await
            .unwrap()
        {
            None => {
                mongo_artists
                    .insert_one(doc! {"artist": artist, "time": 0}, None)
                    .await
                    .unwrap();
                ()
            }
            _ => (),
        }
        match mongo_songs
            .find_one(doc! {"title": &title}, None)
            .await
            .unwrap()
        {
            None => {
                mongo_songs
                    .insert_one(doc! {"title": &title, "artist": artist, "time": 0}, None)
                    .await
                    .unwrap();
                ()
            }
            _ => (),
        }
        let mut old_time = current_time;
        while title
            == mpd_client
                .currentsong()
                .unwrap()
                .unwrap()
                .title
                .clone()
                .unwrap()
        {
            current_time = mpd_client.status().unwrap().time.unwrap().0.num_seconds();
            sleep(Duration::from_millis(999));
            if old_time + 1 <= current_time {
                mongo_artists
                    .update_one(doc! {"artist": artist}, doc! {"$inc": {"time": 1}}, None)
                    .await
                    .unwrap();
                mongo_songs
                    .update_one(doc! {"title": &title}, doc! {"$inc": {"time": 1}}, None)
                    .await
                    .unwrap();
                old_time = current_time;
            } else if old_time > current_time {
                old_time = current_time;
            }
        }
    }
}
