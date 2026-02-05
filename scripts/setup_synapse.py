import json
import os
import sys
from pathlib import Path

def setup():
    print("🎨 Synapse: Semantic Engine Configuration")
    print("-----------------------------------------")
    
    config_path = Path.home() / ".openclaw" / "openclaw.json"
    if not config_path.exists():
        print(f"❌ OpenClaw config not found at {config_path}")
        return

    try:
        with open(config_path, "r") as f:
            config = json.load(f)
    except Exception as e:
        print(f"❌ Error reading config: {e}")
        return

    # 1. Register MCP Server
    synapse_dir = Path(__file__).parent.parent.absolute()
    mcp_entry = {
        "command": str(synapse_dir / "target" / "release" / "synapse"),
        "args": ["--mcp"],
        "env": {
            "GRAPH_STORAGE_PATH": str(synapse_dir / "data" / "graphs")
        },
        "description": "Synapse: High-performance neuro-symbolic knowledge graph"
    }

    if "plugins" not in config:
        config["plugins"] = {"entries": {}}
    
    # Check if we should set as default
    # Since this is a script often run in non-interactive environments during install,
    # we'll look for a flag or default to True if we want to be proactive.
    # But for a good DX, we'll ask if in a TTY.
    
    make_default = True
    if sys.stdin.isatty():
        choice = input("🤔 Do you want to set Synapse as your default memory provider? (Y/n): ").strip().lower()
        make_default = choice in ["", "y", "yes"]

    if make_default:
        print("🧠 Setting Synapse as default memory search provider...")
        if "agents" not in config:
            config["agents"] = {"defaults": {}}
        if "defaults" not in config["agents"]:
            config["agents"]["defaults"] = {}
        
        config["agents"]["defaults"]["memorySearch"] = {
            "provider": "mcp",
            "mcpServer": "synapse"
        }

    # 2. Add to mcpServers (some versions use this top-level or in plugins)
    if "mcpServers" not in config:
        config["mcpServers"] = {}
    config["mcpServers"]["synapse"] = mcp_entry

    # Save config
    try:
        with open(config_path, "w") as f:
            json.dump(config, f, indent=2)
        print(f"✅ Configuration updated successfully at {config_path}")
        print("🚀 Restart OpenClaw to apply changes.")
    except Exception as e:
        print(f"❌ Failed to write config: {e}")

if __name__ == "__main__":
    setup()
