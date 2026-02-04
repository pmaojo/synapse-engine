#!/usr/bin/env python3
"""
Demo del Sistema Híbrido Neuro-Simbólico
Muestra cómo OWL expande los triples extraídos por el SLM
"""
import sys
import os
sys.path.append(os.getcwd())

from agents.domain.services.ontology import OntologyService
from agents.domain.services.reasoning_service import InferenceEngine

def demo_owl_reasoning():
    print("="*70)
    print("DEMO: SISTEMA HÍBRIDO NEURO-SIMBÓLICO")
    print("="*70)
    print("\n📚 Cargando ontología OWL...")
    
    # Cargar ontología
    ontology = OntologyService(["ontology/core.owl", "ontology/agriculture.owl"])
    inference_engine = InferenceEngine(ontology.graph)
    
    print(f"✓ Ontología cargada: {len(list(ontology.graph.subjects()))} entidades\n")
    
    # Simular triples extraídos por el SLM (como los que generamos)
    print("🤖 PASO 1: SLM EXTRAE TRIPLES DEL TEXTO")
    print("-"*70)
    
    # Usar URIs completas para que el motor las reconozca
    slm_triples = [
        ("http://sys.semantic/agriculture#MyFoodForest", "rdf:type", "http://sys.semantic/agriculture#FoodForest"),
        ("http://sys.semantic/agriculture#MySwale", "rdf:type", "http://sys.semantic/agriculture#Swale"),
    ]
    
    print("Texto: 'Mi bosque de alimentos tiene swales.'\n")
    
    print("Triples extraídos por el SLM:")
    for s, p, o in slm_triples:
        # Mostrar versión corta para legibilidad
        s_short = s.split('#')[-1] if '#' in s else s
        o_short = o.split('#')[-1] if '#' in o else o
        print(f"  • ({s_short}, {p}, {o_short})")
    
    print(f"\n📊 Total SLM: {len(slm_triples)} triples\n")
    
    # Aplicar razonamiento OWL
    print("🧠 PASO 2: MOTOR OWL APLICA RAZONAMIENTO LÓGICO")
    print("-"*70)
    
    expanded_triples = inference_engine.expand_triples(slm_triples)
    
    # Identificar triples inferidos (nuevos)
    original_set = set(slm_triples)
    inferred_triples = [t for t in expanded_triples if t not in original_set]
    
    if inferred_triples:
        print("Nuevos triples inferidos por OWL:")
        for s, p, o in inferred_triples:
            print(f"  ✨ ({s}, {p}, {o})  [INFERIDO]")
    else:
        print("⚠️ No se infirieron triples nuevos.")
        print("   (La ontología actual no tiene reglas de inferencia suficientes)")
    
    print(f"\n📊 Total después de OWL: {len(expanded_triples)} triples")
    print(f"   (+{len(inferred_triples)} inferidos)\n")
    
    # Explicación
    print("="*70)
    print("💡 REGLAS DE INFERENCIA APLICADAS")
    print("="*70)
    print("""
1. HERENCIA DE TIPOS (rdfs:subClassOf):
   (X, rdf:type, C) ∧ (C, rdfs:subClassOf, D) → (X, rdf:type, D)

2. JERARQUÍA DE PROPIEDADES (rdfs:subPropertyOf):
   (X, P, Y) ∧ (P, rdfs:subPropertyOf, Q) → (X, Q, Y)

3. DOMINIOS Y RANGOS (rdfs:domain, rdfs:range):
   (X, P, Y) ∧ (P, rdfs:domain, C) → (X, rdf:type, C)
   (X, P, Y) ∧ (P, rdfs:range, C) → (Y, rdf:type, C)
""")
    
    print("="*70)
    print("ARQUITECTURA HÍBRIDA")
    print("="*70)
    print("""
Componente     | Función                    | Tecnología
---------------|----------------------------|------------------
SLM (Neuronal) | Extracción de texto        | GPT-2 + LoRA
OWL (Simbólico)| Razonamiento lógico        | RDFLib + RDFS
Vector Store   | Búsqueda semántica         | Qdrant
Embeddings     | Representación vectorial   | Sentence-BERT
""")

if __name__ == "__main__":
    demo_owl_reasoning()
