//! End-to-end checks through the MCP tool handler.

use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use synapse_core::mcp::tools::handle_tool_call;
use synapse_core::store::SynapseStore;

async fn call(store: &Arc<SynapseStore>, name: &str, arguments: Value) -> Result<String, String> {
    let res = handle_tool_call("tools/call", Some(json!({ "name": name, "arguments": arguments })), store.clone()).await?;
    Ok(res["content"][0]["text"].as_str().unwrap_or_default().to_string())
}

async fn rows(store: &Arc<SynapseStore>, query: &str) -> Vec<Value> {
    let text = call(store, "sparql_query", json!({ "query": query })).await.unwrap();
    serde_json::from_str(&text).unwrap()
}

fn temp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("synapse-it-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(dir.join("people")).unwrap();
    dir
}

#[tokio::test]
async fn okf_bundle_links_resolve_and_reindex_retracts() {
    let store = Arc::new(SynapseStore::in_memory().unwrap());
    let dir = temp_dir();
    fs::write(dir.join("people/alice.md"), "---\ntype: person\ntitle: Alice\n---\nWorks with [[works_with:people/bob]].\n").unwrap();
    fs::write(dir.join("people/bob.md"), "---\ntype: person\n---\nBob.\n").unwrap();
    let args = json!({ "directory_path": dir.to_string_lossy() });

    call(&store, "index_markdown_directory", args.clone()).await.unwrap();
    // The link target is the entity of bob.md, which has a type.
    let q = "SELECT ?t WHERE { <urn:synapse:entity:people/alice> <urn:synapse:prop:works_with> ?b . ?b a ?t }";
    assert_eq!(rows(&store, q).await.len(), 1);
    // bob.md got a backlink section, which does not become a fact.
    assert!(fs::read_to_string(dir.join("people/bob.md")).unwrap().contains("[[people/alice]]"));

    // Unchanged second pass.
    let text = call(&store, "index_markdown_directory", args.clone()).await.unwrap();
    assert!(text.contains("0 changed"), "{text}");

    // Editing the file retracts the old link; deleting a file drops its facts.
    fs::write(dir.join("people/alice.md"), "---\ntype: person\n---\nAlone now.\n").unwrap();
    fs::remove_file(dir.join("people/bob.md")).unwrap();
    let text = call(&store, "index_markdown_directory", args).await.unwrap();
    assert!(text.contains("1 removed"), "{text}");
    assert!(rows(&store, "SELECT * WHERE { ?a <urn:synapse:prop:works_with> ?b }").await.is_empty());
    assert!(rows(&store, "SELECT * WHERE { <urn:synapse:entity:people/bob> ?p ?o }").await.is_empty());

    fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn agent_can_assert_and_retract() {
    let store = Arc::new(SynapseStore::in_memory().unwrap());
    let text = call(
        &store,
        "assert_facts",
        json!({
            "facts": [
                { "subject": "people/bob", "predicate": "a", "object": "Person" },
                { "subject": "people/bob", "predicate": "works_at", "object": "orgs/acme", "object_type": "entity" },
                { "subject": "people/bob", "predicate": "age", "object": "41", "datatype": "xsd:integer" },
                { "subject": "people/bob", "predicate": "rdfs:label", "object": "Bob" }
            ],
            "source": "test",
            "reason": "told by Bob"
        }),
    )
    .await
    .unwrap();
    let assertion = text.split_whitespace().find(|w| w.starts_with("urn:synapse:assertion:")).unwrap().to_string();

    let q = "SELECT * WHERE { <urn:synapse:entity:people/bob> a <urn:synapse:type:Person> ; <urn:synapse:prop:works_at> <urn:synapse:entity:orgs/acme> ; <urn:synapse:prop:age> 41 ; <http://www.w3.org/2000/01/rdf-schema#label> \"Bob\" }";
    assert_eq!(rows(&store, q).await.len(), 1);

    let prov = call(&store, "get_provenance", json!({ "id": assertion })).await.unwrap();
    assert!(prov.contains("told by Bob"), "{prov}");

    let text = call(&store, "retract_facts", json!({ "facts": [ { "subject": "people/bob", "predicate": "age", "object": "41", "datatype": "xsd:integer" } ], "reason": "typo" })).await.unwrap();
    assert!(text.starts_with("Retracted 1 fact(s)"), "{text}");
    assert!(rows(&store, "SELECT * WHERE { ?s <urn:synapse:prop:age> ?o }").await.is_empty());

    call(&store, "retract_facts", json!({ "assertion_id": assertion })).await.unwrap();
    assert!(rows(&store, "SELECT * WHERE { <urn:synapse:entity:people/bob> ?p ?o }").await.is_empty());
}

#[tokio::test]
async fn hostile_ids_are_rejected_or_neutralised() {
    let store = Arc::new(SynapseStore::in_memory().unwrap());
    let res = call(&store, "get_entity_narrative", json!({ "entity_id": "x> ?p ?o } DELETE { ?s ?p ?o } WHERE { <y" })).await;
    assert!(res.is_ok());
    assert!(call(&store, "assert_facts", json!({ "facts": [] })).await.is_err());
}
