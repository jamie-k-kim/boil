//! External documentation adapters.
//!
//! These adapters parse standalone documentation files (e.g., Markdown ADRs)
//! or fetch from external systems (e.g., Notion, Confluence) to link design
//! decisions to the canon graph.

pub mod confluence;
pub mod markdown;
pub mod notion;

pub use confluence::ConfluenceProvider;
pub use markdown::MarkdownProvider;
pub use notion::NotionProvider;
