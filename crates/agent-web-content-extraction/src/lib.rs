use compute_text_density::compute_text_density;
use ego_tree::{iter::Edge, NodeRef};
use extract_content::extract_content_from_stats;
use scraper::{Html, Node, Selector};
use std::collections::HashSet;
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
    let contents = extract_content_from_stats(root, &stats);
    let content_ids = HashSet::<_>::from_iter(contents.into_iter().map(|content| content.id()));

    let is_parent_content = |node_ref: NodeRef<Node>| {
        let mut parent_ref = node_ref.parent();

        while let Some(parent) = parent_ref {
            if content_ids.contains(&parent.id()) {
                return true;
            }

            parent_ref = parent.parent();
        }

        false
    };

    let mut contents = Vec::new();

    for edge in root.traverse() {
        let edge = match edge {
            Edge::Open(node_ref) => node_ref,
            _ => continue,
        };
        let text = match edge.value() {
            Node::Text(text) => text,
            _ => continue,
        };

        if !is_parent_content(edge) {
            continue;
        }

        let trimmed_text = text.trim();

        if trimmed_text.is_empty() {
            continue;
        }

        contents.push(trimmed_text);
    }

    ExtractedContent {
        content: contents.join(" "),
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
