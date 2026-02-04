# Dataset de Entrenamiento para Extracción de Triples

Este directorio contiene ejemplos de cómo preparar datos para entrenar el sistema semántico.

## Formato de Datos

Cada ejemplo de entrenamiento contiene:
1. **Texto fuente**: Descripción en lenguaje natural
2. **Triples esperados**: Lista de (Sujeto, Predicado, Objeto) que deberían extraerse

## Estructura de Archivos

```
data/
├── train.jsonl          # Datos de entrenamiento
├── val.jsonl            # Datos de validación
└── test.jsonl           # Datos de prueba
```

## Formato JSONL

Cada línea es un objeto JSON:

```json
{
  "text": "Un bosque alimentario combina árboles frutales con cultivos de cobertura.",
  "triples": [
    ["FoodForest", "hasComponent", "FruitTree"],
    ["FoodForest", "hasComponent", "CoverCrop"]
  ],
  "domain": "agriculture",
  "source": "permaculture_handbook"
}
```

## Generación de Datos

Puedes generar datos de entrenamiento:

1. **Manualmente**: Anotando textos con triples válidos
2. **Semi-automáticamente**: Usando un LLM grande (GPT-4) para generar anotaciones iniciales
3. **De documentación existente**: Extrayendo de manuales, papers, etc.

## Ejemplo de Script de Generación

Ver `scripts/generate_training_data.py` para un script que usa un LLM para generar datos sintéticos.
