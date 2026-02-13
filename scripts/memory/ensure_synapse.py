#!/usr/bin/env python3
"""
Ensure Synapse gRPC server is running.
If not, start it using start_synapse.sh.
"""
import subprocess
import sys
import os
import time

def is_synapse_running():
    try:
        import socket
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.settimeout(1)
        result = sock.connect_ex(('localhost', 50051))
        sock.close()
        return result == 0
    except:
        return False

def start_synapse():
    script_dir = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    start_script = os.path.join(script_dir, 'start_synapse.sh')
    if os.path.exists(start_script):
        subprocess.run([start_script], check=True)
    else:
        # Fallback: direct start
        env = os.environ.copy()
        env['SYNAPSE_AUTH_TOKENS'] = '{"test": ["*"]}'
        subprocess.Popen(['./bin/synapse'], cwd=script_dir, env=env)
        time.sleep(2)

if __name__ == "__main__":
    if not is_synapse_running():
        print("Synapse not running. Starting...")
        start_synapse()
        # Verify
        time.sleep(2)
        if is_synapse_running():
            print("✅ Synapse started.")
        else:
            print("❌ Failed to start Synapse.")
            sys.exit(1)
    else:
        print("✅ Synapse already running.")