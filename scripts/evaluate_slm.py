#!/usr/bin/env python3
"""
Evaluar modelo SLM antes y después del entrenamiento
"""
import os
os.environ['TOKENIZERS_PARALLELISM'] = 'false'

import torch
from transformers import AutoTokenizer, AutoModelForCausalLM
from peft import PeftModel

def test_model(model_path=None, text="El compost mejora la estructura del suelo"):
    print(f"\n{'='*70}")
    print(f"Probando: {'MODELO ENTRENADO' if model_path else 'MODELO BASE (sin entrenar)'}")
    print(f"{'='*70}")
    print(f"📝 Input: '{text}'")
    
    # Cargar tokenizer
    tokenizer = AutoTokenizer.from_pretrained("gpt2")
    tokenizer.pad_token = tokenizer.eos_token
    
    # Cargar modelo
    if model_path:
        print(f"📂 Cargando modelo entrenado desde: {model_path}")
        base_model = AutoModelForCausalLM.from_pretrained("gpt2")
        model = PeftModel.from_pretrained(base_model, model_path)
        model = model.merge_and_unload()  # Merge LoRA weights
    else:
        print("📂 Cargando modelo base GPT-2...")
        model = AutoModelForCausalLM.from_pretrained("gpt2")
    
    model.eval()
    
    # Generar
    prompt = f"extract triples: {text}\ntriples:"
    inputs = tokenizer(prompt, return_tensors="pt")
    
    with torch.no_grad():
        outputs = model.generate(
            **inputs,
            max_new_tokens=100,
            num_return_sequences=1,
            temperature=0.7,
            do_sample=True,
            pad_token_id=tokenizer.eos_token_id
        )
    
    result = tokenizer.decode(outputs[0], skip_special_tokens=True)
    
    print(f"\n🤖 Output:\n{result}")
    print(f"{'='*70}\n")
    
    return result

if __name__ == "__main__":
    # Textos de prueba en español
    test_texts = [
        "El compost mejora la estructura del suelo",
        "Los swales capturan agua de lluvia",
        "La rotación de cultivos reduce plagas"
    ]
    
    print("\n" + "="*70)
    print("🧪 EVALUACIÓN DEL MODELO SLM")
    print("="*70)
    
    # Test 1: Modelo BASE
    print("\n\n🔬 TEST 1: MODELO BASE (SIN ENTRENAR)")
    print("="*70)
    base_results = []
    for text in test_texts:
        result = test_model(model_path=None, text=text)
        base_results.append(result)
    
    # Test 2: Modelo ENTRENADO
    print("\n\n🔬 TEST 2: MODELO ENTRENADO")
    print("="*70)
    trained_results = []
    model_path = "checkpoints/gpt2-simple/final"
    
    if os.path.exists(model_path):
        for text in test_texts:
            result = test_model(model_path=model_path, text=text)
            trained_results.append(result)
    else:
        print(f"❌ No se encontró el modelo entrenado en: {model_path}")
        print("Ejecuta primero: python scripts/train_simple.py")
        exit(1)
    
    # Comparación
    print("\n" + "="*70)
    print("📊 COMPARACIÓN DE RESULTADOS")
    print("="*70)
    
    for i, text in enumerate(test_texts):
        print(f"\n{i+1}. Texto: '{text}'")
        print(f"   Base:     {base_results[i][:100]}...")
        print(f"   Entrenado: {trained_results[i][:100]}...")
        changed = base_results[i] != trained_results[i]
        print(f"   ¿Cambió? {'✅ SÍ' if changed else '❌ NO'}")
    
    # Resumen
    changes = sum(1 for i in range(len(test_texts)) if base_results[i] != trained_results[i])
    print(f"\n{'='*70}")
    print(f"📈 RESUMEN")
    print(f"{'='*70}")
    print(f"Ejemplos probados: {len(test_texts)}")
    print(f"Cambios detectados: {changes}/{len(test_texts)}")
    print(f"Tasa de cambio: {changes/len(test_texts)*100:.1f}%")
    
    if changes > 0:
        print(f"\n✅ El modelo SÍ aprendió y cambió su comportamiento!")
    else:
        print(f"\n⚠️  El modelo NO cambió significativamente")
