use crate::{ChunkingStrategy, Document, DocumentChunk};

/// A strategy that splits a document into chunks of a fixed size.
///
/// The chunks are created by moving a window of a fixed size over the document.
/// The step size is the number of characters to move the window forward each time.
///
/// It is simple and fast, but results may be suboptimal.
///
/// Each chunk will have maximum `window_size` characters (**not** bytes), and
/// have overlapping content with the previous chunk with a size of `step_size` (in characters, too).
#[derive(Debug, Clone)]
pub struct SlidingWindowChunkingStrategy {
    window_size: usize,
    step_size: usize,
}

impl SlidingWindowChunkingStrategy {
    pub fn new(window_size: usize, step_size: usize) -> Self {
        Self {
            window_size,
            step_size,
        }
    }

    pub fn window_size(&self) -> usize {
        self.window_size
    }

    pub fn step_size(&self) -> usize {
        self.step_size
    }
}

impl ChunkingStrategy for SlidingWindowChunkingStrategy {
    fn chunk(&self, document: &Document) -> Vec<DocumentChunk> {
        let mut index = 0;
        let mut order = 0;
        let mut chunks = Vec::new();
        let chars = Vec::from_iter(document.content.chars());

        while index + self.window_size() <= chars.len() {
            chunks.push(DocumentChunk {
                url: document.header.url.clone(),
                order,
                content: chars[index..index + self.window_size()].iter().collect(),
            });
            index += self.step_size();
            order += 1;
        }

        if index < chars.len() {
            chunks.push(DocumentChunk {
                url: document.header.url.clone(),
                order,
                content: chars[index..].iter().collect(),
            });
        }

        chunks
    }
}
