#!/usr/bin/env python3
"""
Genera datos de entrenamiento sintéticos usando la ontología
y patrones predefinidos para bootstrapping inicial
"""
import json
import random
from pathlib import Path

# Plantillas de texto basadas en la ontología agriculture.owl
TEMPLATES = {
    "isA": [
        "Un {subject} es un {object}",
        "{subject} es un tipo de {object}",
        "{subject} se clasifica como {object}",
    ],
    "hasComponent": [
        "Un {subject} incluye {object}",
        "{subject} tiene {object} como componente",
        "{subject} contiene {object}",
        "{subject} está compuesto por {object}",
    ],
    "improves": [
        "{subject} mejora {object}",
        "{subject} incrementa la calidad de {object}",
        "{subject} optimiza {object}",
    ],
    "produces": [
        "{subject} produce {object}",
        "{subject} genera {object}",
        "{subject} crea {object}",
    ],
    "protects": [
        "{subject} protege {object}",
        "{subject} salvaguarda {object}",
        "{subject} defiende {object}",
    ],
}

# Conceptos de la ontología
CONCEPTS = {
    "FoodForest": ["bosque alimentario", "bosque comestible", "forest garden"],
    "PermacultureSystem": ["sistema de permacultura", "diseño permacultural"],
    "Swales": ["swales", "zanjas de nivel", "canales de infiltración"],
    "WaterManagement": ["gestión del agua", "manejo hídrico"],
    "RegenerativeAgriculture": ["agricultura regenerativa", "farming regenerativo"],
    "SoilHealth": ["salud del suelo", "fertilidad del suelo"],
    "CoverCrop": ["cultivo de cobertura", "abono verde"],
    "Composting": ["compostaje", "compost"],
    "AgroforestrySystems": ["sistemas agroforestales", "agroforestería"],
}

# Triples conocidos de la ontología
KNOWN_TRIPLES = [
    ("FoodForest", "isA", "PermacultureSystem"),
    ("FoodForest", "hasComponent", "FruitTree"),
    ("Swales", "isA", "WaterManagementStructure"),
    ("RegenerativeAgriculture", "improves", "SoilHealth"),
    ("Composting", "produces", "SoilNutrients"),
    ("CoverCrop", "protects", "Soil"),
    ("AgroforestrySystems", "integrates", "Trees"),
]

def generate_synthetic_examples(num_examples=100):
    """Genera ejemplos sintéticos variando templates y conceptos"""
    examples = []
    
    for _ in range(num_examples):
        # Elegir un triple
        subject, predicate, obj = random.choice(KNOWN_TRIPLES)
        
        # Elegir template si existe
        if predicate in TEMPLATES:
            template = random.choice(TEMPLATES[predicate])
            
            # Elegir variación del concepto si existe
            subject_text = random.choice(CONCEPTS.get(subject, [subject]))
            obj_text = random.choice(CONCEPTS.get(obj, [obj]))
            
            # Generar texto
            text = template.format(subject=subject_text, object=obj_text)
            
            # Añadir variaciones
            if random.random() > 0.5:
                text = text.capitalize()
            if random.random() > 0.7:
                text = text + "."
            
            examples.append({
                "text": text,
                "triples": [[subject, predicate, obj]],
                "domain": "agriculture",
                "source": "synthetic_template"
            })
    
    return examples

def main():
    print("Generando datos sintéticos de entrenamiento...")
    
    # Generar ejemplos
    train_examples = generate_synthetic_examples(100)
    val_examples = generate_synthetic_examples(20)
    
    # Guardar
    output_dir = Path("data")
    output_dir.mkdir(exist_ok=True)
    
    with open(output_dir / "train_synthetic.jsonl", "w", encoding="utf-8") as f:
        for ex in train_examples:
            f.write(json.dumps(ex, ensure_ascii=False) + "\n")
    
    with open(output_dir / "val_synthetic.jsonl", "w", encoding="utf-8") as f:
        for ex in val_examples:
            f.write(json.dumps(ex, ensure_ascii=False) + "\n")
    
    print(f"✓ Generados {len(train_examples)} ejemplos de entrenamiento")
    print(f"✓ Generados {len(val_examples)} ejemplos de validación")
    print(f"\nArchivos creados:")
    print(f"  - data/train_synthetic.jsonl")
    print(f"  - data/val_synthetic.jsonl")
    print(f"\nPara usar: python scripts/train.py --train data/train_synthetic.jsonl")

if __name__ == "__main__":
    main()
