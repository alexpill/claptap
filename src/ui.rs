use std::time::Instant;

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::symbols::Marker;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Axis, Block, BorderType, Chart, Dataset, GraphType, Paragraph};

use crate::app::{App, Screen};
use crate::typing::{Limit, Stats, Test};

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
    let [top, body, help] =
        Layout::vertical([Constraint::Length(3), Constraint::Fill(1), Constraint::Length(1)]).areas(f.area());

    let mode = format!("{} {} · {}", app.limit.name(), app.limit.value(), app.lang);
    let top_block = panel("claptap").title(Line::from(format!(" {mode} ")).right_aligned().fg(DIM));
    f.render_widget(Paragraph::new(top_line(app, now)).block(top_block), top);

    match &app.screen {
        Screen::Typing => draw_typing(f, &app.test, body),
        Screen::Results(stats) => draw_results(f, stats, app.save_error.as_deref(), body),
    }

    let keys = Line::from(vec![
        "tab".fg(ACCENT),
        " restart  ".fg(DIM),
        "esc".fg(ACCENT),
        " quit".fg(DIM),
    ]);
    f.render_widget(Paragraph::new(keys).alignment(Alignment::Center), help);
}

fn top_line(app: &App, now: Instant) -> Line<'static> {
    let t = &app.test;
    let progress = match app.limit {
        Limit::Time(s) => format!("{:.0}s", (s as f64 - t.elapsed(now)).max(0.0).ceil()),
        Limit::Words(n) => format!("{}/{n}", t.typed.len() - usize::from(!t.finished())),
    };
    let stat = |label: &'static str, value: String| [format!(" {label} ").fg(DIM), value.bold()];
    let mut spans = Vec::new();
    spans.extend(stat("wpm", format!("{:.0}", t.live_wpm(now))));
    spans.extend(stat("acc", format!("{:.0}%", t.live_accuracy())));
    spans.extend(stat(
        if matches!(app.limit, Limit::Time(_)) {
            "left"
        } else {
            "words"
        },
        progress,
    ));
    Line::from(spans)
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

fn draw_results(f: &mut Frame, stats: &Stats, save_error: Option<&str>, area: Rect) {
    let [summary, chart] = Layout::vertical([Constraint::Length(4), Constraint::Fill(1)]).areas(area);

    let items = [
        ("wpm", format!("{:.0}", stats.wpm)),
        ("raw", format!("{:.0}", stats.raw)),
        ("acc", format!("{:.0}%", stats.accuracy)),
        ("consistency", format!("{:.0}%", stats.consistency)),
        (
            "chars",
            format!("{}/{}", stats.correct_chars, stats.incorrect_chars),
        ),
        ("time", format!("{:.0}s", stats.duration)),
    ];
    let block = panel("result");
    let cols = Layout::horizontal(items.iter().map(|_| Constraint::Fill(1))).split(block.inner(summary));
    f.render_widget(block, summary);
    for ((label, value), col) in items.iter().zip(cols.iter()) {
        let text = vec![
            Line::from(label.fg(DIM)),
            Line::from(value.clone().fg(ACCENT).bold()),
        ];
        f.render_widget(Paragraph::new(text).alignment(Alignment::Center), *col);
    }

    let wpm: Vec<(f64, f64)> = stats.samples.iter().map(|s| (s.second as f64, s.wpm)).collect();
    let raw: Vec<(f64, f64)> = stats.samples.iter().map(|s| (s.second as f64, s.raw)).collect();
    let errors: Vec<(f64, f64)> = stats
        .samples
        .iter()
        .filter(|s| s.errors > 0)
        .map(|s| (s.second as f64, s.raw))
        .collect();
    let max_x = stats.samples.len().max(2) as f64;
    let max_y = (raw.iter().chain(&wpm).map(|p| p.1).fold(0.0, f64::max) / 10.0)
        .ceil()
        .max(1.0)
        * 10.0;

    let datasets = vec![
        Dataset::default()
            .name("raw")
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .fg(DIM)
            .data(&raw),
        Dataset::default()
            .name("wpm")
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .fg(ACCENT)
            .data(&wpm),
        Dataset::default()
            .name("errors")
            .marker(Marker::Dot)
            .graph_type(GraphType::Scatter)
            .fg(BAD)
            .data(&errors),
    ];
    let label = |v: f64| Span::from(format!("{v:.0}")).fg(DIM);
    let mut block = panel("wpm / second");
    if let Some(e) = save_error {
        block = block.title_bottom(Line::from(format!(" {e} ")).fg(BAD));
    }
    let chart_widget = Chart::new(datasets)
        .block(block)
        .x_axis(
            Axis::default()
                .bounds([1.0, max_x])
                .labels([label(1.0), label(max_x)])
                .fg(DIM),
        )
        .y_axis(
            Axis::default()
                .bounds([0.0, max_y])
                .labels([label(0.0), label(max_y / 2.0), label(max_y)])
                .fg(DIM),
        )
        .legend_position(Some(ratatui::widgets::LegendPosition::TopLeft))
        .hidden_legend_constraints((Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)));
    f.render_widget(chart_widget, chart);
}
