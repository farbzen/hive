//! Application state and key handling.

use std::time::Duration;

use anyhow::{Context, Result};
use ratatui::{
    DefaultTerminal,
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    widgets::ListState,
};

use crate::{
    EXEC_FLAG,
    runner::Runner,
    status::{self, Hive, Status},
    ui,
};

pub enum Value {
    Toggle(bool),
    Text(String),
}

/// One HIVE option.
pub struct Field {
    pub flag: char,
    pub label: &'static str,
    pub help: &'static str,
    pub value: Value,
}

impl Field {
    fn toggle(flag: char, label: &'static str, help: &'static str) -> Self {
        Self {
            flag,
            label,
            help,
            value: Value::Toggle(false),
        }
    }

    fn text(flag: char, label: &'static str, help: &'static str) -> Self {
        Self {
            flag,
            label,
            help,
            value: Value::Text(String::new()),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Focus {
    Options,
    Inputs,
    Nodes,
    Tags,
}

const FOCUS_ORDER: [Focus; 4] = [Focus::Options, Focus::Inputs, Focus::Nodes, Focus::Tags];

pub enum Mode {
    Form,
    /// Editing a text field; holds the value to restore on Esc.
    Edit {
        backup: String,
    },
    Run(Runner),
}

pub struct Entry {
    pub name: String,
    pub selected: bool,
}

/// A list of names that can be selected one by one, with its cursor.
#[derive(Default)]
pub struct Selection {
    pub entries: Vec<Entry>,
    pub state: ListState,
}

impl Selection {
    /// Replaces the entries with `names`, keeping the selection of names
    /// that remain.
    fn sync(&mut self, names: &[String]) {
        let previous = std::mem::take(&mut self.entries);
        self.entries = names
            .iter()
            .map(|name| Entry {
                name: name.clone(),
                selected: previous.iter().any(|e| e.selected && &e.name == name),
            })
            .collect();
        let cursor = self.state.selected().unwrap_or(0);
        self.state
            .select((!self.entries.is_empty()).then(|| cursor.min(self.entries.len() - 1)));
    }

    pub fn selected_names(&self) -> Vec<String> {
        self.entries
            .iter()
            .filter(|e| e.selected)
            .map(|e| e.name.clone())
            .collect()
    }

    fn any_selected(&self) -> bool {
        self.entries.iter().any(|e| e.selected)
    }

    fn toggle_current(&mut self) {
        if let Some(entry) = self.state.selected().and_then(|i| self.entries.get_mut(i)) {
            entry.selected = !entry.selected;
        }
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

pub struct App {
    pub fields: Vec<Field>,
    pub inputs: Selection,
    pub nodes: Selection,
    pub tags: Selection,
    pub focus: Focus,
    pub options_state: ListState,
    pub mode: Mode,
    pub status: Status,
    pub hive: Hive,
    pub command: String,
    /// Shown in the footer until the next key press.
    pub error: Option<String>,
    quit: bool,
}

fn default_fields() -> Vec<Field> {
    vec![
        Field::toggle('p', "Pull", "Pull new commits from origin before running"),
        Field::toggle(
            'u',
            "Update all inputs",
            "Update all flake inputs before running",
        ),
        Field::toggle(
            'b',
            "Build only",
            "Build the configurations without deploying them",
        ),
        Field::toggle(
            'd',
            "Dry run",
            "Deploy with dry-activate: only state what WOULD change",
        ),
        Field::toggle('t', "Show trace", "Show trace of the deployment"),
        Field::toggle(
            'n',
            "No rebuild",
            "Skip the deployment and only push existing commits",
        ),
        Field::toggle(
            'y',
            "Yes to all",
            "Answer all prompts with yes (non-interactive)",
        ),
        Field::toggle('P', "Push", "Push new commits to origin"),
        Field::toggle('S', "Don't squash", "Don't squash commits before pushing"),
        Field::text('c', "Comment", "Add a comment to the resulting commit"),
        Field::text(
            'f',
            "Flake",
            "Path to the flake (default: ~/.config/hive/flake)",
        ),
    ]
}

impl App {
    pub fn new() -> Self {
        let mut app = Self {
            fields: default_fields(),
            inputs: Selection::default(),
            nodes: Selection::default(),
            tags: Selection::default(),
            focus: Focus::Options,
            options_state: ListState::default().with_selected(Some(0)),
            mode: Mode::Form,
            status: status::load(""),
            hive: Hive::default(),
            command: String::new(),
            error: None,
            quit: false,
        };
        app.hive.load(app.status.flake.as_deref().ok());
        app.sync_inputs();
        app.update_command();
        app
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        while !self.quit {
            if let Mode::Run(runner) = &mut self.mode {
                runner.poll();
            }
            if self.hive.loading() {
                self.hive.poll();
                if !self.hive.loading() {
                    self.sync_hive();
                    self.update_command();
                }
            }

            terminal.draw(|frame| ui::draw(frame, self))?;

            if !event::poll(Duration::from_millis(30))? {
                continue;
            }
            match event::read()? {
                Event::Key(key) if key.kind != KeyEventKind::Release => {
                    self.on_key(key, terminal)?
                }
                Event::Resize(cols, rows) => {
                    if let Mode::Run(runner) = &mut self.mode {
                        let area = ui::terminal_area(ratatui::layout::Rect::new(0, 0, cols, rows));
                        runner.resize(area.height, area.width);
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn field(&self, flag: char) -> &Field {
        self.fields
            .iter()
            .find(|f| f.flag == flag)
            .expect("unknown flag")
    }

    pub fn toggled(&self, flag: char) -> bool {
        matches!(self.field(flag).value, Value::Toggle(true))
    }

    pub fn text(&self, flag: char) -> &str {
        match &self.field(flag).value {
            Value::Text(text) => text.trim(),
            Value::Toggle(_) => "",
        }
    }

    pub fn selected_field(&self) -> Option<&Field> {
        self.options_state
            .selected()
            .and_then(|i| self.fields.get(i))
    }

    /// The arguments HIVE will be called with.
    pub fn args(&self) -> Vec<String> {
        let mut args = Vec::new();
        for field in &self.fields {
            match &field.value {
                Value::Toggle(true) => args.push(format!("-{}", field.flag)),
                Value::Text(text) if !text.trim().is_empty() => {
                    args.push(format!("-{}", field.flag));
                    let text = text.trim();
                    args.push(if field.flag == 'f' {
                        hive::flake::expand_tilde(text)
                    } else {
                        text.to_owned()
                    });
                }
                _ => {}
            }
        }
        let mut repeat = |flag: &str, names: Vec<String>| {
            for name in names {
                args.push(flag.to_owned());
                args.push(name);
            }
        };
        if !self.toggled('u') {
            repeat("-U", self.inputs.selected_names());
        }
        repeat("-N", self.nodes.selected_names());
        repeat("-T", self.tags.selected_names());
        args
    }

    fn update_command(&mut self) {
        let mut parts = vec!["hive".to_owned()];
        parts.extend(self.args().iter().map(|arg| shell_quote(arg)));
        self.command = parts.join(" ");
    }

    /// Combinations HIVE accepts but silently ignores.
    pub fn hints(&self) -> Vec<String> {
        let mut hints = Vec::new();
        if self.toggled('S') && !self.toggled('P') {
            hints.push("Don't squash has no effect without Push".to_owned());
        }
        if self.toggled('P') && (self.toggled('d') || self.toggled('b')) {
            hints.push("Push is skipped in dry/build-only mode".to_owned());
        }
        if !self.text('c').is_empty() && (self.toggled('d') || self.toggled('b')) {
            hints.push("No commit (and no comment) in dry/build-only mode".to_owned());
        }
        if self.toggled('n')
            && (self.toggled('u')
                || self.inputs.any_selected()
                || !self.text('c').is_empty()
                || self.toggled('b')
                || self.toggled('t'))
        {
            hints.push(
                "No rebuild can't be combined with update, comment, build only or trace".to_owned(),
            );
        }
        if self.toggled('u') && self.inputs.any_selected() {
            hints.push("Update all inputs overrides the input selection".to_owned());
        }
        hints
    }

    /// The nodes the deployment reaches, once the hive has been evaluated.
    pub fn targets(&self) -> Option<Vec<String>> {
        (!self.hive.nodes.is_empty()).then(|| {
            hive::colmena::resolve(
                &self.hive.nodes,
                &self.nodes.selected_names(),
                &self.tags.selected_names(),
            )
        })
    }

    fn reload_status(&mut self) {
        self.status = status::load(self.text('f'));
        self.hive.load(self.status.flake.as_deref().ok());
        self.sync_inputs();
        self.sync_hive();
    }

    fn sync_inputs(&mut self) {
        self.inputs.sync(&self.status.inputs);
    }

    /// Rebuilds the node and tag lists from the evaluated hive, keeping the
    /// selection.
    fn sync_hive(&mut self) {
        let names: Vec<String> = self.hive.nodes.keys().cloned().collect();
        self.nodes.sync(&names);
        self.tags.sync(&self.hive.tags());
        if self.selection(self.focus).is_some_and(Selection::is_empty) {
            self.focus = Focus::Options;
        }
    }

    fn selection(&self, focus: Focus) -> Option<&Selection> {
        match focus {
            Focus::Options => None,
            Focus::Inputs => Some(&self.inputs),
            Focus::Nodes => Some(&self.nodes),
            Focus::Tags => Some(&self.tags),
        }
    }

    /// Moves the focus to the next (or previous) panel, skipping empty lists.
    fn cycle_focus(&mut self, forward: bool) {
        let current = FOCUS_ORDER
            .iter()
            .position(|&focus| focus == self.focus)
            .unwrap_or(0);
        let len = FOCUS_ORDER.len();
        let next = (1..len)
            .map(|step| {
                let offset = if forward { step } else { len - step };
                FOCUS_ORDER[(current + offset) % len]
            })
            .find(|&focus| !self.selection(focus).is_some_and(Selection::is_empty));
        if let Some(focus) = next {
            self.focus = focus;
        }
    }

    fn on_key(&mut self, key: KeyEvent, terminal: &mut DefaultTerminal) -> Result<()> {
        match &mut self.mode {
            Mode::Run(runner) => {
                if runner.running() {
                    match key.code {
                        KeyCode::PageUp => runner.scroll(1),
                        KeyCode::PageDown => runner.scroll(-1),
                        _ => runner.send_key(key),
                    }
                } else {
                    match key.code {
                        KeyCode::PageUp => runner.scroll(1),
                        KeyCode::PageDown => runner.scroll(-1),
                        KeyCode::Char('q') | KeyCode::Esc | KeyCode::Enter => {
                            self.mode = Mode::Form;
                            self.reload_status();
                        }
                        _ => {}
                    }
                }
            }
            Mode::Edit { backup } => {
                let backup = backup.clone();
                let Some(Field {
                    value: Value::Text(text),
                    flag,
                    ..
                }) = self
                    .options_state
                    .selected()
                    .and_then(|i| self.fields.get_mut(i))
                else {
                    self.mode = Mode::Form;
                    return Ok(());
                };
                let flag = *flag;
                match key.code {
                    KeyCode::Enter => {
                        self.mode = Mode::Form;
                        if flag == 'f' {
                            self.reload_status();
                        }
                    }
                    KeyCode::Esc => {
                        *text = backup;
                        self.mode = Mode::Form;
                    }
                    KeyCode::Backspace if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        let keep = text.trim_end().rfind(' ').map_or(0, |i| i + 1);
                        text.truncate(keep);
                    }
                    KeyCode::Backspace => {
                        text.pop();
                    }
                    KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        text.clear()
                    }
                    KeyCode::Char(c) => text.push(c),
                    _ => {}
                }
                self.update_command();
            }
            Mode::Form => {
                self.error = None;
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => self.quit = true,
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.quit = true
                    }
                    KeyCode::Tab | KeyCode::Char('l') => self.cycle_focus(true),
                    KeyCode::BackTab | KeyCode::Char('h') => self.cycle_focus(false),
                    KeyCode::Up | KeyCode::Char('k') => self.list_state().select_previous(),
                    KeyCode::Down | KeyCode::Char('j') => self.list_state().select_next(),
                    KeyCode::Home | KeyCode::Char('g') => self.list_state().select_first(),
                    KeyCode::End | KeyCode::Char('G') => self.list_state().select_last(),
                    KeyCode::Char(' ') | KeyCode::Enter => self.activate(),
                    KeyCode::Char('R') => self.reload_status(),
                    KeyCode::Char('r') => self.start(terminal)?,
                    KeyCode::Char(flag) => self.activate_flag(flag),
                    _ => {}
                }
                self.clamp_selection();
                self.update_command();
            }
        }
        Ok(())
    }

    fn list_state(&mut self) -> &mut ListState {
        match self.focus {
            Focus::Options => &mut self.options_state,
            Focus::Inputs => &mut self.inputs.state,
            Focus::Nodes => &mut self.nodes.state,
            Focus::Tags => &mut self.tags.state,
        }
    }

    /// `select_next`/`select_last` may point past the end until the next render.
    fn clamp_selection(&mut self) {
        let clamp = |state: &mut ListState, len: usize| {
            if let Some(i) = state.selected() {
                state.select((len > 0).then(|| i.min(len - 1)));
            }
        };
        clamp(&mut self.options_state, self.fields.len());
        for selection in [&mut self.inputs, &mut self.nodes, &mut self.tags] {
            clamp(&mut selection.state, selection.entries.len());
        }
    }

    fn activate(&mut self) {
        match self.focus {
            Focus::Options => {
                let Some(field) = self
                    .options_state
                    .selected()
                    .and_then(|i| self.fields.get_mut(i))
                else {
                    return;
                };
                match &mut field.value {
                    Value::Toggle(on) => *on = !*on,
                    Value::Text(text) => {
                        self.mode = Mode::Edit {
                            backup: text.clone(),
                        }
                    }
                }
            }
            Focus::Inputs => self.inputs.toggle_current(),
            Focus::Nodes => self.nodes.toggle_current(),
            Focus::Tags => self.tags.toggle_current(),
        }
    }

    /// Selects the option of `flag` and toggles or edits it.
    fn activate_flag(&mut self, flag: char) {
        let Some(index) = self.fields.iter().position(|f| f.flag == flag) else {
            return;
        };
        self.focus = Focus::Options;
        self.options_state.select(Some(index));
        self.activate();
    }

    fn start(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        let area = ui::terminal_area(terminal.size()?.into());
        let exe = std::env::current_exe().context("could not locate the hive-tui binary")?;
        let mut args = vec![EXEC_FLAG.to_owned()];
        args.extend(self.args());
        match Runner::spawn(&exe, &args, area.height, area.width) {
            Ok(runner) => self.mode = Mode::Run(runner),
            Err(err) => self.error = Some(format!("{err:#}")),
        }
        Ok(())
    }
}

/// Quotes an argument for display, only where the shell would need it.
fn shell_quote(arg: &str) -> String {
    let safe = |c: char| c.is_ascii_alphanumeric() || "-_./=:@+%,".contains(c);
    if !arg.is_empty() && arg.chars().all(safe) {
        arg.to_owned()
    } else {
        format!("'{}'", arg.replace('\'', r"'\''"))
    }
}

#[cfg(test)]
mod tests {
    use ratatui::widgets::ListState;

    use super::{App, Entry, Focus, Mode, Selection, Value, default_fields, shell_quote};
    use crate::status::{Hive, Status};

    fn app_with_inputs(inputs: &[&str]) -> App {
        let inputs: Vec<String> = inputs.iter().map(|name| (*name).to_owned()).collect();
        let mut app = App {
            fields: default_fields(),
            inputs: Selection::default(),
            nodes: Selection::default(),
            tags: Selection::default(),
            focus: Focus::Options,
            options_state: ListState::default().with_selected(Some(0)),
            mode: Mode::Form,
            status: Status {
                flake: Err("no flake configured".to_owned()),
                upstream: None,
                unpushed: None,
                head: None,
                changes: Vec::new(),
                inputs,
            },
            hive: Hive::default(),
            command: String::new(),
            error: None,
            quit: false,
        };
        app.sync_inputs();
        app
    }

    fn app() -> App {
        app_with_inputs(&[])
    }

    fn app_with_hive(nodes: &[(&str, &[&str])]) -> App {
        let mut app = app();
        for (name, tags) in nodes {
            app.hive.nodes.insert(
                (*name).to_owned(),
                tags.iter().map(|tag| (*tag).to_owned()).collect(),
            );
        }
        app.sync_hive();
        app
    }

    fn value_mut(app: &mut App, flag: char) -> &mut Value {
        &mut app
            .fields
            .iter_mut()
            .find(|field| field.flag == flag)
            .expect("known flag")
            .value
    }

    fn switch_on(app: &mut App, flag: char) {
        *value_mut(app, flag) = Value::Toggle(true);
    }

    fn type_text(app: &mut App, flag: char, text: &str) {
        *value_mut(app, flag) = Value::Text(text.to_owned());
    }

    fn select(selection: &mut Selection, name: &str) {
        selection
            .entries
            .iter_mut()
            .find(|entry| entry.name == name)
            .expect("known entry")
            .selected = true;
    }

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn no_arguments_by_default() {
        assert!(app().args().is_empty());
    }

    #[test]
    fn switched_on_toggles_become_flags_in_field_order() {
        let mut app = app();
        switch_on(&mut app, 'P');
        switch_on(&mut app, 'p');
        assert_eq!(app.args(), ["-p", "-P"]);
    }

    #[test]
    fn text_values_are_trimmed_and_blank_ones_skipped() {
        let mut app = app();
        type_text(&mut app, 'c', "  new kernel  ");
        type_text(&mut app, 'f', "   ");
        assert_eq!(app.args(), ["-c", "new kernel"]);
    }

    #[test]
    fn tilde_is_expanded_only_in_the_flake_path() {
        let mut app = app();
        type_text(&mut app, 'c', "~/comment");
        type_text(&mut app, 'f', "~/nixos");
        assert_eq!(
            app.args(),
            [
                "-c".to_owned(),
                "~/comment".to_owned(),
                "-f".to_owned(),
                hive::flake::expand_tilde("~/nixos"),
            ]
        );
    }

    #[test]
    fn selected_inputs_are_updated_individually() {
        let mut app = app_with_inputs(&["home-manager", "nixpkgs"]);
        select(&mut app.inputs, "nixpkgs");
        assert_eq!(app.args(), ["-U", "nixpkgs"]);
    }

    #[test]
    fn updating_all_inputs_drops_the_selection() {
        let mut app = app_with_inputs(&["home-manager", "nixpkgs"]);
        select(&mut app.inputs, "nixpkgs");
        switch_on(&mut app, 'u');
        assert_eq!(app.args(), ["-u"]);
    }

    #[test]
    fn selected_nodes_and_tags_follow_the_inputs() {
        let mut app = app_with_hive(&[("db1", &["core"]), ("web1", &["web"])]);
        app.inputs.sync(&strings(&["nixpkgs"]));
        select(&mut app.tags, "web");
        select(&mut app.nodes, "db1");
        select(&mut app.inputs, "nixpkgs");
        switch_on(&mut app, 'P');
        assert_eq!(
            app.args(),
            ["-P", "-U", "nixpkgs", "-N", "db1", "-T", "web"]
        );
    }

    #[test]
    fn command_quotes_arguments_for_the_shell() {
        let mut app = app();
        switch_on(&mut app, 'y');
        type_text(&mut app, 'c', "it's done");
        app.update_command();
        assert_eq!(app.command, r"hive -y -c 'it'\''s done'");
    }

    #[test]
    fn shell_quote_keeps_safe_arguments() {
        assert_eq!(shell_quote("/etc/nixos"), "/etc/nixos");
        assert_eq!(shell_quote("-U"), "-U");
        assert_eq!(shell_quote("user@host:1+2%,3=4"), "user@host:1+2%,3=4");
    }

    #[test]
    fn shell_quote_wraps_unsafe_and_empty_arguments() {
        assert_eq!(shell_quote("new kernel"), "'new kernel'");
        assert_eq!(shell_quote("~/nixos"), "'~/nixos'");
        assert_eq!(shell_quote(""), "''");
        assert_eq!(shell_quote("it's"), r"'it'\''s'");
    }

    #[test]
    fn no_hints_by_default() {
        assert!(app().hints().is_empty());
    }

    #[test]
    fn hints_name_ignored_combinations() {
        let mut app = app_with_inputs(&["nixpkgs"]);
        switch_on(&mut app, 'S');
        switch_on(&mut app, 'd');
        switch_on(&mut app, 'u');
        type_text(&mut app, 'c', "comment");
        select(&mut app.inputs, "nixpkgs");
        assert_eq!(
            app.hints(),
            [
                "Don't squash has no effect without Push",
                "No commit (and no comment) in dry/build-only mode",
                "Update all inputs overrides the input selection",
            ]
        );
    }

    #[test]
    fn hints_that_push_is_skipped_when_building_only() {
        let mut app = app();
        switch_on(&mut app, 'P');
        switch_on(&mut app, 'b');
        assert_eq!(app.hints(), ["Push is skipped in dry/build-only mode"]);
    }

    #[test]
    fn hints_that_no_rebuild_conflicts_with_rebuild_options() {
        let mut app = app_with_inputs(&["nixpkgs"]);
        switch_on(&mut app, 'n');
        assert!(app.hints().is_empty());
        select(&mut app.inputs, "nixpkgs");
        assert_eq!(
            app.hints(),
            ["No rebuild can't be combined with update, comment, build only or trace"]
        );
    }

    #[test]
    fn targets_are_unknown_before_the_hive_is_evaluated() {
        assert_eq!(app().targets(), None);
    }

    #[test]
    fn targets_resolve_the_node_and_tag_selection() {
        let mut app =
            app_with_hive(&[("db1", &["core"]), ("edge-a", &["web"]), ("web1", &["web"])]);
        assert_eq!(app.targets(), Some(strings(&["db1", "edge-a", "web1"])));
        select(&mut app.tags, "web");
        assert_eq!(app.targets(), Some(strings(&["edge-a", "web1"])));
        select(&mut app.nodes, "db1");
        assert_eq!(app.targets(), Some(strings(&["db1", "edge-a", "web1"])));
    }

    #[test]
    fn hive_lists_nodes_and_their_distinct_tags() {
        let app = app_with_hive(&[("web1", &["web", "core"]), ("db1", &["core"])]);
        let names = |selection: &Selection| -> Vec<String> {
            selection.entries.iter().map(|e| e.name.clone()).collect()
        };
        assert_eq!(names(&app.nodes), ["db1", "web1"]);
        assert_eq!(names(&app.tags), ["core", "web"]);
    }

    #[test]
    fn syncing_keeps_the_selection_of_remaining_entries() {
        let mut app = app_with_inputs(&["home-manager", "nixpkgs", "sops-nix"]);
        select(&mut app.inputs, "nixpkgs");
        select(&mut app.inputs, "sops-nix");
        app.inputs.state.select(Some(2));
        app.status.inputs = strings(&["disko", "nixpkgs"]);
        app.sync_inputs();
        assert_eq!(app.inputs.selected_names(), ["nixpkgs"]);
        assert_eq!(app.inputs.state.selected(), Some(1));
    }

    #[test]
    fn syncing_without_entries_clears_the_cursor() {
        let mut app = app_with_inputs(&["nixpkgs"]);
        app.status.inputs.clear();
        app.sync_inputs();
        assert!(app.inputs.is_empty());
        assert_eq!(app.inputs.state.selected(), None);
    }

    #[test]
    fn focus_leaves_a_list_that_became_empty() {
        let mut app = app_with_hive(&[("db1", &[])]);
        app.focus = Focus::Nodes;
        app.hive.nodes.clear();
        app.sync_hive();
        assert_eq!(app.focus, Focus::Options);
    }

    #[test]
    fn focus_cycles_through_non_empty_lists() {
        let mut app = app_with_hive(&[("db1", &["core"])]);
        app.cycle_focus(true);
        assert_eq!(app.focus, Focus::Nodes);
        app.cycle_focus(true);
        assert_eq!(app.focus, Focus::Tags);
        app.cycle_focus(true);
        assert_eq!(app.focus, Focus::Options);
        app.cycle_focus(false);
        assert_eq!(app.focus, Focus::Tags);
    }

    #[test]
    fn focus_stays_on_the_options_without_lists() {
        let mut app = app();
        app.cycle_focus(true);
        assert_eq!(app.focus, Focus::Options);
    }

    #[test]
    fn selection_is_clamped_to_the_lists() {
        let mut app = app_with_hive(&[("db1", &["core"])]);
        app.inputs.sync(&strings(&["nixpkgs"]));
        app.options_state.select(Some(99));
        app.inputs.state.select(Some(99));
        app.nodes.state.select(Some(99));
        app.tags.state.select(Some(99));
        app.clamp_selection();
        assert_eq!(app.options_state.selected(), Some(app.fields.len() - 1));
        assert_eq!(app.inputs.state.selected(), Some(0));
        assert_eq!(app.nodes.state.selected(), Some(0));
        assert_eq!(app.tags.state.selected(), Some(0));
    }

    #[test]
    fn activating_toggles_options_and_list_entries() {
        let mut app = app_with_hive(&[("db1", &["core"])]);
        app.inputs.sync(&strings(&["nixpkgs"]));
        app.activate();
        assert_eq!(app.args(), ["-p"]);
        for focus in [Focus::Inputs, Focus::Nodes, Focus::Tags] {
            app.focus = focus;
            app.activate();
        }
        assert_eq!(
            app.args(),
            ["-p", "-U", "nixpkgs", "-N", "db1", "-T", "core"]
        );
        app.focus = Focus::Nodes;
        app.activate();
        assert_eq!(app.args(), ["-p", "-U", "nixpkgs", "-T", "core"]);
    }

    #[test]
    fn flag_keys_toggle_their_option_from_a_list() {
        let mut app = app_with_hive(&[("db1", &[])]);
        app.focus = Focus::Nodes;
        app.activate_flag('P');
        assert_eq!(app.args(), ["-P"]);
        assert_eq!(app.focus, Focus::Options);
        assert_eq!(app.selected_field().map(|f| f.flag), Some('P'));
        app.activate_flag('P');
        assert!(app.args().is_empty());
    }

    #[test]
    fn flag_keys_edit_text_options() {
        let mut app = app();
        type_text(&mut app, 'c', "kernel");
        app.activate_flag('c');
        assert!(matches!(&app.mode, Mode::Edit { backup } if backup == "kernel"));
    }

    #[test]
    fn unknown_flag_keys_change_nothing() {
        let mut app = app();
        app.activate_flag('x');
        assert!(app.args().is_empty());
        assert!(matches!(app.mode, Mode::Form));
    }

    #[test]
    fn flag_keys_do_not_collide_with_form_keys() {
        for field in default_fields() {
            assert!(!"qjkhlgGrR".contains(field.flag), "-{}", field.flag);
        }
    }

    #[test]
    fn entries_start_unselected() {
        let app = app_with_inputs(&["nixpkgs"]);
        assert!(matches!(
            app.inputs.entries.as_slice(),
            [Entry { name, selected: false }] if name == "nixpkgs"
        ));
    }
}
