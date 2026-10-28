use std::time::{Instant, SystemTime, UNIX_EPOCH};

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::history::{self, Record};
use crate::typing::{Limit, Stats, Test};
use crate::words;

pub enum Screen {
    Typing,
    Results(Stats),
}

/// One choice in the config bar.
#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    Mode(Limit),
    Length(u64),
    Lang(String),
}

pub struct App {
    pub test: Test,
    pub screen: Screen,
    pub lang: String,
    pub limit: Limit,
    /// Error shown in the help line (history save, bad word list).
    pub error: Option<String>,
    /// Cursor position in the config bar while it has focus.
    pub menu: Option<usize>,
    langs: Vec<String>,
    list: Vec<String>,
}

const TIMES: [u64; 4] = [15, 30, 60, 120];
const WORD_COUNTS: [u64; 4] = [10, 25, 50, 100];

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
            error: None,
            menu: None,
            langs: words::available(),
            list,
        })
    }

    pub fn restart(&mut self) {
        self.test = new_test(&self.list, self.limit);
        self.screen = Screen::Typing;
        self.error = None;
    }

    /// Config bar contents, in display order. Groups are told apart by variant.
    pub fn menu_items(&self) -> Vec<Item> {
        let lengths: &[u64] = match self.limit {
            Limit::Time(_) => &TIMES,
            Limit::Words(_) => &WORD_COUNTS,
        };
        [Item::Mode(Limit::Time(30)), Item::Mode(Limit::Words(25))]
            .into_iter()
            .chain(lengths.iter().map(|&n| Item::Length(n)))
            .chain(self.langs.iter().map(|l| Item::Lang(l.clone())))
            .collect()
    }

    pub fn is_selected(&self, item: &Item) -> bool {
        match item {
            Item::Mode(m) => m.name() == self.limit.name(),
            Item::Length(n) => *n == self.limit.value(),
            Item::Lang(l) => *l == self.lang,
        }
    }

    fn select(&mut self, item: Item) {
        match item {
            Item::Mode(m) if m.name() != self.limit.name() => self.limit = m,
            Item::Mode(_) => {}
            Item::Length(n) => {
                self.limit = match self.limit {
                    Limit::Time(_) => Limit::Time(n),
                    Limit::Words(_) => Limit::Words(n as usize),
                }
            }
            Item::Lang(l) => match words::load(&l) {
                Ok(list) => (self.lang, self.list) = (l, list),
                Err(e) => {
                    self.error = Some(e);
                    return;
                }
            },
        }
        self.restart();
    }

    fn menu_key(&mut self, key: KeyEvent, cursor: usize) -> bool {
        let last = self.menu_items().len() - 1;
        match key.code {
            KeyCode::Esc => self.menu = None,
            KeyCode::Char('q') => return true,
            KeyCode::Left | KeyCode::Char('h') => self.menu = Some(cursor.saturating_sub(1)),
            KeyCode::Right | KeyCode::Char('l') => self.menu = Some((cursor + 1).min(last)),
            KeyCode::Enter | KeyCode::Char(' ') => {
                self.select(self.menu_items()[cursor].clone());
                // The item list can shrink or grow after a mode change.
                self.menu = Some(cursor.min(self.menu_items().len() - 1));
            }
            _ => {}
        }
        false
    }

    /// Returns true when the app should quit.
    pub fn on_key(&mut self, key: KeyEvent, now: Instant) -> bool {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        if ctrl && key.code == KeyCode::Char('c') {
            return true;
        }
        if let Some(cursor) = self.menu {
            return self.menu_key(key, cursor);
        }
        match key.code {
            KeyCode::Esc => {
                // Leaving a running test for the menu abandons it.
                if self.test.started() && !self.test.finished() {
                    self.restart();
                }
                self.menu = Some(0);
            }
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
        self.error = history::append(&record)
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

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::crossterm::event::KeyEvent;

    fn press(app: &mut App, code: KeyCode) -> bool {
        app.on_key(KeyEvent::from(code), Instant::now())
    }

    #[test]
    fn menu_switches_mode_and_length() {
        let mut app = App::new("english".into(), Limit::Time(30)).unwrap();
        press(&mut app, KeyCode::Esc);
        assert_eq!(app.menu, Some(0));
        // Cursor to "words", select it: defaults to 25 words.
        press(&mut app, KeyCode::Char('l'));
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.limit, Limit::Words(25));
        assert_eq!(app.test.words.len(), 25);
        // Next item is the first word count.
        press(&mut app, KeyCode::Right);
        press(&mut app, KeyCode::Char(' '));
        assert_eq!(app.limit, Limit::Words(10));
        // Esc closes the menu, q is then typed, not quit.
        press(&mut app, KeyCode::Esc);
        assert!(!press(&mut app, KeyCode::Char('q')));
        assert_eq!(app.test.typed[0], "q");
    }

    #[test]
    fn esc_abandons_running_test() {
        let mut app = App::new("english".into(), Limit::Words(10)).unwrap();
        press(&mut app, KeyCode::Char('a'));
        press(&mut app, KeyCode::Esc);
        assert!(!app.test.started());
        assert!(press(&mut app, KeyCode::Char('q')));
    }
}
