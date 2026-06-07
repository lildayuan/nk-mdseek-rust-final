pub mod analyzer;
pub mod cli;
pub mod error;
pub mod index;
pub mod insights;
pub mod parser;
pub mod report;
pub mod scanner;
pub mod search;
pub mod storage;
pub mod tokenizer;
pub mod types;

pub use analyzer::{analyze_documents, KnowledgeReport};
pub use error::{MdSeekError, Result};
pub use index::SearchIndex;
pub use insights::{
    diagnose_knowledge_base, suggest_links, KnowledgeHealth, KnowledgeIssue, LinkSuggestion,
    Severity, SuggestionOptions,
};
pub use parser::parse_markdown;
pub use report::{
    render_report, render_report_with_extras, ReportExtras, ReportFormat, ReportRenderer,
};
pub use scanner::{load_documents, scan_markdown_files};
pub use search::{search, SearchOptions, SearchResult};
pub use tokenizer::{SimpleTokenizer, Tokenizer};
pub use types::{Document, DocumentId, Field, Heading, Link, LinkKind};
