#!/usr/bin/env python3
import subprocess
import json
import time
import sys
import os
import threading
import queue

def read_output(pipe, queue, prefix):
    for line in iter(pipe.readline, ''):
        queue.put((prefix, line))
    pipe.close()

def main():
    synapse_path = os.path.join(os.path.dirname(__file__), 'bin/synapse')
    env = os.environ.copy()
    env['SYNAPSE_AUTH_TOKENS'] = '{"test": ["*"]}'
    env['SYNAPSE_MCP_TOKEN'] = 'test'
    env['RUST_LOG'] = 'debug'
    
    print("Starting synapse --mcp")
    proc = subprocess.Popen([synapse_path, '--mcp'], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, env=env, bufsize=1)
    
    q = queue.Queue()
    t1 = threading.Thread(target=read_output, args=(proc.stdout, q, 'OUT'))
    t2 = threading.Thread(target=read_output, args=(proc.stderr, q, 'ERR'))
    t1.daemon = True
    t2.daemon = True
    t1.start()
    t2.start()
    
    # Wait for startup message
    time.sleep(2)
    
    # Send initialize
    init = {"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}
    proc.stdin.write(json.dumps(init) + '\n')
    proc.stdin.flush()
    print("Sent initialize")
    
    # Wait for response
    start = time.time()
    response = None
    while time.time() - start < 5:
        try:
            prefix, line = q.get(timeout=0.1)
            print(f"[{prefix}] {line.rstrip()}")
            if prefix == 'OUT' and line.strip():
                response = line.strip()
                break
        except queue.Empty:
            continue
    
    if response:
        print("✅ Got response:", response)
        # Send initialized notification
        notif = {"jsonrpc": "2.0", "method": "notifications/initialized", "params": {}}
        proc.stdin.write(json.dumps(notif) + '\n')
        proc.stdin.flush()
        print("Sent initialized notification")
        # Send tools/list
        tools = {"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}}
        proc.stdin.write(json.dumps(tools) + '\n')
        proc.stdin.flush()
        print("Sent tools/list")
        # Wait for response
        start = time.time()
        while time.time() - start < 5:
            try:
                prefix, line = q.get(timeout=0.1)
                print(f"[{prefix}] {line.rstrip()}")
                if prefix == 'OUT' and line.strip():
                    print("✅ Tools response:", line.strip())
                    break
            except queue.Empty:
                continue
    else:
        print("❌ No response to initialize")
        # Print any stderr lines
        while True:
            try:
                prefix, line = q.get_nowait()
                if prefix == 'ERR':
                    print(f"STDERR: {line.rstrip()}")
            except queue.Empty:
                break
    
    proc.terminate()
    proc.wait()
    print("Done.")

if __name__ == "__main__":
    main()