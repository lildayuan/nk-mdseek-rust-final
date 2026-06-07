use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;

use crate::error::{MdSeekError, Result};
use crate::parser::parse_markdown;
use crate::types::{Document, DocumentId};

pub fn scan_markdown_files(root: impl AsRef<Path>) -> Result<Vec<PathBuf>> {
    let root = root.as_ref();
    let mut files = Vec::new();
    scan_dir(root, &mut files)?;
    files.sort();
    Ok(files)
}

pub fn load_documents(root: impl AsRef<Path>) -> Result<Vec<Document>> {
    let files = scan_markdown_files(root)?;
    load_documents_from_files(files)
}

fn load_documents_from_files(files: Vec<PathBuf>) -> Result<Vec<Document>> {
    if files.len() < 4 {
        return load_documents_sequential(&files);
    }

    let worker_count = thread::available_parallelism()
        .map(|value| value.get())
        .unwrap_or(1)
        .min(files.len());
    let chunk_size = files.len().div_ceil(worker_count);
    let jobs = files
        .into_iter()
        .enumerate()
        .collect::<Vec<(usize, PathBuf)>>();
    let (sender, receiver) = mpsc::channel();

    thread::scope(|scope| {
        for chunk in jobs.chunks(chunk_size) {
            let sender = sender.clone();
            scope.spawn(move || {
                let batch = chunk
                    .iter()
                    .map(|(index, path)| read_document(*index, path).map(|doc| (*index, doc)))
                    .collect::<Result<Vec<_>>>();
                let _ = sender.send(batch);
            });
        }
    });
    drop(sender);

    let mut indexed_documents = Vec::new();
    for batch in receiver {
        indexed_documents.extend(batch?);
    }
    indexed_documents.sort_by_key(|(index, _)| *index);

    Ok(indexed_documents
        .into_iter()
        .map(|(_, document)| document)
        .collect())
}

fn read_document(index: usize, path: &Path) -> Result<Document> {
    let content = fs::read_to_string(path).map_err(|source| MdSeekError::io(path, source))?;
    Ok(parse_markdown(DocumentId(index), path, &content))
}

fn load_documents_sequential(files: &[PathBuf]) -> Result<Vec<Document>> {
    files
        .iter()
        .enumerate()
        .map(|(index, path)| read_document(index, path))
        .collect()
}

fn scan_dir(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    if should_skip_dir(dir) {
        return Ok(());
    }

    let entries = fs::read_dir(dir).map_err(|source| MdSeekError::io(dir, source))?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let metadata = entry
            .metadata()
            .map_err(|source| MdSeekError::io(&path, source))?;

        if metadata.is_dir() {
            scan_dir(&path, files)?;
        } else if metadata.is_file() && is_markdown_file(&path) {
            files.push(path);
        }
    }

    Ok(())
}

fn should_skip_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| {
            name.starts_with('.')
                || matches!(name, "target" | "node_modules" | "vendor" | "__pycache__")
        })
        .unwrap_or(false)
}

fn is_markdown_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "md" | "markdown" | "mdown"
            )
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::{load_documents, scan_markdown_files};
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn scans_markdown_files_recursively() {
        let root = unique_temp_dir("scan");
        fs::create_dir_all(root.join("nested")).unwrap();
        fs::write(root.join("a.md"), "# A").unwrap();
        fs::write(root.join("nested").join("b.markdown"), "# B").unwrap();
        fs::write(root.join("ignore.txt"), "no").unwrap();

        let files = scan_markdown_files(&root).unwrap();

        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|path| path.ends_with("a.md")));
        assert!(files.iter().any(|path| path.ends_with("b.markdown")));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn loads_documents_with_stable_ids() {
        let root = unique_temp_dir("load");
        fs::create_dir_all(&root).unwrap();
        for index in 0..6 {
            fs::write(
                root.join(format!("note-{index}.md")),
                format!("# Note {index}"),
            )
            .unwrap();
        }

        let documents = load_documents(&root).unwrap();

        assert_eq!(documents.len(), 6);
        for (index, document) in documents.iter().enumerate() {
            assert_eq!(document.id.0, index);
        }

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
}
