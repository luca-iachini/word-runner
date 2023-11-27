use std::{path::PathBuf, thread, time::Duration};

use clap::{Parser, ValueHint};
use document::{Document, Page, TableOfContentNode};
mod document;
use ratatui::{
    backend::CrosstermBackend,
    layout::Layout,
    layout::{Alignment, Constraint, Direction},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Terminal,
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

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let stdout = std::io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut table_of_content_tree_state = TreeState::default();

    let mut doc = document::EpubDoc::open(&args.path).expect("unable to open the epub");
    let table_of_content = doc.table_of_contents();
    let mut pages = doc.pages();
    while let Some(page) = pages.next() {
        let mut words = page.words();
        while let Some(word) = words.next() {
            terminal.draw(|f| {
                let main_layout = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(1)
                    .constraints([Constraint::Length(3), Constraint::Percentage(100)].as_ref())
                    .split(f.size());
                let content_layout = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(vec![Constraint::Percentage(25), Constraint::Percentage(75)])
                    .split(main_layout[1]);
                f.render_widget(current_word(word), main_layout[0]);
                f.render_stateful_widget(
                    table_of_contents(&table_of_content),
                    content_layout[0],
                    &mut table_of_content_tree_state,
                );
                f.render_widget(content(&page), content_layout[1]);
            })?;
            thread::sleep(args.speed);
        }
    }
    Ok(())
}

fn table_of_contents(content: &[TableOfContentNode]) -> Tree<String> {
    let items = content.into_iter().map(Into::into).collect();
    Tree::new(items)
        .expect("all item identifiers are unique")
        .block(Block::default().title("Contents").borders(Borders::ALL))
}

fn content(page: &Page) -> Paragraph {
    let lines: Vec<Line> = page.content.lines().map(|l| Line::raw(l)).collect();
    Paragraph::new(lines)
        .block(
            Block::default()
                .title(format!("Page {}", page.number))
                .borders(Borders::ALL),
        )
        .style(Style::default().fg(Color::White).bg(Color::Black))
}

fn current_word(word: &str) -> Paragraph {
    let (first_half, center, second_half) = split_word(word);
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
