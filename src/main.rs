use std::{fmt::Alignment, path::PathBuf, thread, time::Duration};

use clap::{Parser, ValueHint};
use document::Document;
mod document;
use tui::{
    backend::CrosstermBackend,
    style::{Color, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, Paragraph},
    Terminal,
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
                let (first_half, center, second_half) = split_word(word);
                let word_text: Spans = vec![
                    Span::raw(first_half),
                    Span::styled(center, Style::default().fg(Color::Red)),
                    Span::raw(second_half),
                ]
                .into();
                let widget = Paragraph::new(word_text)
                    .block(
                        Block::default()
                            .title(format!("Page {}", page.number))
                            .borders(Borders::ALL),
                    )
                    .style(Style::default().fg(Color::White).bg(Color::Black));
                f.render_widget(widget, f.size());
            })?;
            thread::sleep(args.speed);
        }
    }
    Ok(())
}

fn split_word(word: &str) -> (String, String, String) {
    let mid = (word.len() - 1) / 2;
    let center = word.chars().nth(mid).unwrap_or_default().to_string();
    let first_half = word.chars().take(mid).collect();
    let second_half = word.chars().skip(mid + 1).collect();
    (first_half, center, second_half)
}
