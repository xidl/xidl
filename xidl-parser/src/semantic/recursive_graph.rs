use std::collections::{HashMap, HashSet};

pub(super) fn collect_recursive_edges(edges: &[(String, String)]) -> HashSet<(String, String)> {
    let mut graph: HashMap<String, Vec<String>> = HashMap::new();
    for (from, to) in edges {
        graph.entry(from.clone()).or_default().push(to.clone());
        graph.entry(to.clone()).or_default();
    }

    let mut visited = HashSet::new();
    let mut order = Vec::new();
    for node in graph.keys() {
        visit(node, &graph, &mut visited, &mut order);
    }

    let mut reverse: HashMap<String, Vec<String>> = HashMap::new();
    for (from, targets) in &graph {
        for to in targets {
            reverse.entry(to.clone()).or_default().push(from.clone());
        }
        reverse.entry(from.clone()).or_default();
    }

    let mut assigned = HashSet::new();
    let mut recursive_nodes = HashSet::new();
    for node in order.into_iter().rev() {
        if assigned.contains(&node) {
            continue;
        }
        let mut component = Vec::new();
        collect_component(&node, &reverse, &mut assigned, &mut component);
        if component.len() > 1 {
            recursive_nodes.extend(component);
            continue;
        }
        if graph
            .get(&node)
            .is_some_and(|targets| targets.iter().any(|target| target == &node))
        {
            recursive_nodes.insert(node);
        }
    }

    edges
        .iter()
        .filter(|(from, to)| recursive_nodes.contains(from) && recursive_nodes.contains(to))
        .cloned()
        .collect()
}

fn visit(
    node: &str,
    graph: &HashMap<String, Vec<String>>,
    visited: &mut HashSet<String>,
    order: &mut Vec<String>,
) {
    if !visited.insert(node.to_string()) {
        return;
    }
    if let Some(targets) = graph.get(node) {
        for target in targets {
            visit(target, graph, visited, order);
        }
    }
    order.push(node.to_string());
}

fn collect_component(
    node: &str,
    reverse: &HashMap<String, Vec<String>>,
    assigned: &mut HashSet<String>,
    component: &mut Vec<String>,
) {
    if !assigned.insert(node.to_string()) {
        return;
    }
    component.push(node.to_string());
    if let Some(targets) = reverse.get(node) {
        for target in targets {
            collect_component(target, reverse, assigned, component);
        }
    }
}
