use std::time::{Instant, SystemTime, UNIX_EPOCH};

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::history::{self, Record};
use crate::keyboard::LAYOUTS;
use crate::stratagem::{Dir, Drill, DrillStats};
use crate::typing::{Limit, Stats, Test};
use crate::words;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mode {
    Time,
    Words,
    Stratagem,
}

impl Mode {
    pub const ALL: [Mode; 3] = [Mode::Time, Mode::Words, Mode::Stratagem];

    pub fn name(self) -> &'static str {
        match self {
            Mode::Time => "time",
            Mode::Words => "words",
            Mode::Stratagem => "stratagem",
        }
    }

    fn lengths(self) -> &'static [u64] {
        match self {
            Mode::Time | Mode::Stratagem => &[15, 30, 60, 120],
            Mode::Words => &[10, 25, 50, 100],
        }
    }

    fn default_length(self) -> u64 {
        match self {
            Mode::Words => 25,
            _ => 30,
        }
    }
}

pub enum Game {
    Typing(Test),
    Stratagem(Drill),
}

impl Game {
    fn started(&self) -> bool {
        match self {
            Game::Typing(t) => t.started(),
            Game::Stratagem(d) => d.started(),
        }
    }

    fn finished(&self) -> bool {
        match self {
            Game::Typing(t) => t.finished(),
            Game::Stratagem(d) => d.finished(),
        }
    }
}

pub enum Screen {
    Playing,
    Results(Stats),
    StratagemResults(DrillStats),
}

/// One choice in the config bar.
#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    Mode(Mode),
    Length(u64),
    Lang(String),
    /// Cycles through keyboard layouts; highlighted while the keyboard is shown.
    Keyboard,
}

pub struct App {
    pub game: Game,
    pub screen: Screen,
    pub mode: Mode,
    /// Seconds for time and stratagem modes, word count for words mode.
    pub length: u64,
    pub lang: String,
    /// Error shown in the help line (history save, bad word list).
    pub error: Option<String>,
    /// Cursor position in the config bar while it has focus.
    pub menu: Option<usize>,
    /// Keyboard layout, index into `LAYOUTS`.
    pub layout: usize,
    /// On-screen keyboard and hands guide (opt-in, `H` in normal mode).
    pub hands: bool,
    /// Help popup open (normal mode only).
    pub help: bool,
    langs: Vec<String>,
    list: Vec<String>,
}

/// Words kept ahead of the cursor in time mode.
const TIME_MODE_BUFFER: usize = 50;

impl App {
    pub fn new(lang: String, mode: Mode, length: u64, layout: usize, hands: bool) -> Result<Self, String> {
        let list = words::load(&lang)?;
        let mut app = App {
            game: Game::Stratagem(Drill::new(0)),
            screen: Screen::Playing,
            mode,
            length: length.max(1),
            lang,
            error: None,
            menu: None,
            layout,
            hands,
            help: false,
            langs: words::available(),
            list,
        };
        app.restart();
        Ok(app)
    }

    pub fn restart(&mut self) {
        self.game = match self.mode {
            Mode::Time => Game::Typing(Test::new(
                words::pick(&self.list, TIME_MODE_BUFFER * 2),
                Limit::Time(self.length),
            )),
            Mode::Words => Game::Typing(Test::new(
                words::pick(&self.list, self.length as usize),
                Limit::Words(self.length as usize),
            )),
            Mode::Stratagem => Game::Stratagem(Drill::new(self.length)),
        };
        self.screen = Screen::Playing;
        self.error = None;
    }

    /// Config bar contents, in display order. Groups are told apart by variant.
    pub fn menu_items(&self) -> Vec<Item> {
        let langs = if self.mode == Mode::Stratagem {
            &[][..]
        } else {
            &self.langs[..]
        };
        Mode::ALL
            .into_iter()
            .map(Item::Mode)
            .chain(self.mode.lengths().iter().map(|&n| Item::Length(n)))
            .chain(langs.iter().map(|l| Item::Lang(l.clone())))
            .chain((self.mode != Mode::Stratagem).then_some(Item::Keyboard))
            .collect()
    }

    pub fn is_selected(&self, item: &Item) -> bool {
        match item {
            Item::Mode(m) => *m == self.mode,
            Item::Length(n) => *n == self.length,
            Item::Lang(l) => *l == self.lang,
            Item::Keyboard => self.hands,
        }
    }

    fn select(&mut self, item: Item) {
        match item {
            Item::Mode(m) if m != self.mode => (self.mode, self.length) = (m, m.default_length()),
            Item::Mode(_) => {}
            Item::Length(n) => self.length = n,
            Item::Keyboard => {
                // Display only: keep the running test. Picking a layout also shows the keyboard.
                self.layout = (self.layout + 1) % LAYOUTS.len();
                self.hands = true;
                return;
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
            KeyCode::Char('?') => self.help = true,
            KeyCode::Char('H') => self.hands = !self.hands,
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
        if self.help {
            // Any key closes the help.
            self.help = false;
            return false;
        }
        if let Some(cursor) = self.menu {
            return self.menu_key(key, cursor);
        }
        match key.code {
            KeyCode::Esc => {
                // Leaving a running test for the menu abandons it.
                if self.game.started() && !self.game.finished() {
                    self.restart();
                }
                self.menu = Some(0);
            }
            KeyCode::Tab => self.restart(),
            _ if !matches!(self.screen, Screen::Playing) || ctrl => {}
            code => match &mut self.game {
                Game::Typing(test) => match code {
                    KeyCode::Backspace => test.backspace(),
                    KeyCode::Char(c) => {
                        test.type_char(c, now);
                        if self.mode == Mode::Time && test.words.len() - test.typed.len() < TIME_MODE_BUFFER {
                            test.words.extend(words::pick(&self.list, TIME_MODE_BUFFER));
                        }
                    }
                    _ => {}
                },
                Game::Stratagem(drill) => {
                    if let Some(dir) = direction(code) {
                        drill.input(dir, now);
                    }
                }
            },
        }
        self.finish_if_done(now);
        false
    }

    pub fn tick(&mut self, now: Instant) {
        match &mut self.game {
            Game::Typing(t) => {
                t.tick(now);
            }
            Game::Stratagem(d) => d.tick(now),
        }
        self.finish_if_done(now);
    }

    fn finish_if_done(&mut self, now: Instant) {
        if !matches!(self.screen, Screen::Playing) || !self.game.finished() {
            return;
        }
        let mut record = Record {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |d| d.as_secs()),
            mode: self.mode.name().to_string(),
            mode_value: self.length,
            language: self.lang.clone(),
            wpm: 0.0,
            raw: 0.0,
            accuracy: 0.0,
            consistency: 0.0,
            correct_chars: 0,
            incorrect_chars: 0,
            duration: 0.0,
        };
        self.screen = match &self.game {
            Game::Typing(test) => {
                let s = test.stats(now);
                record.wpm = s.wpm;
                record.raw = s.raw;
                record.accuracy = s.accuracy;
                record.consistency = s.consistency;
                record.correct_chars = s.correct_chars;
                record.incorrect_chars = s.incorrect_chars;
                record.duration = s.duration;
                Screen::Results(s)
            }
            Game::Stratagem(drill) => {
                let s = drill.stats(now);
                // See Record docs for how stratagem rounds map onto these fields.
                record.language = String::new();
                record.wpm = s.per_minute;
                record.raw = s.inputs_per_minute;
                record.accuracy = s.accuracy;
                record.correct_chars = s.completed;
                record.incorrect_chars = s.errors as usize;
                record.duration = s.duration;
                Screen::StratagemResults(s)
            }
        };
        self.error = history::append(&record)
            .err()
            .map(|e| format!("history not saved: {e}"));
    }
}

/// Arrow keys, plus hjkl.
// shortcut: hjkl always on; gate it behind `vim_mode` (default true) once config exists (phase 5).
fn direction(code: KeyCode) -> Option<Dir> {
    match code {
        KeyCode::Up | KeyCode::Char('k') => Some(Dir::Up),
        KeyCode::Down | KeyCode::Char('j') => Some(Dir::Down),
        KeyCode::Left | KeyCode::Char('h') => Some(Dir::Left),
        KeyCode::Right | KeyCode::Char('l') => Some(Dir::Right),
        _ => None,
    }
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
        let mut app = App::new("english".into(), Mode::Time, 30, 0, false).unwrap();
        press(&mut app, KeyCode::Esc);
        assert_eq!(app.menu, Some(0));
        // Cursor to "words", select it: defaults to 25 words.
        press(&mut app, KeyCode::Char('l'));
        press(&mut app, KeyCode::Enter);
        assert_eq!((app.mode, app.length), (Mode::Words, 25));
        let Game::Typing(test) = &app.game else { panic!() };
        assert_eq!(test.words.len(), 25);
        // Skip "stratagem", then the first word count.
        press(&mut app, KeyCode::Right);
        press(&mut app, KeyCode::Right);
        press(&mut app, KeyCode::Char(' '));
        assert_eq!(app.length, 10);
        // H toggles the keyboard; the last item cycles the layout and shows it, without restarting.
        press(&mut app, KeyCode::Char('H'));
        assert!(app.hands);
        press(&mut app, KeyCode::Char('H'));
        assert!(!app.hands);
        for _ in 0..20 {
            press(&mut app, KeyCode::Right);
        }
        press(&mut app, KeyCode::Enter);
        assert_eq!((app.layout, app.hands), (1, true));
        // ? opens the help, any key closes it without acting.
        press(&mut app, KeyCode::Char('?'));
        assert!(app.help);
        assert!(!press(&mut app, KeyCode::Char('q')));
        assert!(!app.help);
        // Esc closes the menu, q is then typed, not quit.
        press(&mut app, KeyCode::Esc);
        assert!(!press(&mut app, KeyCode::Char('q')));
        let Game::Typing(test) = &app.game else { panic!() };
        assert_eq!(test.typed[0], "q");
    }

    #[test]
    fn esc_abandons_running_test() {
        let mut app = App::new("english".into(), Mode::Words, 10, 0, false).unwrap();
        press(&mut app, KeyCode::Char('a'));
        press(&mut app, KeyCode::Esc);
        assert!(!app.game.started());
        assert!(press(&mut app, KeyCode::Char('q')));
    }

    #[test]
    fn stratagem_mode_takes_arrows_and_hjkl() {
        let mut app = App::new("english".into(), Mode::Stratagem, 30, 0, false).unwrap();
        assert!(
            !app.menu_items()
                .iter()
                .any(|i| matches!(i, Item::Lang(_) | Item::Keyboard))
        );
        let Game::Stratagem(drill) = &app.game else {
            panic!()
        };
        let code = drill.current().code;
        for (i, dir) in code.iter().enumerate() {
            let key = match (dir, i % 2) {
                (Dir::Up, 0) => KeyCode::Up,
                (Dir::Up, _) => KeyCode::Char('k'),
                (Dir::Down, 0) => KeyCode::Down,
                (Dir::Down, _) => KeyCode::Char('j'),
                (Dir::Left, 0) => KeyCode::Left,
                (Dir::Left, _) => KeyCode::Char('h'),
                (Dir::Right, 0) => KeyCode::Right,
                (Dir::Right, _) => KeyCode::Char('l'),
            };
            press(&mut app, key);
        }
        let Game::Stratagem(drill) = &app.game else {
            panic!()
        };
        assert_eq!(drill.times.len(), 1);
    }
}
