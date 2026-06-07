use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::index::SearchIndex;
use crate::types::DocumentId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SearchOptions {
    pub limit: usize,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self { limit: 10 }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SearchResult {
    pub doc_id: DocumentId,
    pub path: PathBuf,
    pub title: String,
    pub score: f32,
    pub matched_terms: Vec<String>,
    pub snippet: Option<String>,
}

pub fn search(index: &SearchIndex, query: &str, options: SearchOptions) -> Vec<SearchResult> {
    let terms = unique_terms(index.tokenize_query(query));
    if terms.is_empty() || index.is_empty() {
        return Vec::new();
    }

    let mut scores: HashMap<DocumentId, f32> = HashMap::new();
    let mut matches: HashMap<DocumentId, HashSet<String>> = HashMap::new();
    let document_count = index.len() as f32;

    for term in &terms {
        let Some(postings) = index.postings(term) else {
            continue;
        };
        let doc_frequency = postings
            .iter()
            .map(|posting| posting.doc_id)
            .collect::<HashSet<_>>()
            .len() as f32;
        let idf = ((document_count + 1.0) / (doc_frequency + 1.0)).ln() + 1.0;

        for posting in postings {
            let length = index.document_length(posting.doc_id).max(1) as f32;
            let tf = posting.frequency() as f32 / length.sqrt();
            let score = tf * idf * posting.field.weight();
            *scores.entry(posting.doc_id).or_insert(0.0) += score;
            matches
                .entry(posting.doc_id)
                .or_default()
                .insert(term.clone());
        }
    }

    let mut results = scores
        .into_iter()
        .filter_map(|(doc_id, score)| {
            let document = index.document(doc_id)?;
            let mut matched_terms = matches
                .remove(&doc_id)
                .unwrap_or_default()
                .into_iter()
                .collect::<Vec<_>>();
            matched_terms.sort();

            Some(SearchResult {
                doc_id,
                path: document.path.clone(),
                title: document.display_title(),
                score,
                snippet: make_snippet(&document.lines, &matched_terms),
                matched_terms,
            })
        })
        .collect::<Vec<_>>();

    results.sort_by(compare_results);
    results.truncate(options.limit.max(1));
    results
}

fn unique_terms(terms: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut unique = Vec::new();
    for term in terms {
        if seen.insert(term.clone()) {
            unique.push(term);
        }
    }
    unique
}

fn make_snippet(lines: &[String], terms: &[String]) -> Option<String> {
    for (index, line) in lines.iter().enumerate() {
        let lower = line.to_lowercase();
        if terms
            .iter()
            .any(|term| lower.contains(&term.to_lowercase()))
        {
            let trimmed = line.trim();
            let snippet = if trimmed.chars().count() > 160 {
                let mut value = trimmed.chars().take(157).collect::<String>();
                value.push_str("...");
                value
            } else {
                trimmed.to_string()
            };
            return Some(format!("L{}: {}", index + 1, snippet));
        }
    }
    None
}

fn compare_results(left: &SearchResult, right: &SearchResult) -> Ordering {
    right
        .score
        .partial_cmp(&left.score)
        .unwrap_or(Ordering::Equal)
        .then_with(|| left.title.cmp(&right.title))
        .then_with(|| left.path.cmp(&right.path))
}

#[cfg(test)]
mod tests {
    use super::{search, SearchOptions};
    use crate::index::SearchIndex;
    use crate::parser::parse_markdown;
    use crate::tokenizer::SimpleTokenizer;
    use crate::types::DocumentId;

    #[test]
    fn returns_ranked_results_with_snippets() {
        let docs = vec![
            parse_markdown(DocumentId(0), "a.md", "# Ownership\nRust ownership rules."),
            parse_markdown(DocumentId(1), "b.md", "# Other\nOwnership appears once."),
        ];
        let index = SearchIndex::build(docs, SimpleTokenizer::default());

        let results = search(&index, "ownership", SearchOptions { limit: 5 });

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].path, std::path::PathBuf::from("a.md"));
        assert!(results[0].snippet.as_ref().unwrap().contains("Ownership"));
    }

    #[test]
    fn empty_query_returns_no_results() {
        let index = SearchIndex::build(Vec::new(), SimpleTokenizer::default());

        assert!(search(&index, "   ", SearchOptions::default()).is_empty());
    }
}
