#!/usr/bin/env python3
"""
Script de entrenamiento DEMO del SLM (rápido para pruebas)
Usa GPT-2 pequeño y pocas épocas para demostrar el aprendizaje
"""
import os
os.environ['TOKENIZERS_PARALLELISM'] = 'false'

import lightning as L
from lightning.pytorch.callbacks import ModelCheckpoint
from lightning.pytorch.loggers import TensorBoardLogger
import torch

from agents.infrastructure.ai.trainer import SemanticSystemModule
from agents.infrastructure.ai.datamodule import SemanticDataModule

def main():
    print("="*60)
    print("ENTRENAMIENTO DEMO DEL SLM (GPT-2)")
    print("="*60)
    
    # Configuración DEMO (rápida)
    config = {
        "model_name": "gpt2",  # Modelo pequeño para demo rápida
        "ontology_path": "ontology/core.owl",
        "max_epochs": 3,  # Pocas épocas para demo
        "batch_size": 1,
        "accumulate_grad_batches": 2,
    }
    
    # Verificar GPU
    if torch.cuda.is_available():
        print(f"✓ GPU disponible: {torch.cuda.get_device_name(0)}")
        accelerator = "gpu"
    else:
        print("⚠ Usando CPU")
        accelerator = "cpu"
    
    # DataModule
    print("\nCargando datos distilados...")
    datamodule = SemanticDataModule(
        train_path="data/train_distilled.jsonl",
        val_path="data/train_distilled.jsonl",  # Usar mismo para demo
        batch_size=config["batch_size"]
    )
    
    # Model
    print(f"\nInicializando {config['model_name']}...")
    model = SemanticSystemModule(
        ontology_path=config["ontology_path"],
        model_name=config["model_name"]
    )
    
    # Callbacks
    checkpoint_callback = ModelCheckpoint(
        dirpath="checkpoints",
        filename="gpt2-demo-{epoch:02d}",
        save_top_k=1,
        monitor="train_loss",
        mode="min"
    )
    
    # Logger
    logger = TensorBoardLogger("logs", name="gpt2_demo")
    
    # Trainer
    trainer = L.Trainer(
        max_epochs=config["max_epochs"],
        accelerator=accelerator,
        devices=1,
        callbacks=[checkpoint_callback],
        logger=logger,
        accumulate_grad_batches=config["accumulate_grad_batches"],
        log_every_n_steps=1,
    )
    
    # Entrenar
    print("\n" + "="*60)
    print("INICIANDO ENTRENAMIENTO")
    print("="*60)
    
    trainer.fit(model, datamodule)
    
    print("\n" + "="*60)
    print("✅ ENTRENAMIENTO COMPLETADO")
    print("="*60)
    print(f"\nCheckpoint: {checkpoint_callback.best_model_path}")

if __name__ == "__main__":
    main()
