use std::{fs::File, io::BufReader, path::Path};

use anyhow::bail;

pub trait Document {
    fn page(&mut self, number: usize) -> anyhow::Result<Page>;
}

pub struct Page {
    number: usize,
    content: String,
}

impl Page {
    fn new(number: usize, content: impl ToString) -> Self {
        Self {
            number,
            content: content.to_string(),
        }
    }
    fn words<'a>(&'a self) -> WordsIterator<'a> {
        WordsIterator {
            index: 0,
            words: self.content.split_whitespace().collect(),
        }
    }
}

struct WordsIterator<'a> {
    index: usize,
    words: Vec<&'a str>,
}

impl<'a> Iterator for WordsIterator<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.words.len() {
            let result = self.words.get(self.index).copied();
            self.index += 1;
            result
        } else {
            None
        }
    }
}

struct EpubDoc {
    doc: epub::doc::EpubDoc<BufReader<File>>,
}

impl EpubDoc {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        Ok(Self {
            doc: epub::doc::EpubDoc::new(path)?,
        })
    }
}

impl Document for EpubDoc {
    fn page(&mut self, number: usize) -> anyhow::Result<Page> {
        let page_id = self.doc.spine.get(number);
        let page_id = match page_id {
            Some(id) => id.to_string(),
            None => bail!("page id not found"),
        };
        let content = self.doc.get_resource(&page_id)?;
        let content = String::from_utf8(content)?;
        let content = html2text::from_read(content.as_bytes(), 100);
        Ok(Page::new(number, content))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use assert2::*;
    use rstest::*;
    use std::path::Path;

    #[rstest]
    fn it_gets_a_page(mut epub: EpubDoc, content: &str) {
        let page = epub.page(2);

        let_assert!(Ok(page) = page);
        check!(page.number == 2);
        check!(page.content == content);
    }

    #[rstest]
    fn it_iterates_over_words(mut epub: EpubDoc) {
        let page = epub.page(2).unwrap();
        let mut words = page.words();
        check!("[Dedication][1]" == words.next().unwrap());
        check!("For" == words.next().unwrap());
    }

    #[fixture]
    fn epub() -> EpubDoc {
        let path = Path::new("test.epub");
        EpubDoc::open(path).unwrap()
    }
    #[fixture]
    fn content() -> &'static str {
        "[Dedication][1]\n\nFor ELLEN,\nwho has been there for everything,\nincluding the books.\n\n—SJD\n\nFor my sister LINDA LEVITT JINES,\nwhose creative genius amazed,\namused, and inspired me.\n\n—SDL\n\n[1]: part0002.html#ded\n"
    }
}
