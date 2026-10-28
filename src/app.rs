use std::time::{Instant, SystemTime, UNIX_EPOCH};

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::history::{self, Record};
use crate::typing::{Limit, Stats, Test};
use crate::words;

pub enum Screen {
    Typing,
    Results(Stats),
}

pub struct App {
    pub test: Test,
    pub screen: Screen,
    pub lang: String,
    pub limit: Limit,
    pub save_error: Option<String>,
    list: Vec<String>,
}

/// Words kept ahead of the cursor in time mode.
const TIME_MODE_BUFFER: usize = 50;

impl App {
    pub fn new(lang: String, limit: Limit) -> Result<Self, String> {
        let list = words::load(&lang)?;
        let test = new_test(&list, limit);
        Ok(App {
            test,
            screen: Screen::Typing,
            lang,
            limit,
            save_error: None,
            list,
        })
    }

    pub fn restart(&mut self) {
        self.test = new_test(&self.list, self.limit);
        self.screen = Screen::Typing;
        self.save_error = None;
    }

    /// Returns true when the app should quit.
    pub fn on_key(&mut self, key: KeyEvent, now: Instant) -> bool {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Esc => return true,
            KeyCode::Char('c') if ctrl => return true,
            KeyCode::Tab => self.restart(),
            KeyCode::Backspace if matches!(self.screen, Screen::Typing) => self.test.backspace(),
            KeyCode::Char(c) if !ctrl && matches!(self.screen, Screen::Typing) => {
                self.test.type_char(c, now);
                if let Limit::Time(_) = self.limit
                    && self.test.words.len() - self.test.typed.len() < TIME_MODE_BUFFER
                {
                    self.test.words.extend(words::pick(&self.list, TIME_MODE_BUFFER));
                }
            }
            _ => {}
        }
        self.finish_if_done(now);
        false
    }

    pub fn tick(&mut self, now: Instant) {
        self.test.tick(now);
        self.finish_if_done(now);
    }

    fn finish_if_done(&mut self, now: Instant) {
        if !matches!(self.screen, Screen::Typing) || !self.test.finished() {
            return;
        }
        let stats = self.test.stats(now);
        let record = Record {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |d| d.as_secs()),
            mode: self.limit.name().to_string(),
            mode_value: self.limit.value(),
            language: self.lang.clone(),
            wpm: stats.wpm,
            raw: stats.raw,
            accuracy: stats.accuracy,
            consistency: stats.consistency,
            correct_chars: stats.correct_chars,
            incorrect_chars: stats.incorrect_chars,
            duration: stats.duration,
        };
        self.save_error = history::append(&record)
            .err()
            .map(|e| format!("history not saved: {e}"));
        self.screen = Screen::Results(stats);
    }
}

fn new_test(list: &[String], limit: Limit) -> Test {
    let n = match limit {
        Limit::Words(n) => n,
        Limit::Time(_) => TIME_MODE_BUFFER * 2,
    };
    Test::new(words::pick(list, n), limit)
}
