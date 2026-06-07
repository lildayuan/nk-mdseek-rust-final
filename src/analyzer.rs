use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};

use crate::types::{Document, Link, LinkKind};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BrokenLink {
    pub source: PathBuf,
    pub target: String,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Backlink {
    pub source: PathBuf,
    pub line: usize,
    pub text: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TagCount {
    pub tag: String,
    pub count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KnowledgeReport {
    pub total_documents: usize,
    pub total_links: usize,
    pub broken_links: Vec<BrokenLink>,
    pub backlinks: HashMap<PathBuf, Vec<Backlink>>,
    pub tags: Vec<TagCount>,
    pub orphan_documents: Vec<PathBuf>,
}

impl KnowledgeReport {
    pub fn backlinks_for(&self, path: impl AsRef<Path>) -> Vec<Backlink> {
        let target = normalize_path(path.as_ref());
        self.backlinks.get(&target).cloned().unwrap_or_default()
    }

    pub fn to_mermaid(&self) -> String {
        let mut lines = vec!["graph TD".to_string()];
        let mut edges = Vec::new();

        for (target, backlinks) in &self.backlinks {
            for backlink in backlinks {
                edges.push(format!(
                    "  {}[\"{}\"] --> {}[\"{}\"]",
                    node_id(&backlink.source),
                    display_path(&backlink.source),
                    node_id(target),
                    display_path(target)
                ));
            }
        }

        edges.sort();
        edges.dedup();
        lines.extend(edges);
        lines.join("\n")
    }
}

pub fn analyze_documents(documents: &[Document]) -> KnowledgeReport {
    let known_paths = documents
        .iter()
        .map(|document| normalize_path(&document.path))
        .collect::<HashSet<_>>();
    let mut backlinks: HashMap<PathBuf, Vec<Backlink>> = HashMap::new();
    let mut broken_links = Vec::new();
    let mut inbound_counts: HashMap<PathBuf, usize> = HashMap::new();
    let mut tag_counts: HashMap<String, usize> = HashMap::new();
    let mut total_links = 0;

    for document in documents {
        for tag in &document.tags {
            *tag_counts.entry(tag.clone()).or_insert(0) += 1;
        }

        for link in &document.links {
            if link.kind == LinkKind::External {
                continue;
            }

            total_links += 1;
            let Some(target) = resolve_internal_link(&document.path, link) else {
                continue;
            };

            if known_paths.contains(&target) {
                *inbound_counts.entry(target.clone()).or_insert(0) += 1;
                backlinks.entry(target).or_default().push(Backlink {
                    source: normalize_path(&document.path),
                    line: link.line,
                    text: link.text.clone().unwrap_or_else(|| link.raw.clone()),
                });
            } else {
                broken_links.push(BrokenLink {
                    source: normalize_path(&document.path),
                    target: link.target.clone(),
                    line: link.line,
                });
            }
        }
    }

    let mut tags = tag_counts
        .into_iter()
        .map(|(tag, count)| TagCount { tag, count })
        .collect::<Vec<_>>();
    tags.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.tag.cmp(&right.tag))
    });

    let mut orphan_documents = known_paths
        .iter()
        .filter(|path| !inbound_counts.contains_key(*path))
        .cloned()
        .collect::<Vec<_>>();
    orphan_documents.sort();

    broken_links.sort_by(|left, right| {
        left.source
            .cmp(&right.source)
            .then_with(|| left.line.cmp(&right.line))
            .then_with(|| left.target.cmp(&right.target))
    });

    for entries in backlinks.values_mut() {
        entries.sort_by(|left, right| {
            left.source
                .cmp(&right.source)
                .then_with(|| left.line.cmp(&right.line))
        });
    }

    KnowledgeReport {
        total_documents: documents.len(),
        total_links,
        broken_links,
        backlinks,
        tags,
        orphan_documents,
    }
}

fn resolve_internal_link(source: &Path, link: &Link) -> Option<PathBuf> {
    let clean = clean_target(&link.target);
    if clean.is_empty() {
        return Some(normalize_path(source));
    }

    let mut target = PathBuf::from(clean);
    if link.kind == LinkKind::Wiki && target.extension().is_none() {
        target.set_extension("md");
    }

    if target.is_relative() {
        let parent = source.parent().unwrap_or_else(|| Path::new(""));
        target = parent.join(target);
    }

    Some(normalize_path(&target))
}

fn clean_target(target: &str) -> &str {
    target
        .split('#')
        .next()
        .unwrap_or(target)
        .split('?')
        .next()
        .unwrap_or(target)
        .trim()
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

fn node_id(path: &Path) -> String {
    let mut value = String::from("N");
    for ch in path.display().to_string().chars() {
        if ch.is_ascii_alphanumeric() {
            value.push(ch);
        } else {
            value.push('_');
        }
    }
    value
}

fn display_path(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::analyze_documents;
    use crate::parser::parse_markdown;
    use crate::types::DocumentId;
    use std::path::PathBuf;

    #[test]
    fn detects_backlinks_broken_links_and_tags() {
        let docs = vec![
            parse_markdown(
                DocumentId(0),
                "notes/a.md",
                "# A\nSee [[b]] and [missing](missing.md).\n#rust",
            ),
            parse_markdown(DocumentId(1), "notes/b.md", "# B\n#rust #search"),
        ];

        let report = analyze_documents(&docs);

        assert_eq!(report.total_documents, 2);
        assert_eq!(report.total_links, 2);
        assert_eq!(report.broken_links.len(), 1);
        assert_eq!(
            report.backlinks_for("notes/b.md")[0].source,
            PathBuf::from("notes/a.md")
        );
        assert_eq!(report.tags[0].tag, "rust");
    }
}
