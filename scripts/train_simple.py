#!/usr/bin/env python3
"""
Entrenamiento simple del SLM sin Lightning
Usa solo PyTorch + Transformers + PEFT (LoRA)
"""
import os
os.environ['TOKENIZERS_PARALLELISM'] = 'false'

import json
import torch
from transformers import (
    AutoTokenizer,
    AutoModelForCausalLM,
    TrainingArguments,
    Trainer,
    DataCollatorForLanguageModeling
)
from peft import LoraConfig, get_peft_model, TaskType
from datasets import Dataset

print("="*60)
print("ENTRENAMIENTO SIMPLE DEL SLM (GPT-2 + LoRA)")
print("="*60)

# Configuración
MODEL_NAME = "gpt2"
TRAIN_FILE = "data/train_synthetic.jsonl"
OUTPUT_DIR = "checkpoints/gpt2-simple"
MAX_EPOCHS = 3
BATCH_SIZE = 2

# Cargar datos
print(f"\n📂 Cargando datos de {TRAIN_FILE}...")
with open(TRAIN_FILE, 'r') as f:
    data = [json.loads(line) for line in f if line.strip()]

print(f"✓ Cargados {len(data)} ejemplos")

# Preparar prompts
def format_example(example):
    text = example['text']
    triples = example.get('triples', [])
    triples_str = str(triples)
    return f"extract triples: {text}\ntriples: {triples_str}"

texts = [format_example(ex) for ex in data]
dataset = Dataset.from_dict({"text": texts})

# Tokenizer
print(f"\n🔤 Cargando tokenizer {MODEL_NAME}...")
tokenizer = AutoTokenizer.from_pretrained(MODEL_NAME)
tokenizer.pad_token = tokenizer.eos_token

# Tokenizar
def tokenize_function(examples):
    return tokenizer(
        examples["text"],
        truncation=True,
        max_length=256,
        padding="max_length"
    )

tokenized_dataset = dataset.map(tokenize_function, batched=True, remove_columns=["text"])

# Modelo base
print(f"\n🤖 Cargando modelo {MODEL_NAME}...")
model = AutoModelForCausalLM.from_pretrained(MODEL_NAME)

# Configurar LoRA
print("\n⚡ Aplicando LoRA...")
lora_config = LoraConfig(
    task_type=TaskType.CAUSAL_LM,
    r=8,  # Rank
    lora_alpha=32,
    lora_dropout=0.1,
    target_modules=["c_attn"]  # GPT-2 attention modules
)

model = get_peft_model(model, lora_config)
model.print_trainable_parameters()

# Training arguments
training_args = TrainingArguments(
    output_dir=OUTPUT_DIR,
    num_train_epochs=MAX_EPOCHS,
    per_device_train_batch_size=BATCH_SIZE,
    save_steps=50,
    save_total_limit=2,
    logging_steps=10,
    learning_rate=2e-4,
    warmup_steps=10,
    fp16=False,  # Usar CPU
    report_to="none"
)

# Data collator
data_collator = DataCollatorForLanguageModeling(
    tokenizer=tokenizer,
    mlm=False
)

# Trainer
trainer = Trainer(
    model=model,
    args=training_args,
    train_dataset=tokenized_dataset,
    data_collator=data_collator,
)

# Entrenar
print("\n" + "="*60)
print("🚀 INICIANDO ENTRENAMIENTO")
print("="*60)

trainer.train()

# Guardar
print("\n💾 Guardando modelo...")
model.save_pretrained(f"{OUTPUT_DIR}/final")
tokenizer.save_pretrained(f"{OUTPUT_DIR}/final")

print("\n" + "="*60)
print("✅ ENTRENAMIENTO COMPLETADO")
print("="*60)
print(f"\nModelo guardado en: {OUTPUT_DIR}/final")
