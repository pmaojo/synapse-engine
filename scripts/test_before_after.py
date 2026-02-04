import sys
import os
import torch

sys.path.append(os.getcwd())

from agents.infrastructure.ai.slm import TrainableSLM

def test_model(model_path=None, text="Los swales capturan agua y previenen la erosión."):
    print(f"\n{'='*60}")
    print(f"Probando Modelo: {'ENTRENADO' if model_path else 'BASE (Sin Entrenar)'}")
    print(f"{'='*60}")
    print(f"Input: '{text}'")
    
    model_name = "gpt2"
    
    if model_path:
        print(f"\nCargando checkpoint: {model_path}")
        from agents.infrastructure.ai.trainer import SemanticSystemModule
        model_wrapper = SemanticSystemModule.load_from_checkpoint(model_path)
        model_wrapper.eval()
        model = model_wrapper.slm
    else:
        print("\nUsando modelo base...")
        model = TrainableSLM(model_name=model_name, use_lora=False)
        model.eval()
        
    # Inferencia
    prompt = f"extract triples: {text}"
    output = model.generate(prompt, max_new_tokens=50)
    
    print(f"\nOutput:\n{output}")
    print(f"{'='*60}\n")
    return output

if __name__ == "__main__":
    # Test 1: Modelo BASE (sin entrenar)
    print("\n🔬 TEST 1: MODELO BASE")
    base_output = test_model(text="El compost mejora la estructura del suelo.")
    
    # Test 2: Modelo ENTRENADO
    print("\n🔬 TEST 2: MODELO ENTRENADO")
    checkpoint_path = "checkpoints/gpt2-demo-epoch=01.ckpt"
    trained_output = test_model(
        model_path=checkpoint_path,
        text="El compost mejora la estructura del suelo."
    )
    
    # Comparación
    print("\n" + "="*60)
    print("📊 COMPARACIÓN")
    print("="*60)
    print(f"Base Output Length: {len(base_output)}")
    print(f"Trained Output Length: {len(trained_output)}")
    print(f"\n¿Cambió el comportamiento? {'✅ SÍ' if base_output != trained_output else '❌ NO'}")
