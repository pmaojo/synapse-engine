#!/usr/bin/env python3
import subprocess
import json
import sys
import os

synapse_path = os.path.join(os.path.dirname(__file__), 'bin/synapse')
env = os.environ.copy()
env['SYNAPSE_AUTH_TOKENS'] = '{"test": ["*"]}'
env['RUST_LOG'] = 'info'

req = json.dumps({
    "jsonrpc": "2.0",
    "id": 1,
    "method": "initialize",
    "params": {}
}) + '\n'

print("Request:", req)
proc = subprocess.Popen([synapse_path, '--mcp'], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, env=env)
try:
    stdout, stderr = proc.communicate(input=req, timeout=5)
    print("STDOUT:", stdout)
    print("STDERR:", stderr)
except subprocess.TimeoutExpired:
    print("Timeout")
    proc.terminate()
    stdout, stderr = proc.communicate()
    print("STDOUT:", stdout)
    print("STDERR:", stderr)
print("Exit code:", proc.returncode)