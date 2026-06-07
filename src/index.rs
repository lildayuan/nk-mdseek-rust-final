use std::collections::HashMap;

use crate::tokenizer::{SimpleTokenizer, Tokenizer};
use crate::types::{Document, DocumentId, Field};

#[derive(Clone, Debug, PartialEq)]
pub struct Posting {
    pub doc_id: DocumentId,
    pub field: Field,
    pub positions: Vec<usize>,
}

impl Posting {
    pub fn frequency(&self) -> usize {
        self.positions.len()
    }
}

#[derive(Clone, Debug)]
pub struct SearchIndex {
    documents: Vec<Document>,
    terms: HashMap<String, Vec<Posting>>,
    doc_lengths: HashMap<DocumentId, usize>,
    tokenizer: SimpleTokenizer,
}

impl SearchIndex {
    pub fn build(documents: Vec<Document>, tokenizer: SimpleTokenizer) -> Self {
        let mut builder: HashMap<(String, DocumentId, Field), Vec<usize>> = HashMap::new();
        let mut doc_lengths = HashMap::new();

        for document in &documents {
            let mut position = 0;
            position = index_field(
                &mut builder,
                &tokenizer,
                document.id,
                Field::Title,
                document.title.as_deref().unwrap_or_default(),
                position,
            );
            position = index_field(
                &mut builder,
                &tokenizer,
                document.id,
                Field::Heading,
                &document
                    .headings
                    .iter()
                    .map(|heading| heading.text.as_str())
                    .collect::<Vec<_>>()
                    .join(" "),
                position,
            );
            position = index_field(
                &mut builder,
                &tokenizer,
                document.id,
                Field::Tag,
                &document.tags.join(" "),
                position,
            );
            position = index_field(
                &mut builder,
                &tokenizer,
                document.id,
                Field::Path,
                &document.path.display().to_string(),
                position,
            );
            position = index_field(
                &mut builder,
                &tokenizer,
                document.id,
                Field::Body,
                &document.body,
                position,
            );
            doc_lengths.insert(document.id, position);
        }

        let mut terms: HashMap<String, Vec<Posting>> = HashMap::new();
        for ((term, doc_id, field), positions) in builder {
            terms.entry(term).or_default().push(Posting {
                doc_id,
                field,
                positions,
            });
        }
        for postings in terms.values_mut() {
            postings.sort_by_key(|posting| (posting.doc_id, posting.field as u8));
        }

        Self {
            documents,
            terms,
            doc_lengths,
            tokenizer,
        }
    }

    pub fn documents(&self) -> &[Document] {
        &self.documents
    }

    pub fn terms(&self) -> &HashMap<String, Vec<Posting>> {
        &self.terms
    }

    pub fn postings(&self, term: &str) -> Option<&[Posting]> {
        self.terms.get(term).map(Vec::as_slice)
    }

    pub fn document(&self, id: DocumentId) -> Option<&Document> {
        self.documents.iter().find(|document| document.id == id)
    }

    pub fn document_length(&self, id: DocumentId) -> usize {
        self.doc_lengths.get(&id).copied().unwrap_or(0)
    }

    pub fn tokenize_query(&self, query: &str) -> Vec<String> {
        self.tokenizer.tokenize(query)
    }

    pub fn len(&self) -> usize {
        self.documents.len()
    }

    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }
}

fn index_field(
    builder: &mut HashMap<(String, DocumentId, Field), Vec<usize>>,
    tokenizer: &SimpleTokenizer,
    doc_id: DocumentId,
    field: Field,
    text: &str,
    start_position: usize,
) -> usize {
    let mut position = start_position;
    for token in tokenizer.tokenize(text) {
        builder
            .entry((token, doc_id, field))
            .or_default()
            .push(position);
        position += 1;
    }
    position
}

#[cfg(test)]
mod tests {
    use super::SearchIndex;
    use crate::parser::parse_markdown;
    use crate::tokenizer::SimpleTokenizer;
    use crate::types::DocumentId;

    #[test]
    fn builds_postings_for_title_and_body() {
        let doc = parse_markdown(
            DocumentId(0),
            "rust.md",
            "# Ownership\nOwnership and borrowing are Rust features.",
        );
        let index = SearchIndex::build(vec![doc], SimpleTokenizer::default());

        assert!(index.postings("ownership").unwrap().len() >= 2);
        assert!(index.postings("borrowing").is_some());
    }
}
