use std::{
    cmp::{max, min},
    path::{Path, PathBuf},
    time::{Duration, Instant},
    u16,
};

use clap::{Parser, ValueHint};
use document::{DocState, DocumentCursor, TableOfContentNode};
mod document;
use ratatui::{
    backend::CrosstermBackend,
    layout::Layout,
    layout::{Alignment, Constraint, Direction},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use strum;
use tui_tree_widget::{Tree, TreeItem, TreeState};

const CONFIG_PATH: &'static str = ".config/";

#[derive(Parser)]
struct Args {
    #[clap(value_hint = ValueHint::AnyPath)]
    path: PathBuf,
    #[clap(short, value_parser = parse_speed)]
    speed: Duration,
}

fn parse_speed(arg: &str) -> Result<std::time::Duration, std::num::ParseIntError> {
    let millis = arg.parse()?;
    Ok(std::time::Duration::from_millis(millis))
}

#[derive(Debug, PartialEq, strum::Display)]
enum Status {
    Running,
    Paused,
}

struct Model {
    should_quit: bool,
    cursor: DocumentCursor,
    table_of_contents: Vec<TreeItem<'static, usize>>,
    table_of_contents_state: TreeState<usize>,
    last_word_change: Instant,
    speed: Duration,
    status: Status,
}

#[derive(PartialEq)]
enum Message {
    Quit,
    PrevWord,
    NextWord,
    PrevLine,
    NextLine,
    PrevSection,
    NextSection,
    IncreaseSpeed,
    DecreaseSpeed,
    ToggleStatus,
    TableOfContentsMessage(TableOfContentsMessage),
}

#[derive(PartialEq)]
enum TableOfContentsMessage {
    Select,
    Left,
    Right,
    Down,
    Up,
}

fn update(model: &mut Model, msg: Message) -> Option<Message> {
    match msg {
        Message::Quit => {
            model.should_quit = true;
            let _ = model.cursor.doc_state().store(Path::new(CONFIG_PATH));
            None
        }
        Message::PrevWord => {
            if !model.cursor.current_section().prev_word() {
                Some(Message::PrevSection)
            } else {
                None
            }
        }
        Message::NextWord => {
            model.last_word_change = Instant::now();
            if !model.cursor.current_section().next_word() {
                Some(Message::NextSection)
            } else {
                None
            }
        }
        Message::PrevLine => {
            if !model.cursor.current_section().prev_line() {
                Some(Message::PrevSection)
            } else {
                None
            }
        }
        Message::NextLine => {
            if !model.cursor.current_section().next_line() {
                Some(Message::NextSection)
            } else {
                None
            }
        }
        Message::PrevSection => {
            model.cursor.prev_section();
            model
                .table_of_contents_state
                .select(model.cursor.toc_index());
            None
        }
        Message::NextSection => {
            model.cursor.next_section();
            model
                .table_of_contents_state
                .select(model.cursor.toc_index());
            None
        }
        Message::DecreaseSpeed => {
            model.speed = min(
                Duration::from_secs(2),
                model.speed.saturating_add(Duration::from_millis(10)),
            );
            None
        }
        Message::IncreaseSpeed => {
            model.speed = max(
                Duration::from_millis(50),
                model.speed.saturating_sub(Duration::from_millis(10)),
            );
            None
        }
        Message::ToggleStatus => match model.status {
            Status::Running => {
                model.status = Status::Paused;
                None
            }
            Status::Paused => {
                model.status = Status::Running;
                Some(Message::NextWord)
            }
        },
        Message::TableOfContentsMessage(msg) => {
            model.status = Status::Paused;
            match msg {
                TableOfContentsMessage::Select => {
                    if let Some(selected) = model.table_of_contents_state.selected().first() {
                        model.cursor.goto_section(*selected);
                    }
                }
                TableOfContentsMessage::Left => model.table_of_contents_state.key_left(),
                TableOfContentsMessage::Right => model.table_of_contents_state.key_right(),
                TableOfContentsMessage::Down => model
                    .table_of_contents_state
                    .key_down(&model.table_of_contents),
                TableOfContentsMessage::Up => model
                    .table_of_contents_state
                    .key_up(&model.table_of_contents),
            };
            None
        }
    }
}

fn view(model: &mut Model, f: &mut Frame) {
    let word = model
        .cursor
        .current_section()
        .current_word()
        .unwrap_or_default();
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Max(5),
                Constraint::Percentage(80),
                Constraint::Max(3),
            ]
            .as_ref(),
        )
        .split(f.size());
    let content_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(vec![Constraint::Percentage(25), Constraint::Percentage(75)])
        .split(main_layout[1]);
    f.render_widget(current_word(&word), main_layout[0]);
    f.render_stateful_widget(
        table_of_contents(model.table_of_contents.clone()),
        content_layout[0],
        &mut model.table_of_contents_state,
    );
    f.render_widget(
        content(&mut model.cursor, content_layout[1].width),
        content_layout[1],
    );
    f.render_widget(status_bar(&model), main_layout[2])
}

fn table_of_contents(content: Vec<TreeItem<'static, usize>>) -> Tree<usize> {
    Tree::new(content)
        .expect("all item identifiers are unique")
        .highlight_style(Style::default().bg(Color::Yellow))
        .block(
            Block::default()
                .title("Table of Contents")
                .borders(Borders::ALL),
        )
}

fn content(cursor: &mut document::DocumentCursor, width: u16) -> Paragraph {
    let mut lines: Vec<Line> = vec![];
    let mut index = 0;
    let current_section = cursor.current_section_or_resize(width as usize - 1);
    let current_line = current_section.current_line();
    let text_lines = current_section.content.lines();
    if let Some(current_line) = current_line {
        for l in text_lines {
            let line = if !l.is_empty() && index == current_line.index {
                let split: Vec<_> = l.trim().split_whitespace().collect();
                let line: Line = if current_section.word_index() != current_line.first_word_index()
                {
                    let pos = current_section.word_index() - current_line.first_word_index();
                    vec![
                        Span::raw(split[..pos].join(" ")),
                        Span::raw(" "),
                        word_cursor(split[pos]),
                        Span::raw(" "),
                        Span::raw(split[pos + 1..].join(" ")),
                    ]
                    .into()
                } else {
                    vec![
                        word_cursor(split[0]),
                        Span::raw(" "),
                        Span::raw(split[1..].join(" ")),
                    ]
                    .into()
                };

                line
            } else {
                Line::raw(l)
            };
            lines.push(line);
            if !l.is_empty() {
                index += 1;
            }
        }
        if current_line.index > 3 {
            lines = lines.into_iter().skip(current_line.index - 3).collect();
        }
    }

    Paragraph::new(lines)
        .block(Block::default().title("Content").borders(Borders::ALL))
        .style(Style::default().fg(Color::White).bg(Color::Black))
}

fn word_cursor(word: &str) -> Span {
    Span::styled(word, Style::default().bg(Color::LightYellow))
}

fn current_word(word: impl ToString) -> Paragraph<'static> {
    let word = word.to_string();
    let word_text: Line = if word.is_empty() {
        Line::raw("")
    } else {
        let (first_half, center, second_half) = split_word(word.to_string().as_str());
        vec![
            Span::raw(first_half),
            Span::styled(center, Style::default().fg(Color::Red)),
            Span::raw(second_half),
        ]
        .into()
    };
    let gauge: Text = vec!["\\/".into(), word_text, "/\\".into()].into();
    Paragraph::new(gauge)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .title(format!("Current Word"))
                .borders(Borders::ALL),
        )
        .style(Style::default().fg(Color::White).bg(Color::Black))
}

fn status_bar(model: &Model) -> Paragraph {
    let status: Line = vec![
        Span::raw(format!("Status: {} ", model.status.to_string())),
        Span::raw(format!(" Speed: {} wpm", 60000 / model.speed.as_millis())),
        Span::raw(format!(
            " Position {}/{}",
            model.cursor.section_index(),
            model.cursor.sections()
        )),
    ]
    .into();
    Paragraph::new(status).block(Block::default().title("Status").borders(Borders::ALL))
}

fn handle_event(model: &Model) -> anyhow::Result<Option<Message>> {
    let timeout = model.speed.saturating_sub(model.last_word_change.elapsed());
    if crossterm::event::poll(timeout)? {
        if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
            match key.code {
                crossterm::event::KeyCode::Char('q') => Ok(Some(Message::Quit)),
                crossterm::event::KeyCode::Right => Ok(Some(Message::NextWord)),
                crossterm::event::KeyCode::Left => Ok(Some(Message::PrevWord)),
                crossterm::event::KeyCode::Up => Ok(Some(Message::PrevLine)),
                crossterm::event::KeyCode::Down => Ok(Some(Message::NextLine)),
                crossterm::event::KeyCode::PageUp => Ok(Some(Message::PrevSection)),
                crossterm::event::KeyCode::PageDown => Ok(Some(Message::NextSection)),
                crossterm::event::KeyCode::Char('+') => Ok(Some(Message::IncreaseSpeed)),
                crossterm::event::KeyCode::Char('-') => Ok(Some(Message::DecreaseSpeed)),
                crossterm::event::KeyCode::Char(' ') => Ok(Some(Message::ToggleStatus)),
                crossterm::event::KeyCode::Char('a') => Ok(Some(Message::TableOfContentsMessage(
                    TableOfContentsMessage::Left,
                ))),
                crossterm::event::KeyCode::Char('d') => Ok(Some(Message::TableOfContentsMessage(
                    TableOfContentsMessage::Right,
                ))),
                crossterm::event::KeyCode::Char('s') => Ok(Some(Message::TableOfContentsMessage(
                    TableOfContentsMessage::Down,
                ))),
                crossterm::event::KeyCode::Char('w') => Ok(Some(Message::TableOfContentsMessage(
                    TableOfContentsMessage::Up,
                ))),
                crossterm::event::KeyCode::Enter => Ok(Some(Message::TableOfContentsMessage(
                    TableOfContentsMessage::Select,
                ))),
                _ => Ok(None),
            }
        } else {
            Ok(None)
        }
    } else {
        if model.status == Status::Running && model.last_word_change.elapsed() >= model.speed {
            return Ok(Some(Message::NextWord));
        }
        Ok(None)
    }
}

fn main() -> anyhow::Result<()> {
    initialize_panic_handler();

    let args = Args::parse();

    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(std::io::stderr(), crossterm::terminal::EnterAlternateScreen)?;

    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stderr()))?;

    let doc = document::EpubDoc::open(&args.path).expect("unable to open the epub");
    let table_of_contents = doc.table_of_contents();
    let table_of_contents: Vec<TreeItem<'static, usize>> =
        table_of_contents.iter().map(Into::into).collect();

    std::fs::create_dir_all(CONFIG_PATH)?;
    let doc_state = DocState::load(
        Path::new(CONFIG_PATH),
        doc.unique_identifier.clone().unwrap(),
    );
    let cursor = DocumentCursor::new(doc, doc_state);
    let mut model = Model {
        should_quit: false,
        cursor,
        table_of_contents,
        table_of_contents_state: TreeState::default(),
        last_word_change: Instant::now(),
        speed: args.speed,
        status: Status::Paused,
    };
    loop {
        // Render the current view
        terminal.draw(|f| {
            view(&mut model, f);
        })?;
        if model.should_quit {
            break;
        }
        let mut current_msg = handle_event(&model)?;
        while current_msg != None {
            current_msg = update(&mut model, current_msg.unwrap());
        }
    }

    crossterm::execute!(std::io::stderr(), crossterm::terminal::LeaveAlternateScreen)?;
    crossterm::terminal::disable_raw_mode()?;
    Ok(())
}

fn split_word(word: &str) -> (String, String, String) {
    let mid = (word.len() - 1) / 2;
    let center = word.chars().nth(mid).unwrap_or_default().to_string();
    let first_half = word.chars().take(mid).collect();
    let second_half = word.chars().skip(mid + 1).collect();
    (first_half, center, second_half)
}

impl<'a> From<&TableOfContentNode> for TreeItem<'a, usize> {
    fn from(value: &TableOfContentNode) -> Self {
        if value.children.is_empty() {
            TreeItem::new_leaf(value.index, value.name.clone())
        } else {
            TreeItem::new(
                value.index,
                value.name.clone(),
                value.children.iter().map(Into::into).collect(),
            )
            .unwrap()
        }
    }
}

pub fn initialize_panic_handler() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        crossterm::execute!(std::io::stderr(), crossterm::terminal::LeaveAlternateScreen).unwrap();
        crossterm::terminal::disable_raw_mode().unwrap();
        original_hook(panic_info);
    }));
}
