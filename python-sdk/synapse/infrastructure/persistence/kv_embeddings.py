import torch
import torch.nn as nn
from transformers import AutoModelForCausalLM, AutoTokenizer
from typing import List, Union, Optional
import copy
import types
import functools

class KVEmbeddingGenerator:
    """
    Generates embeddings using the KV-Embedding method (Training-Free).
    """

    def __init__(
        self,
        model_name: str = "HuggingFaceTB/SmolLM2-135M",
        device: str = None,
        target_layers: Optional[List[int]] = None
    ):
        self.model_name = model_name
        if device is None:
            self.device = "cuda" if torch.cuda.is_available() else "cpu"
        else:
            self.device = device

        print(f"Loading model {model_name} on {self.device}...")
        self.tokenizer = AutoTokenizer.from_pretrained(model_name)
        if self.tokenizer.pad_token is None:
            self.tokenizer.pad_token = self.tokenizer.eos_token

        self.model = AutoModelForCausalLM.from_pretrained(
            model_name,
            dtype=torch.float32,
            trust_remote_code=True
        ).to(self.device)

        # Determine hidden size for vector store compatibility check
        self.hidden_size = self.model.config.hidden_size
        print(f"Model hidden size: {self.hidden_size}")

        # Default target layers if not provided
        # Heuristic: middle layers
        num_layers = len(self.model.model.layers)
        if target_layers is None:
            start = num_layers // 3
            end = (2 * num_layers) // 3
            self.target_layers = list(range(start, end))
        else:
            self.target_layers = target_layers

        print(f"Target layers for KV-Embedding: {self.target_layers}")

    def _get_modified_forward(self, original_forward, layer_idx):
        """
        Creates a modified forward method for the attention module.
        """
        @functools.wraps(original_forward)
        def modified_forward(module_self, hidden_states, attention_mask=None, position_ids=None, past_key_value=None, output_attentions=False, use_cache=False, **kwargs):
            # NOTE: The arguments passed to forward depend on the transformers version and model.
            # LlamaAttention in recent versions (4.57+) takes:
            # (hidden_states, position_embeddings, attention_mask, past_key_values, ...)
            # We need to handle arguments carefully or rely on kwargs.

            # The signature we inspected was:
            # forward(self, hidden_states, position_embeddings, attention_mask, past_key_values, cache_position, **kwargs)

            # However, we are wrapping it. If we use *args, **kwargs we catch them all.
            # But we need access to hidden_states and specific ones.
            # Let's inspect signature inside.

            # But wait, we define the signature explicitly above in my code.
            # `position_ids` might not be in the signature in new versions, it's `position_embeddings` now.
            # If I declare specific args that don't match what caller sends, I might lose data or get errors if they are positional.

            # Let's be generic
            return self._modified_forward_impl(module_self, original_forward, hidden_states, attention_mask, position_ids, past_key_value, output_attentions, use_cache, **kwargs)

        # Better approach: Just implement it with flexible signature
        def implementation(module_self, *args, **kwargs):
            # Extract arguments
            # We assume hidden_states is first arg
            hidden_states = args[0] if len(args) > 0 else kwargs.get("hidden_states")

            # position_embeddings might be 2nd
            position_embeddings = args[1] if len(args) > 1 else kwargs.get("position_embeddings")

            # attention_mask might be 3rd
            attention_mask = args[2] if len(args) > 2 else kwargs.get("attention_mask")

            bsz, q_len, _ = hidden_states.size()

            # Retrieve config attributes safely
            num_heads = getattr(module_self, 'num_heads', None) or getattr(module_self, 'n_heads', None) or getattr(module_self.config, 'num_attention_heads', None)
            num_key_value_heads = getattr(module_self, 'num_key_value_heads', None) or getattr(module_self, 'n_kv_heads', None) or getattr(module_self.config, 'num_key_value_heads', num_heads)
            head_dim = getattr(module_self, 'head_dim', None) or (self.model.config.hidden_size // num_heads)

            # Projections
            query_states = module_self.q_proj(hidden_states)
            key_states = module_self.k_proj(hidden_states)
            value_states = module_self.v_proj(hidden_states)

            # Reshape for heads
            query_states = query_states.view(bsz, q_len, num_heads, head_dim).transpose(1, 2)
            key_states = key_states.view(bsz, q_len, num_key_value_heads, head_dim).transpose(1, 2)
            value_states = value_states.view(bsz, q_len, num_key_value_heads, head_dim).transpose(1, 2)

            # RoPE application
            # In new versions, position_embeddings is passed directly
            if position_embeddings is not None:
                cos, sin = position_embeddings
                query_states, key_states = apply_rotary_pos_emb(query_states, key_states, cos, sin)
            else:
                # Fallback or older version handling
                # We can try to look for rotary_emb module if position_embeddings not passed
                rotary_emb = getattr(module_self, 'rotary_emb', None)
                if rotary_emb:
                    # Need seq_len
                    # This path is risky if we don't know exact call
                     pass

            # --- KV RE-ROUTING LOGIC ---
            # Extract last token KV from the current sequence
            # key_states: [bsz, num_kv_heads, seq_len, head_dim]
            k_last = key_states[:, :, -1:, :] # Keep dims
            v_last = value_states[:, :, -1:, :]

            # Prepend
            # New shape: [bsz, num_kv_heads, 1 + seq_len, head_dim]
            key_states = torch.cat([k_last, key_states], dim=2)
            value_states = torch.cat([v_last, value_states], dim=2)

            # Manual Attention Calculation (simplified for SDPA)
            num_key_value_groups = num_heads // num_key_value_heads
            key_states = repeat_kv(key_states, num_key_value_groups)
            value_states = repeat_kv(value_states, num_key_value_groups)

            # Attn weights
            # Shape: [bsz, num_heads, q_len, kv_len]
            attn_weights = torch.matmul(query_states, key_states.transpose(2, 3)) / (head_dim ** 0.5)

            # Bias injection: First column (index 0) gets +1.0
            attn_weights[:, :, :, 0] += 1.0

            # Masking
            # We want to allow Q_i to attend to K_0 and K_{j+1} where j <= i.
            # Construct causal mask for (q_len, q_len)
            causal_mask = torch.tril(torch.ones(q_len, q_len, device=hidden_states.device, dtype=torch.bool))
            # Prepend a column of True
            # mask shape: [q_len, 1 + q_len]
            extended_mask = torch.cat([
                torch.ones(q_len, 1, device=hidden_states.device, dtype=torch.bool),
                causal_mask
            ], dim=1)

            # Expand to batch/head
            # extended_mask: [1, 1, q_len, 1 + q_len]
            mask_4d = extended_mask[None, None, :, :]

            # Apply mask
            min_dtype = torch.finfo(attn_weights.dtype).min
            attn_weights = torch.where(mask_4d, attn_weights, torch.tensor(min_dtype, dtype=attn_weights.dtype))

            attn_weights = nn.functional.softmax(attn_weights, dim=-1, dtype=torch.float32).to(query_states.dtype)
            attn_output = torch.matmul(attn_weights, value_states)

            # attn_output: [bsz, num_heads, q_len, head_dim]
            attn_output = attn_output.transpose(1, 2).contiguous()
            attn_output = attn_output.reshape(bsz, q_len, -1)

            output = module_self.o_proj(attn_output)

            # Return matching original signature.
            # In 4.57.3, LlamaAttention returns (attn_output, attn_weights)

            return (output, attn_weights)

        return implementation

    def encode(self, text: str) -> torch.Tensor:
        """
        Encodes the text using the KV-Embedding method.
        """
        # 1. Prompt Template
        prompt = f"[Context/Query]: {text} Compress the [Context/Query] in one word:"

        inputs = self.tokenizer(prompt, return_tensors="pt").to(self.device)
        input_ids = inputs["input_ids"]

        # 2. Patching
        original_forwards = {}
        for i in self.target_layers:
            layer = self.model.model.layers[i]
            # Identify attention module
            if hasattr(layer, "self_attn"):
                attn_module = layer.self_attn
                # Store original forward, bound to the instance
                original_forwards[i] = attn_module.forward
                # Apply patch
                # We use types.MethodType to bind the function to the instance
                attn_module.forward = types.MethodType(self._get_modified_forward(attn_module.forward, i), attn_module)

        try:
            # 3. Forward Pass
            with torch.no_grad():
                outputs = self.model(input_ids, output_hidden_states=True)

            # 4. Extract Embeddings
            # Last layer hidden states
            last_hidden_state = outputs.hidden_states[-1] # [1, seq_len, hidden_dim]

            # e_last: embedding of the last token
            e_last = last_hidden_state[:, -1, :]

            # e_mean: mean pooling of all tokens
            e_mean = torch.mean(last_hidden_state, dim=1)

            # Hybrid Pooling
            e_combined = (e_last + e_mean) / 2.0

            # Normalize
            e_norm = torch.nn.functional.normalize(e_combined, p=2, dim=1)

            return e_norm.cpu().numpy()

        finally:
            # Restore original forwards
            for i, original in original_forwards.items():
                self.model.model.layers[i].self_attn.forward = original

    def encode_single(self, text: str):
        return self.encode(text)[0]


# Helper functions needed for the modified forward
def apply_rotary_pos_emb(q, k, cos, sin, position_ids=None, unsqueeze_dim=1):
    """Applies Rotary Position Embedding to the query and key tensors."""
    # cos, sin are (batch, seq_len, head_dim) or similar
    # We might need to unsqueeze.
    # In 4.57.3 they pass cos, sin directly.
    # Check shapes?
    # Usually they are [1, 1, seq_len, head_dim] or [seq_len, head_dim]

    # If cos has 4 dims, we don't need unsqueeze.
    if cos.dim() == 4:
        q_embed = (q * cos) + (rotate_half(q) * sin)
        k_embed = (k * cos) + (rotate_half(k) * sin)
    else:
        # Assuming standard behavior
        cos = cos.unsqueeze(unsqueeze_dim)
        sin = sin.unsqueeze(unsqueeze_dim)
        q_embed = (q * cos) + (rotate_half(q) * sin)
        k_embed = (k * cos) + (rotate_half(k) * sin)

    return q_embed, k_embed

def rotate_half(x):
    """Rotates half the hidden dims of the input."""
    x1 = x[..., : x.shape[-1] // 2]
    x2 = x[..., x.shape[-1] // 2 :]
    return torch.cat((-x2, x1), dim=-1)

def repeat_kv(hidden_states: torch.Tensor, n_rep: int) -> torch.Tensor:
    batch, num_key_value_heads, slen, head_dim = hidden_states.shape
    if n_rep == 1:
        return hidden_states
    hidden_states = hidden_states[:, :, None, :, :].expand(batch, num_key_value_heads, n_rep, slen, head_dim)
    return hidden_states.reshape(batch, num_key_value_heads * n_rep, slen, head_dim)
