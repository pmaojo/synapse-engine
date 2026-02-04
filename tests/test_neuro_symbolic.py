import asyncio
import os
import sys
from unittest.mock import MagicMock

# Mock torch before imports
sys.modules['torch'] = MagicMock()
sys.modules['agents.infrastructure.ai.slm'] = MagicMock()

from agents.tools.nl2cypher import NL2CypherAgent
from agents.validation.ontology_validator import OntologyValidator
from agents.domain.services.ontology import OntologyService
from rdflib import Graph

def test_ontology_validator_numeric():
    print("Testing OntologyValidator numeric constraints...")
    # Mock ontology
    g = Graph()
    g.parse("ontology/core.owl")
    # Add dummy range constraint for testing
    # hasHeight range xsd:float

    # Create dummy ontology file if it doesn't exist
    if not os.path.exists("ontology"):
        os.makedirs("ontology")
    if not os.path.exists("ontology/core.owl"):
        with open("ontology/core.owl", "w") as f:
            f.write("""
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:rdfs="http://www.w3.org/2000/01/rdf-schema#"
         xmlns:owl="http://www.w3.org/2002/07/owl#">
  <owl:Class rdf:about="#Plant"/>
  <owl:DatatypeProperty rdf:about="#hasHeight">
    <rdfs:range rdf:resource="http://www.w3.org/2001/XMLSchema#float"/>
  </owl:DatatypeProperty>
</rdf:RDF>
""")

    svc = OntologyService(["ontology/core.owl"])
    validator = OntologyValidator(svc)

    # Inject a test range
    validator.ranges["hasHeight"] = "float"

    # Test valid
    res1 = validator.validate_triple("Plant", "hasHeight", "5.5")
    print(f"Valid case: {res1['valid']} - Errors: {res1.get('errors')}")
    # Force add for testing if not loaded from dummy owl correctly
    if not res1['valid'] and "Predicate 'hasHeight' not in ontology" in res1['errors']:
        print("DEBUG: Force adding hasHeight to valid properties")
        validator.valid_properties.add("hasHeight")
        res1 = validator.validate_triple("Plant", "hasHeight", "5.5")

    print(f"Valid case retry: {res1['valid']} - Errors: {res1.get('errors')}")
    assert res1['valid'] == True

    # Test invalid
    res2 = validator.validate_triple("Plant", "hasHeight", "tall")
    print(f"Invalid case: {res2['valid']} - Errors: {res2['errors']}")
    assert res2['valid'] == False
    assert "should be numeric" in res2['errors'][0] or "must be a number" in res2['errors'][0]

async def test_nl2cypher_correction():
    print("\nTesting NL2Cypher schema correction...")
    agent = NL2CypherAgent()

    # Simulate a query with a typo in predicate
    # Assuming 'hasHeight' is valid, but 'hasHight' is not
    # We need to ensure validator has 'hasHeight'
    if "hasHeight" not in agent.validator.valid_properties:
        agent.validator.valid_properties.add("hasHeight")

    bad_cypher = "MATCH (n)-[:hasHight]->(m) RETURN n"
    print(f"Original: {bad_cypher}")

    fixed = agent._verify_and_fix_schema(bad_cypher)
    print(f"Fixed:    {fixed}")

    assert "hasHeight" in fixed
    assert "hasHight" not in fixed

    # Test Node Label Correction
    print("\nTesting Node Label correction...")
    # Add 'Plant' to valid classes (should be there from ontology load, but ensuring)
    if "Plant" not in agent.validator.valid_classes:
        agent.validator.valid_classes.add("Plant")

    bad_label_cypher = "MATCH (n:Plnt) RETURN n"
    print(f"Original: {bad_label_cypher}")
    fixed_label = agent._verify_and_fix_schema(bad_label_cypher)
    print(f"Fixed:    {fixed_label}")

    assert "Plant" in fixed_label
    assert "Plnt" not in fixed_label

if __name__ == "__main__":
    test_ontology_validator_numeric()
    asyncio.run(test_nl2cypher_correction())
    print("\n✅ All Neuro-Symbolic verification tests passed!")
