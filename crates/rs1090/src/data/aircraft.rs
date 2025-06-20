use rusqlite::Connection;
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{copy, BufReader, Cursor};
use zip::read::ZipArchive;

#[derive(Debug)]
pub struct Aircraft {
    /// The ICAO 24-bit transponder address
    icao24: String,
    /// The ICAO typecode of the aircraft, e.g. A320, B789, etc.
    pub typecode: Option<String>,
    /// The last known tail number of the aircraft
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

pub async fn update_db() -> Result<()> {
    let mut cache_path = dirs::cache_dir().unwrap_or_default();
    cache_path.push("jet1090");
    if !cache_path.exists() {
        let msg =
            format!("failed to create {:?}", cache_path.to_str().unwrap());
        fs::create_dir_all(&cache_path).expect(&msg);
    }

    let zip_url =
        "https://jetvision.de/resources/sqb_databases/basestation.zip";
    let zip_file_path = "basestation.zip";
    cache_path.push(zip_file_path);

    println!("Downloading basestation.zip...");
    download_file(zip_url, cache_path.to_str().unwrap()).await?;

    // Open the zip file
    let file = File::open(&cache_path)?;
    let reader = BufReader::new(file);
    let mut archive = ZipArchive::new(reader)?;
    let mut sqlite_in_archive = archive.by_index(0)?;

    // Unzip the sqb file
    let cache_dir = cache_path.parent().unwrap();
    let mut sqlite_path = cache_dir.to_path_buf();
    sqlite_path.push(sqlite_in_archive.mangled_name());
    let mut sqlite_file = File::create(&sqlite_path)?;
    copy(&mut sqlite_in_archive, &mut sqlite_file)?;
    Ok(())
}

pub async fn aircraft() -> BTreeMap<String, Aircraft> {
    let mut cache_path = dirs::cache_dir().unwrap_or_default();
    cache_path.push("jet1090");
    if !cache_path.exists() {
        let msg =
            format!("failed to create {:?}", cache_path.to_str().unwrap());
        fs::create_dir_all(&cache_path).expect(&msg);
    }

    let zip_url =
        "https://jetvision.de/resources/sqb_databases/basestation.zip";
    let zip_file_path = "basestation.zip";
    cache_path.push(zip_file_path);

    // Check if the zip file exists
    if !cache_path.exists() {
        println!("Downloading basestation.zip...");
        download_file(zip_url, cache_path.to_str().unwrap())
            .await
            .expect("Failed to download basestation.zip");
    }

    // Open the zip file
    let file = File::open(&cache_path).unwrap();
    let reader = BufReader::new(file);
    let mut archive = ZipArchive::new(reader).unwrap();
    let mut sqlite_in_archive = archive.by_index(0).unwrap();

    // Unzip the sqb file
    let cache_dir = cache_path.parent().unwrap();
    let mut sqlite_path = cache_dir.to_path_buf();
    sqlite_path.push(sqlite_in_archive.mangled_name());
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
