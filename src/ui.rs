use std::time::Instant;

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::symbols::Marker;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Axis, Block, BorderType, Chart, Dataset, GraphType, Paragraph};

use crate::app::{App, Game, Item, Mode, Screen};
use crate::stratagem::{self, Drill, DrillStats};
use crate::typing::{Stats, Test};

const ACCENT: Color = Color::Cyan;
const DIM: Color = Color::DarkGray;
const BAD: Color = Color::Red;
/// Lines of words shown at once while typing.
const VISIBLE_LINES: usize = 3;

fn panel(title: &str) -> Block<'_> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(DIM))
        .title(Span::styled(format!(" {title} "), Style::new().fg(ACCENT).bold()))
}

pub fn draw(f: &mut Frame, app: &App, now: Instant) {
    let [top, bar, body, help] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(f.area());

    let mut mode = format!("{} {}", app.mode.name(), app.length);
    if app.mode != Mode::Stratagem {
        mode = format!("{mode} · {}", app.lang);
    }
    let top_block = panel("claptap").title(Line::from(format!(" {mode} ")).right_aligned().fg(DIM));
    f.render_widget(Paragraph::new(top_line(app, now)).block(top_block), top);
    f.render_widget(Paragraph::new(config_bar(app)).alignment(Alignment::Center), bar);

    match (&app.screen, &app.game) {
        (Screen::Playing, Game::Typing(test)) => draw_typing(f, test, body),
        (Screen::Playing, Game::Stratagem(drill)) => draw_stratagem(f, drill, body, now),
        (Screen::Results(stats), _) => draw_results(f, stats, body),
        (Screen::StratagemResults(stats), _) => draw_stratagem_results(f, stats, body),
    }

    let keys: &[(&str, &str)] = if app.menu.is_some() {
        &[
            ("←→ hl", "move"),
            ("enter", "select"),
            ("esc", "back"),
            ("q", "quit"),
        ]
    } else {
        match (&app.screen, app.mode) {
            (Screen::Playing, Mode::Stratagem) => &[
                ("←↓↑→ hjkl", "code"),
                ("tab", "restart"),
                ("esc", "menu"),
                ("ctrl+c", "quit"),
            ],
            _ => &[("tab", "restart"), ("esc", "menu"), ("ctrl+c", "quit")],
        }
    };
    let line = match &app.error {
        Some(e) => Line::from(e.clone().fg(BAD)),
        None => Line::from_iter(
            keys.iter()
                .flat_map(|(k, a)| [k.fg(ACCENT), format!(" {a}  ").fg(DIM)]),
        ),
    };
    f.render_widget(Paragraph::new(line).alignment(Alignment::Center), help);
}

/// Monkeytype-style option bar: dim until focused with esc, current choices highlighted.
fn config_bar(app: &App) -> Line<'static> {
    let items = app.menu_items();
    let mut spans = Vec::new();
    for (i, item) in items.iter().enumerate() {
        if i > 0 && std::mem::discriminant(item) != std::mem::discriminant(&items[i - 1]) {
            spans.push(" │ ".fg(DIM));
        } else if i > 0 {
            spans.push(" ".into());
        }
        let label = match item {
            Item::Mode(m) => m.name().to_string(),
            Item::Length(n) => n.to_string(),
            Item::Lang(l) => l.clone(),
        };
        let mut style = match (app.is_selected(item), app.menu.is_some()) {
            (true, _) => Style::new().fg(ACCENT).bold(),
            (false, true) => Style::new(),
            (false, false) => Style::new().fg(DIM),
        };
        if app.menu == Some(i) {
            style = style.reversed();
        }
        spans.push(Span::styled(label, style));
    }
    Line::from(spans)
}

fn top_line(app: &App, now: Instant) -> Line<'static> {
    let stat = |label: &'static str, value: String| [format!(" {label} ").fg(DIM), value.bold()];
    let left = |elapsed: f64| format!("{:.0}s", (app.length as f64 - elapsed).max(0.0).ceil());
    let stats = match &app.game {
        Game::Typing(t) => [
            stat("wpm", format!("{:.0}", t.live_wpm(now))),
            stat("acc", format!("{:.0}%", t.live_accuracy())),
            match app.mode {
                Mode::Words => stat(
                    "words",
                    format!("{}/{}", t.typed.len() - usize::from(!t.finished()), app.length),
                ),
                _ => stat("left", left(t.elapsed(now))),
            },
        ]
        .concat(),
        Game::Stratagem(d) => [
            stat("done", d.times.len().to_string()),
            stat("streak", d.streak.to_string()),
            stat("acc", format!("{:.0}%", d.accuracy())),
            stat("left", left(d.elapsed(now))),
        ]
        .concat(),
    };
    Line::from(stats)
}

fn draw_typing(f: &mut Frame, test: &Test, area: Rect) {
    let width = area.width.saturating_sub(4).min(80);
    let height = VISIBLE_LINES as u16 + 2;
    let [_, mid, _] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(height),
        Constraint::Fill(1),
    ])
    .areas(area);
    let [_, rect, _] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(width + 4),
        Constraint::Fill(1),
    ])
    .areas(mid);

    let title = if test.started() { "typing" } else { "start typing" };
    let block = panel(title).padding(ratatui::widgets::Padding::horizontal(1));
    let (lines, cursor_line) = word_lines(test, width as usize);
    let first = cursor_line
        .saturating_sub(1)
        .min(lines.len().saturating_sub(VISIBLE_LINES));
    let shown: Vec<Line> = lines.into_iter().skip(first).take(VISIBLE_LINES).collect();
    f.render_widget(Paragraph::new(shown).block(block), rect);
}

/// Wraps the words into lines of at most `width` columns; returns the lines and the cursor's line.
fn word_lines(test: &Test, width: usize) -> (Vec<Line<'static>>, usize) {
    let current = test.typed.len() - 1;
    let mut lines = vec![Line::default()];
    let mut col = 0;
    let mut cursor_line = 0;

    for (i, word) in test.words.iter().enumerate() {
        let target: Vec<char> = word.chars().collect();
        let typed: Vec<char> = test.typed.get(i).map(|t| t.chars().collect()).unwrap_or_default();
        let len = target.len().max(typed.len());
        if col > 0 && col + len > width {
            lines.push(Line::default());
            col = 0;
        }
        if i == current {
            cursor_line = lines.len() - 1;
        }
        let line = lines.last_mut().unwrap();
        for j in 0..len {
            let (ch, mut style) = match (target.get(j), typed.get(j)) {
                (Some(t), Some(c)) if t == c => (*t, Style::new().fg(Color::Reset)),
                (Some(t), Some(_)) => (*t, Style::new().fg(BAD)),
                // Extra characters typed past the end of the word.
                (None, Some(c)) => (*c, Style::new().fg(BAD).add_modifier(Modifier::DIM)),
                (Some(t), None) if i < current => (*t, Style::new().fg(DIM).underlined()),
                (Some(t), None) => (*t, Style::new().fg(DIM)),
                (None, None) => unreachable!(),
            };
            if i == current && j == typed.len() {
                style = style.reversed();
            }
            line.push_span(Span::styled(ch.to_string(), style));
        }
        let space = if i == current && typed.len() >= len {
            Style::new().reversed()
        } else {
            Style::new()
        };
        line.push_span(Span::styled(" ", space));
        col += len + 1;
    }
    (lines, cursor_line)
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let [_, mid, _] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(height),
        Constraint::Fill(1),
    ])
    .areas(area);
    let [_, rect, _] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(width),
        Constraint::Fill(1),
    ])
    .areas(mid);
    rect
}

fn draw_stratagem(f: &mut Frame, drill: &Drill, area: Rect, now: Instant) {
    let rect = centered(area, area.width.saturating_sub(4).min(64), 9);
    let current = drill.current();
    let flash = drill.last_error.is_some_and(|t| now - t < stratagem::FLASH);
    let arrows: Vec<Span> = current
        .code
        .iter()
        .enumerate()
        .flat_map(|(i, d)| {
            let style = match (flash, i < drill.progress) {
                (true, _) => Style::new().fg(BAD).bold(),
                (false, true) => Style::new().fg(ACCENT).bold(),
                // Done arrows are bold + colored, pending ones dim: readable without color too.
                (false, false) => Style::new().fg(DIM),
            };
            [Span::styled(d.arrow(), style), "   ".into()]
        })
        .collect();
    let next: Vec<&str> = drill.queue[1..]
        .iter()
        .map(|&i| stratagem::BUILTIN[i].name)
        .collect();
    let text = vec![
        Line::default(),
        Line::from(current.name.bold()),
        Line::default(),
        Line::from(arrows),
        Line::default(),
        Line::from(format!("next: {}", next.join(" · ")).fg(DIM)),
    ];
    let title = if drill.started() {
        "stratagem"
    } else {
        "enter the code to start"
    };
    f.render_widget(
        Paragraph::new(text)
            .alignment(Alignment::Center)
            .block(panel(title)),
        rect,
    );
}

/// A boxed row of label/value columns.
fn summary(f: &mut Frame, items: &[(&str, String)], area: Rect) {
    let block = panel("result");
    let cols = Layout::horizontal(items.iter().map(|_| Constraint::Fill(1))).split(block.inner(area));
    f.render_widget(block, area);
    for ((label, value), col) in items.iter().zip(cols.iter()) {
        let text = vec![
            Line::from(label.fg(DIM)),
            Line::from(value.clone().fg(ACCENT).bold()),
        ];
        f.render_widget(Paragraph::new(text).alignment(Alignment::Center), *col);
    }
}

/// Line chart from x = 1 to `max_x`, y from 0 to a round value above every point.
fn chart<'a>(title: &'a str, datasets: Vec<Dataset<'a>>, max_x: f64, max_y: f64) -> Chart<'a> {
    let max_x = max_x.max(2.0);
    let step = if max_y > 10.0 { 10.0 } else { 1.0 };
    let max_y = (max_y / step).ceil().max(1.0) * step;
    let precision = if max_y < 10.0 { 1 } else { 0 };
    let label = |v: f64| Span::from(format!("{v:.precision$}")).fg(DIM);
    Chart::new(datasets)
        .block(panel(title))
        .x_axis(
            Axis::default()
                .bounds([1.0, max_x])
                .labels(["1".to_string().fg(DIM), format!("{max_x:.0}").fg(DIM)])
                .fg(DIM),
        )
        .y_axis(
            Axis::default()
                .bounds([0.0, max_y])
                .labels([label(0.0), label(max_y / 2.0), label(max_y)])
                .fg(DIM),
        )
        .legend_position(Some(ratatui::widgets::LegendPosition::TopLeft))
        .hidden_legend_constraints((Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)))
}

fn line<'a>(name: &'a str, color: Color, data: &'a [(f64, f64)]) -> Dataset<'a> {
    Dataset::default()
        .name(name)
        .marker(Marker::Braille)
        .graph_type(GraphType::Line)
        .fg(color)
        .data(data)
}

fn draw_results(f: &mut Frame, stats: &Stats, area: Rect) {
    let [top, bottom] = Layout::vertical([Constraint::Length(4), Constraint::Fill(1)]).areas(area);
    summary(
        f,
        &[
            ("wpm", format!("{:.0}", stats.wpm)),
            ("raw", format!("{:.0}", stats.raw)),
            ("acc", format!("{:.0}%", stats.accuracy)),
            ("consistency", format!("{:.0}%", stats.consistency)),
            (
                "chars",
                format!("{}/{}", stats.correct_chars, stats.incorrect_chars),
            ),
            ("time", format!("{:.0}s", stats.duration)),
        ],
        top,
    );

    let wpm: Vec<(f64, f64)> = stats.samples.iter().map(|s| (s.second as f64, s.wpm)).collect();
    let raw: Vec<(f64, f64)> = stats.samples.iter().map(|s| (s.second as f64, s.raw)).collect();
    let errors: Vec<(f64, f64)> = stats
        .samples
        .iter()
        .filter(|s| s.errors > 0)
        .map(|s| (s.second as f64, s.raw))
        .collect();
    let max_y = raw.iter().chain(&wpm).map(|p| p.1).fold(0.0, f64::max);
    let datasets = vec![
        line("raw", DIM, &raw),
        line("wpm", ACCENT, &wpm),
        Dataset::default()
            .name("errors")
            .marker(Marker::Dot)
            .graph_type(GraphType::Scatter)
            .fg(BAD)
            .data(&errors),
    ];
    f.render_widget(
        chart("wpm / second", datasets, stats.samples.len() as f64, max_y),
        bottom,
    );
}

fn draw_stratagem_results(f: &mut Frame, stats: &DrillStats, area: Rect) {
    let [top, bottom] = Layout::vertical([Constraint::Length(4), Constraint::Fill(1)]).areas(area);
    summary(
        f,
        &[
            ("done", stats.completed.to_string()),
            ("per min", format!("{:.1}", stats.per_minute)),
            ("acc", format!("{:.0}%", stats.accuracy)),
            ("avg", format!("{:.2}s", stats.avg_time)),
            ("best streak", stats.best_streak.to_string()),
            ("time", format!("{:.0}s", stats.duration)),
        ],
        top,
    );
    let times: Vec<(f64, f64)> = stats
        .times
        .iter()
        .enumerate()
        .map(|(i, t)| ((i + 1) as f64, *t))
        .collect();
    let max_y = times.iter().map(|p| p.1).fold(0.0, f64::max);
    let datasets = vec![line("seconds", ACCENT, &times)];
    f.render_widget(
        chart("seconds / stratagem", datasets, times.len() as f64, max_y),
        bottom,
    );
}
