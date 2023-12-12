use std::{
    fs::File,
    io::{BufReader, BufWriter, Write},
    ops::{Deref, DerefMut},
    path::Path,
    usize,
};

use anyhow::Result;
use epub::doc::NavPoint;
use itertools::Itertools;

#[derive(Debug)]
pub struct TableOfContentNode {
    pub index: usize,
    pub name: String,
    pub children: Vec<TableOfContentNode>,
}

impl TableOfContentNode {
    fn new(value: &NavPoint, doc: &epub::doc::EpubDoc<BufReader<File>>) -> Self {
        Self {
            index: doc.resource_uri_to_chapter(&value.content).unwrap(),
            name: value.label.clone(),
            children: value
                .children
                .iter()
                .map(|t| TableOfContentNode::new(t, doc))
                .collect(),
        }
    }
}

pub struct DocumentCursor {
    doc: EpubDoc,
    current_section: SectionCursor,
}

impl DocumentCursor {
    pub fn new(mut doc: EpubDoc, doc_state: DocState) -> Self {
        doc.set_current_page(doc_state.section_index);
        let mut current_section = doc
            .get_current()
            .map(|c| SectionCursor::new(doc.get_current_page(), c.0, 80))
            .unwrap_or_default();
        current_section.word_index = doc_state.word_index;
        Self {
            doc,
            current_section,
        }
    }
    pub fn section_index(&self) -> usize {
        self.doc.get_current_page()
    }
    pub fn current_section(&mut self) -> &mut SectionCursor {
        &mut self.current_section
    }

    pub fn current_section_or_resize(&mut self, size: usize) -> &mut SectionCursor {
        if self.current_section.size != size {
            self.current_section = SectionCursor::from_resize(&self.current_section, size);
        }
        &mut self.current_section
    }

    pub fn goto_section(&mut self, index: usize) -> bool {
        if self.doc.set_current_page(index) {
            self.load_section();
            true
        } else {
            false
        }
    }

    pub fn prev_section(&mut self) -> bool {
        if self.doc.go_prev() {
            self.load_section();
            true
        } else {
            false
        }
    }

    pub fn next_section(&mut self) -> bool {
        if self.doc.go_next() {
            self.load_section();
            true
        } else {
            false
        }
    }

    pub fn sections(&self) -> usize {
        self.doc.get_num_pages()
    }

    fn load_section(&mut self) {
        self.current_section = self
            .doc
            .get_current()
            .map(|c| {
                SectionCursor::new(
                    self.doc.get_current_page(),
                    c.0,
                    self.current_section().size,
                )
            })
            .unwrap_or_default();
    }

    pub fn toc_index(&self) -> Vec<usize> {
        toc_index(&self, &self.doc.table_of_contents())
    }

    pub fn doc_state(&self) -> DocState {
        DocState {
            identifier: self.doc.unique_identifier.clone().unwrap(),
            section_index: self.current_section.index,
            word_index: self.current_section.word_index,
        }
    }
}

fn toc_index(cursor: &DocumentCursor, toc: &[TableOfContentNode]) -> Vec<usize> {
    let mut res = vec![];
    if toc.is_empty() || cursor.section_index() < cursor.doc.table_of_contents()[0].index {
        return res;
    }

    let mut i = 0;
    while i < toc.len() - 1 {
        if toc[i].index >= cursor.section_index() && cursor.section_index() < toc[i + 1].index {
            break;
        }
        i += 1;
    }
    res.push(toc[i].index);
    if !toc[i].children.is_empty() {
        res.extend(toc_index(cursor, &toc[i].children));
    }
    res
}

#[derive(Debug, Default, Clone)]
pub struct SectionCursor {
    pub index: usize,
    pub content: String,
    pub raw_content: Vec<u8>,
    pub lines: Vec<Line>,
    word_index: usize,
    line_index: usize,
    size: usize,
}

impl SectionCursor {
    fn new(number: usize, raw_content: Vec<u8>, size: usize) -> Self {
        let content = String::from_utf8(raw_content.clone()).unwrap();
        let content = html2text::from_read(content.as_bytes(), size);
        let lines = lines(content.clone());
        let word_index = lines
            .first()
            .map(|l| l.word_indexes.first())
            .flatten()
            .copied()
            .unwrap_or_default();
        Self {
            index: number,
            content,
            raw_content,
            lines,
            word_index,
            line_index: 0,
            size,
        }
    }

    fn from_resize(other: &Self, size: usize) -> Self {
        let mut result = SectionCursor::new(other.index, other.raw_content.clone(), size);
        result.word_index = other.word_index;
        result.line_index = result
            .lines
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                e.word_indexes.first().copied().unwrap_or_default() <= other.word_index
                    && other.word_index <= e.word_indexes.last().copied().unwrap_or_default()
            })
            .map(|(i, _)| i)
            .next()
            .unwrap_or_default();
        result
    }

    pub fn word_index(&self) -> usize {
        self.word_index
    }

    pub fn current_line(&self) -> Option<&Line> {
        self.line(self.line_index)
    }

    pub fn current_word(&self) -> Option<String> {
        self.current_line()?.current_word(self.word_index)
    }

    pub fn line(&self, index: usize) -> Option<&Line> {
        self.lines.get(index)
    }

    pub fn prev_word(&mut self) -> bool {
        let index = self
            .current_line()
            .map(|l| l.prev_word(self.word_index))
            .flatten();

        if let Some(index) = index {
            self.word_index = index;
            true
        } else {
            self.prev_line()
        }
    }

    pub fn next_word(&mut self) -> bool {
        let index = self
            .current_line()
            .map(|l| l.next_word(self.word_index))
            .flatten();

        if let Some(index) = index {
            self.word_index = index;
            true
        } else {
            self.next_line()
        }
    }

    pub fn next_line(&mut self) -> bool {
        if self.line_index + 1 > self.lines.len() {
            return false;
        }

        self.line_index += 1;

        self.word_index = self
            .current_line()
            .map(|l| l.first_word_index())
            .unwrap_or_default();
        true
    }

    pub fn prev_line(&mut self) -> bool {
        if self.line_index == 0 {
            return false;
        }
        self.line_index = self.line_index.saturating_sub(1);
        self.word_index = self
            .current_line()
            .map(|l| l.last_word_index())
            .unwrap_or_default();
        true
    }
}

fn lines(content: String) -> Vec<Line> {
    let mut result = vec![];
    let mut global_words_index = 0;
    for (i, l) in content.lines().filter(|l| !l.is_empty()).enumerate() {
        let valid_words: Vec<usize> = l
            .split_whitespace()
            .enumerate()
            //.filter(|(_, w)| w.chars().any(char::is_alphabetic))
            .map(|(i, _)| global_words_index + i)
            .collect();
        global_words_index = valid_words.last().copied().unwrap_or_default();
        result.push(Line {
            index: i,
            word_indexes: valid_words,
            content: l.to_string(),
        });
    }
    result
}

#[derive(Debug, Clone, Default)]
pub struct Line {
    pub index: usize,
    pub word_indexes: Vec<usize>,
    pub content: String,
}

impl Line {
    pub fn first_word_index(&self) -> usize {
        self.word_indexes.first().copied().unwrap_or_default()
    }
    pub fn last_word_index(&self) -> usize {
        self.word_indexes.last().copied().unwrap_or_default()
    }
    pub fn current_word(&self, global_word_index: usize) -> Option<String> {
        let index = self.word_position(global_word_index)?;
        self.content
            .split_whitespace()
            .nth(index)
            .map(|s| s.to_string())
    }
    pub fn word_position(&self, global_word_index: usize) -> Option<usize> {
        self.word_indexes
            .iter()
            .find_position(|w| **w == global_word_index)
            .map(|(i, _)| i)
    }

    fn prev_word(&self, global_word_index: usize) -> Option<usize> {
        let line_index = self.word_position(global_word_index)?;
        if line_index > 0 {
            self.word_indexes.get(line_index - 1).copied()
        } else {
            None
        }
    }
    fn next_word(&self, global_word_index: usize) -> Option<usize> {
        let line_index = self.word_position(global_word_index)?;
        if line_index < self.word_indexes.len() - 1 {
            self.word_indexes.get(line_index + 1).copied()
        } else {
            None
        }
    }
}

pub struct EpubDoc(epub::doc::EpubDoc<BufReader<File>>, Vec<TableOfContentNode>);

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
        let doc = epub::doc::EpubDoc::new(path)?;
        let toc = doc
            .toc
            .iter()
            .map(|t| TableOfContentNode::new(t, &doc))
            .collect();
        Ok(Self(doc, toc))
    }
    pub fn table_of_contents(&self) -> &[TableOfContentNode] {
        &self.1
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct DocState {
    pub identifier: String,
    pub section_index: usize,
    pub word_index: usize,
}

impl DocState {
    pub fn new(identifier: String) -> Self {
        Self {
            identifier,
            section_index: 0,
            word_index: 0,
        }
    }
    pub fn load(config_dir: &Path, identifier: String) -> Self {
        let config_path = config_dir.join(identifier.clone());
        let state = std::fs::read_to_string(config_path).ok();

        if let Some(state) = state {
            serde_json::from_str(&state).unwrap_or_else(|_| Self::new(identifier))
        } else {
            Self::new(identifier)
        }
    }
    pub fn store(&self, config_dir: &Path) -> anyhow::Result<()> {
        let config_path = config_dir.join(self.identifier.clone());
        let file = File::create(config_path)?;
        let mut writer = BufWriter::new(file);
        serde_json::to_writer(&mut writer, &self)?;
        writer.flush()?;
        Ok(())
    }
}
#[cfg(test)]
mod test {
    use super::*;
    use assert2::*;
    use rstest::*;
    use std::path::Path;

    #[rstest]
    fn it_gets_a_section(epub: EpubDoc) {
        let mut cursor = DocumentCursor::new(epub, DocState::new("xxxx".to_string()));

        let_assert!(section = cursor.current_section());
        check!(section.index == 1);

        cursor.next_section();
        check!(cursor.doc.spine.len() > 1);
        let_assert!(section = cursor.current_section());
        check!(section.index == 2);
    }

    #[rstest]
    fn it_gets_identifier(epub: EpubDoc) {
        dbg!(&epub.unique_identifier);
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
