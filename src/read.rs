use once_cell::sync::Lazy;
use rusqlite::{params, Connection};
use std::{
    io::{BufRead, BufReader, Read, Seek},
    sync::Mutex,
};
use zip::ZipArchive;

pub static DB: Lazy<Mutex<Connection>> =
    Lazy::new(|| Mutex::new(Connection::open("unihan.db").unwrap()));

pub fn init_db(zipfile: impl Read + Seek) -> Result<(), crate::InternalError> {
    let mut zip = ZipArchive::new(zipfile)?;

    let mut conn = DB.lock()?;

    conn.execute(
        "CREATE TABLE field (
            character INTEGER NOT NULL,
            name      TEXT NOT NULL,
            value     TEXT NOT NULL,
            PRIMARY KEY (character, name)
        )",
        [],
    )?;

    for i in 0..zip.len() {
        let file = zip.by_index(i)?;
        let tx = conn.transaction()?;
        for line in BufReader::new(file).lines() {
            let line = line?;
            if !line.starts_with("U+") {
                continue;
            }

            let cols: Vec<_> = line.split('\t').collect();

            // trim off "U+", parse from hex
            let char_value = u32::from_str_radix(&cols[0][2..], 16)?;

            tx.execute(
                "INSERT INTO field VALUES (?1, ?2, ?3)",
                params![char_value, cols[1], cols[2]],
            )?;
        }
        tx.commit()?;
    }

    Ok(())
}
