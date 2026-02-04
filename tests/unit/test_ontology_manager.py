import os
import json
import pytest
from agents.domain.services.ontology import OntologyService

TEST_REGISTRY = "test_ontology_registry.json"

@pytest.fixture
def clean_registry():
    if os.path.exists(TEST_REGISTRY):
        os.remove(TEST_REGISTRY)
    yield
    if os.path.exists(TEST_REGISTRY):
        os.remove(TEST_REGISTRY)

def test_ontology_persistence(clean_registry):
    # 1. Initialize empty
    service = OntologyService([], persistence_file=TEST_REGISTRY)
    assert len(service.sources) == 0

    # 2. Add a source
    service.add_ontology_source("ontology/core.owl", "file")
    assert len(service.sources) == 1

    # 3. Verify persistence
    with open(TEST_REGISTRY, 'r') as f:
        data = json.load(f)
        assert len(data) == 1
        assert data[0]['path'] == "ontology/core.owl"

    # 4. Re-initialize and verify load
    service2 = OntologyService([], persistence_file=TEST_REGISTRY)
    assert len(service2.sources) == 1
    assert service2.sources[0].path == "ontology/core.owl"

def test_add_remove_source(clean_registry):
    service = OntologyService([], persistence_file=TEST_REGISTRY)

    service.add_ontology_source("ontology/core.owl", "file")
    assert len(service.sources) == 1

    service.remove_ontology_source("ontology/core.owl")
    assert len(service.sources) == 0

    # Check persistence cleared
    with open(TEST_REGISTRY, 'r') as f:
        data = json.load(f)
        assert len(data) == 0

def test_url_source_handling(clean_registry):
    # This just tests the logic, not the actual network call (mocked or assumed offline for unit test)
    service = OntologyService([], persistence_file=TEST_REGISTRY)

    url = "http://example.org/ontology.owl"
    service.add_ontology_source(url, "url")

    assert len(service.sources) == 1
    assert service.sources[0].type == "url"
