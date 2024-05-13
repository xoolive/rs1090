use rusqlite::Connection;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{copy, BufReader, Cursor};
use std::path::Path;
use zip::read::ZipArchive;

#[derive(Debug)]
pub struct Aircraft {
    icao24: String,
    pub typecode: Option<String>,
    pub registration: Option<String>,
}

type Result<T> =
    std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

async fn download_file(url: &str, destination: &str) -> Result<()> {
    let response = reqwest::get(url).await?.bytes().await?;
    let mut file = File::create(destination)?;
    let mut content = Cursor::new(response);
    copy(&mut content, &mut file)?;
    Ok(())
}

pub async fn aircraft() -> BTreeMap<String, Aircraft> {
    let zip_url =
        "https://jetvision.de/resources/sqb_databases/basestation.zip";
    let zip_file_path = "basestation.zip";

    // Check if the zip file exists
    if !Path::new(zip_file_path).exists() {
        println!("Downloading basestation.zip...");
        let _ = download_file(zip_url, zip_file_path).await;
    }

    // Open the zip file
    let file = File::open(zip_file_path).unwrap();
    let reader = BufReader::new(file);
    let mut archive = ZipArchive::new(reader).unwrap();
    let mut sqlite_in_archive = archive.by_index(0).unwrap();
    let sqlite_path = sqlite_in_archive.mangled_name();
    let mut sqlite_file = File::create(&sqlite_path).unwrap();
    copy(&mut sqlite_in_archive, &mut sqlite_file).unwrap();

    // Read the SQLite file and establish a connection
    let sqlite_connection = Connection::open(sqlite_path).unwrap();

    let mut stmt = sqlite_connection
        .prepare("SELECT ModeS, Registration, ICAOTypeCode FROM Aircraft")
        .unwrap();

    let mut aircraftdb = BTreeMap::new();

    let rows = stmt
        .query_map([], |row| {
            Ok(Aircraft {
                icao24: row.get(0).unwrap(),
                registration: row.get(1).unwrap_or_default(),
                typecode: row.get(2).unwrap_or_default(),
            })
        })
        .unwrap();

    for entry in rows.flatten() {
        aircraftdb.insert(
            entry.icao24.to_owned().to_lowercase(),
            Aircraft {
                icao24: entry.icao24.to_owned().to_lowercase(),
                registration: entry.registration,
                typecode: entry.typecode,
            },
        );
    }

    aircraftdb
}
