use std::{
    path::PathBuf,
    time::{Duration, SystemTime},
};

use clap::{Parser, ValueHint};
use document::{Document, DocumentCursor, TableOfContentNode};
mod document;
use ratatui::{
    backend::CrosstermBackend,
    layout::Layout,
    layout::{Alignment, Constraint, Direction},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use tui_tree_widget::{Tree, TreeItem, TreeState};

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

#[derive(Debug, PartialEq)]
enum Status {
    Running,
    Paused,
}

struct Model<D: Document> {
    should_quit: bool,
    cursor: DocumentCursor<D>,
    table_of_contents: Vec<TableOfContentNode>,
    table_of_contents_state: TreeState<String>,
    last_word_change: SystemTime,
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
}

fn update<D: Document>(model: &mut Model<D>, msg: Message) -> Option<Message> {
    match msg {
        Message::Quit => {
            model.should_quit = true;
            None
        }
        Message::PrevWord => {
            model.cursor.prev_word();
            None
        }
        Message::NextWord => {
            model.cursor.next_word();
            model.last_word_change = SystemTime::now();
            if model.cursor.current_word().is_none() {
                Some(Message::NextSection)
            } else {
                None
            }
        }
        Message::PrevLine => {
            model.cursor.prev_line();
            None
        }
        Message::NextLine => {
            model.cursor.next_line();
            None
        }
        Message::PrevSection => {
            model.cursor.prev_section();
            None
        }
        Message::NextSection => {
            model.cursor.next_section();
            None
        }
        Message::DecreaseSpeed => {
            model.speed = model.speed.saturating_add(Duration::from_millis(25));
            None
        }
        Message::IncreaseSpeed => {
            model.speed = model.speed.saturating_sub(Duration::from_millis(25));
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
    }
}

fn view<D: Document>(model: &mut Model<D>, f: &mut Frame) {
    let word = model.cursor.current_word().unwrap_or_default();
    let page = model
        .cursor
        .current_section()
        .map(|p| p.content)
        .unwrap_or_default();
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Length(3), Constraint::Percentage(100)].as_ref())
        .split(f.size());
    let content_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(vec![Constraint::Percentage(25), Constraint::Percentage(75)])
        .split(main_layout[1]);
    f.render_widget(current_word(&word), main_layout[0]);
    f.render_stateful_widget(
        table_of_contents(&model.table_of_contents),
        content_layout[0],
        &mut model.table_of_contents_state,
    );
    f.render_widget(
        content(
            &page,
            model.cursor.word_index(),
            model.cursor.current_line(),
        ),
        content_layout[1],
    );
}

fn table_of_contents(content: &[TableOfContentNode]) -> Tree<String> {
    let items = content.into_iter().map(Into::into).collect();
    Tree::new(items)
        .expect("all item identifiers are unique")
        .block(
            Block::default()
                .title("Table of Contents")
                .borders(Borders::ALL),
        )
}

fn content(section: &str, current_word: usize, current_line: Option<document::Line>) -> Paragraph {
    let mut lines: Vec<Line> = vec![];
    let mut index = 0;
    if let Some(current_line) = current_line {
        for l in section.lines() {
            let split: Vec<_> = l.trim().split_whitespace().collect();
            let line = if !l.is_empty() && index == current_line.index {
                let line: Line = if current_word - current_line.word_indexes.0 > 0 {
                    let pos = current_word - current_line.word_indexes.0;
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
    Paragraph::new(word_text)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .title(format!("Current Word"))
                .borders(Borders::ALL),
        )
        .style(Style::default().fg(Color::White).bg(Color::Black))
}

fn handle_event<D: Document>(model: &Model<D>) -> anyhow::Result<Option<Message>> {
    if crossterm::event::poll(std::time::Duration::from_millis(250))? {
        if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
            match key.code {
                crossterm::event::KeyCode::Char('q') => Ok(Some(Message::Quit)),
                crossterm::event::KeyCode::Right => Ok(Some(Message::NextWord)),
                crossterm::event::KeyCode::Left => Ok(Some(Message::PrevWord)),
                crossterm::event::KeyCode::Up => Ok(Some(Message::PrevLine)),
                crossterm::event::KeyCode::Down => Ok(Some(Message::NextLine)),
                crossterm::event::KeyCode::PageDown => Ok(Some(Message::PrevSection)),
                crossterm::event::KeyCode::PageUp => Ok(Some(Message::NextSection)),
                crossterm::event::KeyCode::Char('+') => Ok(Some(Message::IncreaseSpeed)),
                crossterm::event::KeyCode::Char('-') => Ok(Some(Message::DecreaseSpeed)),
                crossterm::event::KeyCode::Char(' ') => Ok(Some(Message::ToggleStatus)),
                _ => Ok(None),
            }
        } else {
            Ok(None)
        }
    } else {
        if model.status == Status::Running
            && model.last_word_change.elapsed().unwrap() >= model.speed
        {
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
    let cursor = DocumentCursor::new(doc);
    let mut model = Model {
        should_quit: false,
        cursor,
        table_of_contents,
        table_of_contents_state: TreeState::default(),
        last_word_change: SystemTime::now(),
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

impl<'a> From<&TableOfContentNode> for TreeItem<'a, String> {
    fn from(value: &TableOfContentNode) -> Self {
        if value.children.is_empty() {
            TreeItem::new_leaf(value.name.clone(), value.name.clone())
        } else {
            TreeItem::new(
                value.name.clone(),
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
