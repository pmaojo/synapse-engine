"""
TDD Test Suite for SLM Training Pipeline
"""
import pytest
import torch
import sys
import os

# Add project root to path
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))

from agents.infrastructure.ai.slm import TrainableSLM


class TestSLMInference:
    """Test suite for SLM inference capabilities"""
    
    @pytest.fixture
    def model(self):
        """Fixture: Create a small test model"""
        return TrainableSLM(model_name="gpt2", use_lora=False)
    
    def test_model_initialization(self, model):
        """Test 1: Model should initialize without errors"""
        assert model is not None
        assert model.model is not None
        assert model.tokenizer is not None
        
    def test_model_has_pad_token(self, model):
        """Test 2: Model should have a pad token configured"""
        assert model.tokenizer.pad_token is not None
        
    def test_model_can_generate(self, model):
        """Test 3: Model should generate text from prompt"""
        prompt = "extract triples: Los swales capturan agua."
        output = model.generate(prompt, max_new_tokens=32)
        
        assert output is not None
        assert isinstance(output, str)
        assert len(output) > 0
        
    def test_model_output_differs_from_input(self, model):
        """Test 4: Generated output should extend the input"""
        prompt = "extract triples: El compost mejora el suelo."
        output = model.generate(prompt, max_new_tokens=32)
        
        # Output should contain the prompt + generated text
        assert len(output) >= len(prompt)
        
    def test_forward_pass(self, model):
        """Test 5: Model should handle forward pass for training"""
        inputs = model.tokenizer(
            "test input",
            return_tensors="pt",
            padding=True,
            truncation=True
        )
        
        # Forward pass should work
        outputs = model(
            input_ids=inputs['input_ids'],
            attention_mask=inputs['attention_mask'],
            labels=inputs['input_ids']
        )
        
        assert outputs is not None
        assert hasattr(outputs, 'loss')


class TestSLMTrainingData:
    """Test suite for training data format"""
    
    def test_training_data_exists(self):
        """Test 6: Training data file should exist"""
        data_path = "data/train_distilled.jsonl"
        assert os.path.exists(data_path), f"Training data not found at {data_path}"
        
    def test_training_data_format(self):
        """Test 7: Training data should be valid JSONL"""
        import json
        data_path = "data/train_distilled.jsonl"
        
        with open(data_path, 'r') as f:
            for line in f:
                data = json.loads(line)
                assert 'text' in data
                assert 'triples' in data
                assert isinstance(data['triples'], list)


if __name__ == "__main__":
    # Run tests
    pytest.main([__file__, "-v", "--tb=short"])
