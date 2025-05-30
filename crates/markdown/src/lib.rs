use article_link::ArticleLinkScanner;
use fmtm_ytmimi_markdown_fmt::{Config, OrderedListMarker, UnorderedListMarker};
use markdown_it::{
    MarkdownIt,
    plugins::cmark::block::{heading::ATXHeading, lheading::SetextHeader},
};
use math_equation::MathEquationScanner;
use std::{borrow::Cow, sync::OnceLock};
use table_of_contents::{TocMarkerScanner, TocScanner};

mod article_link;
mod external_link;
mod math_equation;
mod table_of_contents;

pub fn render_article_markdown(text: &str) -> String {
    static INSTANCE: OnceLock<MarkdownIt> = OnceLock::new();
    let mut parsed = INSTANCE.get_or_init(article_markdown).parse(text);

    // Make markdown headings one level smaller, so that h1 becomes h2 etc, and markdown titles
    // are smaller than page title.
    parsed.walk_mut(|node, _| {
        if let Some(heading) = node.cast_mut::<ATXHeading>() {
            heading.level += 1;
        }
        if let Some(heading) = node.cast_mut::<SetextHeader>() {
            heading.level += 1;
        }
    });
    parsed.render()
}

pub fn render_comment_markdown(text: &str) -> String {
    static INSTANCE: OnceLock<MarkdownIt> = OnceLock::new();
    INSTANCE.get_or_init(common_markdown).parse(text).render()
}

fn article_markdown() -> MarkdownIt {
    let mut parser = common_markdown();
    let p = &mut parser;
    markdown_it::plugins::cmark::inline::image::add(p);

    // Extensions from various authors
    markdown_it_heading_anchors::add(p);
    markdown_it_block_spoiler::add(p);
    markdown_it_footnote::add(p);
    markdown_it_sub::add(p);
    markdown_it_sup::add(p);

    // Ibis custom extensions
    parser.inline.add_rule::<ArticleLinkScanner>();
    parser.inline.add_rule::<MathEquationScanner>();
    parser.inline.add_rule::<TocMarkerScanner>();
    parser.add_rule::<TocScanner>();

    parser
}

fn common_markdown() -> MarkdownIt {
    let mut parser = MarkdownIt::new();
    let p = &mut parser;
    {
        // Markdown-it inline core features. Image is disabled to prevent embedding external
        // images. Later we need to add proper image support using pictrs.
        // External link uses a custom impl further down with icon.
        use markdown_it::plugins::cmark::inline::*;
        newline::add(p);
        escape::add(p);
        backticks::add(p);
        emphasis::add(p);
        autolink::add(p);
        entity::add(p);
    }

    {
        // Markdown-it block core features. Unchanged from defaults.
        use markdown_it::plugins::cmark::block::*;
        code::add(p);
        fence::add(p);
        blockquote::add(p);
        hr::add(p);
        list::add(p);
        reference::add(p);
        heading::add(p);
        lheading::add(p);
        paragraph::add(p);
    }

    {
        // Some of the extras from markdown-it, others are intentionally excluded.
        use markdown_it::plugins::extra::*;
        strikethrough::add(p);
        tables::add(p);
        typographer::add(p);
    }

    external_link::add(p);

    parser
}

pub fn format_markdown(text: &str) -> Result<String, std::fmt::Error> {
    const CONFIG: Config = Config {
        max_width: Some(80),
        fixed_zero_padding: Some(0),
        fixed_number: None,
        fixed_ordered_list_marker: Some(OrderedListMarker::Period),
        fixed_unordered_list_marker: Some(UnorderedListMarker::Hyphen),
        fixed_indentation: Some(Cow::Borrowed("    ")),
        fixed_emphasis_marker: Some("*"),
        fixed_strong_marker: Some("**"),
    };
    fmtm::format_with_config(text, CONFIG)
}
