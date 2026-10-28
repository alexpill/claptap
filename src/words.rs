use rand::seq::IndexedRandom;

const ENGLISH: &str = include_str!("../words/english.txt");
const FRENCH: &str = include_str!("../words/french.txt");

/// Loads a word list: `<config>/words/<lang>.txt` if it exists, else a built-in list.
pub fn load(lang: &str) -> Result<Vec<String>, String> {
    let user = crate::dirs().map(|d| d.config_dir().join("words").join(format!("{lang}.txt")));
    let text = match user.and_then(|p| std::fs::read_to_string(p).ok()) {
        Some(t) => t,
        None => match lang {
            "english" => ENGLISH.to_string(),
            "french" => FRENCH.to_string(),
            _ => return Err(format!("unknown language '{lang}' (built-in: english, french)")),
        },
    };
    let words: Vec<String> = text.split_whitespace().map(str::to_string).collect();
    if words.is_empty() {
        return Err(format!("word list '{lang}' is empty"));
    }
    Ok(words)
}

pub fn pick(list: &[String], n: usize) -> Vec<String> {
    let mut rng = rand::rng();
    let mut out: Vec<String> = Vec::with_capacity(n);
    while out.len() < n {
        let w = list.choose(&mut rng).unwrap();
        // Avoid the same word twice in a row when the list allows it.
        if list.len() == 1 || out.last() != Some(w) {
            out.push(w.clone());
        }
    }
    out
}

/// Built-in languages, then user word lists from `<config>/words/`.
pub fn available() -> Vec<String> {
    let mut langs = vec!["english".to_string(), "french".to_string()];
    let dir = crate::dirs().map(|d| d.config_dir().join("words"));
    if let Some(entries) = dir.and_then(|d| std::fs::read_dir(d).ok()) {
        let mut user: Vec<String> = entries
            .filter_map(|e| {
                let p = e.ok()?.path();
                (p.extension()? == "txt").then(|| p.file_stem()?.to_str().map(str::to_string))?
            })
            .filter(|l| !langs.contains(l))
            .collect();
        user.sort();
        langs.extend(user);
    }
    langs
}
