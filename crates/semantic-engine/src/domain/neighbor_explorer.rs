use crate::domain::explorer_types::{ExplorerOptions, ScoringStrategy};
use crate::server::proto::Neighbor;
use crate::store::SynapseStore;
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;

pub struct NeighborExplorer {
    store: Arc<SynapseStore>,
}

impl NeighborExplorer {
    pub fn new(store: Arc<SynapseStore>) -> Self {
        Self { store }
    }

    pub async fn explore(&self, options: ExplorerOptions) -> Vec<Neighbor> {
        let mut neighbors = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        // Start with the initial node
        if let Some(start_uri) = self.store.get_uri(options.node_id) {
            queue.push_back((start_uri.clone(), 0));
            visited.insert(start_uri);
        }

        while let Some((current_uri, current_depth)) = queue.pop_front() {
            if current_depth >= options.depth {
                continue;
            }

            let next_depth = current_depth + 1;
            let base_score = 1.0 / next_depth as f32;

            let mut layer_count = 0;

            // Query outgoing
            if options.direction.is_outgoing() {
                self.query_direction(
                    &current_uri,
                    true,
                    &options,
                    next_depth,
                    base_score,
                    &mut visited,
                    &mut neighbors,
                    &mut queue,
                    &mut layer_count,
                );
            }

            // Query incoming
            if options.direction.is_incoming() {
                self.query_direction(
                    &current_uri,
                    false,
                    &options,
                    next_depth,
                    base_score,
                    &mut visited,
                    &mut neighbors,
                    &mut queue,
                    &mut layer_count,
                );
            }
        }

        // Sort by score (highest first)
        neighbors.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        neighbors
    }

    fn query_direction(
        &self,
        uri: &str,
        is_outgoing: bool,
        options: &ExplorerOptions,
        depth: usize,
        base_score: f32,
        visited: &mut HashSet<String>,
        neighbors: &mut Vec<Neighbor>,
        queue: &mut VecDeque<(String, usize)>,
        layer_count: &mut usize,
    ) {
        if *layer_count >= options.limit_per_layer && options.limit_per_layer > 0 {
            return;
        }

        let pattern = if is_outgoing {
            (
                oxigraph::model::NamedNodeRef::new(uri)
                    .ok()
                    .map(|n| n.into()),
                None,
                None,
                None,
            )
        } else {
            (
                None,
                None,
                oxigraph::model::NamedNodeRef::new(uri)
                    .ok()
                    .map(|n| n.into()),
                None,
            )
        };

        for quad in self
            .store
            .store
            .quads_for_pattern(pattern.0, pattern.1, pattern.2, pattern.3)
            .flatten()
        {
            if *layer_count >= options.limit_per_layer && options.limit_per_layer > 0 {
                break;
            }

            let pred = quad.predicate.to_string();
            if let Some(ref filter) = options.edge_filter {
                if !pred.contains(filter) {
                    continue;
                }
            }

            let target_term = if is_outgoing {
                quad.object
            } else {
                quad.subject.into()
            };
            let target_uri = target_term.to_string();

            // Type filter logic
            if let Some(ref type_filter) = options.node_type_filter {
                if !self.matches_type(&target_term, type_filter) {
                    continue;
                }
            }

            if !visited.contains(&target_uri) {
                visited.insert(target_uri.clone());
                let target_id = self.store.get_or_create_id(&target_uri);

                let mut score = base_score;
                if options.scoring_strategy == ScoringStrategy::Degree {
                    let clean_uri = match &target_term {
                        oxigraph::model::Term::NamedNode(n) => n.as_str(),
                        _ => &target_uri,
                    };
                    let degree = self.store.get_degree(clean_uri);
                    if degree > 0 {
                        // Progressive penalty to ensure deterministic ranking:
                        // degree 1 -> 1.0, degree 2 -> 1.41, degree 3 -> 1.73
                        score /= (degree as f32).sqrt();
                    }
                }

                neighbors.push(Neighbor {
                    node_id: target_id,
                    edge_type: pred,
                    uri: target_uri.clone(),
                    direction: if is_outgoing {
                        "outgoing".to_string()
                    } else {
                        "incoming".to_string()
                    },
                    depth: depth as u32,
                    score,
                });
                queue.push_back((target_uri, depth));
                *layer_count += 1;
            }
        }
    }

    fn matches_type(&self, term: &oxigraph::model::Term, type_uri: &str) -> bool {
        if let oxigraph::model::Term::NamedNode(ref n) = term {
            let rdf_type = oxigraph::model::NamedNodeRef::new(
                "http://www.w3.org/1999/02/22-rdf-syntax-ns#type",
            )
            .unwrap();
            if let Ok(target_type) = oxigraph::model::NamedNodeRef::new(type_uri) {
                self.store
                    .store
                    .quads_for_pattern(
                        Some(n.into()),
                        Some(rdf_type),
                        Some(target_type.into()),
                        None,
                    )
                    .next()
                    .is_some()
            } else {
                false
            }
        } else {
            false
        }
    }
}
