//! Rendering of the option form and the run view.

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Position, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, HighlightSpacing, List, ListItem, Paragraph, Wrap},
};
use tui_term::widget::{Cursor, PseudoTerminal};

use crate::{
    app::{App, Entry, Focus, Mode, Selection, Value},
    runner::Runner,
    status,
};

const HIGHLIGHT: &str = "› ";
const LABEL_WIDTH: usize = 9;
const TEXT_FLAG_WIDTH: usize = "-x ".len();

pub fn draw(frame: &mut Frame, app: &mut App) {
    if let Mode::Run(runner) = &app.mode {
        draw_run(frame, runner, &app.command);
    } else {
        draw_form(frame, app);
    }
}

/// Area of the embedded terminal inside the run view.
pub fn terminal_area(area: Rect) -> Rect {
    let [body, _] = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(area);
    Block::bordered().inner(body)
}

fn keys(pairs: &[(&str, &str)]) -> Line<'static> {
    let mut spans = Vec::new();
    for (i, (key, action)) in pairs.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("  "));
        }
        spans.push(Span::styled(
            key.to_string(),
            Style::new().bold().fg(Color::Cyan),
        ));
        spans.push(Span::raw(format!(" {action}")));
    }
    Line::from(spans)
}

fn panel(title: &str, focused: bool) -> Block<'_> {
    let block = Block::bordered().title(format!(" {title} "));
    if focused {
        block.border_style(Style::new().fg(Color::Cyan))
    } else {
        block
    }
}

fn draw_form(frame: &mut Frame, app: &mut App) {
    let editing = matches!(app.mode, Mode::Edit { .. });
    let hints = app.hints();

    let [main, command, footer] = Layout::vertical([
        Constraint::Min(8),
        Constraint::Length(4),
        Constraint::Length(1),
    ])
    .areas(frame.area());
    let [left, middle, right] = Layout::horizontal([
        Constraint::Length(46),
        Constraint::Length(34),
        Constraint::Min(30),
    ])
    .areas(main);
    let [nodes_area, tags_area] =
        Layout::vertical([Constraint::Fill(2), Constraint::Fill(1)]).areas(middle);
    let [options_area, inputs_area] = Layout::vertical([
        Constraint::Length(app.fields.len() as u16 + 2),
        Constraint::Min(3),
    ])
    .areas(left);
    let hint_height = if hints.is_empty() {
        0
    } else {
        hints.len() as u16 + 2
    };
    let [status_area, hints_area] =
        Layout::vertical([Constraint::Min(5), Constraint::Length(hint_height)]).areas(right);

    // Options
    let placeholder = |flag: char| match flag {
        'f' => "saved flake",
        _ => "none",
    };
    let items: Vec<ListItem> = app
        .fields
        .iter()
        .map(|field| {
            let flag = Span::styled(format!("-{}", field.flag), Style::new().dark_gray());
            let line = match &field.value {
                Value::Toggle(on) => Line::from(vec![
                    if *on {
                        Span::styled("[x] ", Style::new().green().bold())
                    } else {
                        Span::raw("[ ] ")
                    },
                    Span::raw(format!("{:<width$}", field.label, width = LABEL_WIDTH + 13)),
                    flag,
                ]),
                Value::Text(text) => Line::from(vec![
                    Span::raw(format!("{:<LABEL_WIDTH$}", field.label)),
                    Span::styled(format!("-{} ", field.flag), Style::new().dark_gray()),
                    if text.is_empty() {
                        Span::styled(
                            format!("({})", placeholder(field.flag)),
                            Style::new().dark_gray(),
                        )
                    } else {
                        Span::styled(text.clone(), Style::new().yellow())
                    },
                ]),
            };
            ListItem::new(line)
        })
        .collect();
    let options_focused = app.focus == Focus::Options;
    let list = List::new(items)
        .block(panel("Options", options_focused))
        .highlight_symbol(HIGHLIGHT)
        .highlight_spacing(HighlightSpacing::Always)
        .highlight_style(if options_focused {
            Style::new().reversed()
        } else {
            Style::new().bold()
        });
    frame.render_stateful_widget(list, options_area, &mut app.options_state);

    if editing
        && let Some(index) = app.options_state.selected()
        && let Value::Text(text) = &app.fields[index].value
    {
        let inner = Block::bordered().inner(options_area);
        let x = inner.x
            + (HIGHLIGHT.chars().count() + LABEL_WIDTH + TEXT_FLAG_WIDTH + text.chars().count())
                as u16;
        let y = inner.y + (index - app.options_state.offset()) as u16;
        frame.set_cursor_position(Position::new(x.min(inner.right().saturating_sub(1)), y));
    }

    // Inputs
    let update_all = app.toggled('u');
    let title = if update_all {
        "Update inputs -U (all via -u)"
    } else {
        "Update inputs -U"
    };
    let block = panel(title, app.focus == Focus::Inputs);
    draw_checklist(
        frame,
        inputs_area,
        &mut app.inputs,
        block,
        app.focus == Focus::Inputs,
        update_all,
        |_| None,
    );

    // Nodes and tags
    let mut block = panel("Nodes -N", app.focus == Focus::Nodes);
    if app.hive.loading() {
        block = block.title_bottom(Line::from(" evaluating hive… ").yellow());
    } else if let Some(err) = &app.hive.error {
        block = block.title_bottom(Line::from(format!(" {err} ")).red());
    }
    let tags_of = |entry: &Entry| {
        app.hive
            .nodes
            .get(&entry.name)
            .filter(|tags| !tags.is_empty())
            .map(|tags| tags.join(", "))
    };
    draw_checklist(
        frame,
        nodes_area,
        &mut app.nodes,
        block,
        app.focus == Focus::Nodes,
        false,
        tags_of,
    );
    let node_count = |entry: &Entry| {
        let count = app
            .hive
            .nodes
            .values()
            .filter(|tags| tags.contains(&entry.name))
            .count();
        Some(format!("{count} node(s)"))
    };
    draw_checklist(
        frame,
        tags_area,
        &mut app.tags,
        panel("Tags -T", app.focus == Focus::Tags),
        app.focus == Focus::Tags,
        false,
        node_count,
    );

    // Status
    frame.render_widget(
        Paragraph::new(status_lines(app))
            .block(panel("Status", false))
            .wrap(Wrap { trim: false }),
        status_area,
    );

    if !hints.is_empty() {
        let lines: Vec<Line> = hints
            .iter()
            .map(|h| Line::from(format!("! {h}")).yellow())
            .collect();
        frame.render_widget(
            Paragraph::new(lines).block(panel("Hints", false)),
            hints_area,
        );
    }

    // Command preview
    let help = match app.focus {
        Focus::Options => app.selected_field().map_or("", |f| f.help),
        Focus::Inputs => "Update only the selected inputs before running",
        Focus::Nodes => "Deploy to the selected nodes (nothing selected: all nodes)",
        Focus::Tags => "Deploy to all nodes with one of the selected tags",
    };
    let lines = vec![
        Line::from(help).dark_gray(),
        Line::from(vec![
            Span::styled("$ ", Style::new().green()),
            Span::raw(app.command.clone()),
        ]),
    ];
    frame.render_widget(
        Paragraph::new(lines).block(panel("Command", false)),
        command,
    );

    let footer_keys = if let Some(error) = &app.error {
        Line::from(format!("Error: {error}")).red().bold()
    } else if editing {
        keys(&[
            ("Enter", "apply"),
            ("Esc", "cancel"),
            ("C-u", "clear"),
            ("C-⌫", "delete word"),
        ])
    } else {
        keys(&[
            ("j/k", "move"),
            ("Space", "toggle/edit"),
            ("p…f", "set flag"),
            ("Tab", "next list"),
            ("r", "run HIVE"),
            ("R", "reload"),
            ("q", "quit"),
        ])
    };
    frame.render_widget(footer_keys, footer);
}

/// A list of entries with checkboxes and an optional dimmed detail per
/// entry; `disabled` greys out the list and its checkmarks.
fn draw_checklist(
    frame: &mut Frame,
    area: Rect,
    selection: &mut Selection,
    block: Block,
    focused: bool,
    disabled: bool,
    detail: impl Fn(&Entry) -> Option<String>,
) {
    let items: Vec<ListItem> = selection
        .entries
        .iter()
        .map(|entry| {
            let check = if entry.selected && !disabled {
                Span::styled("[x] ", Style::new().green().bold())
            } else {
                Span::raw("[ ] ")
            };
            let mut spans = vec![check, Span::raw(entry.name.clone())];
            if let Some(detail) = detail(entry) {
                spans.push(Span::styled(
                    format!("  {detail}"),
                    Style::new().dark_gray(),
                ));
            }
            ListItem::new(Line::from(spans))
        })
        .collect();
    let list = List::new(items)
        .block(block)
        .style(if disabled {
            Style::new().dark_gray()
        } else {
            Style::new()
        })
        .highlight_symbol(HIGHLIGHT)
        .highlight_spacing(HighlightSpacing::Always)
        .highlight_style(if focused {
            Style::new().reversed()
        } else {
            Style::new()
        });
    frame.render_stateful_widget(list, area, &mut selection.state);
}

fn status_lines(app: &App) -> Vec<Line<'static>> {
    let status = &app.status;
    let row = |label: &str, value: Span<'static>| {
        Line::from(vec![
            Span::styled(format!("{label:<10}"), Style::new().bold()),
            value,
        ])
    };
    let dim = |text: String| Span::styled(text, Style::new().dark_gray());

    let mut lines = Vec::new();

    match &status.flake {
        Ok(path) => lines.push(row("Flake", Span::raw(status::display_path(path)))),
        Err(err) => {
            lines.push(row("Flake", Span::styled(err.clone(), Style::new().red())));
            return lines;
        }
    }

    lines.push(row(
        "Targets",
        match app.targets() {
            Some(targets) if targets.is_empty() => {
                Span::styled("none - the selection matches no node", Style::new().red())
            }
            Some(targets) if targets.len() == app.hive.nodes.len() => {
                Span::raw(format!("all ({} nodes)", targets.len()))
            }
            Some(targets) => Span::raw(targets.join(", ")),
            None if app.hive.loading() => Span::styled("evaluating hive…", Style::new().yellow()),
            None => dim("unknown - the hive could not be evaluated".to_owned()),
        },
    ));
    lines.push(row(
        "Upstream",
        match &status.upstream {
            Some(upstream) => Span::raw(upstream.clone()),
            None => Span::styled("none - pushing will fail", Style::new().yellow()),
        },
    ));
    if let Some(count) = status.unpushed {
        let style = if count > 0 {
            Style::new().yellow()
        } else {
            Style::new()
        };
        lines.push(row(
            "Unpushed",
            Span::styled(format!("{count} commit(s)"), style),
        ));
    }
    if let Some(head) = &status.head {
        lines.push(row("HEAD", Span::raw(head.clone())));
    }

    lines.push(Line::default());
    if status.changes.is_empty() {
        lines.push(row(
            "Changes",
            dim("none - HIVE will ask before deploying anyway".to_owned()),
        ));
    } else {
        lines.push(row(
            "Changes",
            Span::raw(format!(
                "{} file(s), will be staged and committed",
                status.changes.len()
            )),
        ));
        for change in &status.changes {
            let (code, file) = change.split_at(change.len().min(3));
            let color = match code.trim() {
                "??" | "A" => Color::Green,
                "D" => Color::Red,
                _ => Color::Yellow,
            };
            lines.push(Line::from(vec![
                Span::styled(format!("  {code:<3}"), Style::new().fg(color)),
                Span::raw(file.to_owned()),
            ]));
        }
    }
    lines
}

fn draw_run(frame: &mut Frame, runner: &Runner, command: &str) {
    let [body, footer] =
        Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(frame.area());

    let (color, state) = match &runner.exit {
        None => (Color::Yellow, "running".to_owned()),
        Some(exit) if exit.success() => (Color::Green, "finished".to_owned()),
        Some(exit) => (
            Color::Red,
            format!("failed (exit code {})", exit.exit_code()),
        ),
    };

    let parser = runner.parser.lock().unwrap();
    let screen = parser.screen();
    let scrolled = screen.scrollback();

    let mut title = vec![
        Span::raw(" "),
        Span::styled(state, Style::new().fg(color).add_modifier(Modifier::BOLD)),
    ];
    if scrolled > 0 {
        title.push(Span::raw(format!(" · scrolled back {scrolled} lines")));
    }
    title.push(Span::raw(" "));

    let block = Block::bordered()
        .border_style(Style::new().fg(color))
        .title(Line::from(title))
        .title_bottom(Line::from(format!(" $ {command} ")).dark_gray());
    let cursor = Cursor::default().visibility(runner.running() && scrolled == 0);
    frame.render_widget(
        PseudoTerminal::new(screen).block(block).cursor(cursor),
        body,
    );

    let footer_keys = if runner.running() {
        keys(&[
            ("PgUp/PgDn", "scroll"),
            ("keys", "go to HIVE"),
            ("C-c", "abort HIVE"),
        ])
    } else {
        keys(&[("PgUp/PgDn", "scroll"), ("q/Enter", "back to options")])
    };
    frame.render_widget(footer_keys, footer);
}
