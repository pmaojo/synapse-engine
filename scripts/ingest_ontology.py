import os
import sys
import argparse
from rdflib import Graph
from synapse import get_client

def ingest_ontology(file_path, namespace="programming"):
    print(f"📄 Parsing ontology file: {file_path}")
    g = Graph()
    
    # Auto-detect format
    ext = os.path.splitext(file_path)[1].lower()
    if ext == ".ttl":
        g.parse(file_path, format="turtle")
    elif ext == ".owl" or ext == ".rdf":
        g.parse(file_path, format="xml")
    else:
        g.parse(file_path)

    print(f"🔍 Found {len(g)} triples.")
    
    client = get_client()
    triples = []
    
    # Process in batches to avoid gRPC message size limits
    batch_size = 500
    total_added = 0
    
    for s, p, o in g:
        triples.append({
            "subject": str(s),
            "predicate": str(p),
            "object": str(o)
        })
        
        if len(triples) >= batch_size:
            res = client.ingest_triples(triples, namespace=namespace)
            if "error" in res:
                print(f"❌ Error in batch: {res['error']}")
            else:
                total_added += len(triples)
                print(f"✅ Ingested {total_added}/{len(g)} triples...")
            triples = []

    if triples:
        res = client.ingest_triples(triples, namespace=namespace)
        if "error" in res:
            print(f"❌ Error in final batch: {res['error']}")
        else:
            total_added += len(triples)
            print(f"✅ Ingested {total_added}/{len(g)} triples.")

    print(f"🎉 Completed ingestion for namespace '{namespace}'.")

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("file", help="Path to ontology file (OWL/TTL/RDF)")
    parser.add_argument("--namespace", default="programming", help="Namespace for triples")
    args = parser.parse_args()
    
    if not os.path.exists(args.file):
        print(f"❌ File not found: {args.file}")
        sys.exit(1)
        
    ingest_ontology(args.file, args.namespace)
