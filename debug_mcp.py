#!/usr/bin/env python3
import subprocess
import json
import time
import sys
import os

def main():
    synapse_path = os.path.join(os.path.dirname(__file__), 'bin/synapse')
    env = os.environ.copy()
    env['SYNAPSE_AUTH_TOKENS'] = '{"test": ["*"]}'
    env['RUST_LOG'] = 'debug'
    
    print(f"Starting {synapse_path} --mcp")
    proc = subprocess.Popen([synapse_path, '--mcp'], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, env=env)
    
    # Wait a bit for initialization
    time.sleep(2)
    
    # Check if process is still alive
    if proc.poll() is not None:
        stdout, stderr = proc.communicate()
        print("Process exited early:")
        print("STDOUT:", stdout)
        print("STDERR:", stderr)
        sys.exit(1)
    
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
    
    # Wait for response with timeout
    start = time.time()
    response = None
    while time.time() - start < 5:
        line = proc.stdout.readline()
        if line:
            response = line.strip()
            break
        time.sleep(0.1)
    
    if response:
        print("Received:", response)
        # Parse and check
        try:
            data = json.loads(response)
            if 'result' in data:
                print("✅ Handshake successful")
            else:
                print("❌ Handshake failed:", data.get('error'))
        except json.JSONDecodeError as e:
            print("❌ Invalid JSON:", e)
    else:
        print("❌ No response within 5 seconds")
        # Try to read stderr
        stderr_lines = []
        while True:
            line = proc.stderr.readline()
            if not line:
                break
            stderr_lines.append(line)
        if stderr_lines:
            print("STDERR:", ''.join(stderr_lines))
    
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
    
    start = time.time()
    while time.time() - start < 5:
        line = proc.stdout.readline()
        if line:
            print("Tools response:", line.strip())
            break
        time.sleep(0.1)
    
    proc.terminate()
    proc.wait()
    print("Done.")

if __name__ == "__main__":
    main()