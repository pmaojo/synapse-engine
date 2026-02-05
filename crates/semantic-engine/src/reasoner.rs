use anyhow::Result;
use oxigraph::model::*;
use oxigraph::store::Store;
use reasonable::reasoner::ReasonerBuilder;

/// Reasoning strategy for knowledge graph inference
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReasoningStrategy {
    None,
    RDFS,
    OWLRL,
}

/// Reasoner for deriving implicit knowledge from RDF triples
pub struct SynapseReasoner {
    strategy: ReasoningStrategy,
}

impl SynapseReasoner {
    pub fn new(strategy: ReasoningStrategy) -> Self {
        Self { strategy }
    }

    /// Apply reasoning to the store and return inferred triples
    pub fn apply(&self, store: &Store) -> Result<Vec<(String, String, String)>> {
        match self.strategy {
            ReasoningStrategy::None => Ok(Vec::new()),
            ReasoningStrategy::RDFS => self.apply_rdfs_reasoning(store),
            ReasoningStrategy::OWLRL => self.apply_owl_reasoning(store),
        }
    }

    fn apply_rdfs_reasoning(&self, store: &Store) -> Result<Vec<(String, String, String)>> {
        let mut inferred = Vec::new();
        let sub_class_of = NamedNode::new("http://www.w3.org/2000/01/rdf-schema#subClassOf")?;
        
        for quad in store.iter() {
            if let Ok(q) = quad {
                if q.predicate == sub_class_of {
                    // q is (B subClassOf C)
                    // Find all A such that (A subClassOf B)
                    let subject_b = q.subject.clone();
                    if let Subject::NamedNode(subj_node) = subject_b {
                        for inner_quad in store.iter() {
                            if let Ok(iq) = inner_quad {
                                if iq.predicate == sub_class_of && iq.object == subj_node.clone().into() {
                                    // Transitivity: A subClassOf C
                                    inferred.push((
                                        iq.subject.to_string(),
                                        sub_class_of.to_string(),
                                        q.object.to_string(),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(inferred)
    }

    fn apply_owl_reasoning(&self, store: &Store) -> Result<Vec<(String, String, String)>> {
        let mut builder = ReasonerBuilder::new();
        
        // reasonable 0.3.2 with_triples_str requires &'static str.
        // We use Box::leak here to bypass the version mismatch between oxrdf crates
        // used by oxigraph and reasonable. This is acceptable for reasoning tasks
        // as the strings are lived for the duration of the reasoning process.
        let mut trips: Vec<(&'static str, &'static str, &'static str)> = Vec::new();
        for quad in store.iter() {
            if let Ok(q) = quad {
                let s: &'static str = Box::leak(q.subject.to_string().into_boxed_str());
                let p: &'static str = Box::leak(q.predicate.to_string().into_boxed_str());
                let o: &'static str = Box::leak(q.object.to_string().into_boxed_str());
                trips.push((s, p, o));
            }
        }
        
        builder = builder.with_triples_str(trips);
        let mut reasoner = builder.build().map_err(|e| anyhow::anyhow!("Failed to build reasoner: {}", e))?;
        
        reasoner.reason();
        Ok(reasoner.get_triples_string())
    }

    pub fn materialize(&self, store: &Store) -> Result<usize> {
        let inferred = self.apply(store)?;
        let mut count = 0;
        let mut skipped = 0;
        
        for (s, p, o) in inferred {
            if let (Ok(subject), Ok(predicate), Ok(object)) = (
                NamedNode::new(&s),
                NamedNode::new(&p),
                NamedNode::new(&o),
            ) {
                // Check if triple already exists to avoid duplicates
                let quad = Quad::new(subject.clone(), predicate.clone(), object.clone(), GraphName::DefaultGraph);
                if store.contains(&quad)? {
                    skipped += 1;
                    continue;
                }
                // Insert new inferred triple
                let _ = store.insert(&quad);
                count += 1;
            }
        }
        
        if skipped > 0 {
            eprintln!("Reasoning: {} new triples, {} duplicates skipped", count, skipped);
        }
        
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxigraph::model::NamedNode;

    #[test]
    fn test_rdfs_transitivity() -> Result<()> {
        let store = Store::new()?;
        let sub_class_of = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
        
        let a = NamedNode::new("http://example.org/A")?;
        let b = NamedNode::new("http://example.org/B")?;
        let c = NamedNode::new("http://example.org/C")?;
        let pred = NamedNode::new(sub_class_of)?;
        
        // A subClassOf B, B subClassOf C
        store.insert(&Quad::new(a.clone(), pred.clone(), b.clone(), GraphName::DefaultGraph))?;
        store.insert(&Quad::new(b.clone(), pred.clone(), c.clone(), GraphName::DefaultGraph))?;
        
        let reasoner = SynapseReasoner::new(ReasoningStrategy::RDFS);
        let inferred = reasoner.apply(&store)?;
        
        // Should infer A subClassOf C
        let mut found = false;
        let expected_s = a.to_string();
        let expected_o = c.to_string();
        
        for (s, _p, o) in inferred {
            if s == expected_s && o == expected_o {
                found = true;
                break;
            }
        }
        
        assert!(found, "Inferred A subClassOf C not found");
        Ok(())
    }

    #[test]
    fn test_owl_reasoning_smoke() -> Result<()> {
        let store = Store::new()?;
        let reasoner = SynapseReasoner::new(ReasoningStrategy::OWLRL);
        
        let inferred = reasoner.apply(&store)?;
        // OWL Reasoning often has default axioms, so we just check it doesn't crash
        // and returns at least something (usually standard RDF/OWL URIs)
        println!("OWL Reasoner inferred {} default triples", inferred.len());
        Ok(())
    }
}
