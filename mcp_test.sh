#!/bin/bash
# Test MCP functionality via stdio

SYNAPSE_BIN="./target/release/synapse"

echo "=== MCP Verification Test ==="

# 1. Start Server in Background (simulated interaction)
# We'll use a python script to handle the JSON-RPC stateful interaction
cat <<EOF > verify_mcp.py
import subprocess
import json
import sys
import time

def rpc(proc, method, params=None, id=1):
    req = {"jsonrpc": "2.0", "id": id, "method": method}
    if params:
        req["params"] = params
    
    json_req = json.dumps(req)
    print(f"-> {json_req}", file=sys.stderr)
    proc.stdin.write(json_req + "\n")
    proc.stdin.flush()
    
    while True:
        line = proc.stdout.readline()
        if not line:
            break
        try:
            resp = json.loads(line)
            if resp.get("id") == id:
                print(f"<- {json.dumps(resp, indent=2)}", file=sys.stderr)
                return resp
        except json.JSONDecodeError:
            continue
    return None

proc = subprocess.Popen(
    ["$SYNAPSE_BIN", "--mcp"],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=sys.stderr, # Allow logs to show
    text=True,
    bufsize=1
)

try:
    # 1. Initialize
    rpc(proc, "initialize", {"capabilities": {}, "clientInfo": {"name": "test", "version": "1.0"}}, 1)
    
    # 2. List Tools
    tools = rpc(proc, "tools/list", {}, 2)
    tool_names = [t["name"] for t in tools["result"]["tools"]]
    print(f"\nAvailable tools: {tool_names}", file=sys.stderr)
    
    if "disambiguate" not in tool_names:
        print("❌ Missing 'disambiguate' tool!", file=sys.stderr)
        sys.exit(1)

    # 3. Ingest Triples
    rpc(proc, "tools/call", {
        "name": "ingest_triples",
        "arguments": {
            "namespace": "test_mcp",
            "triples": [
                {"subject": "http://example.org/Alice", "predicate": "http://schema.org/knows", "object": "http://example.org/Bob"},
                {"subject": "http://example.org/Bob", "predicate": "http://schema.org/knows", "object": "http://example.org/Charlie"}
            ]
        }
    }, 3)

    # 4. Ingest Text (RAG)
    rpc(proc, "tools/call", {
        "name": "ingest_text",
        "arguments": {
            "namespace": "test_mcp",
            "uri": "http://example.org/bio/Alice",
            "content": "Alice is a software engineer who loves Rust and graphs."
        }
    }, 4)

    # 5. Hybrid Search
    res = rpc(proc, "tools/call", {
        "name": "hybrid_search",
        "arguments": {
            "namespace": "test_mcp",
            "query": "engineer",
            "limit": 5
        }
    }, 5)
    
    # 6. Vector Stats
    rpc(proc, "tools/call", {
        "name": "vector_stats",
        "arguments": { "namespace": "test_mcp" }
    }, 6)

    # 7. Disambiguate
    rpc(proc, "tools/call", {
        "name": "disambiguate",
        "arguments": { "namespace": "test_mcp", "threshold": 0.1 } # Low threshold to ensure match
    }, 7)

finally:
    proc.terminate()
EOF

python3 verify_mcp.py
