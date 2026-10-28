use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// One finished test, stored as a line of `<data>/history.jsonl`.
// shortcut: append-only JSONL read fully into memory; move to SQLite if history gets past ~100k runs.
#[derive(Serialize, Deserialize, Debug)]
pub struct Record {
    pub timestamp: u64,
    pub mode: String,
    pub mode_value: u64,
    pub language: String,
    pub wpm: f64,
    pub raw: f64,
    pub accuracy: f64,
    pub consistency: f64,
    pub correct_chars: usize,
    pub incorrect_chars: usize,
    pub duration: f64,
}

pub fn path() -> Option<PathBuf> {
    crate::dirs().map(|d| d.data_dir().join("history.jsonl"))
}

pub fn append(record: &Record) -> std::io::Result<()> {
    let path = path().ok_or_else(|| std::io::Error::other("no home directory"))?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut f = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(f, "{}", serde_json::to_string(record)?)
}
