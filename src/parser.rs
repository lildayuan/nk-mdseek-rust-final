use std::path::Path;

use crate::types::{Document, DocumentId, Heading, Link, LinkKind};

pub fn parse_markdown(id: DocumentId, path: impl AsRef<Path>, content: &str) -> Document {
    let path = path.as_ref().to_path_buf();
    let lines: Vec<String> = content.lines().map(ToOwned::to_owned).collect();
    let mut headings = Vec::new();
    let mut links = Vec::new();
    let mut tags = Vec::new();
    let mut in_code_fence = false;

    for (line_index, line) in lines.iter().enumerate() {
        let line_no = line_index + 1;
        let trimmed = line.trim_start();

        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_code_fence = !in_code_fence;
            continue;
        }

        if !in_code_fence {
            if let Some(heading) = parse_heading(trimmed, line_no) {
                headings.push(heading);
            }

            links.extend(parse_wiki_links(line, line_no));
            links.extend(parse_markdown_links(line, line_no));
            tags.extend(parse_tags(line));
        }
    }

    tags.sort();
    tags.dedup();

    let title = headings
        .iter()
        .find(|heading| heading.level == 1)
        .or_else(|| headings.first())
        .map(|heading| heading.text.clone());

    Document {
        id,
        path,
        title,
        headings,
        body: content.to_string(),
        lines,
        links,
        tags,
    }
}

fn parse_heading(line: &str, line_no: usize) -> Option<Heading> {
    let marker_len = line.chars().take_while(|ch| *ch == '#').count();
    if marker_len == 0 || marker_len > 6 {
        return None;
    }

    let rest = line.get(marker_len..)?;
    if !rest.starts_with(' ') {
        return None;
    }

    let text = rest.trim().trim_end_matches('#').trim().to_string();
    if text.is_empty() {
        return None;
    }

    Some(Heading {
        level: marker_len,
        text,
        line: line_no,
    })
}

fn parse_wiki_links(line: &str, line_no: usize) -> Vec<Link> {
    let mut links = Vec::new();
    let mut remaining = line;

    while let Some(start) = remaining.find("[[") {
        let after_start = &remaining[start + 2..];
        let Some(end) = after_start.find("]]") else {
            break;
        };

        let value = &after_start[..end];
        let raw = format!("[[{value}]]");
        let mut parts = value.splitn(2, '|');
        let target = parts.next().unwrap_or_default().trim().to_string();
        let text = parts.next().map(|part| part.trim().to_string());

        if !target.is_empty() {
            links.push(Link {
                raw,
                target,
                text,
                line: line_no,
                kind: LinkKind::Wiki,
            });
        }

        remaining = &after_start[end + 2..];
    }

    links
}

fn parse_markdown_links(line: &str, line_no: usize) -> Vec<Link> {
    let mut links = Vec::new();
    let bytes = line.as_bytes();
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] != b'[' || (index > 0 && bytes[index - 1] == b'!') {
            index += 1;
            continue;
        }

        let Some(close_label_rel) = line[index + 1..].find(']') else {
            break;
        };
        let close_label = index + 1 + close_label_rel;
        if !line[close_label + 1..].starts_with('(') {
            index = close_label + 1;
            continue;
        }

        let target_start = close_label + 2;
        let Some(close_target_rel) = line[target_start..].find(')') else {
            break;
        };
        let target_end = target_start + close_target_rel;
        let text = line[index + 1..close_label].trim();
        let target = line[target_start..target_end].trim();

        if !target.is_empty() {
            let kind = if is_external_target(target) {
                LinkKind::External
            } else {
                LinkKind::Markdown
            };
            links.push(Link {
                raw: line[index..=target_end].to_string(),
                target: target.to_string(),
                text: if text.is_empty() {
                    None
                } else {
                    Some(text.to_string())
                },
                line: line_no,
                kind,
            });
        }

        index = target_end + 1;
    }

    links
}

fn parse_tags(line: &str) -> Vec<String> {
    let mut tags = Vec::new();
    let mut chars = line.char_indices().peekable();

    while let Some((index, ch)) = chars.next() {
        if ch != '#' {
            continue;
        }

        if index == 0 && line[index..].starts_with("# ") {
            continue;
        }

        let mut tag = String::new();
        while let Some((_, next)) = chars.peek().copied() {
            if next.is_alphanumeric() || next == '_' || next == '-' || next == '/' {
                tag.push(next);
                chars.next();
            } else {
                break;
            }
        }

        if !tag.is_empty() {
            tags.push(tag.to_lowercase());
        }
    }

    tags
}

fn is_external_target(target: &str) -> bool {
    target.starts_with("http://") || target.starts_with("https://") || target.starts_with("mailto:")
}

#[cfg(test)]
mod tests {
    use super::parse_markdown;
    use crate::types::{DocumentId, LinkKind};

    #[test]
    fn parses_headings_links_and_tags() {
        let doc = parse_markdown(
            DocumentId(0),
            "notes/rust.md",
            "# Rust Notes\nSee [[Ownership|owning]] and [Borrowing](borrow.md).\n#rust #systems",
        );

        assert_eq!(doc.title.as_deref(), Some("Rust Notes"));
        assert_eq!(doc.headings.len(), 1);
        assert_eq!(doc.links.len(), 2);
        assert_eq!(doc.links[0].kind, LinkKind::Wiki);
        assert_eq!(doc.links[1].kind, LinkKind::Markdown);
        assert_eq!(doc.tags, vec!["rust", "systems"]);
    }

    #[test]
    fn ignores_markdown_inside_code_fences() {
        let doc = parse_markdown(
            DocumentId(0),
            "notes/code.md",
            "```rust\n# Not A Heading\n[[not-a-link]]\n```\n# Real Heading",
        );

        assert_eq!(doc.headings.len(), 1);
        assert_eq!(doc.links.len(), 0);
        assert_eq!(doc.title.as_deref(), Some("Real Heading"));
    }
}
