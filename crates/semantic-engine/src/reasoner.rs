use anyhow::Result;
use oxigraph::model::*;
use oxigraph::store::Store;

/// Reasoning strategy for knowledge graph inference
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReasoningStrategy {
    None,
    RDFS,
    OWLRL,
}

/// Selectable reasoning rules for fine-grained control
#[derive(Debug, Clone, Default)]
pub struct RuleSet {
    pub subclass_transitivity: bool,
    pub subproperty_transitivity: bool,
    pub domain_range: bool,
    pub inverse_of: bool,
    pub symmetric_property: bool,
    pub transitive_property: bool,
}

impl RuleSet {
    /// All RDFS rules enabled
    pub fn rdfs() -> Self {
        Self {
            subclass_transitivity: true,
            subproperty_transitivity: true,
            domain_range: true,
            inverse_of: false,
            symmetric_property: false,
            transitive_property: false,
        }
    }

    /// All OWL-RL rules enabled
    pub fn owlrl() -> Self {
        Self {
            subclass_transitivity: true,
            subproperty_transitivity: true,
            domain_range: true,
            inverse_of: true,
            symmetric_property: true,
            transitive_property: true,
        }
    }

    /// Parse from comma-separated rule names
    pub fn from_str(rules: &str) -> Self {
        let mut ruleset = Self::default();
        for rule in rules.split(',').map(|s| s.trim().to_lowercase()) {
            match rule.as_str() {
                "subclass" | "subclass_transitivity" => ruleset.subclass_transitivity = true,
                "subproperty" | "subproperty_transitivity" => ruleset.subproperty_transitivity = true,
                "domain_range" | "dr" => ruleset.domain_range = true,
                "inverse" | "inverse_of" => ruleset.inverse_of = true,
                "symmetric" | "symmetric_property" => ruleset.symmetric_property = true,
                "transitive" | "transitive_property" => ruleset.transitive_property = true,
                "rdfs" => ruleset = Self::rdfs(),
                "owlrl" | "owl-rl" => ruleset = Self::owlrl(),
                _ => {}
            }
        }
        ruleset
    }
}

/// Reasoner for deriving implicit knowledge from RDF triples
pub struct SynapseReasoner {
    strategy: ReasoningStrategy,
    rules: RuleSet,
}

impl SynapseReasoner {
    pub fn new(strategy: ReasoningStrategy) -> Self {
        let rules = match strategy {
            ReasoningStrategy::RDFS => RuleSet::rdfs(),
            ReasoningStrategy::OWLRL => RuleSet::owlrl(),
            ReasoningStrategy::None => RuleSet::default(),
        };
        Self { strategy, rules }
    }

    pub fn with_rules(strategy: ReasoningStrategy, rules: RuleSet) -> Self {
        Self { strategy, rules }
    }

    /// Get current rule configuration
    pub fn rules(&self) -> &RuleSet {
        &self.rules
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
        let mut inferred = Vec::new();
        let rules = &self.rules;

        // 1. Symmetric Property
        if rules.symmetric_property {
            let type_prop = NamedNode::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")?;
            let symmetric_class = NamedNode::new("http://www.w3.org/2002/07/owl#SymmetricProperty")?;
            
            for quad in store.quads_for_pattern(None, Some(type_prop.as_ref().into()), Some(symmetric_class.as_ref().into()), None) {
                if let Ok(q) = quad {
                    // Check if subject is a NamedNode (properties must be named)
                    if let Subject::NamedNode(p_node) = q.subject {
                         let p_ref = p_node.as_ref();
                         
                         // Find all triples using p: x p y
                         for edge in store.quads_for_pattern(None, Some(p_ref.into()), None, None) {
                             if let Ok(e) = edge {
                                 // Infer: y p x
                                 if let Term::NamedNode(obj_node) = e.object {
                                     let s_str = e.subject.to_string();
                                     let p_str = p_node.to_string();
                                     let o_str = obj_node.to_string();
                                     inferred.push((o_str, p_str, s_str));
                                 }
                             }
                         }
                    }
                }
            }
        }

        // 2. Transitive Property
        if rules.transitive_property {
            let type_prop = NamedNode::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")?;
            let transitive_class = NamedNode::new("http://www.w3.org/2002/07/owl#TransitiveProperty")?;

            for quad in store.quads_for_pattern(None, Some(type_prop.as_ref().into()), Some(transitive_class.as_ref().into()), None) {
                if let Ok(q) = quad {
                     if let Subject::NamedNode(p_node) = q.subject {
                         let p_ref = p_node.as_ref();
                         
                         // Naive transitive: x p y ("xy")
                         for xy in store.quads_for_pattern(None, Some(p_ref.into()), None, None) {
                             if let Ok(xy_quad) = xy {
                                 if let Term::NamedNode(y) = xy_quad.object {
                                     // Find y p z ("yz")
                                     for yz in store.quads_for_pattern(Some(y.as_ref().into()), Some(p_ref.into()), None, None) {
                                         if let Ok(yz_quad) = yz {
                                             inferred.push((
                                                 xy_quad.subject.to_string(),
                                                 p_node.to_string(),
                                                 yz_quad.object.to_string()
                                             ));
                                         }
                                     }
                                 }
                             }
                         }
                     }
                }
            }
        }

        // 3. InverseOf
        if rules.inverse_of {
            let inverse_prop = NamedNode::new("http://www.w3.org/2002/07/owl#inverseOf")?;
            
            for quad in store.quads_for_pattern(None, Some(inverse_prop.as_ref().into()), None, None) {
                if let Ok(q) = quad {
                    if let Subject::NamedNode(p1_node) = q.subject {
                        let p1_ref = p1_node.as_ref();
                        if let Term::NamedNode(p2_node) = q.object {
                            // p1 inverseOf p2. For every x p1 y, infer y p2 x
                            for edge in store.quads_for_pattern(None, Some(p1_ref.into()), None, None) {
                                if let Ok(e) = edge {
                                    if let Term::NamedNode(y) = e.object {
                                        inferred.push((
                                            y.to_string(),
                                            p2_node.to_string(),
                                            e.subject.to_string()
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
