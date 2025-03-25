use scraper::{Html, Node};
use std::collections::BTreeSet;

pub fn strip_non_content_tags(html: &mut Html) {
    let non_content_tags = BTreeSet::from_iter([
        "script", "noscript", "style", "nav", "header", "footer", "img", "svg", "video", "audio",
        "form", "label", "input", "select", "option", "button", "object", "embed", "iframe",
        "canvas", "map", "area", "picture", "source", "track", "wbr", "slot", "template",
        "datalist",
    ]);
    let mut removal_queue = Vec::new();

    for node in html.root_element().descendants() {
        match node.value() {
            Node::Doctype(_) | Node::Comment(_) | Node::ProcessingInstruction(_) => {
                removal_queue.push(node.id());
            }
            Node::Element(element) if non_content_tags.contains(&element.name()) => {
                removal_queue.push(node.id());
            }
            _ => {}
        }
    }

    for node_id in removal_queue {
        html.tree.get_mut(node_id).unwrap().detach();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_non_content_tags() {
        let mut html = Html::parse_document(
            "<html><head></head><body><script>test</script><nav>test</nav><footer>test</footer><!--comment--></body></html>",
        );
        strip_non_content_tags(&mut html);
        assert_eq!(html.html(), "<html><head></head><body></body></html>");
    }
}
