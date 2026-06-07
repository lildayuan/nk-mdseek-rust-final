use std::fs;
use std::path::PathBuf;

use mdseek::analyze_documents;
use mdseek::index::SearchIndex;
use mdseek::scanner::load_documents;
use mdseek::search::{search, SearchOptions};
use mdseek::storage::{load_cache, save_cache};
use mdseek::tokenizer::SimpleTokenizer;
use mdseek::{render_report, ReportFormat};

#[test]
fn scans_searches_analyzes_and_caches_markdown_notes() {
    let root = unique_temp_dir("flow");
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("rust.md"),
        "# Rust Ownership\nOwnership controls borrowing.\nSee [[search]].\n#rust",
    )
    .unwrap();
    fs::write(
        root.join("search.md"),
        "# Search\nInverted index and ranking.\n[Broken](missing.md)\n#rust #search",
    )
    .unwrap();

    let documents = load_documents(&root).unwrap();
    let index = SearchIndex::build(documents.clone(), SimpleTokenizer::default());
    let results = search(&index, "ownership borrowing", SearchOptions { limit: 5 });

    assert_eq!(documents.len(), 2);
    assert_eq!(results.len(), 1);
    assert!(results[0].title.contains("Rust"));

    let report = analyze_documents(&documents);
    assert_eq!(report.total_documents, 2);
    assert_eq!(report.total_links, 2);
    assert_eq!(report.broken_links.len(), 1);
    assert_eq!(report.tags[0].tag, "rust");
    assert!(
        render_report(&report, index.terms().len(), ReportFormat::Markdown)
            .contains("mdseek Knowledge Report")
    );

    let cache = root.join(".mdseek-cache");
    save_cache(&cache, &documents).unwrap();
    let cached_documents = load_cache(&cache).unwrap();
    assert_eq!(cached_documents.len(), 2);

    fs::remove_dir_all(root).unwrap();
}

fn unique_temp_dir(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "mdseek-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    path
}
