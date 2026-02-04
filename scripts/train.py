#!/usr/bin/env python3
"""
Script de entrenamiento del SLM

REQUISITOS DE HARDWARE:
- CPU: Funcional (lento)
- GPU: Recomendado 8GB+ VRAM
- RAM: 16GB mínimo, 32GB recomendado
- Disco: ~5GB para modelo Phi-2

El modelo usa LoRA para reducir memoria/tiempo de entrenamiento.
"""
import os
os.environ['TOKENIZERS_PARALLELISM'] = 'false'  # Evitar warnings

import lightning as L
from lightning.pytorch.callbacks import ModelCheckpoint, EarlyStopping
from lightning.pytorch.loggers import TensorBoardLogger
import torch

# Importar componentes
from agents.infrastructure.ai.trainer import SemanticSystemModule
from agents.infrastructure.ai.datamodule import SemanticDataModule

def main():
    print("="*60)
    print("ENTRENAMIENTO DEL SISTEMA SEMÁNTICO")
    print("="*60)
    
    # Configuración
    config = {
        "model_name": "microsoft/phi-2",  # Modelo pequeño (~2.7B parámetros)
        "ontology_path": "ontology/core.owl",
        "max_epochs": 5,
        "batch_size": 2,  # Pequeño para ahorrar memoria
        "accumulate_grad_batches": 4,  # Simula batch_size=8
    }
    
    # Verificar GPU
    if torch.cuda.is_available():
        print(f"✓ GPU disponible: {torch.cuda.get_device_name(0)}")
        print(f"  VRAM: {torch.cuda.get_device_properties(0).total_memory / 1e9:.1f}GB")
        accelerator = "gpu"
        devices = 1
    else:
        print("⚠ GPU no disponible. Usando CPU (será lento)")
        print("  Recomendación: Reducir max_epochs a 1 o usar Colab/Kaggle")
        accelerator = "cpu"
        devices = 1
    
    # DataModule
    print("\nCargando datos...")
    datamodule = SemanticDataModule(
        train_path="data/train.jsonl",
        val_path="data/val.jsonl",
        batch_size=config["batch_size"]
    )
    
    # Model
    print(f"\nInicializando modelo {config['model_name']}...")
    print("(Primera vez descargará el modelo, puede tardar varios minutos)")
    model = SemanticSystemModule(
        ontology_path=config["ontology_path"],
        model_name=config["model_name"]
    )
    
    # Callbacks
    checkpoint_callback = ModelCheckpoint(
        dirpath="checkpoints",
        filename="semantic-{epoch:02d}-{train_loss:.2f}",
        save_top_k=3,
        monitor="train_loss",
        mode="min"
    )
    
    early_stop_callback = EarlyStopping(
        monitor="train_loss",
        patience=3,
        mode="min"
    )
    
    # Logger
    logger = TensorBoardLogger("logs", name="semantic_system")
    
    # Trainer
    trainer = L.Trainer(
        max_epochs=config["max_epochs"],
        accelerator=accelerator,
        devices=devices,
        callbacks=[checkpoint_callback, early_stop_callback],
        logger=logger,
        accumulate_grad_batches=config["accumulate_grad_batches"],
        gradient_clip_val=1.0,  # Evitar explosión de gradientes
        precision="16-mixed" if accelerator == "gpu" else "32",  # Mixed precision en GPU
        log_every_n_steps=1,
    )
    
    # Entrenar
    print("\n" + "="*60)
    print("INICIANDO ENTRENAMIENTO")
    print("="*60)
    print(f"Épocas: {config['max_epochs']}")
    print(f"Batch size efectivo: {config['batch_size'] * config['accumulate_grad_batches']}")
    print(f"Logs: tensorboard --logdir logs")
    print("="*60 + "\n")
    
    trainer.fit(model, datamodule)
    
    print("\n" + "="*60)
    print("✅ ENTRENAMIENTO COMPLETADO")
    print("="*60)
    print(f"\nCheckpoints guardados en: checkpoints/")
    print(f"Logs en: logs/")
    print(f"\nPara visualizar: tensorboard --logdir logs")

if __name__ == "__main__":
    main()
