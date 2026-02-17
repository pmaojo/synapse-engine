import os
import sys

import pytest

# Add the project root to sys.path to allow importing synapse
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "../python-sdk")))

from synapse.infrastructure.web.client import SemanticEngineClient


def test_client_connection_success(monkeypatch: pytest.MonkeyPatch) -> None:
    """Client reports connection when mcporter is available."""

    def _fake_run(*_args, **_kwargs):
        class _Result:
            pass

        return _Result()

    monkeypatch.setattr("subprocess.run", _fake_run)

    client = SemanticEngineClient(namespace="test")
    assert client.connect() is True


def test_client_connection_failure(monkeypatch: pytest.MonkeyPatch) -> None:
    """Client reports disconnection when mcporter invocation fails."""

    def _raise_error(*_args, **_kwargs):
        raise FileNotFoundError("mcporter not found")

    monkeypatch.setattr("subprocess.run", _raise_error)

    client = SemanticEngineClient(namespace="test")
    assert client.connect() is False


def test_ingest_triples_parses_wrapped_content(monkeypatch: pytest.MonkeyPatch) -> None:
    """Client extracts JSON content wrapped in MCP list payloads."""

    wrapped_response = [{"text": '{"nodes_added": 1, "edges_added": 1}'}]
    client = SemanticEngineClient(namespace="test")
    monkeypatch.setattr(client, "_call_tool", lambda *_args, **_kwargs: wrapped_response)

    result = client.ingest_triples(
        [{"subject": "Synapse", "predicate": "isA", "object": "MemoryEngine"}],
    )

    assert result["nodes_added"] == 1
    assert result["edges_added"] == 1


def test_get_all_triples_returns_expected_payload(monkeypatch: pytest.MonkeyPatch) -> None:
    """Client returns triples from list_triples response."""

    triples = [
        {"subject": "s1", "predicate": "p1", "object": "o1"},
        {"subject": "s2", "predicate": "p2", "object": "o2"},
    ]
    client = SemanticEngineClient(namespace="test")
    monkeypatch.setattr(client, "_call_tool", lambda *_args, **_kwargs: {"triples": triples})

    result = client.get_all_triples()
    assert result == triples
