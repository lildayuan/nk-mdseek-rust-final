use std::fs;
use std::path::Path;

use crate::error::{MdSeekError, Result};
use crate::parser::parse_markdown;
use crate::types::{Document, DocumentId};

const CACHE_HEADER: &str = "MDSEEK_CACHE_V1";

pub fn save_cache(path: impl AsRef<Path>, documents: &[Document]) -> Result<()> {
    let path = path.as_ref();
    let mut output = String::new();
    output.push_str(CACHE_HEADER);
    output.push('\n');

    for document in documents {
        output.push_str("DOC\t");
        output.push_str(&document.id.0.to_string());
        output.push('\t');
        output.push_str(&escape(&document.path.display().to_string()));
        output.push('\t');
        output.push_str(&escape(&document.body));
        output.push('\n');
    }

    fs::write(path, output).map_err(|source| MdSeekError::io(path, source))
}

pub fn load_cache(path: impl AsRef<Path>) -> Result<Vec<Document>> {
    let path = path.as_ref();
    let content = fs::read_to_string(path).map_err(|source| MdSeekError::io(path, source))?;
    let mut lines = content.lines();
    let header = lines
        .next()
        .ok_or_else(|| MdSeekError::Storage("cache file is empty".to_string()))?;
    if header != CACHE_HEADER {
        return Err(MdSeekError::Storage(format!(
            "unsupported cache format: {header}"
        )));
    }

    let mut documents = Vec::new();
    for (line_index, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let parts = line.split('\t').collect::<Vec<_>>();
        if parts.len() != 4 || parts[0] != "DOC" {
            return Err(MdSeekError::Storage(format!(
                "invalid cache record at line {}",
                line_index + 2
            )));
        }

        let id = parts[1].parse::<usize>().map_err(|_| {
            MdSeekError::Storage(format!("invalid document id at line {}", line_index + 2))
        })?;
        let path = unescape(parts[2])?;
        let body = unescape(parts[3])?;
        documents.push(parse_markdown(DocumentId(id), path, &body));
    }

    Ok(documents)
}

fn escape(value: &str) -> String {
    let mut escaped = String::new();
    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '\t' => escaped.push_str("\\t"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn unescape(value: &str) -> Result<String> {
    let mut output = String::new();
    let mut chars = value.chars();

    while let Some(ch) = chars.next() {
        if ch != '\\' {
            output.push(ch);
            continue;
        }

        let Some(next) = chars.next() else {
            return Err(MdSeekError::Storage(
                "invalid trailing escape sequence".to_string(),
            ));
        };

        match next {
            '\\' => output.push('\\'),
            't' => output.push('\t'),
            'n' => output.push('\n'),
            'r' => output.push('\r'),
            other => {
                return Err(MdSeekError::Storage(format!(
                    "invalid escape sequence: \\{other}"
                )));
            }
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::{load_cache, save_cache};
    use crate::parser::parse_markdown;
    use crate::types::DocumentId;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn saves_and_loads_document_cache() {
        let cache_path = unique_temp_file("cache");
        let docs = vec![parse_markdown(
            DocumentId(0),
            "notes/a.md",
            "# A\nBody with\ttab and slash \\.",
        )];

        save_cache(&cache_path, &docs).unwrap();
        let loaded = load_cache(&cache_path).unwrap();

        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].body, docs[0].body);
        assert_eq!(loaded[0].path, docs[0].path);

        fs::remove_file(cache_path).unwrap();
    }

    fn unique_temp_file(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "mdseek-{name}-{}-{}.cache",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        path
    }
}
