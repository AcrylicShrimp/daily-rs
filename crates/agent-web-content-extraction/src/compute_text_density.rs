use ego_tree::NodeId;
use scraper::{ElementRef, Node};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct TextDensityStat<'a> {
    pub order: usize,
    pub element: ElementRef<'a>,
    pub density: f64,
    pub density_sum: f64,
    /// The number of **child** tags.
    pub tag_count: usize,
    /// The total text length of the node including all descendants.
    pub text_length: usize,
    /// The number of **child** link tags.
    pub link_tag_count: usize,
    /// The total link text length of the node including all descendants.
    pub link_text_length: usize,
}

pub fn compute_text_density(root: ElementRef) -> HashMap<NodeId, TextDensityStat> {
    let mut stats = collect_stats(root);

    if stats.is_empty() {
        return stats;
    }

    let root_stat = stats.get(&root.id());
    let root_stat = match root_stat {
        Some(root_stat) => root_stat,
        None => return stats,
    };

    let root_text_length = root_stat.text_length;
    let root_link_text_length = root_stat.link_text_length;
    let root_link_text_ratio = root_link_text_length as f64 / (root_text_length as f64).max(1.0);

    struct TextDensity {
        node_id: NodeId,
        density: f64,
    }

    let text_densities: Vec<_> = stats
        .iter()
        .map(|(node_id, stat)| TextDensity {
            node_id: *node_id,
            density: compute_text_density_of_node(stat, root_link_text_ratio),
        })
        .collect();

    for TextDensity { node_id, density } in text_densities {
        let stat = stats.get_mut(&node_id).unwrap();
        stat.density = density;

        let parent = match stat.element.parent() {
            Some(parent) => parent,
            None => continue,
        };
        let parent_stat = match stats.get_mut(&parent.id()) {
            Some(parent_stat) => parent_stat,
            None => continue,
        };

        parent_stat.density_sum += density;
    }

    let root_stat = stats.get_mut(&root.id());
    let root_stat = match root_stat {
        Some(root_stat) => root_stat,
        None => return stats,
    };
    let root_density = root_stat.density;

    // NOTE: Heading tags are more important than other tags,
    // because they logically divide the page into sections,
    // highlighting the main topics of each section.
    //
    // But headings are typically shorter than other tags,
    // so we need to compensate for that by boosting their density.
    for stat in stats.values_mut() {
        let headings = ["h1", "h2", "h3", "h4", "h5", "h6"];

        if !headings.contains(&stat.element.value().name()) {
            continue;
        }

        let compensated_text_length =
            stat.text_length + (root_density * stat.tag_count.max(1) as f64) as usize;
        let compensated_density = compensated_text_length as f64;

        stat.density = compensated_density;
        stat.density_sum += compensated_density;
    }

    stats
}

fn compute_text_density_of_node(stat: &TextDensityStat, root_link_text_ratio: f64) -> f64 {
    if stat.element.value().name() == "a" {
        compute_link_text_density_of_node(stat, root_link_text_ratio)
    } else {
        compute_normal_text_density_of_node(stat)
    }
}

fn compute_normal_text_density_of_node(stat: &TextDensityStat) -> f64 {
    let tag_count = (stat.tag_count as f64).max(1.0);
    let text_length = stat.text_length as f64;
    text_length / tag_count
}

fn compute_link_text_density_of_node(stat: &TextDensityStat, root_link_text_ratio: f64) -> f64 {
    let tag_count = (stat.tag_count as f64).max(1.0);
    let text_length = stat.text_length as f64;
    let link_tag_count = (stat.link_tag_count as f64).max(1.0);
    let link_text_length = stat.link_text_length as f64;

    let text_density = text_length / tag_count;
    let log_base = ((text_length / (text_length - link_text_length).max(1.0)) * link_text_length
        + root_link_text_ratio * text_length
        + 1e-3)
        .ln();
    let log_arg = ((text_length / link_text_length.max(1.0)) * tag_count) / link_tag_count;

    text_density * log_arg.max(1.0).log(log_base.max(2.0))
}

fn collect_stats(root: ElementRef) -> HashMap<NodeId, TextDensityStat> {
    let mut stats = HashMap::new();
    augment_stats(root, &mut stats);
    stats
}

fn augment_stats<'a>(node: ElementRef<'a>, stats: &mut HashMap<NodeId, TextDensityStat<'a>>) {
    let text_length = compute_text_length_of_node(node);
    let is_link = node.value().name() == "a";
    let order = stats.len();

    stats.insert(
        node.id(),
        TextDensityStat {
            order,
            element: node,
            density: 0.0,
            density_sum: 0.0,
            tag_count: 0,
            text_length,
            link_tag_count: 0,
            link_text_length: if is_link { text_length } else { 0 },
        },
    );

    let mut parent = node.parent();

    while let Some(parent_node) = parent {
        let parent_ref = match ElementRef::wrap(parent_node) {
            Some(parent_ref) => parent_ref,
            None => break,
        };
        let parent_stat = match stats.get_mut(&parent_ref.id()) {
            Some(parent_stat) => parent_stat,
            None => break,
        };

        parent_stat.tag_count += 1;
        parent_stat.text_length += text_length;

        if is_link {
            parent_stat.link_tag_count += 1;
            parent_stat.link_text_length += text_length;
        }

        parent = parent_ref.parent();
    }

    for child in node.child_elements() {
        augment_stats(child, stats);
    }
}

fn compute_text_length_of_node(node: ElementRef) -> usize {
    let mut length = 0;

    for child in node.children() {
        let text = match child.value() {
            Node::Text(text) => text,
            _ => continue,
        };
        let trimmed = text.trim();

        if trimmed.is_empty() {
            continue;
        }

        if length != 0 {
            length += 1;
        }

        length += trimmed.chars().count();
    }

    length
}

#[cfg(test)]
mod tests {
    use super::*;
    use scraper::{Html, Selector};

    #[test]
    fn test_simple_page() {
        let html = r#"
            <!DOCTYPE html>
            <html>
            <head>
                <title>Example Page</title>
            </head>
            <body>
                <h1>Main Title</h1>
                <p>This is a paragraph with some text. <a href="https://example.com">Link</a>.</p>
                <div>
                    <p>Another paragraph.</p>
                </div>
            </body>
            </html>
        "#;

        let document = Html::parse_document(html);
        let root = document
            .select(&Selector::parse("body").unwrap())
            .next()
            .unwrap();
        let density_map = compute_text_density(root);

        // Basic checks
        assert!(!density_map.is_empty());

        // Check the root element
        let root_stat = density_map.get(&root.id()).expect("Root element not found");
        assert_eq!(root_stat.tag_count, 5); // <body>, <h1>, <a>, <div>, <p>
        assert_eq!(root_stat.link_tag_count, 1); // <a>

        //Check a paragraph
        let p_selector = Selector::parse("p").unwrap();
        let first_p = document
            .select(&p_selector)
            .next()
            .expect("First <p> not found");
        let first_p_stat = density_map
            .get(&first_p.id())
            .expect("First <p> stat not found");
        assert_eq!(first_p_stat.tag_count, 1);
        assert_eq!(first_p_stat.link_tag_count, 1);

        // Check the link
        let a_selector = Selector::parse("a").unwrap();
        let a = document.select(&a_selector).next().expect("<a> not found");
        let a_stat = density_map.get(&a.id()).expect("<a> stat not found");
        assert_eq!(a_stat.tag_count, 0); // No *child* tags within the <a> tag
        assert_eq!(a_stat.link_tag_count, 0); // No *child* tags within the <a> tag
        assert!(a_stat.density == 0.0); // Density is 0 because there is only text within the <a> tag
    }

    #[test]
    fn test_empty_page() {
        let html = r#"<!DOCTYPE html><html><head></head><body></body></html>"#;
        let document = Html::parse_document(html);
        let root = document
            .select(&Selector::parse("body").unwrap())
            .next()
            .unwrap();
        let density_map = compute_text_density(root);
        assert_eq!(density_map.len(), 1);

        let root_stat = density_map.get(&root.id()).unwrap();
        assert_eq!(root_stat.text_length, 0);
    }

    #[test]
    fn test_page_with_nested_links() {
        let html = r#"
          <!DOCTYPE html>
          <html>
          <body>
              <p>Text <a href="\#">Link 1 <a href="\#">Link 2</a></a></p>
          </body>
          </html>
      "#;
        let document = Html::parse_document(html);
        let root = document.root_element();
        let density_map = compute_text_density(root);

        let p_selector = Selector::parse("p").unwrap();
        let p_element = document.select(&p_selector).next().unwrap();
        let p_stat = density_map.get(&p_element.id()).unwrap();

        assert_eq!(p_stat.link_tag_count, 2); // p has two child link tags (outer and inner a tags)
        assert_eq!(p_stat.tag_count, 2); // p has two child tags (outer and inner a tags)
    }

    #[test]
    fn test_text_only() {
        let html = r#"<!DOCTYPE html>
        <html>
        <body>
        This is just text.
        </body>
        </html>"#;

        let document = Html::parse_document(html);
        let root = document.root_element();
        let density_map = compute_text_density(root);

        let body_selector = Selector::parse("body").unwrap();
        let body = document.select(&body_selector).next().unwrap();
        let body_stat = density_map.get(&body.id()).unwrap();
        assert_eq!(body_stat.tag_count, 0);
        assert_eq!(body_stat.link_tag_count, 0);
        assert_eq!(body_stat.text_length, 18);
        assert!(body_stat.density > 0.0);
    }

    #[test]
    fn test_no_text() {
        let html = r#"<!DOCTYPE html>
        <html>
        <head><title>Title</title></head>
        <body>
        <img src="image.jpg" />
        </body>
        </html>"#;

        let document = Html::parse_document(html);
        let root = document.root_element();
        let density_map = compute_text_density(root);

        let body_selector = Selector::parse("body").unwrap();
        let body = document.select(&body_selector).next().unwrap();
        let body_stat = density_map.get(&body.id()).unwrap();

        assert_eq!(body_stat.text_length, 0);
    }
}
