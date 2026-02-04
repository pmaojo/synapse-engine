use anyhow::Result;
use oxigraph::model::*;
use oxigraph::store::Store;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::RwLock;

pub struct SynapseStore {
    pub store: Store,
    pub namespace: String,
    // Mapping for gRPC compatibility (ID <-> URI)
    pub id_to_uri: RwLock<HashMap<u32, String>>,
    pub uri_to_id: RwLock<HashMap<String, u32>>,
    pub next_id: std::sync::atomic::AtomicU32,
}

impl SynapseStore {
    pub fn open(namespace: &str, storage_path: &str) -> Result<Self> {
        let path = PathBuf::from(storage_path).join(namespace);
        let store = Store::open(path)?;
        
        Ok(Self {
            store,
            namespace: namespace.to_string(),
            id_to_uri: RwLock::new(HashMap::new()),
            uri_to_id: RwLock::new(HashMap::new()),
            next_id: std::sync::atomic::AtomicU32::new(1),
        })
    }

    pub fn get_or_create_id(&self, uri: &str) -> u32 {
        {
            let map = self.uri_to_id.read().unwrap();
            if let Some(&id) = map.get(uri) {
                return id;
            }
        }
        
        let mut uri_map = self.uri_to_id.write().unwrap();
        let mut id_map = self.id_to_uri.write().unwrap();
        
        if let Some(&id) = uri_map.get(uri) {
            return id;
        }
        
        let id = self.next_id.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        uri_map.insert(uri.to_string(), id);
        id_map.insert(id, uri.to_string());
        id
    }

    pub fn get_uri(&self, id: u32) -> Option<String> {
        self.id_to_uri.read().unwrap().get(&id).cloned()
    }

    pub fn ingest_triples(&self, triples: Vec<(String, String, String)>) -> Result<(u32, u32)> {
        let mut added = 0;
        
        for (s, p, o) in triples {
            let subject_uri = self.ensure_uri(&s);
            let predicate_uri = self.ensure_uri(&p);
            let object_uri = self.ensure_uri(&o);
            
            let subject = Subject::NamedNode(NamedNode::new_unchecked(&subject_uri));
            let predicate = NamedNode::new_unchecked(&predicate_uri);
            let object = Term::NamedNode(NamedNode::new_unchecked(&object_uri));
            
            let quad = Quad::new(subject, predicate, object, GraphName::DefaultGraph);
            if self.store.insert(&quad)? {
                added += 1;
            }
        }

        Ok((added, 0))
    }

    fn ensure_uri(&self, s: &str) -> String {
        if s.starts_with("http") {
            s.to_string()
        } else {
            format!("http://synapse.os/{}", s)
        }
    }
}
