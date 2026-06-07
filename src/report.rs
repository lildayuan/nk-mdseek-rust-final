use std::fmt::Write;
use std::path::Path;

use crate::analyzer::KnowledgeReport;
use crate::insights::{KnowledgeHealth, LinkSuggestion, Severity};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReportFormat {
    Markdown,
    Html,
}

impl ReportFormat {
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "md" | "markdown" => Some(Self::Markdown),
            "html" | "htm" => Some(Self::Html),
            _ => None,
        }
    }

    pub fn default_extension(self) -> &'static str {
        match self {
            Self::Markdown => "md",
            Self::Html => "html",
        }
    }
}

pub trait ReportRenderer {
    fn render(
        &self,
        report: &KnowledgeReport,
        indexed_terms: usize,
        extras: ReportExtras<'_>,
    ) -> String;
}

#[derive(Clone, Debug, Default)]
pub struct MarkdownRenderer;

#[derive(Clone, Debug, Default)]
pub struct HtmlRenderer;

#[derive(Clone, Copy, Debug)]
pub struct ReportExtras<'a> {
    pub health: Option<&'a KnowledgeHealth>,
    pub suggestions: &'a [LinkSuggestion],
}

impl<'a> ReportExtras<'a> {
    pub fn none() -> Self {
        Self {
            health: None,
            suggestions: &[],
        }
    }
}

pub fn render_report(
    report: &KnowledgeReport,
    indexed_terms: usize,
    format: ReportFormat,
) -> String {
    render_report_with_extras(report, indexed_terms, format, ReportExtras::none())
}

pub fn render_report_with_extras(
    report: &KnowledgeReport,
    indexed_terms: usize,
    format: ReportFormat,
    extras: ReportExtras<'_>,
) -> String {
    match format {
        ReportFormat::Markdown => MarkdownRenderer.render(report, indexed_terms, extras),
        ReportFormat::Html => HtmlRenderer.render(report, indexed_terms, extras),
    }
}

impl ReportRenderer for MarkdownRenderer {
    fn render(
        &self,
        report: &KnowledgeReport,
        indexed_terms: usize,
        extras: ReportExtras<'_>,
    ) -> String {
        let mut output = String::new();
        writeln!(output, "# mdseek Knowledge Report").ok();
        writeln!(output).ok();
        writeln!(output, "## Summary").ok();
        writeln!(output).ok();
        writeln!(output, "| Metric | Value |").ok();
        writeln!(output, "|---|---:|").ok();
        writeln!(output, "| Documents | {} |", report.total_documents).ok();
        writeln!(output, "| Internal links | {} |", report.total_links).ok();
        writeln!(output, "| Indexed terms | {indexed_terms} |").ok();
        writeln!(output, "| Broken links | {} |", report.broken_links.len()).ok();
        writeln!(
            output,
            "| Orphan documents | {} |",
            report.orphan_documents.len()
        )
        .ok();

        write_health_markdown(&mut output, extras.health);
        write_suggestions_markdown(&mut output, extras.suggestions);
        write_tags_markdown(&mut output, report);
        write_broken_links_markdown(&mut output, report);
        write_orphans_markdown(&mut output, report);
        write_backlinks_markdown(&mut output, report);

        writeln!(output).ok();
        writeln!(output, "## Graph").ok();
        writeln!(output).ok();
        writeln!(output, "```mermaid").ok();
        writeln!(output, "{}", report.to_mermaid()).ok();
        writeln!(output, "```").ok();

        output
    }
}

impl ReportRenderer for HtmlRenderer {
    fn render(
        &self,
        report: &KnowledgeReport,
        indexed_terms: usize,
        extras: ReportExtras<'_>,
    ) -> String {
        let mut output = String::new();
        writeln!(output, "<!doctype html>").ok();
        writeln!(output, "<html lang=\"en\">").ok();
        writeln!(output, "<head>").ok();
        writeln!(output, "  <meta charset=\"utf-8\">").ok();
        writeln!(
            output,
            "  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">"
        )
        .ok();
        writeln!(output, "  <title>mdseek Knowledge Report</title>").ok();
        writeln!(output, "  <style>{}</style>", report_css()).ok();
        writeln!(output, "</head>").ok();
        writeln!(output, "<body>").ok();
        writeln!(output, "  <main>").ok();
        writeln!(output, "    <h1>mdseek Knowledge Report</h1>").ok();
        writeln!(output, "    <section>").ok();
        writeln!(output, "      <h2>Summary</h2>").ok();
        writeln!(output, "      <dl class=\"metrics\">").ok();
        write_metric(&mut output, "Documents", report.total_documents);
        write_metric(&mut output, "Internal links", report.total_links);
        write_metric(&mut output, "Indexed terms", indexed_terms);
        write_metric(&mut output, "Broken links", report.broken_links.len());
        write_metric(
            &mut output,
            "Orphan documents",
            report.orphan_documents.len(),
        );
        writeln!(output, "      </dl>").ok();
        writeln!(output, "    </section>").ok();

        write_health_html(&mut output, extras.health);
        write_suggestions_html(&mut output, extras.suggestions);
        write_tags_html(&mut output, report);
        write_broken_links_html(&mut output, report);
        write_orphans_html(&mut output, report);
        write_backlinks_html(&mut output, report);

        writeln!(output, "    <section>").ok();
        writeln!(output, "      <h2>Mermaid Graph</h2>").ok();
        writeln!(
            output,
            "      <pre><code>{}</code></pre>",
            escape_html(&report.to_mermaid())
        )
        .ok();
        writeln!(output, "    </section>").ok();
        writeln!(output, "  </main>").ok();
        writeln!(output, "</body>").ok();
        writeln!(output, "</html>").ok();

        output
    }
}

fn write_health_markdown(output: &mut String, health: Option<&KnowledgeHealth>) {
    let Some(health) = health else {
        return;
    };

    writeln!(output).ok();
    writeln!(output, "## Knowledge Health").ok();
    writeln!(output).ok();
    writeln!(output, "Score: **{}/100**", health.score).ok();
    if health.issues.is_empty() {
        writeln!(output).ok();
        writeln!(output, "No major issues found.").ok();
        return;
    }

    writeln!(output).ok();
    for issue in &health.issues {
        writeln!(
            output,
            "- **{}**: {}",
            severity_name(issue.severity),
            issue.title
        )
        .ok();
        writeln!(output, "  - Detail: {}", issue.detail).ok();
        writeln!(output, "  - Action: {}", issue.action).ok();
    }
}

fn write_suggestions_markdown(output: &mut String, suggestions: &[LinkSuggestion]) {
    writeln!(output).ok();
    writeln!(output, "## Suggested Missing Links").ok();
    writeln!(output).ok();
    if suggestions.is_empty() {
        writeln!(output, "No high-confidence missing links found.").ok();
        return;
    }

    for suggestion in suggestions.iter().take(10) {
        writeln!(
            output,
            "- `{}` -> `{}` ({:.1})",
            suggestion.source.display(),
            suggestion.target.display(),
            suggestion.score
        )
        .ok();
        for reason in &suggestion.reasons {
            writeln!(output, "  - {}", reason).ok();
        }
    }
}

fn write_health_html(output: &mut String, health: Option<&KnowledgeHealth>) {
    let Some(health) = health else {
        return;
    };

    writeln!(output, "    <section>").ok();
    writeln!(output, "      <h2>Knowledge Health</h2>").ok();
    writeln!(
        output,
        "      <p class=\"score\">Score: <strong>{}/100</strong></p>",
        health.score
    )
    .ok();
    if health.issues.is_empty() {
        writeln!(output, "      <p>No major issues found.</p>").ok();
    } else {
        writeln!(output, "      <ul>").ok();
        for issue in &health.issues {
            writeln!(
                output,
                "        <li><strong>{}</strong>: {}<br><span>{}</span><br><em>{}</em></li>",
                escape_html(severity_name(issue.severity)),
                escape_html(&issue.title),
                escape_html(&issue.detail),
                escape_html(&issue.action)
            )
            .ok();
        }
        writeln!(output, "      </ul>").ok();
    }
    writeln!(output, "    </section>").ok();
}

fn write_suggestions_html(output: &mut String, suggestions: &[LinkSuggestion]) {
    writeln!(output, "    <section>").ok();
    writeln!(output, "      <h2>Suggested Missing Links</h2>").ok();
    if suggestions.is_empty() {
        writeln!(
            output,
            "      <p>No high-confidence missing links found.</p>"
        )
        .ok();
    } else {
        writeln!(output, "      <ol>").ok();
        for suggestion in suggestions.iter().take(10) {
            writeln!(
                output,
                "        <li><code>{}</code> -&gt; <code>{}</code> <strong>{:.1}</strong>",
                escape_html(&suggestion.source.display().to_string()),
                escape_html(&suggestion.target.display().to_string()),
                suggestion.score
            )
            .ok();
            writeln!(output, "          <ul>").ok();
            for reason in &suggestion.reasons {
                writeln!(output, "            <li>{}</li>", escape_html(reason)).ok();
            }
            writeln!(output, "          </ul>").ok();
            writeln!(output, "        </li>").ok();
        }
        writeln!(output, "      </ol>").ok();
    }
    writeln!(output, "    </section>").ok();
}

fn write_tags_markdown(output: &mut String, report: &KnowledgeReport) {
    writeln!(output).ok();
    writeln!(output, "## Top Tags").ok();
    writeln!(output).ok();
    if report.tags.is_empty() {
        writeln!(output, "No tags found.").ok();
        return;
    }
    for tag in report.tags.iter().take(12) {
        writeln!(output, "- `#{}`: {}", tag.tag, tag.count).ok();
    }
}

fn write_broken_links_markdown(output: &mut String, report: &KnowledgeReport) {
    writeln!(output).ok();
    writeln!(output, "## Broken Links").ok();
    writeln!(output).ok();
    if report.broken_links.is_empty() {
        writeln!(output, "No broken internal links.").ok();
        return;
    }
    for link in &report.broken_links {
        writeln!(
            output,
            "- `{}` line {} -> `{}`",
            link.source.display(),
            link.line,
            link.target
        )
        .ok();
    }
}

fn write_orphans_markdown(output: &mut String, report: &KnowledgeReport) {
    writeln!(output).ok();
    writeln!(output, "## Orphan Documents").ok();
    writeln!(output).ok();
    if report.orphan_documents.is_empty() {
        writeln!(output, "No orphan documents.").ok();
        return;
    }
    for path in &report.orphan_documents {
        writeln!(output, "- `{}`", path.display()).ok();
    }
}

fn write_backlinks_markdown(output: &mut String, report: &KnowledgeReport) {
    writeln!(output).ok();
    writeln!(output, "## Backlink Overview").ok();
    writeln!(output).ok();
    let mut entries = backlink_entries(report);
    if entries.is_empty() {
        writeln!(output, "No backlinks found.").ok();
        return;
    }
    for (target, count) in entries.drain(..).take(12) {
        writeln!(output, "- `{}`: {}", target.display(), count).ok();
    }
}

fn write_tags_html(output: &mut String, report: &KnowledgeReport) {
    writeln!(output, "    <section>").ok();
    writeln!(output, "      <h2>Top Tags</h2>").ok();
    if report.tags.is_empty() {
        writeln!(output, "      <p>No tags found.</p>").ok();
    } else {
        writeln!(output, "      <ul>").ok();
        for tag in report.tags.iter().take(12) {
            writeln!(
                output,
                "        <li><code>#{}</code> {}</li>",
                escape_html(&tag.tag),
                tag.count
            )
            .ok();
        }
        writeln!(output, "      </ul>").ok();
    }
    writeln!(output, "    </section>").ok();
}

fn write_broken_links_html(output: &mut String, report: &KnowledgeReport) {
    writeln!(output, "    <section>").ok();
    writeln!(output, "      <h2>Broken Links</h2>").ok();
    if report.broken_links.is_empty() {
        writeln!(output, "      <p>No broken internal links.</p>").ok();
    } else {
        writeln!(output, "      <ul>").ok();
        for link in &report.broken_links {
            writeln!(
                output,
                "        <li><code>{}</code> line {} -&gt; <code>{}</code></li>",
                escape_html(&link.source.display().to_string()),
                link.line,
                escape_html(&link.target)
            )
            .ok();
        }
        writeln!(output, "      </ul>").ok();
    }
    writeln!(output, "    </section>").ok();
}

fn write_orphans_html(output: &mut String, report: &KnowledgeReport) {
    writeln!(output, "    <section>").ok();
    writeln!(output, "      <h2>Orphan Documents</h2>").ok();
    if report.orphan_documents.is_empty() {
        writeln!(output, "      <p>No orphan documents.</p>").ok();
    } else {
        writeln!(output, "      <ul>").ok();
        for path in &report.orphan_documents {
            writeln!(
                output,
                "        <li><code>{}</code></li>",
                escape_html(&path.display().to_string())
            )
            .ok();
        }
        writeln!(output, "      </ul>").ok();
    }
    writeln!(output, "    </section>").ok();
}

fn write_backlinks_html(output: &mut String, report: &KnowledgeReport) {
    writeln!(output, "    <section>").ok();
    writeln!(output, "      <h2>Backlink Overview</h2>").ok();
    let entries = backlink_entries(report);
    if entries.is_empty() {
        writeln!(output, "      <p>No backlinks found.</p>").ok();
    } else {
        writeln!(output, "      <ol>").ok();
        for (target, count) in entries.into_iter().take(12) {
            writeln!(
                output,
                "        <li><code>{}</code> {}</li>",
                escape_html(&target.display().to_string()),
                count
            )
            .ok();
        }
        writeln!(output, "      </ol>").ok();
    }
    writeln!(output, "    </section>").ok();
}

fn write_metric(output: &mut String, label: &str, value: usize) {
    writeln!(
        output,
        "        <div><dt>{}</dt><dd>{}</dd></div>",
        escape_html(label),
        value
    )
    .ok();
}

fn backlink_entries(report: &KnowledgeReport) -> Vec<(&Path, usize)> {
    let mut entries = report
        .backlinks
        .iter()
        .map(|(path, backlinks)| (path.as_path(), backlinks.len()))
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(right.0)));
    entries
}

fn escape_html(value: &str) -> String {
    let mut escaped = String::new();
    for ch in value.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn severity_name(severity: Severity) -> &'static str {
    match severity {
        Severity::Critical => "critical",
        Severity::Warning => "warning",
        Severity::Info => "info",
    }
}

fn report_css() -> &'static str {
    "body{font-family:system-ui,-apple-system,BlinkMacSystemFont,sans-serif;margin:0;background:#f8fafc;color:#111827}main{max-width:960px;margin:0 auto;padding:32px}section{margin:28px 0}h1,h2{line-height:1.2}.metrics{display:grid;grid-template-columns:repeat(auto-fit,minmax(150px,1fr));gap:12px}.metrics div{border:1px solid #d1d5db;background:white;padding:12px}.metrics dt{font-size:13px;color:#4b5563}.metrics dd{font-size:24px;font-weight:700;margin:4px 0 0}.score{font-size:22px}code,pre{background:#eef2ff}code{padding:2px 4px}pre{padding:16px;overflow:auto;border:1px solid #c7d2fe}li{margin:6px 0}"
}

#[cfg(test)]
mod tests {
    use super::{render_report, render_report_with_extras, ReportExtras, ReportFormat};
    use crate::analyzer::analyze_documents;
    use crate::insights::{diagnose_knowledge_base, suggest_links, SuggestionOptions};
    use crate::parser::parse_markdown;
    use crate::types::DocumentId;

    #[test]
    fn renders_markdown_report_with_summary_and_graph() {
        let docs = vec![
            parse_markdown(DocumentId(0), "notes/a.md", "# A\nSee [[b]].\n#rust"),
            parse_markdown(DocumentId(1), "notes/b.md", "# B"),
        ];
        let report = analyze_documents(&docs);

        let rendered = render_report(&report, 12, ReportFormat::Markdown);

        assert!(rendered.contains("# mdseek Knowledge Report"));
        assert!(rendered.contains("| Documents | 2 |"));
        assert!(rendered.contains("```mermaid"));
        assert!(rendered.contains("notes/b.md"));
    }

    #[test]
    fn renders_html_report_and_escapes_content() {
        let docs = vec![parse_markdown(
            DocumentId(0),
            "notes/a.md",
            "# A\n[bad](<missing>.md)",
        )];
        let report = analyze_documents(&docs);

        let rendered = render_report(&report, 3, ReportFormat::Html);

        assert!(rendered.contains("<!doctype html>"));
        assert!(rendered.contains("Indexed terms"));
        assert!(rendered.contains("&lt;missing&gt;.md"));
    }

    #[test]
    fn parses_report_format_aliases() {
        assert_eq!(ReportFormat::parse("md"), Some(ReportFormat::Markdown));
        assert_eq!(ReportFormat::parse("html"), Some(ReportFormat::Html));
        assert_eq!(ReportFormat::parse("pdf"), None);
    }

    #[test]
    fn enriched_report_contains_health_and_suggestions() {
        let docs = vec![
            parse_markdown(DocumentId(0), "notes/a.md", "# A\nB topic.\n#demo"),
            parse_markdown(DocumentId(1), "notes/b.md", "# B\nA topic.\n#demo"),
        ];
        let report = analyze_documents(&docs);
        let suggestions = suggest_links(&docs, SuggestionOptions::default());
        let health = diagnose_knowledge_base(&report, &suggestions);

        let rendered = render_report_with_extras(
            &report,
            5,
            ReportFormat::Markdown,
            ReportExtras {
                health: Some(&health),
                suggestions: &suggestions,
            },
        );

        assert!(rendered.contains("Knowledge Health"));
        assert!(rendered.contains("Suggested Missing Links"));
    }
}
