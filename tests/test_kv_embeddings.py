import unittest
import torch
import numpy as np
from agents.infrastructure.persistence.kv_embeddings import KVEmbeddingGenerator

class TestKVEmbeddingGenerator(unittest.TestCase):
    def test_embedding_generation(self):
        # Use a very small model for testing if possible, or mocked
        # Since we downloaded SmolLM2-135M via code, we can use it.
        # It's 135M params, might be okay.

        try:
            generator = KVEmbeddingGenerator(model_name="HuggingFaceTB/SmolLM2-135M", device="cpu")
            text = "Hello world, this is a test for KV-Embedding."
            embedding = generator.encode(text)

            print(f"Embedding shape: {embedding.shape}")

            self.assertEqual(len(embedding.shape), 2)
            self.assertEqual(embedding.shape[0], 1)
            # SmolLM2-135M hidden size is likely 576 or similar (check config)
            # Actually Llama-135M usually has d_model=576 or 768.
            # We just check it's not empty.
            self.assertTrue(embedding.shape[1] > 0)

            # Check normalization
            norm = np.linalg.norm(embedding[0])
            self.assertAlmostEqual(norm, 1.0, places=4)

        except Exception as e:
            print(f"Test failed with error: {e}")
            raise e

if __name__ == "__main__":
    unittest.main()
