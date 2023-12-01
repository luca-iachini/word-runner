use std::{
    fs::File,
    io::BufReader,
    ops::{Deref, DerefMut},
    path::Path,
    usize,
};

use anyhow::Result;
use epub::doc::NavPoint;

#[derive(Debug)]
pub struct TableOfContentNode {
    pub index: usize,
    pub name: String,
    pub children: Vec<TableOfContentNode>,
}

impl From<&NavPoint> for TableOfContentNode {
    fn from(value: &NavPoint) -> Self {
        Self {
            index: value.play_order,
            name: value.label.clone(),
            children: value.children.iter().map(Into::into).collect(),
        }
    }
}

pub struct DocumentCursor {
    doc: EpubDoc,
    word_index: usize,
    line_index: usize,
    current_section: Option<Section>,
}

impl DocumentCursor {
    pub fn new(doc: EpubDoc) -> Self {
        Self {
            doc,
            word_index: 0,
            line_index: 0,
            current_section: None,
        }
    }

    pub fn current_section<'a>(&'a mut self) -> Option<&'a Section> {
        if self.current_section.is_none() {
            let current = self.doc.get_current()?;
            self.current_section = Some(Section::new(self.doc.get_current_page(), current.0))
        }
        self.current_section.as_ref()
    }

    pub fn current_line<'a>(&'a mut self) -> Option<&'a Line> {
        self.current_section()?.line(self.line_index)
    }

    pub fn current_word(&mut self) -> Option<String> {
        self.current_section()?.word(self.word_index)
    }

    pub fn word_index(&self) -> usize {
        self.word_index
    }

    pub fn prev_section(&mut self) {
        self.word_index = 0;
        self.line_index = 0;
        self.doc.go_prev();
    }

    pub fn next_section(&mut self) {
        self.word_index = 0;
        self.line_index = 0;
        self.doc.go_next();
    }

    pub fn go_to_section(&mut self, _section: usize) {
        //TODO
        self.word_index = 0;
        self.line_index = 0;
    }

    pub fn prev_word(&mut self) {
        self.word_index = self.word_index.saturating_sub(1);
        let start_of_line = self.current_line().map(|l| l.word_indexes.0).unwrap();
        if self.word_index < start_of_line {
            self.prev_line();
        }
    }

    pub fn next_word(&mut self) {
        if let Some(end_of_line) = self.current_line().map(|l| l.word_indexes.1) {
            self.word_index += 1;
            if self.word_index > end_of_line {
                self.next_line();
            }
        } else {
            self.next_line();
        }
    }

    pub fn next_line(&mut self) {
        self.line_index += 1;

        if self.current_line().is_none() {
            self.next_section()
        }
        self.word_index = self
            .current_line()
            .map(|l| l.word_indexes.0)
            .unwrap_or_default();
    }

    pub fn prev_line(&mut self) {
        if self.line_index == 0 {
            self.prev_section();
            self.line_index = self
                .current_section()
                .map(|s| s.lines.len())
                .unwrap_or_default();
            return;
        }
        self.line_index = self.line_index.saturating_sub(1);
        self.word_index = self
            .current_line()
            .map(|l| l.word_indexes.0)
            .unwrap_or_default();
    }
}

#[derive(Debug, Default, Clone)]
pub struct Section {
    pub number: usize,
    pub content: String,
    pub lines: Vec<Line>,
}

impl Section {
    fn new(number: usize, content: Vec<u8>) -> Self {
        let content = String::from_utf8(content).unwrap();
        let lines = lines(content.clone());
        Self {
            number,
            content,
            lines,
        }
    }

    pub fn line(&self, index: usize) -> Option<&Line> {
        self.lines.get(index)
    }

    pub fn word(&self, index: usize) -> Option<String> {
        self.content
            .split_whitespace()
            .nth(index)
            .map(ToString::to_string)
    }
}
fn lines(content: String) -> Vec<Line> {
    let mut result = vec![];
    let mut global_words_index = 0;
    for (i, l) in content.lines().filter(|l| !l.is_empty()).enumerate() {
        let words = l.split_whitespace().count();
        let end_word_index = if words == 0 {
            global_words_index
        } else {
            global_words_index + words - 1
        };
        result.push(Line {
            index: i,
            word_indexes: (global_words_index, end_word_index),
            words,
            content: l.to_string(),
        });
        global_words_index += words;
    }
    result
}

#[derive(Debug, Clone, Default)]
pub struct Line {
    pub index: usize,
    pub word_indexes: (usize, usize),
    pub words: usize,
    pub content: String,
}

pub struct EpubDoc(epub::doc::EpubDoc<BufReader<File>>);

impl Deref for EpubDoc {
    type Target = epub::doc::EpubDoc<BufReader<File>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for EpubDoc {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl EpubDoc {
    pub fn open(path: &Path) -> Result<Self> {
        Ok(Self(epub::doc::EpubDoc::new(path)?))
    }
    pub fn table_of_contents(&self) -> Vec<TableOfContentNode> {
        self.0.toc.iter().map(Into::into).collect()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use assert2::*;
    use rstest::*;
    use std::path::Path;

    //   #[rstest]
    //   fn it_gets_a_section(mut epub: EpubDoc, content: &str) {
    //       let page = epub.section(2);

    //       let_assert!(Ok(page) = page);
    //       check!(page.number == 2);
    //       check!(page.content == content);
    //   }

    //   #[rstest]
    //   fn it_gets_current_word(epub: EpubDoc) {
    //       let mut cursor = DocumentCursor::new(epub);
    //       cursor.next_section();
    //       cursor.next_section();
    //       //cursor.next_word();
    //       assert_eq!(cursor.current_word(), Some("[Dedication][1]".to_string()));
    //   }

    //   #[rstest]
    //   fn it_moves_between_words(epub: EpubDoc) {
    //       let mut cursor = DocumentCursor::new(epub);
    //       cursor.next_section();
    //       cursor.next_section();
    //       cursor.next_word();
    //       assert_eq!(cursor.current_word(), Some("For".to_string()));
    //       cursor.prev_word();
    //       assert_eq!(cursor.current_word(), Some("[Dedication][1]".to_string()));
    //   }

    //   #[rstest]
    //   fn it_moves_on_not_empty_lines(epub: EpubDoc) {
    //       let mut cursor = DocumentCursor::new(epub);
    //       cursor.next_section();
    //       cursor.next_section();
    //       cursor.next_word();
    //       let_assert!(Some(line) = cursor.current_line());
    //       assert_eq!(line.word_indexes, (1, 2));
    //       assert_eq!(line.index, 2);
    //       cursor.prev_word();
    //       let_assert!(Some(line) = cursor.current_line());
    //       assert_eq!(line.word_indexes, (0, 0));
    //       assert_eq!(line.index, 0);
    //   }

    #[rstest]
    fn it_get_table_of_content(epub: EpubDoc) {
        let toc = epub.table_of_contents();
        dbg!(toc);
        check!(false);
    }

    #[fixture]
    fn epub() -> EpubDoc {
        let path = Path::new("Extreme ProgrammingExplained.epub");
        EpubDoc::open(path).unwrap()
    }
    #[fixture]
    fn content() -> &'static str {
        "[Dedication][1]\n\nFor ELLEN,\nwho has been there for everything,\nincluding the books.\n\n—SJD\n\nFor my sister LINDA LEVITT JINES,\nwhose creative genius amazed,\namused, and inspired me.\n\n—SDL\n\n[1]: part0002.html#ded\n"
    }
}
