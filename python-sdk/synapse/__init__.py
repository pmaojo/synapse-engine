"""
Synapse Python SDK

Provides a high-level interface to the Synapse Semantic Engine (Rust).
"""
from synapse.infrastructure.web.client import SemanticEngineClient, get_client

__version__ = "0.5.1"
__all__ = ["SemanticEngineClient", "get_client"]

# Core namespaces
from synapse import domain, application, infrastructure, tools
