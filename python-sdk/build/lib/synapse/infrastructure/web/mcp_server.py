"""MCP (Model Context Protocol) Server Implementation"""
import json
import asyncio
from typing import Dict, Any, List
from synapse.infrastructure.persistence.graph_client import GraphClient
from synapse.infrastructure.persistence.vector_store import VectorStore
import logging

logger = logging.getLogger(__name__)

class MCPServer:
    """MCP Server exposing the semantic system as tools for LLMs"""
    
    def __init__(self, ontology_paths: List[str]):
        # Initialize components
        self.graph_client = GraphClient()
        self.vector_store = VectorStore()
        self.pipeline = SemanticPipeline(
            ontology_paths,
            self.graph_client,
            self.vector_store
        )
        
        # Define available tools
        self.tools = {
            "query_knowledge_graph": self.query_knowledge_graph,
            "add_observation": self.add_observation,
            "validate_hypothesis": self.validate_hypothesis,
            "get_ontology_classes": self.get_ontology_classes,
            "sparql_query": self.sparql_query
        }
    
    async def handle_request(self, request: Dict[str, Any]) -> Dict[str, Any]:
        """Handle MCP request"""
        method = request.get("method")
        params = request.get("params", {})
        
        if method == "tools/list":
            return self.list_tools()
        elif method == "tools/call":
            tool_name = params.get("name")
            arguments = params.get("arguments", {})
            return await self.call_tool(tool_name, arguments)
        else:
            return {"error": f"Unknown method: {method}"}
    
    def list_tools(self) -> Dict[str, Any]:
        """List available tools"""
        return {
            "tools": [
                {
                    "name": "query_knowledge_graph",
                    "description": "Query the knowledge graph using RAG (hybrid symbolic + vector search)",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "query": {"type": "string", "description": "Natural language query"},
                            "top_k": {"type": "integer", "description": "Number of results", "default": 5}
                        },
                        "required": ["query"]
                    }
                },
                {
                    "name": "add_observation",
                    "description": "Add new information to the knowledge graph",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "text": {"type": "string", "description": "Text containing facts to extract"},
                            "source": {"type": "string", "description": "Source of the information"}
                        },
                        "required": ["text"]
                    }
                },
                {
                    "name": "validate_hypothesis",
                    "description": "Check if a statement is consistent with the ontology",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "statement": {"type": "string", "description": "Statement to validate"}
                        },
                        "required": ["statement"]
                    }
                },
                {
                    "name": "get_ontology_classes",
                    "description": "Get all classes defined in the ontology",
                    "inputSchema": {"type": "object", "properties": {}}
                },
                {
                    "name": "sparql_query",
                    "description": "Execute a SPARQL query against the knowledge graph",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "query": {"type": "string", "description": "SPARQL query"}
                        },
                        "required": ["query"]
                    }
                }
            ]
        }
    
    async def call_tool(self, tool_name: str, arguments: Dict[str, Any]) -> Dict[str, Any]:
        """Call a tool"""
        if tool_name not in self.tools:
            return {"error": f"Tool not found: {tool_name}"}
        
        try:
            result = await self.tools[tool_name](arguments)
            return {"content": [{"type": "text", "text": json.dumps(result, indent=2)}]}
        except Exception as e:
            logger.error(f"Error calling tool {tool_name}: {e}")
            return {"error": str(e)}
    
    async def query_knowledge_graph(self, args: Dict[str, Any]) -> Dict[str, Any]:
        """Query the knowledge graph"""
        query = args["query"]
        top_k = args.get("top_k", 5)
        return self.pipeline.query(query, mode="rag")
    
    async def add_observation(self, args: Dict[str, Any]) -> Dict[str, Any]:
        """Add new observation to the graph"""
        text = args["text"]
        source = args.get("source", "unknown")
        result = self.pipeline.process_text(text)
        return {
            "status": "success",
            "source": source,
            **result
        }
    
    async def validate_hypothesis(self, args: Dict[str, Any]) -> Dict[str, Any]:
        """Validate a hypothesis"""
        statement = args["statement"]
        # Process as text and check if it produces valid triples
        result = self.pipeline.process_text(statement)
        is_valid = result["validated"] > 0
        return {
            "statement": statement,
            "is_valid": is_valid,
            "confidence": 0.8 if is_valid else 0.2,
            "details": result
        }
    
    async def get_ontology_classes(self, args: Dict[str, Any]) -> Dict[str, Any]:
        """Get all ontology classes"""
        classes = self.pipeline.sparql_engine.get_all_classes()
        return {"classes": classes}
    
    async def sparql_query(self, args: Dict[str, Any]) -> Dict[str, Any]:
        """Execute SPARQL query"""
        query = args["query"]
        return self.pipeline.query(query, mode="sparql")

async def run_mcp_server(ontology_paths: List[str], host: str = "localhost", port: int = 3000):
    """Run the MCP server"""
    server = MCPServer(ontology_paths)
    logger.info(f"MCP Server started on {host}:{port}")
    logger.info(f"Available tools: {list(server.tools.keys())}")
    # In production, this would set up HTTP/WebSocket server
    # For now, just keep the server alive
    while True:
        await asyncio.sleep(1)
