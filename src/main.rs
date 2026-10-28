mod app;
mod history;
mod keyboard;
mod stratagem;
mod typing;
mod ui;
mod words;

use std::time::{Duration, Instant};

use clap::Parser;
use directories::ProjectDirs;
use ratatui::crossterm::event::{self, Event, KeyEventKind};

use app::{App, Mode};

/// A typing test for the terminal, with a stratagem-code drill.
#[derive(Parser)]
#[command(version)]
struct Args {
    /// Word list: built-in (english, french) or <config>/words/<name>.txt
    #[arg(short, long, default_value = "english")]
    lang: String,
    /// Time mode: test length in seconds (also the stratagem round length)
    #[arg(short, long, default_value_t = 30, conflicts_with = "words")]
    time: u64,
    /// Words mode: number of words to type
    #[arg(short, long)]
    words: Option<u64>,
    /// Stratagem mode: enter arrow codes (arrows or hjkl) against the clock
    #[arg(short, long, conflicts_with = "words")]
    stratagem: bool,
    /// Keyboard layout for the on-screen guide: qwerty, azerty, qwertz, colemak, dvorak
    #[arg(short = 'k', long, default_value = "qwerty")]
    layout: String,
    /// Show the on-screen keyboard and hands (toggle with H in normal mode)
    #[arg(long)]
    hands: bool,
}

pub fn dirs() -> Option<ProjectDirs> {
    ProjectDirs::from("", "", "claptap")
}

fn main() {
    let args = Args::parse();
    let (mode, length) = match (args.words, args.stratagem) {
        (Some(n), _) => (Mode::Words, n),
        (None, true) => (Mode::Stratagem, args.time),
        (None, false) => (Mode::Time, args.time),
    };
    let layout = match keyboard::Layout::by_name(&args.layout) {
        Some(i) => i,
        None => {
            let name = &args.layout;
            let names: Vec<_> = keyboard::LAYOUTS.iter().map(|l| l.name).collect();
            eprintln!(
                "claptap: unknown layout '{name}' (available: {})",
                names.join(", ")
            );
            std::process::exit(2);
        }
    };
    let mut app = match App::new(args.lang, mode, length, layout, args.hands) {
        Ok(app) => app,
        Err(e) => {
            eprintln!("claptap: {e}");
            std::process::exit(1);
        }
    };

    let mut terminal = ratatui::init();
    let result = run(&mut terminal, &mut app);
    ratatui::restore();
    if let Err(e) = result {
        eprintln!("claptap: {e}");
        std::process::exit(1);
    }
}

fn run(terminal: &mut ratatui::DefaultTerminal, app: &mut App) -> std::io::Result<()> {
    loop {
        let now = Instant::now();
        app.tick(now);
        terminal.draw(|f| ui::draw(f, app, now))?;
        // Poll with a timeout so the timer keeps moving while idle.
        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
            && app.on_key(key, Instant::now())
        {
            return Ok(());
        }
    }
}
