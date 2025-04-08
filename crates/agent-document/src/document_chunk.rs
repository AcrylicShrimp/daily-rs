use serde::{Deserialize, Serialize};

/// Represents a chunk of a document.
///
/// A single document is typically split into multiple chunks.
/// Each chunk is a part of the document and contains a part of the information.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DocumentChunk {
    /// The url of the chunk.
    pub url: String,
    /// The order of the chunk.
    pub order: u32,
    /// The content of the chunk.
    pub content: String,
}
