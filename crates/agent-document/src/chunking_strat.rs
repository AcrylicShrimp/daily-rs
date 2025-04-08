use crate::{Document, DocumentChunk};

pub trait ChunkingStrategy {
    fn chunk(&self, document: &Document) -> Vec<DocumentChunk>;
}
