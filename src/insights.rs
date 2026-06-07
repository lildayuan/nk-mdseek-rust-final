use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::analyzer::KnowledgeReport;
use crate::tokenizer::{SimpleTokenizer, Tokenizer};
use crate::types::{Document, LinkKind};

#[derive(Clone, Debug, PartialEq)]
pub struct LinkSuggestion {
    pub source: PathBuf,
    pub target: PathBuf,
    pub score: f32,
    pub reasons: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SuggestionOptions {
    pub limit: usize,
    pub min_score: u32,
}

impl Default for SuggestionOptions {
    fn default() -> Self {
        Self {
            limit: 12,
            min_score: 5,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Severity {
    Critical,
    Warning,
    Info,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KnowledgeIssue {
    pub severity: Severity,
    pub title: String,
    pub detail: String,
    pub action: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KnowledgeHealth {
    pub score: u8,
    pub issues: Vec<KnowledgeIssue>,
}

#[derive(Clone, Debug)]
struct DocumentProfile {
    path: PathBuf,
    title: String,
    title_terms: HashSet<String>,
    heading_terms: HashSet<String>,
    body_terms: HashSet<String>,
    tags: HashSet<String>,
    linked_keys: HashSet<String>,
    body_lowercase: String,
}

pub fn suggest_links(documents: &[Document], options: SuggestionOptions) -> Vec<LinkSuggestion> {
    let tokenizer = SimpleTokenizer::default().with_min_len(2);
    let profiles = documents
        .iter()
        .map(|document| profile_document(document, &tokenizer))
        .collect::<Vec<_>>();
    let title_keys = profiles
        .iter()
        .map(|profile| (path_key(&profile.path), profile.path.clone()))
        .collect::<HashMap<_, _>>();
    let mut suggestions = Vec::new();

    for source in &profiles {
        for target in &profiles {
            if source.path == target.path || already_linked(source, target, &title_keys) {
                continue;
            }

            let (score, reasons) = score_pair(source, target);
            if score >= options.min_score {
                suggestions.push(LinkSuggestion {
                    source: source.path.clone(),
                    target: target.path.clone(),
                    score: score as f32,
                    reasons,
                });
            }
        }
    }

    suggestions.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.source.cmp(&right.source))
            .then_with(|| left.target.cmp(&right.target))
    });
    suggestions.truncate(options.limit.max(1));
    suggestions
}

pub fn diagnose_knowledge_base(
    report: &KnowledgeReport,
    suggestions: &[LinkSuggestion],
) -> KnowledgeHealth {
    let mut score: i32 = 100;
    let mut issues = Vec::new();

    if !report.broken_links.is_empty() {
        let penalty = (report.broken_links.len() as i32 * 8).min(40);
        score -= penalty;
        issues.push(KnowledgeIssue {
            severity: Severity::Critical,
            title: "Broken internal links".to_string(),
            detail: format!(
                "{} internal links point to missing notes.",
                report.broken_links.len()
            ),
            action: "Run `mdseek links` and fix or remove the listed links.".to_string(),
        });
    }

    if !report.orphan_documents.is_empty() {
        let penalty = (report.orphan_documents.len() as i32 * 5).min(30);
        score -= penalty;
        issues.push(KnowledgeIssue {
            severity: Severity::Warning,
            title: "Orphan documents".to_string(),
            detail: format!(
                "{} notes have no incoming internal links.",
                report.orphan_documents.len()
            ),
            action: "Add backlinks from related notes or archive stale notes.".to_string(),
        });
    }

    if report.tags.is_empty() && report.total_documents > 0 {
        score -= 10;
        issues.push(KnowledgeIssue {
            severity: Severity::Info,
            title: "No tags found".to_string(),
            detail: "The knowledge base has notes but no parsed tags.".to_string(),
            action: "Add lightweight #tags to improve grouping and discovery.".to_string(),
        });
    }

    if report.total_documents >= 2 && report.total_links == 0 {
        score -= 20;
        issues.push(KnowledgeIssue {
            severity: Severity::Warning,
            title: "No internal link graph".to_string(),
            detail: "Multiple notes exist, but none of them link to each other.".to_string(),
            action: "Use `mdseek suggest-links` to find likely missing links.".to_string(),
        });
    }

    if !suggestions.is_empty() {
        let penalty = (suggestions.len() as i32 * 2).min(16);
        score -= penalty;
        issues.push(KnowledgeIssue {
            severity: Severity::Info,
            title: "Potential missing links".to_string(),
            detail: format!(
                "{} high-confidence note relationships are not linked yet.",
                suggestions.len()
            ),
            action: "Review `mdseek suggest-links` suggestions and add useful wiki links."
                .to_string(),
        });
    }

    issues.sort_by(|left, right| {
        left.severity
            .cmp(&right.severity)
            .then_with(|| left.title.cmp(&right.title))
    });

    KnowledgeHealth {
        score: score.clamp(0, 100) as u8,
        issues,
    }
}

fn profile_document(document: &Document, tokenizer: &SimpleTokenizer) -> DocumentProfile {
    let title = document.display_title();
    let title_terms = tokenizer
        .tokenize(&title)
        .into_iter()
        .collect::<HashSet<_>>();
    let heading_text = document
        .headings
        .iter()
        .map(|heading| heading.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let heading_terms = tokenizer
        .tokenize(&heading_text)
        .into_iter()
        .collect::<HashSet<_>>();
    let body_terms = tokenizer
        .tokenize(&document.body)
        .into_iter()
        .collect::<HashSet<_>>();
    let tags = document.tags.iter().cloned().collect::<HashSet<_>>();
    let linked_keys = document
        .links
        .iter()
        .filter(|link| link.kind != LinkKind::External)
        .map(|link| normalize_link_key(&link.target))
        .collect::<HashSet<_>>();

    DocumentProfile {
        path: document.path.clone(),
        title,
        title_terms,
        heading_terms,
        body_terms,
        tags,
        linked_keys,
        body_lowercase: document.body.to_lowercase(),
    }
}

fn score_pair(source: &DocumentProfile, target: &DocumentProfile) -> (u32, Vec<String>) {
    let mut score = 0;
    let mut reasons = Vec::new();

    let shared_tags = intersection(&source.tags, &target.tags);
    if !shared_tags.is_empty() {
        score += shared_tags.len() as u32 * 4;
        reasons.push(format!("shared tags: {}", shared_tags.join(", ")));
    }

    let title_overlap = intersection(&source.body_terms, &target.title_terms);
    if !title_overlap.is_empty() {
        score += title_overlap.len() as u32 * 3;
        reasons.push(format!(
            "source mentions target title terms: {}",
            title_overlap.join(", ")
        ));
    }

    let heading_overlap = intersection(&source.heading_terms, &target.heading_terms);
    if !heading_overlap.is_empty() {
        score += heading_overlap.len() as u32 * 2;
        reasons.push(format!("related headings: {}", heading_overlap.join(", ")));
    }

    let cross_title_overlap = intersection(&source.title_terms, &target.body_terms);
    if !cross_title_overlap.is_empty() {
        score += cross_title_overlap.len() as u32 * 2;
        reasons.push(format!(
            "target mentions source title terms: {}",
            cross_title_overlap.join(", ")
        ));
    }

    let title_phrase = target.title.to_lowercase();
    if title_phrase.chars().count() >= 4 && source.body_lowercase.contains(&title_phrase) {
        score += 6;
        reasons.push("source contains the target title phrase".to_string());
    }

    if reasons.is_empty() {
        reasons.push("weak lexical similarity".to_string());
    }

    (score, reasons)
}

fn already_linked(
    source: &DocumentProfile,
    target: &DocumentProfile,
    title_keys: &HashMap<String, PathBuf>,
) -> bool {
    let target_key = path_key(&target.path);
    let target_stem = target
        .path
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_lowercase())
        .unwrap_or_else(|| target_key.clone());

    source.linked_keys.iter().any(|link_key| {
        link_key == &target_key
            || link_key == &target_stem
            || title_keys
                .get(link_key)
                .map(|path| path == &target.path)
                .unwrap_or(false)
    })
}

fn normalize_link_key(target: &str) -> String {
    let mut value = target
        .split('#')
        .next()
        .unwrap_or(target)
        .split('?')
        .next()
        .unwrap_or(target)
        .trim()
        .trim_matches('<')
        .trim_matches('>')
        .to_lowercase();

    if value.ends_with(".md") {
        value.truncate(value.len() - 3);
    }
    value.replace('\\', "/")
}

fn path_key(path: &Path) -> String {
    path.with_extension("")
        .display()
        .to_string()
        .replace('\\', "/")
        .to_lowercase()
}

fn intersection(left: &HashSet<String>, right: &HashSet<String>) -> Vec<String> {
    let mut values = left.intersection(right).cloned().collect::<Vec<_>>();
    values.sort();
    values
}

#[cfg(test)]
mod tests {
    use super::{diagnose_knowledge_base, suggest_links, Severity, SuggestionOptions};
    use crate::analyzer::analyze_documents;
    use crate::parser::parse_markdown;
    use crate::types::DocumentId;

    #[test]
    fn suggests_missing_links_from_title_mentions_and_tags() {
        let docs = vec![
            parse_markdown(
                DocumentId(0),
                "notes/ownership.md",
                "# Ownership\nBorrowing works with ownership in Rust.\n#rust",
            ),
            parse_markdown(
                DocumentId(1),
                "notes/borrowing.md",
                "# Borrowing\nReferences avoid moving values.\n#rust",
            ),
        ];

        let suggestions = suggest_links(
            &docs,
            SuggestionOptions {
                limit: 5,
                min_score: 4,
            },
        );

        assert!(suggestions
            .iter()
            .any(|item| item.source.ends_with("ownership.md")
                && item.target.ends_with("borrowing.md")));
    }

    #[test]
    fn skips_existing_internal_links() {
        let docs = vec![
            parse_markdown(
                DocumentId(0),
                "notes/ownership.md",
                "# Ownership\nSee [[borrowing]].\n#rust",
            ),
            parse_markdown(DocumentId(1), "notes/borrowing.md", "# Borrowing\n#rust"),
        ];

        let suggestions = suggest_links(&docs, SuggestionOptions::default());

        assert!(!suggestions
            .iter()
            .any(|item| item.source.ends_with("ownership.md")
                && item.target.ends_with("borrowing.md")));
    }

    #[test]
    fn diagnoses_broken_links_orphans_and_missing_links() {
        let docs = vec![
            parse_markdown(
                DocumentId(0),
                "notes/a.md",
                "# A\nSee [missing](missing.md).\n#rust",
            ),
            parse_markdown(DocumentId(1), "notes/b.md", "# B\nA topic.\n#rust"),
        ];
        let report = analyze_documents(&docs);
        let suggestions = suggest_links(&docs, SuggestionOptions::default());

        let health = diagnose_knowledge_base(&report, &suggestions);

        assert!(health.score < 100);
        assert!(health
            .issues
            .iter()
            .any(|issue| issue.severity == Severity::Critical));
    }
}
