import lightning as L
import torch
from torch import nn, optim
from .slm import TrainableSLM
import os

class SemanticSystemModule(L.LightningModule):
    """
    Lightning Module for training the semantic system synapse.
    Optimizes the SLM and agent policies based on reward signals.
    """
    def __init__(self, ontology_path: str = "ontology/core.owl", model_name: str = "microsoft/phi-2"):
        super().__init__()
        self.save_hyperparameters()
        
        # Initialize SLM
        self.slm = TrainableSLM(model_name=model_name)
        
        # Initialize Agents
        self.ontology = OntologyService([ontology_path])
        self.extractor = ExtractorAgent()  # Uses mock/rules for now
        self.mapper = OntologyMapperAgent(self.ontology)
        self.validator = TripleValidatorAgent(self.ontology)
        
    def forward(self, text: str):
        """Full inference pass"""
        # 1. Extract
        extraction_input = AgentInput(text=text)
        extraction_result = self.extractor.forward(extraction_input)
        
        # 2. Map
        map_input = AgentInput(text=text, context={'triples': extraction_result.triples})
        mapping_result = self.mapper.forward(map_input)
        
        # 3. Validate
        valid_input = AgentInput(text=text, context={'triples': mapping_result.triples})
        validation_result = self.validator.forward(valid_input)
        
        return validation_result

    def training_step(self, batch, batch_idx):
        """
        Training step:
        1. Run forward pass
        2. Calculate reward based on valid triples
        3. Compute loss (negative reward + language modeling loss)
        """
        # Batch is a list of dicts from collate_fn
        total_loss = 0.0
        total_reward = 0.0
        
        for item in batch:
            text = item["text"]
            target_triples = item.get("triples", [])
            
            # Forward pass through agents
            result = self.forward(text)
            
            # Calculate Reward
            reward = self.calculate_reward(result.triples, target_triples)
            total_reward += reward
            
            # Language Modeling Loss (Self-Supervised)
            # We want the SLM to be good at generating the triples textually
            prompt = f"Extract triples from: {text}\nTriples:"
            target_text = self._format_triples_from_list(target_triples) if target_triples else self._format_triples(result.triples)
            
            inputs = self.slm.tokenizer(
                prompt + " " + target_text, 
                return_tensors="pt", 
                truncation=True, 
                padding="max_length",
                max_length=128
            ).to(self.device)
            
            # Standard Causal LM loss
            outputs = self.slm(
                input_ids=inputs.input_ids,
                attention_mask=inputs.attention_mask,
                labels=inputs.input_ids # AutoModelForCausalLM calculates loss automatically if labels provided
            )
            lm_loss = outputs.loss
            total_loss += lm_loss
        
        # Average over batch
        avg_loss = total_loss / len(batch)
        avg_reward = total_reward / len(batch)
        
        self.log("train_loss", avg_loss)
        self.log("train_reward", avg_reward)
        
        return avg_loss


    def calculate_reward(self, predicted_triples, target_triples):
        """Calculate reward based on validity and match with targets"""
        if not predicted_triples:
            return 0.0
            
        # 1. Validity Reward
        valid_count = len(predicted_triples)
        
        # 2. Target Match Reward (if targets provided)
        match_score = 0.0
        if target_triples:
            pred_set = set((t.subject, t.predicate, t.object) for t in predicted_triples)
            target_set = set(tuple(t) for t in target_triples)
            intersection = pred_set.intersection(target_set)
            match_score = len(intersection) / len(target_set) if target_set else 0
            
        return (valid_count * 0.1) + (match_score * 1.0)

    def _format_triples(self, triples):
        """Format triples as text for the SLM"""
        if isinstance(triples, list) and len(triples) > 0 and hasattr(triples[0], 'subject'):
            return "\n".join([f"({t.subject}, {t.predicate}, {t.object})" for t in triples])
        return str(triples)

    def _format_triples_from_list(self, triples):
        """Format list of [s, p, o] as text for the SLM"""
        return "\n".join([f"({t[0]}, {t[1]}, {t[2]})" for t in triples])

    def configure_optimizers(self):
        return optim.AdamW(self.parameters(), lr=2e-5)
