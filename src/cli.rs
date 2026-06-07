use std::env;
use std::fs;
use std::path::PathBuf;

use crate::analyzer::analyze_documents;
use crate::error::{MdSeekError, Result};
use crate::index::SearchIndex;
use crate::insights::{diagnose_knowledge_base, suggest_links, Severity, SuggestionOptions};
use crate::report::{render_report_with_extras, ReportExtras, ReportFormat};
use crate::scanner::load_documents;
use crate::search::{search, SearchOptions};
use crate::storage::{load_cache, save_cache};
use crate::tokenizer::SimpleTokenizer;
use crate::types::Document;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Command {
    Index {
        root: PathBuf,
        cache: PathBuf,
    },
    Search {
        query: String,
        root: PathBuf,
        cache: Option<PathBuf>,
        limit: usize,
        case_sensitive: bool,
    },
    Stats {
        root: PathBuf,
        cache: Option<PathBuf>,
    },
    Links {
        root: PathBuf,
        cache: Option<PathBuf>,
    },
    Backlinks {
        target: PathBuf,
        root: PathBuf,
        cache: Option<PathBuf>,
    },
    Graph {
        root: PathBuf,
        cache: Option<PathBuf>,
    },
    Report {
        root: PathBuf,
        cache: Option<PathBuf>,
        format: ReportFormat,
        output: Option<PathBuf>,
    },
    SuggestLinks {
        root: PathBuf,
        cache: Option<PathBuf>,
        limit: usize,
        min_score: u32,
    },
    Doctor {
        root: PathBuf,
        cache: Option<PathBuf>,
    },
    Help,
}

pub fn run<I, S>(args: I) -> Result<()>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let command = parse_args(args.into_iter().map(Into::into).collect())?;
    execute(command)
}

pub fn parse_args(args: Vec<String>) -> Result<Command> {
    let mut parser = ArgParser::new(args);
    let Some(command) = parser.next_command()? else {
        return Ok(Command::Help);
    };

    match command.as_str() {
        "index" => {
            let root = parser.next_value("root directory")?;
            let cache = parser
                .optional_path("--cache")?
                .unwrap_or_else(|| root.join(".mdseek-cache"));
            parser.finish()?;
            Ok(Command::Index { root, cache })
        }
        "search" => {
            let query = parser.next_string("query")?;
            let root = parser
                .optional_path("--root")?
                .unwrap_or(env::current_dir()?);
            let cache = parser.optional_path("--cache")?;
            let limit = parser.optional_usize("--limit")?.unwrap_or(10);
            let case_sensitive = parser.optional_flag("--case-sensitive")?;
            parser.finish()?;
            Ok(Command::Search {
                query,
                root,
                cache,
                limit,
                case_sensitive,
            })
        }
        "stats" => {
            let root = parser
                .optional_path("--root")?
                .unwrap_or(env::current_dir()?);
            let cache = parser.optional_path("--cache")?;
            parser.finish()?;
            Ok(Command::Stats { root, cache })
        }
        "links" => {
            let root = parser
                .optional_path("--root")?
                .unwrap_or(env::current_dir()?);
            let cache = parser.optional_path("--cache")?;
            parser.finish()?;
            Ok(Command::Links { root, cache })
        }
        "backlinks" => {
            let target = parser.next_value("target markdown file")?;
            let root = parser
                .optional_path("--root")?
                .unwrap_or(env::current_dir()?);
            let cache = parser.optional_path("--cache")?;
            parser.finish()?;
            Ok(Command::Backlinks {
                target,
                root,
                cache,
            })
        }
        "graph" => {
            let root = parser
                .optional_path("--root")?
                .unwrap_or(env::current_dir()?);
            let cache = parser.optional_path("--cache")?;
            parser.finish()?;
            Ok(Command::Graph { root, cache })
        }
        "report" => {
            let root = parser
                .optional_path("--root")?
                .unwrap_or(env::current_dir()?);
            let cache = parser.optional_path("--cache")?;
            let format = parser
                .optional_string("--format")?
                .map(|value| {
                    ReportFormat::parse(&value).ok_or_else(|| {
                        MdSeekError::InvalidArgs(
                            "--format must be markdown, md, html, or htm".to_string(),
                        )
                    })
                })
                .transpose()?
                .unwrap_or(ReportFormat::Markdown);
            let output = parser.optional_path("--output")?;
            parser.finish()?;
            Ok(Command::Report {
                root,
                cache,
                format,
                output,
            })
        }
        "suggest-links" => {
            let root = parser
                .optional_path("--root")?
                .unwrap_or(env::current_dir()?);
            let cache = parser.optional_path("--cache")?;
            let limit = parser.optional_usize("--limit")?.unwrap_or(12);
            let min_score = parser.optional_usize("--min-score")?.unwrap_or(5) as u32;
            parser.finish()?;
            Ok(Command::SuggestLinks {
                root,
                cache,
                limit,
                min_score,
            })
        }
        "doctor" => {
            let root = parser
                .optional_path("--root")?
                .unwrap_or(env::current_dir()?);
            let cache = parser.optional_path("--cache")?;
            parser.finish()?;
            Ok(Command::Doctor { root, cache })
        }
        "help" | "-h" | "--help" => Ok(Command::Help),
        other => Err(MdSeekError::InvalidArgs(format!(
            "unknown command '{other}'. Run `mdseek help`."
        ))),
    }
}

fn execute(command: Command) -> Result<()> {
    match command {
        Command::Index { root, cache } => {
            let documents = load_documents(&root)?;
            save_cache(&cache, &documents)?;
            println!(
                "Indexed {} Markdown files into {}",
                documents.len(),
                cache.display()
            );
        }
        Command::Search {
            query,
            root,
            cache,
            limit,
            case_sensitive,
        } => {
            let documents = documents_from(root, cache)?;
            let tokenizer = SimpleTokenizer::new(case_sensitive);
            let index = SearchIndex::build(documents, tokenizer);
            let results = search(&index, &query, SearchOptions { limit });

            if results.is_empty() {
                println!("No results.");
                return Ok(());
            }

            for (rank, result) in results.iter().enumerate() {
                println!(
                    "{}. {} ({:.3})",
                    rank + 1,
                    result.path.display(),
                    result.score
                );
                println!("   title: {}", result.title);
                println!("   terms: {}", result.matched_terms.join(", "));
                if let Some(snippet) = &result.snippet {
                    println!("   {snippet}");
                }
            }
        }
        Command::Stats { root, cache } => {
            let documents = documents_from(root, cache)?;
            let report = analyze_documents(&documents);
            let index = SearchIndex::build(documents, SimpleTokenizer::default());

            println!("Documents: {}", report.total_documents);
            println!("Internal links: {}", report.total_links);
            println!("Indexed terms: {}", index.terms().len());
            println!("Broken links: {}", report.broken_links.len());
            println!("Orphan documents: {}", report.orphan_documents.len());
            if !report.tags.is_empty() {
                println!("Top tags:");
                for tag in report.tags.iter().take(8) {
                    println!("  #{} {}", tag.tag, tag.count);
                }
            }
        }
        Command::Links { root, cache } => {
            let documents = documents_from(root, cache)?;
            let report = analyze_documents(&documents);

            println!("Broken links: {}", report.broken_links.len());
            for broken in &report.broken_links {
                println!(
                    "  {}:{} -> {}",
                    broken.source.display(),
                    broken.line,
                    broken.target
                );
            }

            println!("Orphan documents: {}", report.orphan_documents.len());
            for path in &report.orphan_documents {
                println!("  {}", path.display());
            }
        }
        Command::Backlinks {
            target,
            root,
            cache,
        } => {
            let documents = documents_from(root, cache)?;
            let report = analyze_documents(&documents);
            let backlinks = report.backlinks_for(&target);

            if backlinks.is_empty() {
                println!("No backlinks for {}", target.display());
                return Ok(());
            }

            println!("Backlinks for {}:", target.display());
            for backlink in backlinks {
                println!(
                    "  {}:{} {}",
                    backlink.source.display(),
                    backlink.line,
                    backlink.text
                );
            }
        }
        Command::Graph { root, cache } => {
            let documents = documents_from(root, cache)?;
            let report = analyze_documents(&documents);
            println!("{}", report.to_mermaid());
        }
        Command::Report {
            root,
            cache,
            format,
            output,
        } => {
            let documents = documents_from(root, cache)?;
            let report = analyze_documents(&documents);
            let suggestions = suggest_links(&documents, SuggestionOptions::default());
            let health = diagnose_knowledge_base(&report, &suggestions);
            let index = SearchIndex::build(documents, SimpleTokenizer::default());
            let rendered = render_report_with_extras(
                &report,
                index.terms().len(),
                format,
                ReportExtras {
                    health: Some(&health),
                    suggestions: &suggestions,
                },
            );

            if let Some(output) = output {
                fs::write(&output, rendered).map_err(|source| MdSeekError::io(&output, source))?;
                println!(
                    "Wrote {} report to {}",
                    format.default_extension(),
                    output.display()
                );
            } else {
                println!("{rendered}");
            }
        }
        Command::SuggestLinks {
            root,
            cache,
            limit,
            min_score,
        } => {
            let documents = documents_from(root, cache)?;
            let suggestions = suggest_links(&documents, SuggestionOptions { limit, min_score });

            if suggestions.is_empty() {
                println!("No high-confidence missing links found.");
                return Ok(());
            }

            println!("Suggested missing links:");
            for (index, suggestion) in suggestions.iter().enumerate() {
                println!(
                    "{}. {} -> {} ({:.1})",
                    index + 1,
                    suggestion.source.display(),
                    suggestion.target.display(),
                    suggestion.score
                );
                for reason in &suggestion.reasons {
                    println!("   - {reason}");
                }
            }
        }
        Command::Doctor { root, cache } => {
            let documents = documents_from(root, cache)?;
            let report = analyze_documents(&documents);
            let suggestions = suggest_links(&documents, SuggestionOptions::default());
            let health = diagnose_knowledge_base(&report, &suggestions);

            println!("Knowledge health score: {}/100", health.score);
            if health.issues.is_empty() {
                println!("No major issues found.");
                return Ok(());
            }

            for issue in &health.issues {
                println!("[{}] {}", severity_label(issue.severity), issue.title);
                println!("  {}", issue.detail);
                println!("  action: {}", issue.action);
            }
        }
        Command::Help => {
            print_help();
        }
    }

    Ok(())
}

fn documents_from(root: PathBuf, cache: Option<PathBuf>) -> Result<Vec<Document>> {
    match cache {
        Some(cache) if cache.exists() => load_cache(cache),
        Some(cache) => Err(MdSeekError::InvalidArgs(format!(
            "cache file {} does not exist",
            cache.display()
        ))),
        None => load_documents(root),
    }
}

fn print_help() {
    println!(
        "\
mdseek - local Markdown knowledge-base search and analysis

USAGE:
  mdseek index <root> [--cache <file>]
  mdseek search <query> [--root <dir>] [--cache <file>] [--limit <n>] [--case-sensitive]
  mdseek stats [--root <dir>] [--cache <file>]
  mdseek links [--root <dir>] [--cache <file>]
  mdseek backlinks <file> [--root <dir>] [--cache <file>]
  mdseek graph [--root <dir>] [--cache <file>]
  mdseek report [--root <dir>] [--cache <file>] [--format markdown|html] [--output <file>]
  mdseek suggest-links [--root <dir>] [--cache <file>] [--limit <n>] [--min-score <n>]
  mdseek doctor [--root <dir>] [--cache <file>]

EXAMPLES:
  mdseek index ./notes
  mdseek search \"rust ownership\" --root ./notes --limit 5
  mdseek links --root ./notes
  mdseek graph --root ./notes
  mdseek report --root ./notes --format html --output report.html
  mdseek suggest-links --root ./notes
  mdseek doctor --root ./notes
"
    );
}

fn severity_label(severity: Severity) -> &'static str {
    match severity {
        Severity::Critical => "critical",
        Severity::Warning => "warning",
        Severity::Info => "info",
    }
}

struct ArgParser {
    args: Vec<String>,
    index: usize,
}

impl ArgParser {
    fn new(args: Vec<String>) -> Self {
        Self { args, index: 1 }
    }

    fn next_command(&mut self) -> Result<Option<String>> {
        if self.index >= self.args.len() {
            return Ok(None);
        }
        let value = self.args[self.index].clone();
        self.index += 1;
        Ok(Some(value))
    }

    fn next_string(&mut self, name: &str) -> Result<String> {
        if self.index >= self.args.len() {
            return Err(MdSeekError::InvalidArgs(format!("missing {name}")));
        }
        let value = self.args[self.index].clone();
        if value.starts_with("--") {
            return Err(MdSeekError::InvalidArgs(format!("missing {name}")));
        }
        self.index += 1;
        Ok(value)
    }

    fn next_value(&mut self, name: &str) -> Result<PathBuf> {
        self.next_string(name).map(PathBuf::from)
    }

    fn optional_path(&mut self, flag: &str) -> Result<Option<PathBuf>> {
        let Some(flag_index) = self.find_flag(flag) else {
            return Ok(None);
        };
        let value_index = flag_index + 1;
        if value_index >= self.args.len() || self.args[value_index].starts_with("--") {
            return Err(MdSeekError::InvalidArgs(format!(
                "missing value for {flag}"
            )));
        }

        let value = PathBuf::from(self.args.remove(value_index));
        self.args.remove(flag_index);
        Ok(Some(value))
    }

    fn optional_usize(&mut self, flag: &str) -> Result<Option<usize>> {
        let Some(flag_index) = self.find_flag(flag) else {
            return Ok(None);
        };
        let value_index = flag_index + 1;
        if value_index >= self.args.len() || self.args[value_index].starts_with("--") {
            return Err(MdSeekError::InvalidArgs(format!(
                "missing value for {flag}"
            )));
        }

        let value = self.args.remove(value_index);
        self.args.remove(flag_index);
        let parsed = value
            .parse::<usize>()
            .map_err(|_| MdSeekError::InvalidArgs(format!("{flag} must be a positive number")))?;
        Ok(Some(parsed.max(1)))
    }

    fn optional_string(&mut self, flag: &str) -> Result<Option<String>> {
        let Some(flag_index) = self.find_flag(flag) else {
            return Ok(None);
        };
        let value_index = flag_index + 1;
        if value_index >= self.args.len() || self.args[value_index].starts_with("--") {
            return Err(MdSeekError::InvalidArgs(format!(
                "missing value for {flag}"
            )));
        }

        let value = self.args.remove(value_index);
        self.args.remove(flag_index);
        Ok(Some(value))
    }

    fn optional_flag(&mut self, flag: &str) -> Result<bool> {
        if let Some(flag_index) = self.find_flag(flag) {
            self.args.remove(flag_index);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn finish(&self) -> Result<()> {
        if self.index == self.args.len() {
            Ok(())
        } else {
            Err(MdSeekError::InvalidArgs(format!(
                "unexpected argument '{}'",
                self.args[self.index]
            )))
        }
    }

    fn find_flag(&self, flag: &str) -> Option<usize> {
        self.args[self.index..]
            .iter()
            .position(|value| value == flag)
            .map(|offset| self.index + offset)
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_args, Command};
    use crate::report::ReportFormat;
    use std::path::PathBuf;

    #[test]
    fn parses_search_command() {
        let command = parse_args(vec![
            "mdseek".to_string(),
            "search".to_string(),
            "ownership".to_string(),
            "--root".to_string(),
            "notes".to_string(),
            "--limit".to_string(),
            "3".to_string(),
        ])
        .unwrap();

        assert_eq!(
            command,
            Command::Search {
                query: "ownership".to_string(),
                root: PathBuf::from("notes"),
                cache: None,
                limit: 3,
                case_sensitive: false,
            }
        );
    }

    #[test]
    fn parses_search_options_in_any_order() {
        let command = parse_args(vec![
            "mdseek".to_string(),
            "search".to_string(),
            "ownership".to_string(),
            "--limit".to_string(),
            "3".to_string(),
            "--case-sensitive".to_string(),
            "--root".to_string(),
            "notes".to_string(),
        ])
        .unwrap();

        assert_eq!(
            command,
            Command::Search {
                query: "ownership".to_string(),
                root: PathBuf::from("notes"),
                cache: None,
                limit: 3,
                case_sensitive: true,
            }
        );
    }

    #[test]
    fn rejects_unknown_command() {
        let err = parse_args(vec!["mdseek".to_string(), "wat".to_string()]).unwrap_err();

        assert!(err.to_string().contains("unknown command"));
    }

    #[test]
    fn parses_report_command() {
        let command = parse_args(vec![
            "mdseek".to_string(),
            "report".to_string(),
            "--format".to_string(),
            "html".to_string(),
            "--output".to_string(),
            "report.html".to_string(),
            "--root".to_string(),
            "notes".to_string(),
        ])
        .unwrap();

        assert_eq!(
            command,
            Command::Report {
                root: PathBuf::from("notes"),
                cache: None,
                format: ReportFormat::Html,
                output: Some(PathBuf::from("report.html")),
            }
        );
    }

    #[test]
    fn parses_suggest_links_command() {
        let command = parse_args(vec![
            "mdseek".to_string(),
            "suggest-links".to_string(),
            "--root".to_string(),
            "notes".to_string(),
            "--limit".to_string(),
            "7".to_string(),
            "--min-score".to_string(),
            "6".to_string(),
        ])
        .unwrap();

        assert_eq!(
            command,
            Command::SuggestLinks {
                root: PathBuf::from("notes"),
                cache: None,
                limit: 7,
                min_score: 6,
            }
        );
    }

    #[test]
    fn parses_doctor_command() {
        let command = parse_args(vec![
            "mdseek".to_string(),
            "doctor".to_string(),
            "--root".to_string(),
            "notes".to_string(),
        ])
        .unwrap();

        assert_eq!(
            command,
            Command::Doctor {
                root: PathBuf::from("notes"),
                cache: None,
            }
        );
    }
}
