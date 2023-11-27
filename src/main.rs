use std::{path::PathBuf, thread, time::Duration};

use clap::{Parser, ValueHint};
use document::{Document, DocumentCursor, Page, TableOfContentNode};
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

struct Model<'a> {
    should_quit: bool,
    cursor: DocumentCursor<'a>,
    table_of_contents: Vec<TableOfContentNode>,
    table_of_contents_state: TreeState<String>,
}

#[derive(PartialEq)]
enum Message {
    Quit,
    NextWord,
    NextPage,
}

fn update(model: &mut Model, msg: Message) -> Option<Message> {
    match msg {
        Message::Quit => model.should_quit = true, // You can handle cleanup and exit here
        Message::NextWord => model.cursor.next_word(),
        Message::NextPage => model.cursor.next_page(),
    };
    None
}

fn view(model: &mut Model, f: &mut Frame) {
    let word = model.cursor.current_word().unwrap_or("ciao".to_string());
    let page = model
        .cursor
        .current_page()
        .map(|p| p.content)
        .unwrap_or("ciao".to_string());
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
    f.render_widget(content(&page), content_layout[1]);
}

fn handle_event(_: &Model) -> anyhow::Result<Option<Message>> {
    let message = if crossterm::event::poll(std::time::Duration::from_millis(250))? {
        if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
            match key.code {
                crossterm::event::KeyCode::Char('q') => Message::Quit,
                _ => return Ok(None),
            }
        } else {
            return Ok(None);
        }
    } else {
        return Ok(None);
    };
    Ok(Some(message))
}

fn parse_speed(arg: &str) -> Result<std::time::Duration, std::num::ParseIntError> {
    let millis = arg.parse()?;
    Ok(std::time::Duration::from_millis(millis))
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(std::io::stderr(), crossterm::terminal::EnterAlternateScreen)?;

    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stderr()))?;

    let mut doc = document::EpubDoc::open(&args.path).expect("unable to open the epub");
    let table_of_contents = doc.table_of_contents();
    let mut model = Model {
        should_quit: false,
        cursor: doc.cursor(),
        table_of_contents,
        table_of_contents_state: TreeState::default(),
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

fn table_of_contents(content: &[TableOfContentNode]) -> Tree<String> {
    let items = content.into_iter().map(Into::into).collect();
    Tree::new(items)
        .expect("all item identifiers are unique")
        .block(Block::default().title("Contents").borders(Borders::ALL))
}

fn content(page: &str) -> Paragraph {
    let lines: Vec<Line> = page.lines().map(|l| Line::raw(l)).collect();
    Paragraph::new(lines)
        .block(Block::default().title("Page").borders(Borders::ALL))
        .style(Style::default().fg(Color::White).bg(Color::Black))
}

fn current_word(word: impl ToString) -> Paragraph<'static> {
    let (first_half, center, second_half) = split_word(word.to_string().as_str());
    let word_text: Line = vec![
        Span::raw(first_half),
        Span::styled(center, Style::default().fg(Color::Red)),
        Span::raw(second_half),
    ]
    .into();
    Paragraph::new(word_text)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .title(format!("current word"))
                .borders(Borders::ALL),
        )
        .style(Style::default().fg(Color::White).bg(Color::Black))
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
