mod epub;

pub trait Document {
    fn page(&mut self, number: usize) -> anyhow::Result<Page>;
}

pub struct Page {
    pub number: usize,
    pub content: String,
}
