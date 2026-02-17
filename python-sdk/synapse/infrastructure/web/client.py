"""
MCP Client for Semantic Engine (Rust Backend)
Provides a Python interface to the production Rust graph storage via mcporter
"""
import subprocess
import json
from typing import List, Tuple, Optional
import os

class SemanticEngineClient:
    """
    Python client for the Rust semantic-engine via mcporter.
    Provides persistent, production-grade graph storage.
    """
    def __init__(self, namespace: str = "default"):
        self.default_namespace = namespace
        
    def connect(self) -> bool:
        """Verify mcporter is available"""
        try:
            subprocess.run(["mcporter", "--version"], capture_output=True, check=True)
            return True
        except Exception:
            print("⚠️  mcporter not found in PATH")
            return False

    def _call_tool(self, tool: str, arguments: dict) -> dict:
        # We use --output json but we must be careful with mcporter's output
        cmd = ["mcporter", "call", f"synapse.{tool}", "--output", "json"]
        for k, v in arguments.items():
            cmd.append(f"{k}={json.dumps(v)}")
        
        try:
            result = subprocess.run(cmd, capture_output=True, text=True, check=True)
            stdout = result.stdout.split("[mcporter]")[0].strip()
            
            # Try to parse as pure JSON first
            try:
                return json.loads(stdout)
            except json.JSONDecodeError:
                # If it's the JS-like object { content: [...], isError: ... }
                # we try to extract the text from content
                import re
                text_match = re.search(r"text:\s*'([^']*)'", stdout)
                if text_match:
                    text_content = text_match.group(1)
                    try:
                        return json.loads(text_content)
                    except:
                        return {"message": text_content, "isError": "isError: true" in stdout}
                
                return {"error": "Failed to parse MCP output", "raw": stdout}
                
        except subprocess.CalledProcessError as e:
            return {"error": e.stderr or str(e)}
        except Exception as e:
            return {"error": str(e)}

    def ingest_triples(self, triples: List[dict], namespace: Optional[str] = None) -> dict:
        """Send triples to Rust backend via MCP"""
        ns = namespace or self.default_namespace
        response = self._call_tool("ingest_triples", {
            "triples": triples,
            "namespace": ns
        })
        if "error" in response:
            return response
            
        # Extract from content if wrapped
        if isinstance(response, list) and len(response) > 0:
            text = response[0].get("text", "{}")
            try:
                return json.loads(text)
            except:
                return {"message": text}
        return response

    def hybrid_search(self, query: str, namespace: Optional[str] = None, vector_k: int = 10, graph_depth: int = 1) -> List[dict]:
        """Perform hybrid search via MCP"""
        ns = namespace or self.default_namespace
        response = self._call_tool("hybrid_search", {
            "query": query,
            "namespace": ns,
            "vector_k": vector_k,
            "graph_depth": graph_depth
        })
        if "error" in response:
            print(f"Error in search: {response['error']}")
            return []
        
        # Parse the inner JSON result from MCP
        try:
            # Synapse returns SearchToolResult as JSON string in the first content item
            # or directly if mcporter handles it.
            # Based on mcp_stdio.rs: serialize_result returns a JSON string.
            if isinstance(response, list) and len(response) > 0:
                data = json.loads(response[0].get("text", "{}"))
                return data.get("results", [])
            elif isinstance(response, dict) and "results" in response:
                return response["results"]
            return []
        except Exception as e:
            print(f"Failed to parse search results: {e}")
            return []

    def sparql_query(self, query: str, namespace: Optional[str] = None) -> List[dict]:
        """Execute SPARQL query via MCP"""
        ns = namespace or self.default_namespace
        response = self._call_tool("sparql_query", {
            "query": query,
            "namespace": ns
        })
        
        if "error" in response:
            print(f"Error in SPARQL: {response['error']}")
            return []
        
        if isinstance(response, list):
            return response
            
        if isinstance(response, dict):
            if response.get("isError"):
                print(f"SPARQL Tool Error: {response.get('message')}")
                return []
            
            # If message contains JSON string
            message = response.get("message", "")
            if message.startswith("[") or message.startswith("{"):
                try:
                    return json.loads(message)
                except:
                    pass
            
        return []

    def get_all_triples(self, namespace: Optional[str] = None) -> List[dict]:
        """Get all triples via MCP list_triples"""
        ns = namespace or self.default_namespace
        response = self._call_tool("list_triples", {
            "namespace": ns,
            "limit": 10000
        })
        if "error" in response:
            return []
        
        try:
            if isinstance(response, dict) and "triples" in response:
                return response["triples"]
            return []
        except Exception:
            return []

    def delete_tenant_data(self, namespace: str) -> dict:
        """Delete namespace data via MCP"""
        return self._call_tool("delete_namespace", {"namespace": namespace})
    
    def ingest_text(self, uri: str, content: str, namespace: Optional[str] = None) -> dict:
        """Ingest text via MCP"""
        ns = namespace or self.default_namespace
        return self._call_tool("ingest_text", {
            "uri": uri,
            "content": content,
            "namespace": ns
        })

    def apply_reasoning(self, namespace: Optional[str] = None, strategy: str = "rdfs", materialize: bool = False) -> dict:
        """Apply reasoning via MCP"""
        ns = namespace or self.default_namespace
        return self._call_tool("apply_reasoning", {
            "namespace": ns,
            "strategy": strategy,
            "materialize": materialize
        })

    def close(self):
        """No-op for MCP CLI-based client"""
        pass

# Global singleton instance
_client: Optional[SemanticEngineClient] = None

def get_client() -> SemanticEngineClient:
    """Get or create the global client instance and eagerly verify connectivity."""
    global _client
    if _client is None:
        _client = SemanticEngineClient()
        _client.connect()
    return _client
