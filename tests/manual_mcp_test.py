import subprocess
import json
import sys
import os
import time

# Path to the binary
BINARY_PATH = os.path.abspath(os.path.join(os.path.dirname(__file__), "../target/release/synapse"))
NAMESPACE = "manual_mcp_test"

def run_mcp_test():
    print(f"🚀 Starting Manual MCP Test using {BINARY_PATH}...")

    if not os.path.exists(BINARY_PATH):
        print(f"❌ Binary not found at {BINARY_PATH}. Please build first.")
        sys.exit(1)

    # Start the process
    process = subprocess.Popen(
        [BINARY_PATH, "--mcp"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=sys.stderr, # Log stderr to console
        text=True,
        bufsize=1 # Line buffered
    )

    request_id = 0

    def send_request(method, params=None):
        nonlocal request_id
        request_id += 1
        req = {
            "jsonrpc": "2.0",
            "id": request_id,
            "method": method
        }
        if params:
            req["params"] = params

        json_req = json.dumps(req)
        print(f"\n📤 Sending: {method}")
        # print(f"   Payload: {json_req}")

        process.stdin.write(json_req + "\n")
        process.stdin.flush()

        # Read response
        line = process.stdout.readline()
        if not line:
            print("❌ Unexpected EOF from server")
            return None

        try:
            resp = json.loads(line)
            if "error" in resp and resp["error"]:
                 print(f"❌ Error from server: {resp['error']}")
            # print(f"📥 Received: {json.dumps(resp, indent=2)}")
            return resp
        except json.JSONDecodeError:
            print(f"❌ Failed to decode JSON: {line}")
            return None

    try:
        # 1. Initialize
        resp = send_request("initialize")
        assert resp and "result" in resp, "Initialize failed"
        print("✅ Initialize successful")

        # 2. List Tools
        resp = send_request("tools/list")
        assert resp and "result" in resp, "List tools failed"
        tools = resp["result"]["tools"]
        tool_names = [t["name"] for t in tools]
        print(f"✅ Found {len(tools)} tools: {', '.join(tool_names)}")
        assert "ingest_triples" in tool_names
        assert "sparql_query" in tool_names

        # 3. Clean up (Delete Namespace)
        print("\n🧹 Cleaning up namespace...")
        resp = send_request("tools/call", {
            "name": "delete_namespace",
            "arguments": {"namespace": NAMESPACE}
        })
        # Might fail if empty but that's fine, check structure
        assert resp and "result" in resp, "Delete namespace failed"

        # 4. Ingest Triples
        print("\n📥 Ingesting triples...")
        triples = [
            {"subject": "http://mcp.test/X", "predicate": "http://mcp.test/relates", "object": "http://mcp.test/Y"},
            {"subject": "http://mcp.test/Y", "predicate": "http://mcp.test/relates", "object": "http://mcp.test/Z"}
        ]
        resp = send_request("tools/call", {
            "name": "ingest_triples",
            "arguments": {
                "namespace": NAMESPACE,
                "triples": triples
            }
        })
        assert resp and "result" in resp, "Ingest failed"

        result_data = resp["result"]
        # print(f"DEBUG: result_data = {json.dumps(result_data)}")

        if result_data.get("isError"):
            print(f"❌ Ingest Tool execution failed: {result_data['content'][0]['text']}")
            sys.exit(1)

        content_text = result_data["content"][0]["text"]
        print(f"DEBUG: content_text repr: {repr(content_text)}")

        content = json.loads(content_text)
        print(f"   Result: {content['message']}")
        assert content["edges_added"] >= 2

        # Debug: List Triples
        print("\n📜 Listing Triples...")
        resp = send_request("tools/call", {
            "name": "list_triples",
            "arguments": {"namespace": NAMESPACE}
        })
        print(f"   Triples: {resp['result']['content'][0]['text']}")

        # 5. SPARQL Query
        print("\n❓ SPARQL Query...")
        query = "SELECT ?s ?o WHERE { GRAPH ?g { ?s <http://mcp.test/relates> ?o } }"
        resp = send_request("tools/call", {
            "name": "sparql_query",
            "arguments": {
                "namespace": NAMESPACE,
                "query": query
            }
        })
        assert resp and "result" in resp
        # The result text is a JSON string of SPARQL results
        sparql_text = resp["result"]["content"][0]["text"]
        if resp["result"].get("isError"):
             print(f"❌ SPARQL Tool execution failed: {sparql_text}")
             sys.exit(1)

        sparql_json = json.loads(sparql_text)
        if isinstance(sparql_json, list):
             count = len(sparql_json)
        else:
             count = len(sparql_json["results"]["bindings"])
        print(f"   Found {count} bindings")
        assert count >= 2

        # 6. Hybrid Search
        print("\n🔎 Hybrid Search...")
        resp = send_request("tools/call", {
            "name": "hybrid_search",
            "arguments": {
                "namespace": NAMESPACE,
                "query": "test",
                "vector_k": 10
            }
        })
        assert resp and "result" in resp
        search_text = resp["result"]["content"][0]["text"]
        if resp["result"].get("isError"):
             print(f"❌ Hybrid Search Tool execution failed: {search_text}")
             sys.exit(1)

        search_res = json.loads(search_text)
        print(f"   Found {len(search_res['results'])} results")

        # 7. Get Neighbors
        print("\n🕸️ Get Neighbors...")
        resp = send_request("tools/call", {
            "name": "get_neighbors",
            "arguments": {
                "namespace": NAMESPACE,
                "uri": "http://mcp.test/X"
            }
        })
        assert resp and "result" in resp
        neighbors_text = resp["result"]["content"][0]["text"]
        if resp["result"].get("isError"):
             print(f"❌ Get Neighbors Tool execution failed: {neighbors_text}")
             sys.exit(1)

        neighbors_res = json.loads(neighbors_text)
        neighbors = neighbors_res["neighbors"]
        print(f"   Found {len(neighbors)} neighbors")
        assert len(neighbors) >= 1

        # 8. Cleanup
        print("\n🧹 Final Cleanup...")
        send_request("tools/call", {
            "name": "delete_namespace",
            "arguments": {"namespace": NAMESPACE}
        })
        print("✅ Test Complete!")

    except Exception as e:
        print(f"\n❌ Test Failed: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)
    finally:
        process.terminate()

if __name__ == "__main__":
    run_mcp_test()
