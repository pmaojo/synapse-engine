# Guía de Entrenamiento del SLM

## ¿Podrá ejecutarse el modelo?

### Requisitos Mínimos
- **CPU**: Cualquier CPU moderna (pero será LENTO)
- **RAM**: 16GB mínimo
- **Disco**: 5GB libres
- **Tiempo**: ~30min/época en CPU, ~5min/época en GPU

### Requisitos Recomendados
- **GPU**: NVIDIA con 8GB+ VRAM (RTX 3060, T4, V100, etc.)
- **RAM**: 32GB
- **Disco**: 10GB libres
- **Tiempo**: ~2-3min/época

### Optimizaciones Implementadas

1. **LoRA (Low-Rank Adaptation)**:
   - Solo entrena ~0.1% de los parámetros del modelo
   - Reduce memoria de 16GB → ~4GB
   - Reduce tiempo de entrenamiento 10x

2. **Mixed Precision (GPU)**:
   - Usa float16 en lugar de float32
   - Ahorra 50% de memoria
   - 2x más rápido

3. **Gradient Accumulation**:
   - Simula batches grandes sin usar más memoria
   - batch_size=2, accumulate=4 → efectivo: 8

4. **Modelo Pequeño**:
   - Phi-2: 2.7B parámetros (vs Llama-13B o GPT-3.5)
   - Cabe en GPUs consumer

## Cómo Entrenar

### Paso 1: Preparar Datos
```bash
# Ya tienes 8 ejemplos en data/train.jsonl
# Para mejores resultados, añade más (100+ recomendado)
```

### Paso 2: Ejecutar Entrenamiento
```bash
python scripts/train.py
```

### Paso 3: Monitorear
```bash
# En otra terminal
tensorboard --logdir logs
# Abre http://localhost:6006
```

### Paso 4: Usar Modelo Entrenado
```python
from agents.inference.slm import TrainableSLM

# Cargar checkpoint
slm = TrainableSLM.load_from_checkpoint("checkpoints/semantic-epoch=04.ckpt")

# Usar para extracción
text = "La agroforestería combina árboles con cultivos"
result = slm.generate(f"Extract triples from: {text}\\nTriples:")
print(result)
```

## Alternativas si No Tienes GPU

### Opción 1: Google Colab (GRATIS)
1. Subir el proyecto a Google Drive
2. Abrir notebook en Colab con GPU T4 gratis
3. Entrenar allí

### Opción 2: Reducir Dataset
- Usa solo 10-20 ejemplos
- Reduce epochs a 1-2
- Funcionará pero con peor calidad

### Opción 3: Usar Modelo Más Pequeño
En `scripts/train.py` cambiar:
```python
"model_name": "microsoft/phi-1_5",  # 1.3B parámetros (mitad de Phi-2)
```

## Métricas Esperadas

Con 8 ejemplos de entrenamiento:
- **Epoch 1**: loss ~2.5, reward ~0.3
- **Epoch 3**: loss ~1.2, reward ~0.6
- **Epoch 5**: loss ~0.8, reward ~0.8

Con 100+ ejemplos:
- **Epoch 1**: loss ~3.0, reward ~0.2
- **Epoch 5**: loss ~0.5, reward ~0.9
- **Epoch 10**: loss ~0.3, reward ~0.95

## Troubleshooting

### Error: CUDA out of memory
```python
# Solución: Reducir batch_size
"batch_size": 1,  # En lugar de 2
```

### Error: Model download fails
```bash
# Solución: Descargar manualmente
huggingface-cli login
huggingface-cli download microsoft/phi-2
```

### Warning: Slow training
- Normal en CPU
- Considera usar Colab o reducir epochs
