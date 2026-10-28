use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Limit {
    Time(u64),
    Words(usize),
}

/// One second of the test, used for the results graph.
#[derive(Clone, Debug, PartialEq)]
pub struct Sample {
    pub second: u64,
    /// Cumulative WPM from the start up to the end of this second.
    pub wpm: f64,
    /// Keystrokes in this second only, as WPM.
    pub raw: f64,
    pub errors: u32,
}

#[derive(Clone, Debug)]
pub struct Stats {
    pub wpm: f64,
    pub raw: f64,
    pub accuracy: f64,
    pub consistency: f64,
    pub correct_chars: usize,
    pub incorrect_chars: usize,
    pub duration: f64,
    pub samples: Vec<Sample>,
}

pub struct Test {
    pub words: Vec<String>,
    /// What the user typed for each word so far; the last entry is the current word.
    pub typed: Vec<String>,
    pub limit: Limit,
    start: Option<Instant>,
    end: Option<Instant>,
    /// (seconds since start, correct) for every non-backspace keystroke.
    keystrokes: Vec<(f64, bool)>,
}

impl Test {
    pub fn new(words: Vec<String>, limit: Limit) -> Self {
        Test {
            words,
            typed: vec![String::new()],
            limit,
            start: None,
            end: None,
            keystrokes: Vec::new(),
        }
    }

    pub fn started(&self) -> bool {
        self.start.is_some()
    }

    pub fn finished(&self) -> bool {
        self.end.is_some()
    }

    pub fn elapsed(&self, now: Instant) -> f64 {
        match self.start {
            Some(s) => (self.end.unwrap_or(now) - s).as_secs_f64(),
            None => 0.0,
        }
    }

    /// Ends a time-mode test once its duration is over. Returns true if it just ended.
    pub fn tick(&mut self, now: Instant) -> bool {
        if let (Limit::Time(secs), Some(start), None) = (self.limit, self.start, self.end)
            && now - start >= Duration::from_secs(secs)
        {
            self.end = Some(start + Duration::from_secs(secs));
            return true;
        }
        false
    }

    pub fn type_char(&mut self, c: char, now: Instant) {
        if self.finished() {
            return;
        }
        let start = *self.start.get_or_insert(now);
        let t = (now - start).as_secs_f64();
        let i = self.typed.len() - 1;

        if c == ' ' {
            if self.typed[i].is_empty() {
                return;
            }
            self.keystrokes.push((t, self.typed[i] == self.words[i]));
            if self.typed.len() == self.words.len() {
                self.end = Some(now);
            } else {
                self.typed.push(String::new());
            }
            return;
        }

        let pos = self.typed[i].chars().count();
        let correct = self.words[i].chars().nth(pos) == Some(c);
        self.typed[i].push(c);
        self.keystrokes.push((t, correct));
        // The last word ends the test without a trailing space, like monkeytype.
        if self.typed.len() == self.words.len() && self.typed[i] == self.words[i] {
            self.end = Some(now);
        }
    }

    pub fn backspace(&mut self) {
        if self.finished() {
            return;
        }
        let i = self.typed.len() - 1;
        if self.typed[i].pop().is_none() && i > 0 && self.typed[i - 1] != self.words[i - 1] {
            // Only allow going back into a previous word if it has mistakes.
            self.typed.pop();
        }
    }

    /// Characters typed correctly: matching positions in words, plus a space per finished correct word.
    fn char_counts(&self) -> (usize, usize) {
        let (mut ok, mut bad) = (0, 0);
        for (i, typed) in self.typed.iter().enumerate() {
            let target: Vec<char> = self.words[i].chars().collect();
            for (j, c) in typed.chars().enumerate() {
                if target.get(j) == Some(&c) {
                    ok += 1;
                } else {
                    bad += 1;
                }
            }
            if i + 1 < self.typed.len() && *typed == self.words[i] {
                ok += 1;
            }
        }
        (ok, bad)
    }

    pub fn live_wpm(&self, now: Instant) -> f64 {
        wpm(self.char_counts().0, self.elapsed(now))
    }

    pub fn live_accuracy(&self) -> f64 {
        accuracy(&self.keystrokes)
    }

    pub fn stats(&self, now: Instant) -> Stats {
        let duration = self.elapsed(now);
        let (correct_chars, incorrect_chars) = self.char_counts();
        let seconds = duration.ceil().max(1.0) as u64;
        let mut samples = Vec::new();
        for s in 1..=seconds {
            let (lo, hi) = ((s - 1) as f64, s as f64);
            let upto = self.keystrokes.iter().filter(|k| k.0 < hi);
            let in_sec: Vec<_> = self.keystrokes.iter().filter(|k| k.0 >= lo && k.0 < hi).collect();
            // The last second may be partial.
            let span = (duration.min(hi) - lo).max(0.001);
            samples.push(Sample {
                second: s,
                wpm: wpm(upto.filter(|k| k.1).count(), duration.min(hi)),
                raw: wpm(in_sec.len(), span),
                errors: in_sec.iter().filter(|k| !k.1).count() as u32,
            });
        }
        Stats {
            wpm: wpm(correct_chars, duration),
            raw: wpm(self.keystrokes.len(), duration),
            accuracy: accuracy(&self.keystrokes),
            consistency: consistency(&samples.iter().map(|s| s.raw).collect::<Vec<_>>()),
            correct_chars,
            incorrect_chars,
            duration,
            samples,
        }
    }
}

fn wpm(chars: usize, secs: f64) -> f64 {
    if secs <= 0.0 {
        return 0.0;
    }
    chars as f64 / 5.0 * 60.0 / secs
}

fn accuracy(keys: &[(f64, bool)]) -> f64 {
    if keys.is_empty() {
        return 100.0;
    }
    keys.iter().filter(|k| k.1).count() as f64 * 100.0 / keys.len() as f64
}

/// 100 minus the coefficient of variation of per-second raw speed, clamped to 0..=100.
fn consistency(raw: &[f64]) -> f64 {
    let n = raw.len() as f64;
    let mean = raw.iter().sum::<f64>() / n.max(1.0);
    if mean == 0.0 {
        return 0.0;
    }
    let sd = (raw.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / n).sqrt();
    (100.0 - sd / mean * 100.0).clamp(0.0, 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test(words: &[&str], limit: Limit) -> Test {
        Test::new(words.iter().map(|w| w.to_string()).collect(), limit)
    }

    fn type_str(t: &mut Test, s: &str, start: Instant, step_ms: u64) {
        for (i, c) in s.chars().enumerate() {
            t.type_char(c, start + Duration::from_millis(step_ms * i as u64));
        }
    }

    #[test]
    fn perfect_words_test() {
        let mut t = test(&["hello", "world"], Limit::Words(2));
        let start = Instant::now();
        // 11 keystrokes, 100ms apart: last one at 1.0s.
        type_str(&mut t, "hello world", start, 100);
        assert!(t.finished());
        let s = t.stats(start + Duration::from_secs(5));
        assert_eq!(s.duration, 1.0);
        assert_eq!(s.correct_chars, 11);
        assert_eq!(s.accuracy, 100.0);
        assert!((s.wpm - 132.0).abs() < 1e-9);
    }

    #[test]
    fn errors_and_backspace() {
        let mut t = test(&["ab", "cd"], Limit::Words(2));
        let start = Instant::now();
        type_str(&mut t, "ax ", start, 10);
        assert_eq!(t.typed, vec!["ax", ""]);
        // Back into the wrong word, fix it.
        t.backspace();
        t.backspace();
        assert_eq!(t.typed, vec!["a"]);
        type_str(&mut t, "b cd", start + Duration::from_millis(100), 10);
        assert!(t.finished());
        let s = t.stats(start);
        assert_eq!(s.incorrect_chars, 0);
        assert!(s.accuracy < 100.0);
    }

    #[test]
    fn cannot_go_back_into_correct_word() {
        let mut t = test(&["ab", "cd"], Limit::Words(2));
        type_str(&mut t, "ab ", Instant::now(), 10);
        t.backspace();
        assert_eq!(t.typed, vec!["ab", ""]);
    }

    #[test]
    fn time_limit_ends_test() {
        let mut t = test(&["a", "b", "c"], Limit::Time(15));
        let start = Instant::now();
        t.type_char('a', start);
        assert!(!t.tick(start + Duration::from_secs(14)));
        assert!(t.tick(start + Duration::from_secs(16)));
        assert_eq!(t.elapsed(start + Duration::from_secs(99)), 15.0);
        assert_eq!(t.stats(start).samples.len(), 15);
    }

    #[test]
    fn consistency_bounds() {
        assert_eq!(consistency(&[50.0, 50.0, 50.0]), 100.0);
        assert_eq!(consistency(&[]), 0.0);
        assert!(consistency(&[10.0, 90.0]) < 50.0);
    }
}
