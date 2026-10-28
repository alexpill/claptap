mod app;
mod history;
mod typing;
mod ui;
mod words;

use std::time::{Duration, Instant};

use clap::Parser;
use directories::ProjectDirs;
use ratatui::crossterm::event::{self, Event, KeyEventKind};

use app::App;
use typing::Limit;

/// A typing test for the terminal.
#[derive(Parser)]
#[command(version)]
struct Args {
    /// Word list: built-in (english, french) or <config>/words/<name>.txt
    #[arg(short, long, default_value = "english")]
    lang: String,
    /// Time mode: test length in seconds
    #[arg(short, long, default_value_t = 30, conflicts_with = "words")]
    time: u64,
    /// Words mode: number of words to type
    #[arg(short, long)]
    words: Option<usize>,
}

pub fn dirs() -> Option<ProjectDirs> {
    ProjectDirs::from("", "", "claptap")
}

fn main() {
    let args = Args::parse();
    let limit = match args.words {
        Some(n) => Limit::Words(n.max(1)),
        None => Limit::Time(args.time.max(1)),
    };
    let mut app = match App::new(args.lang, limit) {
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
