#!/usr/bin/env python3
import subprocess
import json
import time
import sys
import os

def test_mcp():
    synapse_path = os.path.join(os.path.dirname(__file__), 'bin/synapse')
    cmd = [synapse_path, '--mcp']
    print(f"Starting {cmd}...")
    proc = subprocess.Popen(cmd, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    time.sleep(1)  # give it a moment
    
    # Send initialize request
    init_req = {
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    }
    print("Sending:", json.dumps(init_req))
    proc.stdin.write(json.dumps(init_req) + '\n')
    proc.stdin.flush()
    
    # Read response with timeout
    for _ in range(10):
        line = proc.stdout.readline()
        if line:
            print("Received:", line.strip())
            break
        time.sleep(0.1)
    else:
        print("No response")
    
    # Send tools/list request
    tools_req = {
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    }
    print("Sending:", json.dumps(tools_req))
    proc.stdin.write(json.dumps(tools_req) + '\n')
    proc.stdin.flush()
    
    for _ in range(10):
        line = proc.stdout.readline()
        if line:
            print("Received:", line.strip())
            break
        time.sleep(0.1)
    
    proc.terminate()
    proc.wait()

if __name__ == "__main__":
    test_mcp()