//! The `pick` popup: the claimable tasks of a corpus, filtered as you type.
//! The choice goes to `pick` in `$HERDR_PLUGIN_STATE_DIR`, where `work`
//! waits for it: an id, or nothing for a cancel.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::style::{Attribute, Print, SetAttribute};
use crossterm::terminal::{self, ClearType};
use crossterm::{cursor, execute, queue};

use crate::ank::ContextTask;

/// The file in the state directory that carries the selection.
pub const SELECTION: &str = "pick";

/// The tasks a claim would take: ready (nothing blocks them) and held by nobody.
pub fn claimable(tasks: Vec<ContextTask>) -> Vec<ContextTask> {
    tasks
        .into_iter()
        .filter(|t| t.ready && t.state == "open")
        .collect()
}

/// The tasks whose id or title contains `filter`, ignoring case.
pub fn filter<'a>(tasks: &'a [ContextTask], filter: &str) -> Vec<&'a ContextTask> {
    let needle = filter.to_lowercase();
    tasks
        .iter()
        .filter(|t| {
            t.id.to_lowercase().contains(&needle) || t.title.to_lowercase().contains(&needle)
        })
        .collect()
}

pub fn selection_path(state_dir: &Path) -> PathBuf {
    state_dir.join(SELECTION)
}

/// Writes the chosen id, or an empty file for a cancel.
pub fn write_selection(state_dir: &Path, id: Option<&str>) -> io::Result<()> {
    fs::create_dir_all(state_dir)?;
    // Written aside then renamed, so `work` never reads half an id.
    let tmp = state_dir.join(format!("{SELECTION}.tmp"));
    fs::write(&tmp, id.unwrap_or(""))?;
    fs::rename(tmp, selection_path(state_dir))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Backspace,
    Up,
    Down,
    Enter,
    Esc,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    Continue,
    Selected(String),
    Cancelled,
}

/// The picker's state, apart from the terminal.
#[derive(Debug)]
pub struct Picker {
    tasks: Vec<ContextTask>,
    filter: String,
    cursor: usize,
}

impl Picker {
    pub fn new(tasks: Vec<ContextTask>) -> Self {
        Picker {
            tasks,
            filter: String::new(),
            cursor: 0,
        }
    }

    pub fn visible(&self) -> Vec<&ContextTask> {
        filter(&self.tasks, &self.filter)
    }

    pub fn filter_text(&self) -> &str {
        &self.filter
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn key(&mut self, key: Key) -> Step {
        match key {
            Key::Char(c) => self.filter.push(c),
            Key::Backspace => {
                self.filter.pop();
            }
            Key::Up => self.cursor = self.cursor.saturating_sub(1),
            Key::Down => self.cursor += 1,
            Key::Enter => {
                return match self.visible().get(self.cursor) {
                    Some(task) => Step::Selected(task.id.clone()),
                    None => Step::Continue,
                }
            }
            Key::Esc => return Step::Cancelled,
        }
        self.cursor = self.cursor.min(self.visible().len().saturating_sub(1));
        Step::Continue
    }
}

/// Runs the picker on the terminal until a choice or a cancel.
pub fn run_terminal(tasks: Vec<ContextTask>) -> io::Result<Step> {
    let mut picker = Picker::new(tasks);
    let mut out = io::stdout();
    terminal::enable_raw_mode()?;
    execute!(out, terminal::EnterAlternateScreen, cursor::Hide)?;
    let result = (|| loop {
        draw(&mut out, &picker)?;
        let Event::Key(press) = event::read()? else {
            continue;
        };
        if press.kind != KeyEventKind::Press {
            continue;
        }
        let key = match press.code {
            KeyCode::Char('c') if press.modifiers.contains(KeyModifiers::CONTROL) => Key::Esc,
            KeyCode::Char(c) => Key::Char(c),
            KeyCode::Backspace => Key::Backspace,
            KeyCode::Up => Key::Up,
            KeyCode::Down => Key::Down,
            KeyCode::Enter => Key::Enter,
            KeyCode::Esc => Key::Esc,
            _ => continue,
        };
        match picker.key(key) {
            Step::Continue => {}
            done => return Ok(done),
        }
    })();
    execute!(out, cursor::Show, terminal::LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;
    result
}

fn draw(out: &mut impl Write, picker: &Picker) -> io::Result<()> {
    let (_, rows) = terminal::size().unwrap_or((80, 24));
    queue!(
        out,
        terminal::Clear(ClearType::All),
        cursor::MoveTo(0, 0),
        Print(format!("work a task > {}", picker.filter_text())),
    )?;
    let visible = picker.visible();
    if visible.is_empty() {
        queue!(
            out,
            cursor::MoveTo(0, 2),
            Print("no claimable task matches")
        )?;
    }
    for (i, task) in visible
        .iter()
        .enumerate()
        .take(rows.saturating_sub(3) as usize)
    {
        queue!(out, cursor::MoveTo(0, i as u16 + 2))?;
        if i == picker.cursor() {
            queue!(out, SetAttribute(Attribute::Reverse))?;
        }
        queue!(
            out,
            Print(format!("{}  {}", task.short, task.title)),
            SetAttribute(Attribute::Reset)
        )?;
    }
    queue!(
        out,
        cursor::MoveTo(0, rows.saturating_sub(1)),
        Print("type to filter  ↑↓ move  enter work it  esc cancel"),
    )?;
    out.flush()
}
