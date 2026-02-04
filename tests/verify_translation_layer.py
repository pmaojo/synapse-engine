import unittest
from unittest.mock import MagicMock, patch
import sys
import os

# Add project root to path
sys.path.append(os.getcwd())

from agents.validation.ontology_validator import OntologyValidator, RewardSignal

class TestTranslationLayer(unittest.TestCase):
    def setUp(self):
        # Mock OntologyService
        self.mock_ontology = MagicMock()
        
        # Mock graph query results to populate valid classes/properties
        def query_side_effect(query):
            if "owl:Class" in query:
                return [[("http://example.org/Compost")], [("http://example.org/Soil")]]
            if "owl:ObjectProperty" in query:
                return [[("http://example.org/improves")]]
            return []
        
        self.mock_ontology.graph.query.side_effect = query_side_effect
        
        # Initialize validator
        self.validator = OntologyValidator(self.mock_ontology)

    @patch('agents.validation.ontology_validator.get_air')
    @patch('litellm.completion')
    def test_spanish_translation(self, mock_completion, mock_get_air):
        # Setup mock LLM response
        mock_response = MagicMock()
        mock_response.choices[0].message.content = '["Compost", "improves", "Soil"]'
        mock_completion.return_value = mock_response
        
        # Test triple: ("El compost", "mejora", "el suelo")
        # Should fail initial validation, trigger translation, and pass re-validation
        result = self.validator.validate_triple("El compost", "mejora", "el suelo")
        
        # Verify results
        self.assertTrue(result["valid"], "Triple should be valid after translation")
        self.assertIn("translated", result["suggestions"], "Should have translated suggestion")
        self.assertEqual(result["suggestions"]["translated"], ("Compost", "improves", "Soil"))
        
        print("\n✅ Translation Layer Verification Successful!")
        print(f"Original: ('El compost', 'mejora', 'el suelo')")
        print(f"Translated: {result['suggestions']['translated']}")

if __name__ == '__main__':
    unittest.main()
