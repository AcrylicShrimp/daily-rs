use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents a document which contains valuable information.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Document {
    /// The header of the document.
    pub header: DocumentHeader,
    /// The content of the document.
    pub content: String,
}

/// Represents the header of a document.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DocumentHeader {
    /// The origin of the document. Typically a normalized short name of the source.
    ///
    /// e.g. `github`, `notion`, `google docs`, `slack`, etc.
    pub origin: String,
    /// The url of the document. It must uniquely identify the document.
    pub url: String,
    /// The title of the document.
    pub title: String,
    /// The description of the document.
    pub description: Option<String>,
    /// The date and time when the document was authored.
    ///
    /// If there is no way to determine the date and time when the document was authored,
    /// the date and time when the document was fetched should be used.
    pub authored_at: DateTime<Utc>,
}
