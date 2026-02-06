# Contributing to Synapse 🧠⛓️

Thank you for your interest in contributing to Synapse! We are building the memory layer for the next generation of AI agents, and your help is invaluable.

## 🏗️ Development Setup

Synapse is a hybrid project (Rust + Python). You will need:
- **Rust** (stable)
- **Python** (3.10+)
- **Protoc** (for gRPC code generation)

### 1. Backend (Rust)
```bash
cd crates/semantic-engine
cargo build
```

### 2. Frontend & SDK (Python)
```bash
python -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
pip install -e ./python-sdk
```

## 🛠️ Contribution Workflow

1.  **Fork the repository** and create your branch from `main`.
2.  **Make your changes**. If you're adding a new feature, please include tests.
3.  **Run the checks**:
    *   `cargo fmt` and `cargo clippy` for Rust code.
    *   `pytest` for Python code.
4.  **Submit a Pull Request** with a clear description of the changes and the problem they solve.

## 🧪 Testing Guidelines

We use a combination of unit tests and E2E tests.
- **Rust tests**: `cargo test` in the relevant crate.
- **Python tests**: `pytest tests/` (Ensure the Rust server is running for integration tests).

## 📜 Code of Conduct

Please be respectful and constructive in all interactions within the project. We follow standard open-source etiquette.

## ⚖️ License

By contributing to Synapse, you agree that your contributions will be licensed under the **MIT License**.
