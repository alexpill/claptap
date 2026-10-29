use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// One finished test, stored as a line of `<data>/history.jsonl`.
///
/// Stratagem rounds (`mode = "stratagem"`) reuse the fields: `wpm` = stratagems per minute,
/// `raw` = inputs per minute, `correct_chars` = stratagems completed, `incorrect_chars` = wrong
/// inputs, `language` is empty and `consistency` is 0.
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
    append_to(
        &path().ok_or_else(|| std::io::Error::other("no home directory"))?,
        record,
    )
}

fn append_to(path: &Path, record: &Record) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut f = OpenOptions::new()
        .create(true)
        .read(true)
        .append(true)
        .open(path)?;
    // A run cut short mid-write leaves a line without its newline: don't glue this record onto it.
    let mut last = [0u8];
    if f.seek(SeekFrom::End(0))? > 0 {
        f.seek(SeekFrom::End(-1))?;
        f.read_exact(&mut last)?;
        if last[0] != b'\n' {
            writeln!(f)?;
        }
    }
    writeln!(f, "{}", serde_json::to_string(record)?)
}

/// All records, plus how many lines were skipped because they couldn't be parsed.
/// A missing history file is an empty history.
#[expect(dead_code, reason = "read by the stats dashboard (phase 3)")]
pub fn read_all() -> std::io::Result<(Vec<Record>, usize)> {
    read_from(&path().ok_or_else(|| std::io::Error::other("no home directory"))?)
}

fn read_from(path: &Path) -> std::io::Result<(Vec<Record>, usize)> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(parse(&text)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok((Vec::new(), 0)),
        Err(e) => Err(e),
    }
}

fn parse(text: &str) -> (Vec<Record>, usize) {
    let mut skipped = 0;
    let records = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).map_err(|_| skipped += 1).ok())
        .collect();
    (records, skipped)
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"{"timestamp":1,"mode":"time","mode_value":30,"language":"english","wpm":80.0,"raw":85.0,"accuracy":97.0,"consistency":70.0,"correct_chars":200,"incorrect_chars":5,"duration":30.0}"#;

    #[test]
    fn parse_skips_bad_lines() {
        let text = format!("{GOOD}\nnot json\n\n{{\"timestamp\":2}}\n{GOOD}\n{{\"timestamp\":3,\"mo");
        let (records, skipped) = parse(&text);
        assert_eq!((records.len(), skipped), (2, 3));
    }

    #[test]
    fn append_after_truncated_line() {
        let dir = std::env::temp_dir().join(format!("claptap-test-{}", std::process::id()));
        let path = dir.join("history.jsonl");
        assert_eq!(
            read_from(&path).unwrap().0.len(),
            0,
            "missing file is empty history"
        );
        fs::create_dir_all(&dir).unwrap();
        fs::write(&path, format!("{GOOD}\n{{\"timestamp\":3,\"mo")).unwrap();
        append_to(&path, &parse(GOOD).0.remove(0)).unwrap();
        let (records, skipped) = read_from(&path).unwrap();
        fs::remove_dir_all(&dir).unwrap();
        assert_eq!((records.len(), skipped), (2, 1));
    }
}
