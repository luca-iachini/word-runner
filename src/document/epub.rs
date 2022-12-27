use std::{fs::File, io::BufReader, path::Path};

use anyhow::bail;

use super::{Document, Page};

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
        Ok(Page { number, content })
    }
}

#[cfg(test)]
mod test {
    use crate::document::Document;
    use std::path::Path;

    #[test]
    fn open_epub() {
        let path = Path::new("test.epub");
        assert2::assert!(super::EpubDoc::open(path).is_ok());
    }

    #[test]
    fn get_page_two() {
        let path = Path::new("test.epub");
        let mut epub = super::EpubDoc::open(path).unwrap();
        let page = epub.page(2);

        assert2::let_assert!(Ok(page) = page);
        assert2::assert!(page.number == 2);
        assert2::assert!(page.content == "[Dedication][1]\n\nFor ELLEN,\nwho has been there for everything,\nincluding the books.\n\n—SJD\n\nFor my sister LINDA LEVITT JINES,\nwhose creative genius amazed,\namused, and inspired me.\n\n—SDL\n\n[1]: part0002.html#ded\n");
    }
}
