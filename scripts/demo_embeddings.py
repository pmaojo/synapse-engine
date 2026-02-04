#!/usr/bin/env python3
"""
Demo: Sistema de Embeddings y Fine-Tuning
Muestra cómo se usan embeddings para búsqueda semántica
y cómo entrenar un modelo personalizado
"""
import sys
import os
sys.path.append(os.getcwd())

from agents.infrastructure.persistence.embeddings import EmbeddingGenerator
from agents.infrastructure.persistence.vector_store import VectorStore
import numpy as np

def demo_embeddings():
    print("="*70)
    print("DEMO: EMBEDDINGS Y BÚSQUEDA SEMÁNTICA")
    print("="*70)
    
    # Inicializar
    print("\n📦 Inicializando componentes...")
    embedder = EmbeddingGenerator()
    vector_store = VectorStore(collection_name="demo_embeddings", dimension=384)
    
    print(f"✓ Modelo: {embedder.model_name}")
    print(f"✓ Dimensión: {embedder.dimension}D")
    print(f"✓ Vector Store: Qdrant (local)\n")
    
    # Datos de ejemplo del dominio agrícola
    concepts = [
        ("compost", "Material orgánico descompuesto que mejora el suelo"),
        ("swale", "Zanja a nivel que captura agua de escorrentía"),
        ("guild", "Grupo de plantas que se benefician mutuamente"),
        ("mulch", "Capa protectora sobre el suelo"),
        ("nitrogen_fixer", "Planta que fija nitrógeno atmosférico"),
    ]
    
    print("🌱 PASO 1: INDEXAR CONCEPTOS DEL DOMINIO")
    print("-"*70)
    
    for concept_id, description in concepts:
        # Generar embedding
        embedding = embedder.encode_single(description)
        
        # Guardar en vector store
        vector_store.add(
            node_id=concept_id,
            vector=embedding,
            metadata={"description": description}
        )
        print(f"  ✓ {concept_id}: {embedding[:3]}... (384D)")
    
    print(f"\n📊 Total indexado: {len(concepts)} conceptos\n")
    
    # Búsqueda semántica
    print("🔍 PASO 2: BÚSQUEDA SEMÁNTICA")
    print("-"*70)
    
    queries = [
        "¿Qué mejora la fertilidad del suelo?",
        "Técnicas para conservar agua",
        "Plantas que enriquecen el suelo"
    ]
    
    for query in queries:
        print(f"\nQuery: '{query}'")
        query_embedding = embedder.encode_single(query)
        results = vector_store.search(query_embedding, top_k=2)
        
        print("  Resultados:")
        for i, result in enumerate(results, 1):
            print(f"    {i}. {result.node_id} (score: {result.score:.3f})")
            print(f"       {result.metadata['description']}")
    
    print("\n" + "="*70)
    print("💡 FINE-TUNING DE EMBEDDINGS")
    print("="*70)
    print("""
Para entrenar embeddings específicos del dominio:

1. PREPARAR DATOS DE ENTRENAMIENTO:
   - Pares positivos: (texto, concepto) relacionados
   - Pares negativos: (texto, concepto) NO relacionados
   
   Ejemplo:
   + ("El compost mejora el suelo", "compost")
   + ("Swales capturan agua", "swale")
   - ("El compost mejora el suelo", "nitrogen_fixer")

2. USAR SENTENCE-TRANSFORMERS:
   ```python
   from sentence_transformers import SentenceTransformer, losses
   from torch.utils.data import DataLoader
   
   model = SentenceTransformer('all-MiniLM-L6-v2')
   train_dataloader = DataLoader(train_data, batch_size=16)
   
   train_loss = losses.CosineSimilarityLoss(model)
   model.fit(
       train_objectives=[(train_dataloader, train_loss)],
       epochs=10
   )
   ```

3. BENEFICIOS:
   - Embeddings alineados con tu ontología
   - Mejor precisión en búsqueda semántica
   - Captura relaciones específicas del dominio

4. MÉTRICAS:
   - Precision@K: % de resultados relevantes en top-K
   - MRR (Mean Reciprocal Rank): Posición del primer relevante
   - nDCG: Calidad del ranking completo
""")
    
    print("="*70)
    print("ARQUITECTURA ACTUAL")
    print("="*70)
    print(f"""
Modelo Base:     {embedder.model_name}
Dimensión:       {embedder.dimension}D
Vector Store:    Qdrant (persistente)
Uso:             RAG, búsqueda semántica, clustering
Fine-Tuning:     Posible con datos del dominio
""")

if __name__ == "__main__":
    demo_embeddings()
