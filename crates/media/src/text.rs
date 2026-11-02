use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use serde::Serialize;
use std::fs::File;
use std::io::Read;
use std::path::Path;

const READ_LIMIT: usize = 512;
const SNIPPET_CHARS: usize = 420;

#[derive(Debug, Clone, Serialize)]
pub struct TextSnippet {
    /// Plain text for accessibility and for non-Markdown cards.
    pub text: String,
    /// Safe, generated HTML for Markdown cards. Raw HTML from the file is dropped.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub html: Option<String>,
    pub truncated: bool,
}

/// Read a short, safe preview for a text or Markdown file. This is a card
/// preview, not a document reader: it never reads more than 512 bytes.
pub fn text_snippet(path: &Path) -> Option<TextSnippet> {
    let name = path.file_name()?.to_string_lossy();
    if name.starts_with('.') {
        return None;
    }
    let ext = path.extension()?.to_string_lossy().to_lowercase();
    let markdown = matches!(ext.as_str(), "md" | "markdown" | "mdx");
    if !markdown && ext != "txt" {
        return None;
    }

    let mut bytes = Vec::with_capacity(READ_LIMIT);
    File::open(path).ok()?.take(READ_LIMIT as u64).read_to_end(&mut bytes).ok()?;
    let source = String::from_utf8_lossy(&bytes);
    let text = compact_text(&source);
    if text.trim().is_empty() {
        return None;
    }
    let truncated = source.chars().count() >= SNIPPET_CHARS || bytes.len() == READ_LIMIT;
    let html = markdown.then(|| markdown_to_html(&source));
    Some(TextSnippet { text, html, truncated })
}

fn compact_text(source: &str) -> String {
    let mut out = String::with_capacity(SNIPPET_CHARS);
    for line in source.lines() {
        let line = line.trim_end();
        if line.trim().is_empty() && out.ends_with('\n') {
            continue;
        }
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(line);
        if out.chars().count() >= SNIPPET_CHARS {
            break;
        }
    }
    out.chars().take(SNIPPET_CHARS).collect()
}

/// A deliberately small Markdown renderer: only tags this function emits are
/// allowed, links are reduced to their text, and raw HTML events are dropped.
fn markdown_to_html(source: &str) -> String {
    let mut out = String::new();
    let options = Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    for event in Parser::new_ext(source, options) {
        match event {
            Event::Start(tag) => match tag {
                Tag::Paragraph => out.push_str("<p>"),
                Tag::Heading { .. } => out.push_str("<p class=\"md-h\">"),
                Tag::Emphasis => out.push_str("<em>"),
                Tag::Strong => out.push_str("<strong>"),
                Tag::Strikethrough => out.push_str("<s>"),
                Tag::CodeBlock(_) => out.push_str("<pre>"),
                Tag::List(_) => out.push_str("<ul>"),
                Tag::Item => out.push_str("<li>"),
                Tag::BlockQuote(_) => out.push_str("<blockquote>"),
                Tag::Link { .. } => out.push_str("<span>"),
                Tag::Image { .. } => out.push_str("<span class=\"md-img\">"),
                _ => {}
            },
            Event::End(tag) => match tag {
                TagEnd::Paragraph | TagEnd::Heading(_) => out.push_str("</p>"),
                TagEnd::Emphasis => out.push_str("</em>"),
                TagEnd::Strong => out.push_str("</strong>"),
                TagEnd::Strikethrough => out.push_str("</s>"),
                TagEnd::CodeBlock => out.push_str("</pre>"),
                TagEnd::List(_) => out.push_str("</ul>"),
                TagEnd::Item => out.push_str("</li>"),
                TagEnd::BlockQuote(_) => out.push_str("</blockquote>"),
                TagEnd::Link | TagEnd::Image => out.push_str("</span>"),
                _ => {}
            },
            Event::Text(text) => escape_into(&mut out, &text),
            Event::Code(code) => {
                out.push_str("<code>");
                escape_into(&mut out, &code);
                out.push_str("</code>");
            }
            Event::SoftBreak => out.push(' '),
            Event::HardBreak => out.push_str("<br>"),
            Event::Rule => out.push_str("<hr>"),
            Event::TaskListMarker(done) => {
                out.push_str(if done { "<span>☑ </span>" } else { "<span>☐ </span>" });
            }
            // Raw HTML, inline HTML and footnotes are intentionally not emitted.
            _ => {}
        }
        if out.len() > SNIPPET_CHARS * 6 {
            break;
        }
    }
    out.chars().take(SNIPPET_CHARS * 4).collect()
}

fn escape_into(out: &mut String, text: &str) {
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn text_snippet_reads_txt_and_strips_markdown_html() {
        let dir = tempfile::tempdir().unwrap();
        let txt = dir.path().join("note.txt");
        std::fs::write(&txt, "Hello\nworld\n").unwrap();
        let snippet = text_snippet(&txt).unwrap();
        assert_eq!(snippet.text, "Hello\nworld");
        assert!(snippet.html.is_none());

        let md = dir.path().join("note.md");
        let mut file = std::fs::File::create(&md).unwrap();
        writeln!(file, "# Heading\n\n**bold** and `code`\n\n<script>alert(1)</script>").unwrap();
        let html = text_snippet(&md).unwrap().html.unwrap();
        assert!(html.contains("md-h"));
        assert!(html.contains("<strong>bold</strong>"));
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;") || !html.contains("script"));
    }

    #[test]
    fn dotfiles_and_unsupported_extensions_are_not_read() {
        let dir = tempfile::tempdir().unwrap();
        let env = dir.path().join(".env");
        std::fs::write(&env, "SECRET=1").unwrap();
        assert!(text_snippet(&env).is_none());
        let bin = dir.path().join("image.png");
        std::fs::write(&bin, "x").unwrap();
        assert!(text_snippet(&bin).is_none());
    }
}
