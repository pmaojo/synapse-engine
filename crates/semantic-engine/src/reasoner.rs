use anyhow::Result;
use oxigraph::model::{GraphName, NamedNode, Quad, Subject, Term};
use oxigraph::store::Store;

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum ReasoningStrategy {
    None,
    RDFS,
    OWLRL,
}

pub struct SynapseReasoner {
    pub strategy: ReasoningStrategy,
}

impl SynapseReasoner {
    pub fn new(strategy: ReasoningStrategy) -> Self {
        Self { strategy }
    }

    /// Apply reasoning to a store and return inferred triples (without inserting)
    pub fn apply(&self, store: &Store) -> Result<Vec<(String, String, String)>> {
        let mut inferred = Vec::new();

        match self.strategy {
            ReasoningStrategy::None => {}
            ReasoningStrategy::RDFS => {
                // RDFS: SubClassOf Transitivity
                // If A subClassOf B, and B subClassOf C -> A subClassOf C
                let subclass_prop =
                    NamedNode::new("http://www.w3.org/2000/01/rdf-schema#subClassOf")?;

                for q1 in store
                    .quads_for_pattern(None, Some(subclass_prop.as_ref()), None, None)
                    .flatten()
                {
                    if let Subject::NamedNode(a) = q1.subject {
                        if let Term::NamedNode(b) = q1.object {
                            for q2 in store
                                .quads_for_pattern(
                                    Some(b.as_ref().into()),
                                    Some(subclass_prop.as_ref()),
                                    None,
                                    None,
                                )
                                .flatten()
                            {
                                if let Term::NamedNode(c) = q2.object {
                                    inferred.push((
                                        a.as_str().to_string(),
                                        subclass_prop.as_str().to_string(),
                                        c.as_str().to_string(),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
            ReasoningStrategy::OWLRL => {
                // OWL-RL: TransitiveProperty
                // If p is TransitiveProperty, and x p y, y p z -> x p z
                let type_prop = NamedNode::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")?;
                let transitive_class =
                    NamedNode::new("http://www.w3.org/2002/07/owl#TransitiveProperty")?;

                // Find all transitive properties
                for q in store
                    .quads_for_pattern(
                        None,
                        Some(type_prop.as_ref()),
                        Some(transitive_class.as_ref().into()),
                        None,
                    )
                    .flatten()
                {
                    if let Subject::NamedNode(p_node) = q.subject {
                        let p_ref = p_node.as_ref();

                        // Naive transitive: x p y ("xy")
                        for xy_quad in store
                            .quads_for_pattern(None, Some(p_ref), None, None)
                            .flatten()
                        {
                            if let Subject::NamedNode(x) = xy_quad.subject {
                                if let Term::NamedNode(y) = xy_quad.object {
                                    // Find y p z ("yz")
                                    for yz_quad in store
                                        .quads_for_pattern(
                                            Some(y.as_ref().into()),
                                            Some(p_ref),
                                            None,
                                            None,
                                        )
                                        .flatten()
                                    {
                                        if let Term::NamedNode(z) = yz_quad.object {
                                            inferred.push((
                                                x.as_str().to_string(),
                                                p_node.as_str().to_string(),
                                                z.as_str().to_string(),
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // OWL-RL: SymmetricProperty
                // If p is SymmetricProperty, and x p y -> y p x
                let symmetric_class =
                    NamedNode::new("http://www.w3.org/2002/07/owl#SymmetricProperty")?;

                for q in store
                    .quads_for_pattern(
                        None,
                        Some(type_prop.as_ref()),
                        Some(symmetric_class.as_ref().into()),
                        None,
                    )
                    .flatten()
                {
                    if let Subject::NamedNode(p_node) = q.subject {
                        let p_ref = p_node.as_ref();

                        for e in store
                            .quads_for_pattern(None, Some(p_ref), None, None)
                            .flatten()
                        {
                            // Infer: y p x
                            if let Subject::NamedNode(s_node) = e.subject {
                                if let Term::NamedNode(obj_node) = e.object {
                                    inferred.push((
                                        obj_node.as_str().to_string(),
                                        p_node.as_str().to_string(),
                                        s_node.as_str().to_string(),
                                    ));
                                }
                            }
                        }
                    }
                }

                // OWL-RL: inverseOf
                // If p1 inverseOf p2, and x p1 y -> y p2 x
                let inverse_prop = NamedNode::new("http://www.w3.org/2002/07/owl#inverseOf")?;

                for q in store
                    .quads_for_pattern(None, Some(inverse_prop.as_ref()), None, None)
                    .flatten()
                {
                    if let Subject::NamedNode(p1_node) = q.subject {
                        let p1_ref = p1_node.as_ref();
                        if let Term::NamedNode(p2_node) = q.object {
                            // p1 inverseOf p2. For every x p1 y, infer y p2 x
                            for e in store
                                .quads_for_pattern(None, Some(p1_ref), None, None)
                                .flatten()
                            {
                                if let Subject::NamedNode(x) = e.subject {
                                    if let Term::NamedNode(y) = e.object {
                                        inferred.push((
                                            y.as_str().to_string(),
                                            p2_node.as_str().to_string(),
                                            x.as_str().to_string(),
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(inferred)
    }

    /// Apply reasoning and persist inferred triples
    pub fn materialize(&self, store: &Store) -> Result<usize> {
        let mut total_inferred = 0;

        // Fixed-point iteration loop
        loop {
            let inferred = self.apply(store)?;
            if inferred.is_empty() {
                break;
            }

            let mut new_triples = 0;
            for (s, p, o) in inferred {
                let s_node = NamedNode::new(s)?;
                let p_node = NamedNode::new(p)?;
                let o_node = NamedNode::new(o)?;

                let quad = Quad::new(s_node, p_node, o_node, GraphName::DefaultGraph);

                // Only count if actually new
                // Note: store.contains checks exact match including graph name.
                // We insert into DefaultGraph.
                if !store.contains(&quad)? {
                    store.insert(&quad)?;
                    new_triples += 1;
                }
            }

            if new_triples == 0 {
                break;
            }
            total_inferred += new_triples;
        }

        Ok(total_inferred)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rdfs_transitivity() -> Result<()> {
        let store = Store::new()?;
        let reasoner = SynapseReasoner::new(ReasoningStrategy::RDFS);

        let a = NamedNode::new("http://example.org/A")?;
        let b = NamedNode::new("http://example.org/B")?;
        let c = NamedNode::new("http://example.org/C")?;
        let sub_class_of = NamedNode::new("http://www.w3.org/2000/01/rdf-schema#subClassOf")?;

        store.insert(&Quad::new(
            a.clone(),
            sub_class_of.clone(),
            b.clone(),
            GraphName::DefaultGraph,
        ))?;
        store.insert(&Quad::new(
            b.clone(),
            sub_class_of.clone(),
            c.clone(),
            GraphName::DefaultGraph,
        ))?;

        let inferred = reasoner.apply(&store)?;
        assert!(!inferred.is_empty());
        assert!(inferred.contains(&(
            a.as_str().to_string(),
            sub_class_of.as_str().to_string(),
            c.as_str().to_string()
        )));

        Ok(())
    }

    #[test]
    fn test_owl_transitive_property() -> Result<()> {
        let store = Store::new()?;
        let reasoner = SynapseReasoner::new(ReasoningStrategy::OWLRL);

        let p = NamedNode::new("http://example.org/ancestorOf")?;
        let type_prop = NamedNode::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")?;
        let trans_class = NamedNode::new("http://www.w3.org/2002/07/owl#TransitiveProperty")?;

        // p a TransitiveProperty
        store.insert(&Quad::new(
            p.clone(),
            type_prop,
            trans_class,
            GraphName::DefaultGraph,
        ))?;

        let x = NamedNode::new("http://example.org/grandparent")?;
        let y = NamedNode::new("http://example.org/parent")?;
        let z = NamedNode::new("http://example.org/child")?;

        // x p y
        store.insert(&Quad::new(
            x.clone(),
            p.clone(),
            y.clone(),
            GraphName::DefaultGraph,
        ))?;
        // y p z
        store.insert(&Quad::new(
            y.clone(),
            p.clone(),
            z.clone(),
            GraphName::DefaultGraph,
        ))?;

        let inferred = reasoner.apply(&store)?;
        assert!(inferred.contains(&(
            x.as_str().to_string(),
            p.as_str().to_string(),
            z.as_str().to_string()
        )));

        Ok(())
    }
}
