use std::{path::PathBuf, thread, time::Duration};

use clap::{Parser, ValueHint};
use document::Document;
mod document;
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::Layout,
    layout::{Alignment, Constraint, Direction, Rect},
    style::{Color, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};

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

    let mut doc = document::EpubDoc::open(&args.path).expect("unable to open the epub");
    let mut pages = doc.pages();
    while let Some(page) = pages.next() {
        let mut words = page.words();
        while let Some(word) = words.next() {
            terminal.draw(|f| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(1)
                    .constraints([Constraint::Percentage(20), Constraint::Percentage(80)].as_ref())
                    .split(f.size());
                f.render_widget(current_word(word), chunks[0]);
                f.render_widget(content(&page.content, page.number), chunks[1]);
            })?;
            thread::sleep(args.speed);
        }
    }
    Ok(())
}

fn content(content: &str, page_number: usize) -> Paragraph {
    Paragraph::new(Span::raw(content))
        .block(
            Block::default()
                .title(format!("Page {}", page_number))
                .borders(Borders::ALL),
        )
        .style(Style::default().fg(Color::White).bg(Color::Black))
}

fn current_word(word: &str) -> Paragraph {
    let (first_half, center, second_half) = split_word(word);
    let word_text: Spans = vec![
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
