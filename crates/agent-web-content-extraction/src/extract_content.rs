use crate::compute_text_density::TextDensityStat;
use ego_tree::NodeId;
use scraper::ElementRef;
use std::collections::HashMap;

pub fn extract_content_from_stats<'a>(
    root: ElementRef<'a>,
    stats: &HashMap<NodeId, TextDensityStat<'a>>,
) -> Vec<ElementRef<'a>> {
    if stats.is_empty() {
        return Vec::new();
    }

    let threshold = compute_threshold(stats);
    let mut contents: Vec<ElementRef<'_>> = Vec::new();
    let mut mark_as_content = |node: ElementRef<'a>| {
        for content in &contents {
            if content.id() == node.id() {
                return;
            }

            if is_parent_of(*content, node) {
                return;
            }
        }

        contents.retain(|content| !is_parent_of(node, *content));
        contents.push(node);
    };

    mark_content(root, stats, threshold, &mut mark_as_content);

    contents
}

fn compute_threshold(stats: &HashMap<NodeId, TextDensityStat>) -> f64 {
    let max_density_sum = stats
        .iter()
        .reduce(|(acc_node_id, acc_stat), (node_id, stat)| {
            if stat.density_sum < acc_stat.density_sum {
                (acc_node_id, acc_stat)
            } else {
                (node_id, stat)
            }
        });
    let (element, max_density_sum) = match max_density_sum {
        Some((_, stat)) => (stat.element, stat.density_sum),
        None => return 0.0,
    };

    let mut parent = element.parent();
    let mut min_density = max_density_sum;

    while let Some(parent_node) = parent {
        let parent_ref = match ElementRef::wrap(parent_node) {
            Some(parent_ref) => parent_ref,
            None => break,
        };
        let parent_stat = match stats.get(&parent_ref.id()) {
            Some(parent_stat) => parent_stat,
            None => break,
        };

        if parent_stat.density < min_density {
            min_density = parent_stat.density;
        }

        parent = parent_ref.parent();
    }

    min_density
}

fn mark_content<'b, 'a: 'b>(
    node: ElementRef<'a>,
    stats: &HashMap<NodeId, TextDensityStat<'a>>,
    density_threshold: f64,
    mark_as_content: &mut (impl FnMut(ElementRef<'a>) + 'b),
) {
    let stat = match stats.get(&node.id()) {
        Some(stats) => stats,
        None => return,
    };

    if stat.density < density_threshold {
        return;
    }

    let max_density_sum_descendant = find_max_density_sum_descendant(node, stats);
    let max_density_sum_descendant = match max_density_sum_descendant {
        Some(max_density_sum_descendant) => max_density_sum_descendant,
        None => {
            mark_as_content(node);
            return;
        }
    };

    mark_as_content(max_density_sum_descendant);

    for child in node.child_elements() {
        mark_content(child, stats, density_threshold, mark_as_content);
    }
}

fn find_max_density_sum_descendant<'a>(
    node: ElementRef<'a>,
    stats: &HashMap<NodeId, TextDensityStat>,
) -> Option<ElementRef<'a>> {
    let stat = stats.get(&node.id())?;
    let mut max_density_sum = stat.density_sum;
    let mut max_density_sum_descendant = Some(node);

    for child in node.descendants() {
        let child_ref = match ElementRef::wrap(child) {
            Some(child_ref) => child_ref,
            None => continue,
        };
        let child_stat = match stats.get(&child_ref.id()) {
            Some(stats) => stats,
            None => continue,
        };

        if max_density_sum < child_stat.density_sum {
            max_density_sum = child_stat.density_sum;
            max_density_sum_descendant = Some(child_ref);
        }
    }

    max_density_sum_descendant
}

fn is_parent_of(parent: ElementRef, child: ElementRef) -> bool {
    let mut current = child.parent();

    while let Some(current_node) = current {
        if current_node.id() == parent.id() {
            return true;
        }

        current = current_node.parent();
    }

    false
}
