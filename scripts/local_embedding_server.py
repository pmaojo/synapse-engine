import os
import uvicorn
from fastapi import FastAPI, HTTPException
from pydantic import BaseModel
from typing import List, Union
from sentence_transformers import SentenceTransformer

app = FastAPI()

# Cache models in memory
models = {}

class EmbeddingRequest(BaseModel):
    inputs: Union[str, List[str]]

@app.post("/{model_id:path}/pipeline/feature-extraction")
async def feature_extraction(model_id: str, request: EmbeddingRequest):
    """
    Mimic HuggingFace Inference API for feature extraction.
    """
    global models

    # Handle model loading
    if model_id not in models:
        try:
            print(f"Loading model: {model_id}")
            # This will download the model to the local cache if not present
            models[model_id] = SentenceTransformer(model_id)
        except Exception as e:
            raise HTTPException(status_code=500, detail=f"Failed to load model {model_id}: {str(e)}")

    model = models[model_id]

    # Process inputs
    sentences = request.inputs
    single_input = False
    if isinstance(sentences, str):
        sentences = [sentences]
        single_input = True

    try:
        embeddings = model.encode(sentences)

        # Return list of lists (vectors)
        # Note: If single input was provided, we still return a list of vectors
        # to be consistent with batch processing, OR we could return a single vector
        # if that's what HF does. But VectorStore handles both.
        # However, VectorStore sends `vec![text]` (a list) even for single text in `embed()`.
        # So we should return a list of lists.
        return embeddings.tolist()
    except Exception as e:
         raise HTTPException(status_code=500, detail=f"Embedding failed: {str(e)}")

@app.get("/health")
async def health():
    return {"status": "ok"}

if __name__ == "__main__":
    port = int(os.getenv("PORT", 8000))
    print(f"Starting local embedding server on port {port}...")
    uvicorn.run(app, host="0.0.0.0", port=port)
