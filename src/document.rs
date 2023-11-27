use std::{
    fs::File,
    io::{BufReader, Seek},
    path::Path,
};

use anyhow::{anyhow, bail, Result};
use epub::doc::NavPoint;

pub struct TableOfContentNode {
    pub name: String,
    pub children: Vec<TableOfContentNode>,
}

impl From<&NavPoint> for TableOfContentNode {
    fn from(value: &NavPoint) -> Self {
        Self {
            name: value.label.clone(),
            children: value.children.iter().map(Into::into).collect(),
        }
    }
}

pub trait Document {
    fn page(&mut self, number: usize) -> Result<Page>;
    fn pages<'a>(&'a mut self) -> PagesIterator<'a>;
    fn table_of_contents(&self) -> Vec<TableOfContentNode>;
}

pub struct PagesIterator<'a> {
    index: usize,
    document: Box<&'a mut dyn Document>,
}

impl<'a> Iterator for PagesIterator<'a> {
    type Item = Page;

    fn next(&mut self) -> Option<Self::Item> {
        let result = self.document.page(self.index).ok()?;
        self.index += 1;
        Some(result)
    }
}

#[derive(Debug, Default, Clone)]
pub struct Page {
    pub number: usize,
    pub content: String,
}

impl Page {
    fn new(number: usize, content: impl ToString) -> Self {
        Self {
            number,
            content: content.to_string(),
        }
    }
    pub fn words<'a>(&'a self) -> WordsIterator<'a> {
        WordsIterator {
            index: 0,
            words: self.content.split_whitespace().collect(),
        }
    }
}

pub struct WordsIterator<'a> {
    index: usize,
    words: Vec<&'a str>,
}

impl<'a> Iterator for WordsIterator<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        let result = self.words.get(self.index).copied()?;
        self.index += 1;
        Some(result)
    }
}

pub struct EpubDoc {
    doc: epub::doc::EpubDoc<BufReader<File>>,
}

impl EpubDoc {
    pub fn open(path: &Path) -> Result<Self> {
        Ok(Self {
            doc: epub::doc::EpubDoc::new(path)?,
        })
    }
}

impl Document for EpubDoc {
    fn page(&mut self, number: usize) -> Result<Page> {
        let page_id = self.doc.spine.get(number);
        let page_id = match page_id {
            Some(id) => id.to_string(),
            None => bail!("page id not found"),
        };
        let (content, _mime) = self
            .doc
            .get_resource(&page_id)
            .ok_or(anyhow!("no resource"))?;
        let content = String::from_utf8(content)?;
        let content = html2text::from_read(content.as_bytes(), 100);
        Ok(Page::new(number, content))
    }

    fn pages<'a>(&'a mut self) -> PagesIterator<'a> {
        PagesIterator {
            index: 0,
            document: Box::new(self),
        }
    }

    fn table_of_contents(&self) -> Vec<TableOfContentNode> {
        self.doc.toc.iter().map(Into::into).collect()
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
    fn it_iterates_over_pages(mut epub: EpubDoc) {
        let mut pages = epub.pages();

        let_assert!(Some(page) = pages.next());
        check!(page.number == 0);
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
