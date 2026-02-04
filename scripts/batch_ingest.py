#!/usr/bin/env python3
"""
Batch Processor - Automated ingestion of all DataSyn files
Processes files line-by-line and stores in Rust backend
"""
import sys
import os
from pathlib import Path
from dotenv import load_dotenv

# Load environment variables
load_dotenv()

sys.path.append(os.getcwd())

from agents.application.pipelines.datasyn import DataSynPipeline
from agents.infrastructure.web.client import get_client

def main():
    print("🚀 Starting Batch Ingestion")
    print("="*60)
    
    # Initialize
    pipeline = DataSynPipeline()
    rust_client = get_client()
    
    if not rust_client.connected:
        print("❌ Rust backend not connected!")
        print("   Start it with: ./start_rust_server.sh")
        return
    
    print(f"✅ Connected to Rust backend\n")
    
    # Find all files
    data_dir = Path("documents/DataSyn")
    files = list(data_dir.glob("*.csv")) + list(data_dir.glob("*.md")) + list(data_dir.glob("*.json"))
    
    print(f"📁 Found {len(files)} files to process:\n")
    for f in files:
        print(f"  - {f.name}")
    
    print("\n" + "="*60)
    
    total_triples = 0
    
    # Process each file
    for i, filepath in enumerate(files, 1):
        print(f"\n[{i}/{len(files)}] Processing: {filepath.name}")
        print("-"*60)
        
        result = pipeline.run(str(filepath))
        
        if result.success:
            triples_count = result.data.get("triples_extracted", 0)
            total_triples += triples_count
            print(f"✅ Success: {triples_count} triples extracted")
            print(f"⏱️  Time: {result.execution_time:.2f}s")
        else:
            print(f"❌ Failed: {result.data.get('error', 'Unknown error')}")
    
    print("\n" + "="*60)
    print(f"🎉 BATCH COMPLETE")
    print(f"📊 Total triples extracted: {total_triples}")
    print("="*60)

if __name__ == "__main__":
    main()
