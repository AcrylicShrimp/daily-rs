use scraper::{Html, Selector};

pub fn strip_non_content_tags(html: &mut Html) {
    let non_content_tags = [
        "script", "noscript", "style", "nav", "header", "footer", "img", "svg", "video", "audio",
        "form", "label", "input", "select", "option", "button", "object", "embed", "iframe",
        "canvas", "map", "area",
    ];

    for tag in non_content_tags {
        remove_tags(html, tag);
    }
}

fn remove_tags(html: &mut Html, tag_name: &str) {
    let tag_selector = Selector::parse(tag_name).unwrap();
    let node_ids = Vec::from_iter(html.select(&tag_selector).map(|tag| tag.id()));

    for node_id in node_ids {
        html.tree.get_mut(node_id).unwrap().detach();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_non_content_tags() {
        let mut html = Html::parse_document(
            "<html><head></head><body><script>test</script><nav>test</nav><footer>test</footer></body></html>",
        );
        strip_non_content_tags(&mut html);
        assert_eq!(html.html(), "<html><head></head><body></body></html>");
    }

    #[test]
    fn test_remove_tags() {
        let mut html =
            Html::parse_document("<html><head></head><body><script>test</script></body></html>");
        remove_tags(&mut html, "script");
        assert_eq!(html.html(), "<html><head></head><body></body></html>");
    }
}
