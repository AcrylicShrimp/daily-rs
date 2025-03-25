use compute_text_density::compute_text_density;
use extract_content::extract_content_from_stats;
use scraper::{Html, Selector};
use strip_non_content_tags::strip_non_content_tags;

mod compute_text_density;
mod extract_content;
mod strip_non_content_tags;

pub struct ExtractedContent {
    pub content: String,
}

pub fn extract_content(html: &str) -> ExtractedContent {
    let mut html = Html::parse_document(html);

    strip_non_content_tags(&mut html);

    let root = html
        .select(&Selector::parse("body").unwrap())
        .next()
        .unwrap();

    let stats = compute_text_density(root);
    let mut contents = extract_content_from_stats(root, &stats);

    contents.sort_unstable_by_key(|content| {
        stats
            .get(&content.id())
            .map(|stat| stat.order)
            .unwrap_or_default()
    });

    let mut fragments = Vec::with_capacity(contents.len() * 4);

    for content in contents {
        for text in content.text() {
            let trimmed = text.trim();

            if trimmed.is_empty() {
                continue;
            }

            fragments.push(text.trim());
        }
    }

    ExtractedContent {
        content: fragments.join(" "),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_content() {
        let html = include_str!("../test/example.html");
        let content = extract_content(html);
        println!("content: {}", content.content);
    }
}
