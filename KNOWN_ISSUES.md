# 🐛 Known Issues

## 📦 Dependencies

### Transformers & HuggingFace Hub
- **Issue**: `transformers` (v4.57.1) requires `huggingface-hub<1.0`, but newer versions (v1.1.4+) are often installed by other packages like `accelerate`.
- **Workaround**: Downgrade `huggingface-hub` manually if you encounter `ImportError`.
  ```bash
  pip install "huggingface-hub<1.0"
  ```
- **Status**: Fixed in current environment by pinning `huggingface-hub==0.36.0`.

### PyTorch & SymPy
- **Issue**: Reinstalling `transformers` can sometimes break `sympy` or `torch` due to version mismatches (`NameError: name 'Number' is not defined`).
- **Workaround**: Force reinstall `torch` and `sympy`.
  ```bash
  pip install --force-reinstall torch sympy
  ```

## 🏗️ Architecture

### Module Paths
- **Issue**: The project is transitioning from a flat structure to a Domain-Driven Design (DDD) structure. Some legacy imports might still exist in older test files.
- **Status**: `app.py` and core agents are fully migrated to `agents.infrastructure`, `agents.domain`, and `agents.application`.

### Qdrant Concurrency
- **Issue**: Local Qdrant storage (`./qdrant_storage`) locks the directory, preventing multiple clients from accessing it simultaneously.
- **Status**: Fixed by introducing `agents.infrastructure.di_container.py` to manage a shared `QdrantClient` instance.

## 🧪 Testing

- **Automated Tests**: The E2E test suite (`tests/test_e2e.py`) has been reactivated and updated to work with the new DDD module structure.
- **Verification**: Core functionality is verified via `pytest tests/` and the Gradio UI (`app.py`).
