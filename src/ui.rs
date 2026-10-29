use std::time::Instant;

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::symbols::Marker;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Axis, Block, BorderType, Chart, Dataset, GraphType, Paragraph};

use crate::app::{App, Game, Item, Mode, Screen};
use crate::keyboard::{self, FINGER_NAMES, Layout as KeyLayout};
use crate::stratagem::{self, Drill, DrillStats};
use crate::typing::{Next, Stats, Test};

const ACCENT: Color = Color::Cyan;
const DIM: Color = Color::DarkGray;
const BAD: Color = Color::Red;
/// Normal mode (option bar focused): orange, far from the cyan accent used while typing.
const NORMAL: Color = Color::Indexed(208);
/// Lines of words shown at once while typing.
const VISIBLE_LINES: usize = 3;
/// Smallest terminal that fits the results summary and graph; the config bar can ask for more.
const MIN_WIDTH: u16 = 70;
const MIN_HEIGHT: u16 = 16;

fn panel(title: &str) -> Block<'_> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(DIM))
        .title(Span::styled(format!(" {title} "), Style::new().fg(ACCENT).bold()))
}

pub fn draw(f: &mut Frame, app: &App, now: Instant) {
    let bar_line = config_bar(app);
    let min_width = MIN_WIDTH.max(bar_line.width() as u16);
    let area = f.area();
    if area.width < min_width || area.height < MIN_HEIGHT {
        let text = vec![
            Line::from("terminal too small".fg(BAD).bold()),
            Line::from(format!(
                "{}×{}, needs {min_width}×{MIN_HEIGHT}",
                area.width, area.height
            )),
            Line::from("ctrl+c quits".fg(DIM)),
        ];
        let y = area.height.saturating_sub(text.len() as u16) / 2;
        let rect = Rect {
            y: area.y + y,
            height: area.height - y,
            ..area
        };
        f.render_widget(Paragraph::new(text).alignment(Alignment::Center), rect);
        return;
    }
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
    let mut top_block = panel("claptap").title(Line::from(format!(" {mode} ")).right_aligned().fg(DIM));
    if app.menu.is_some() {
        top_block = top_block.border_style(Style::new().fg(NORMAL));
    }
    f.render_widget(Paragraph::new(top_line(app, now)).block(top_block), top);
    f.render_widget(Paragraph::new(bar_line).alignment(Alignment::Center), bar);

    match (&app.screen, &app.game) {
        (Screen::Playing, Game::Typing(test)) => match app.layout {
            i if app.hands && body.height >= KEYBOARD_HEIGHT + VISIBLE_LINES as u16 + 4 => {
                let [words, keys] =
                    Layout::vertical([Constraint::Fill(1), Constraint::Length(KEYBOARD_HEIGHT)]).areas(body);
                draw_typing(f, test, words);
                draw_keyboard(f, &keyboard::LAYOUTS[i], test.next_key(), keys);
            }
            _ => draw_typing(f, test, body),
        },
        (Screen::Playing, Game::Stratagem(drill)) => draw_stratagem(f, drill, body, now),
        (Screen::Results(stats), _) => draw_results(f, stats, body),
        (Screen::StratagemResults(stats), _) => draw_stratagem_results(f, stats, body),
    }

    let keys: &[(&str, &str)] = if app.menu.is_some() {
        &[
            ("←→ hl", "move"),
            ("enter", "select"),
            ("H", "keyboard"),
            ("?", "help"),
            ("esc", "back"),
            ("q", "quit"),
        ]
    } else {
        match (&app.screen, app.mode) {
            (Screen::Playing, Mode::Stratagem) => &[
                ("←↓↑→ hjkl", "code"),
                ("tab", "restart"),
                ("esc", "normal mode"),
                ("ctrl+c", "quit"),
            ],
            _ => &[("tab", "restart"), ("esc", "normal mode"), ("ctrl+c", "quit")],
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
    // Vim-style mode badge, bottom left.
    let (badge, color) = if app.menu.is_some() {
        (" NORMAL ", NORMAL)
    } else {
        (" INSERT ", ACCENT)
    };
    f.render_widget(
        Paragraph::new(Span::styled(
            badge,
            Style::new().bg(color).fg(Color::Black).bold(),
        )),
        Rect {
            width: badge.len() as u16,
            ..help
        },
    );

    if app.help {
        draw_help(f, f.area());
    }
}

const HELP: &[(&str, &[(&str, &str)])] = &[
    (
        "typing",
        &[
            ("tab", "restart the test"),
            ("esc", "normal mode (drops the test)"),
            ("ctrl+c", "quit"),
        ],
    ),
    ("stratagem", &[("←↓↑→ / hjkl", "enter the code")]),
    (
        "normal mode",
        &[
            ("←→ / hl", "move in the option bar"),
            ("enter / space", "pick the option"),
            ("H", "show / hide keyboard and hands"),
            ("?", "this help"),
            ("esc", "back to typing"),
            ("q", "quit"),
        ],
    ),
];

fn draw_help(f: &mut Frame, area: Rect) {
    let mut lines = Vec::new();
    for (section, keys) in HELP {
        if !lines.is_empty() {
            lines.push(Line::default());
        }
        lines.push(Line::from(section.fg(ACCENT).bold()));
        for (key, action) in keys.iter() {
            lines.push(Line::from(vec![format!("  {key:<15}").bold(), action.fg(DIM)]));
        }
    }
    lines.push(Line::default());
    lines.push(Line::from("any key to close".fg(DIM)).centered());
    let height = lines.len() as u16 + 2;
    let rect = centered(area, 56.min(area.width), height.min(area.height));
    f.render_widget(ratatui::widgets::Clear, rect);
    let block = panel("help").padding(ratatui::widgets::Padding::horizontal(1));
    f.render_widget(Paragraph::new(lines).block(block), rect);
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
            Item::Lang => app.lang.clone(),
            Item::Keyboard => format!("⌨ {}", keyboard::LAYOUTS[app.layout].name),
        };
        let mut style = match (app.is_selected(item), app.menu.is_some()) {
            (true, _) => Style::new().fg(ACCENT).bold(),
            (false, true) => Style::new(),
            (false, false) => Style::new().fg(DIM),
        };
        if app.menu == Some(i) {
            style = style.bg(NORMAL).fg(Color::Black);
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

const KEYBOARD_HEIGHT: u16 = 9;
/// Left offset of each key row, in columns: ISO stagger with 4-column keys (tab 1.5, caps 1.75, shift 1.25).
const ROW_OFFSETS: [usize; 4] = [0, 6, 7, 5];
const ROW_PREFIX: [&str; 4] = ["", " ⇥", " ⇪", " ⇧"];
const ROW_SUFFIX: [&str; 4] = [" ⌫", "", "", "  ⇧"];
/// Columns where the space bar starts (under C on QWERTY) and its width.
const SPACE_AT: usize = 17;
const SPACE_WIDTH: usize = 19;

/// (vivid, dim) colors per finger kind: pinky, ring, middle, index, thumb. Same on both hands.
const FINGER_COLORS: [(u8, u8); 5] = [(170, 53), (75, 24), (114, 22), (221, 58), (250, 238)];
const FINGER_KIND: [usize; 9] = [0, 1, 2, 3, 3, 2, 1, 0, 4];
const FINGER_SHORT: [&str; 5] = ["pinky", "ring", "middle", "index", "thumb"];

fn finger_color(finger: u8, vivid: bool) -> Color {
    let (v, d) = FINGER_COLORS[FINGER_KIND[finger as usize]];
    Color::Indexed(if vivid { v } else { d })
}

/// Key cell: dim background of its finger's color, vivid when that finger is up next,
/// white for the key itself (bold too, so it never relies on color alone).
fn key_style(finger: u8, active: &[u8], target: bool) -> Style {
    match (target, active.contains(&finger)) {
        (true, _) => Style::new().bg(Color::Indexed(231)).fg(Color::Indexed(16)).bold(),
        (false, true) => Style::new().bg(finger_color(finger, true)).fg(Color::Indexed(16)),
        (false, false) => Style::new()
            .bg(finger_color(finger, false))
            .fg(Color::Indexed(252)),
    }
}

fn draw_keyboard(f: &mut Frame, layout: &KeyLayout, next: Option<Next>, area: Rect) {
    let hint = next.map(|n| layout.hint(n));
    let active: &[u8] = hint.as_ref().map_or(&[], |h| &h.fingers);
    let target = hint.as_ref().and_then(|h| h.key.as_ref());
    let shift_on = target.is_some_and(|k| k.shift);

    let mut lines = Vec::new();
    for (row, keys) in layout.rows.iter().enumerate() {
        let prefix_style = if row == 3 && shift_on && active.contains(&0) {
            Style::new().fg(finger_color(0, true)).bold()
        } else {
            DIM.into()
        };
        let mut spans = vec![Span::styled(
            format!("{:<w$}", ROW_PREFIX[row], w = ROW_OFFSETS[row]),
            prefix_style,
        )];
        for (col, k) in keys.chars().enumerate() {
            let is_target = target.is_some_and(|t| (t.row, t.col) == (row, col));
            let mut style = key_style(keyboard::finger(row, col), active, is_target);
            // The bumps on F and J (home row index keys).
            if row == 2 && (col == 3 || col == 6) {
                style = style.underlined();
            }
            spans.push(Span::styled(format!(" {k} "), style));
            spans.push(" ".into());
        }
        let suffix_style = match (row, next) {
            (0, Some(Next::Backspace)) => Style::new().fg(BAD).bold(),
            (3, _) if shift_on && active.contains(&7) => Style::new().fg(finger_color(7, true)).bold(),
            _ => DIM.into(),
        };
        spans.push(Span::styled(ROW_SUFFIX[row], suffix_style));
        lines.push(Line::from(spans));
    }
    let space = key_style(keyboard::THUMB, active, false);
    lines.push(Line::from(vec![
        " ".repeat(SPACE_AT).into(),
        Span::styled(format!("{:^SPACE_WIDTH$}", "space"), space),
    ]));
    lines.push(Line::default());

    let text = match (next, target) {
        (Some(Next::Backspace), _) => vec!["⌫ backspace".fg(BAD).bold(), "  ·  right pinky".fg(DIM)],
        (Some(Next::Char(' ')), _) => vec!["space".bold(), "  ·  thumb".fg(DIM)],
        (Some(Next::Char(c)), Some(k)) => {
            let mut v = vec![
                c.to_string().bold(),
                format!("  ·  {}", FINGER_NAMES[k.finger as usize]).fg(DIM),
            ];
            if k.shift {
                v.push(format!(" + {} on ⇧", FINGER_NAMES[active[1] as usize]).fg(DIM));
            }
            v
        }
        (Some(Next::Char(c)), None) => vec![format!("{c} is not on {}", layout.name).fg(DIM)],
        (None, _) => vec![],
    };
    lines.push(Line::from(text).centered());

    let board_width = lines.iter().map(|l| l.width()).max().unwrap_or(0) as u16;
    let hands = hands(active);
    let hands_width = hands.iter().map(|l| l.width()).max().unwrap_or(0) as u16;
    let gap = 4;
    let with_hands = area.width >= board_width + gap + hands_width + 4;
    let width = if with_hands {
        board_width + gap + hands_width
    } else {
        board_width
    };
    let rect = centered(area, (width + 4).min(area.width), KEYBOARD_HEIGHT);
    let block = panel(layout.name).padding(ratatui::widgets::Padding::horizontal(1));
    let inner = block.inner(rect);
    f.render_widget(block, rect);
    let [board, _, right] = Layout::horizontal([
        Constraint::Length(board_width),
        Constraint::Length(gap),
        Constraint::Fill(1),
    ])
    .areas(inner);
    f.render_widget(Paragraph::new(lines), board);
    if with_hands {
        f.render_widget(Paragraph::new(hands), right);
    }
}

/// Two hands drawn with box characters, fingers colored like their keys; the active ones filled.
fn hands(active: &[u8]) -> Vec<Line<'static>> {
    // Fingers left to right on screen: left pinky..index, then right index..pinky.
    const HEIGHTS: [usize; 8] = [2, 3, 4, 3, 3, 4, 3, 2];
    const GAP: &str = "      ";
    let finger_cell = |finger: u8, row: usize| -> Span<'static> {
        let h = HEIGHTS[finger as usize];
        let on = active.contains(&finger);
        let glyph = match (row + h).cmp(&4) {
            std::cmp::Ordering::Less => return " ".into(),
            std::cmp::Ordering::Equal => {
                if on {
                    "▄"
                } else {
                    "╻"
                }
            }
            std::cmp::Ordering::Greater => {
                if on {
                    "█"
                } else {
                    "┃"
                }
            }
        };
        Span::styled(glyph, Style::new().fg(finger_color(finger, true)))
    };
    let mut lines = Vec::new();
    for row in 0..4 {
        let mut spans = Vec::new();
        for finger in 0..4u8 {
            spans.push(finger_cell(finger, row));
            spans.push(" ".into());
        }
        spans.push(format!(" {GAP} ").into());
        for finger in 4..8u8 {
            spans.push(" ".into());
            spans.push(finger_cell(finger, row));
        }
        lines.push(Line::from(spans));
    }
    let thumb = if active.contains(&keyboard::THUMB) {
        Style::new().fg(finger_color(keyboard::THUMB, true)).bold()
    } else {
        DIM.into()
    };
    lines.push(Line::from("╰─┴─┴─┴─╮      ╭─┴─┴─┴─╯".fg(DIM)));
    lines.push(Line::from(vec![
        "        ╰─ ".fg(DIM),
        Span::styled("▂▂", thumb),
        " ─╯".fg(DIM),
    ]));
    let label = match active.first() {
        Some(&f) if f == keyboard::THUMB => "thumb".to_string(),
        Some(&f) => format!(
            "{} · {}",
            if f < 4 { "left" } else { "right" },
            FINGER_SHORT[FINGER_KIND[f as usize]]
        ),
        None => String::new(),
    };
    lines.push(Line::from(format!("{label:^23}").fg(DIM)));
    lines
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
