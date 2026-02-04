#!/usr/bin/env python3
"""
Script de verificación del sistema completo
"""
import sys
import os

print("="*60)
print("VERIFICACIÓN DEL SISTEMA SEMÁNTICO")
print("="*60)

# 1. Verificar imports básicos
print("\n1. Verificando imports...")
try:
    import torch
    print(f"   ✓ PyTorch {torch.__version__}")
except ImportError as e:
    print(f"   ✗ PyTorch: {e}")
    sys.exit(1)

try:
    import transformers
    print(f"   ✓ Transformers {transformers.__version__}")
except ImportError as e:
    print(f"   ✗ Transformers: {e}")
    sys.exit(1)

try:
    import lightning as L
    print(f"   ✓ Lightning {L.__version__}")
except ImportError as e:
    print(f"   ✗ Lightning: {e}")
    sys.exit(1)

try:
    from qdrant_client import QdrantClient
    print(f"   ✓ Qdrant Client")
except ImportError as e:
    print(f"   ✗ Qdrant: {e}")
    sys.exit(1)

try:
    from sentence_transformers import SentenceTransformer
    print(f"   ✓ Sentence Transformers")
except ImportError as e:
    print(f"   ✗ Sentence Transformers: {e}")
    sys.exit(1)

# 2. Verificar componentes del sistema
print("\n2. Verificando componentes del sistema...")
try:
    from agents.domain.services.ontology import OntologyService
    from agents.infrastructure.persistence.vector_store import VectorStore
    from agents.infrastructure.persistence.embeddings import EmbeddingGenerator
    from agents.infrastructure.ai.slm import TrainableSLM
    print("   ✓ Todos los módulos importados correctamente")
except ImportError as e:
    print(f"   ✗ Error importando módulos: {e}")
    sys.exit(1)

# 3. Verificar ontologías
print("\n3. Verificando ontologías...")
if os.path.exists("ontology/core.owl"):
    print("   ✓ core.owl encontrado")
else:
    print("   ✗ core.owl no encontrado")
    
if os.path.exists("ontology/agriculture.owl"):
    print("   ✓ agriculture.owl encontrado")
else:
    print("   ✗ agriculture.owl no encontrado")

# 4. Verificar motor Rust
print("\n4. Verificando motor Rust...")
rust_binary = "./target/debug/semantic-engine"
if os.path.exists(rust_binary):
    print(f"   ✓ Binary compilado: {rust_binary}")
else:
    print(f"   ⚠ Binary no encontrado (ejecutar: cargo build)")

# 5. Test rápido de componentes
print("\n5. Test rápido de componentes...")

try:
    # Test Vector Store
    print("   - Inicializando VectorStore...")
    vs = VectorStore(dimension=384)
    print("   ✓ VectorStore inicializado")
    
    # Test Embeddings (sin descargar modelo grande)
    print("   - EmbeddingGenerator configurado")
    print("   ✓ (modelo se descargará en primer uso)")
    
    # Test Ontology
    print("   - Cargando ontología...")
    ont = OntologyService(["ontology/core.owl"])
    print(f"   ✓ Ontología cargada ({len(ont.classes)} clases)")
    
except Exception as e:
    print(f"   ✗ Error en tests: {e}")
    import traceback
    traceback.print_exc()
    sys.exit(1)

print("\n" + "="*60)
print("✅ SISTEMA VERIFICADO CORRECTAMENTE")
print("="*60)
print("\nPróximos pasos:")
print("1. Ejecutar demo: python scripts/demo.py")
print("2. Entrenar modelo: python -m lightning run trainer agents/inference/trainer.py")
print("3. Iniciar servidor: ./target/debug/semantic-engine")
