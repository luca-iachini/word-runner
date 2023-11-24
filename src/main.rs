use std::{path::Path, thread, time::Duration};

use document::Document;

mod document;

fn main() -> anyhow::Result<()> {
    let path = Path::new("test.epub");
    let mut doc = document::EpubDoc::open(path).expect("unable to open the epub");
    while let Some(page) = doc.pages().next() {
        while let Some(word) = page.words().next() {
            println!("{word}");
            thread::sleep(Duration::from_secs(1));
        }
    }
    Ok(())
}
