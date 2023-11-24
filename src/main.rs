use std::{path::PathBuf, thread, time::Duration};

use clap::{Parser, ValueHint};
use document::Document;

mod document;

#[derive(Parser)]
struct Args {
    #[clap(value_hint = ValueHint::AnyPath)]
    path: PathBuf,
    #[clap(short, value_parser = parse_speed)]
    speed: Duration,
}

fn parse_speed(arg: &str) -> Result<std::time::Duration, std::num::ParseIntError> {
    let seconds = arg.parse()?;
    Ok(std::time::Duration::from_secs(seconds))
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let mut doc = document::EpubDoc::open(&args.path).expect("unable to open the epub");
    let mut pages = doc.pages();
    while let Some(page) = pages.next() {
        println!("###### page {}", page.number);
        let mut words = page.words();
        while let Some(word) = words.next() {
            println!("{word}");
            thread::sleep(args.speed);
        }
    }
    Ok(())
}
