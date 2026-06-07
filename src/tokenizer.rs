pub trait Tokenizer {
    fn tokenize(&self, text: &str) -> Vec<String>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SimpleTokenizer {
    case_sensitive: bool,
    min_len: usize,
}

impl SimpleTokenizer {
    pub fn new(case_sensitive: bool) -> Self {
        Self {
            case_sensitive,
            min_len: 1,
        }
    }

    pub fn with_min_len(mut self, min_len: usize) -> Self {
        self.min_len = min_len.max(1);
        self
    }

    fn normalize(&self, value: &str) -> String {
        if self.case_sensitive {
            value.to_string()
        } else {
            value.to_lowercase()
        }
    }
}

impl Default for SimpleTokenizer {
    fn default() -> Self {
        Self::new(false)
    }
}

impl Tokenizer for SimpleTokenizer {
    fn tokenize(&self, text: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut current = String::new();

        for ch in text.chars() {
            if ch.is_alphanumeric() || ch == '_' || ch == '-' {
                current.push(ch);
            } else if !current.is_empty() {
                let token = self.normalize(&current);
                if token.chars().count() >= self.min_len {
                    tokens.push(token);
                }
                current.clear();
            }
        }

        if !current.is_empty() {
            let token = self.normalize(&current);
            if token.chars().count() >= self.min_len {
                tokens.push(token);
            }
        }

        tokens
    }
}

#[cfg(test)]
mod tests {
    use super::{SimpleTokenizer, Tokenizer};

    #[test]
    fn tokenizes_ascii_and_cjk_text() {
        let tokenizer = SimpleTokenizer::default();
        let tokens = tokenizer.tokenize("Rust 所有权, borrow-checker!");

        assert_eq!(tokens, vec!["rust", "所有权", "borrow-checker"]);
    }

    #[test]
    fn honors_case_sensitive_mode() {
        let tokenizer = SimpleTokenizer::new(true);

        assert_eq!(tokenizer.tokenize("Rust rust"), vec!["Rust", "rust"]);
    }
}
