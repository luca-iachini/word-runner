use std::{fs::File, io::BufReader, path::Path};

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
    fn section(&mut self, number: usize) -> Result<Section>;
    fn cursor<'a>(&'a mut self) -> DocumentCursor<'a>;
    fn table_of_contents(&self) -> Vec<TableOfContentNode>;
}

pub struct DocumentCursor<'a> {
    doc: Box<&'a mut dyn Document>,
    page_index: usize,
    word_index: usize,
}

impl<'a> DocumentCursor<'a> {
    fn new(doc: &'a mut impl Document) -> Self {
        Self {
            doc: Box::new(doc),
            page_index: 0,
            word_index: 0,
        }
    }

    pub fn current_section(&mut self) -> Option<Section> {
        self.doc.section(self.page_index).ok()
    }

    pub fn current_word(&mut self) -> Option<String> {
        self.current_section()?.word(self.word_index)
    }

    pub fn next_section(&mut self) {
        self.page_index += 1;
    }

    pub fn next_word(&mut self) {
        self.word_index += 1;
    }
}

#[derive(Debug, Default, Clone)]
pub struct Section {
    pub number: usize,
    pub content: String,
}

impl Section {
    fn new(number: usize, content: impl ToString) -> Self {
        Self {
            number,
            content: content.to_string(),
        }
    }
    pub fn word(&self, index: usize) -> Option<String> {
        self.content
            .split_whitespace()
            .nth(index)
            .map(ToString::to_string)
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
    fn section(&mut self, number: usize) -> Result<Section> {
        let section_id = self.doc.spine.get(number);
        let section_id = match section_id {
            Some(id) => id.to_string(),
            None => bail!("page id not found"),
        };
        let (content, _mime) = self
            .doc
            .get_resource(&section_id)
            .ok_or(anyhow!("no resource"))?;
        let content = String::from_utf8(content)?;
        let content = html2text::from_read(content.as_bytes(), 100);
        Ok(Section::new(number, content))
    }

    fn table_of_contents(&self) -> Vec<TableOfContentNode> {
        self.doc.toc.iter().map(Into::into).collect()
    }
    fn cursor<'a>(&'a mut self) -> DocumentCursor<'a> {
        DocumentCursor::new(self)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use assert2::*;
    use rstest::*;
    use std::path::Path;

    #[rstest]
    fn it_gets_a_section(mut epub: EpubDoc, content: &str) {
        let page = epub.section(2);

        let_assert!(Ok(page) = page);
        check!(page.number == 2);
        check!(page.content == content);
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
