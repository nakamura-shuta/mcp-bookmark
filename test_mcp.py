#!/usr/bin/env python3
"""Test MCP server functionality"""
import json
import subprocess
import sys

def send_request(proc, request):
    """Send a request and get response"""
    request_str = json.dumps(request)
    print(f"→ Sending: {request_str}", file=sys.stderr)
    
    # Send length-prefixed message (for stdio transport)
    proc.stdin.write(request_str + "\n")
    proc.stdin.flush()
    
    # Read response
    response_line = proc.stdout.readline()
    if response_line:
        response = json.loads(response_line)
        print(f"← Response: {json.dumps(response, indent=2)}", file=sys.stderr)
        return response
    return None

def main():
    # Start the MCP server
    cmd = ["/Users/nakamura.shuta/dev/rust/mcp-bookmark/target/release/mcp-bookmark"]
    proc = subprocess.Popen(
        cmd,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        bufsize=0
    )
    
    try:
        # Test initialize
        print("\n=== Testing Initialize ===", file=sys.stderr)
        response = send_request(proc, {
            "jsonrpc": "2.0",
            "method": "initialize",
            "params": {
                "protocolVersion": "1.0.0",
                "capabilities": {}
            },
            "id": 1
        })
        
        # Test list resources
        print("\n=== Testing List Resources ===", file=sys.stderr)
        response = send_request(proc, {
            "jsonrpc": "2.0",
            "method": "resources/list",
            "params": {},
            "id": 2
        })
        
        # Test list tools
        print("\n=== Testing List Tools ===", file=sys.stderr)
        response = send_request(proc, {
            "jsonrpc": "2.0",
            "method": "tools/list",
            "params": {},
            "id": 3
        })
        
    finally:
        proc.terminate()
        proc.wait()

if __name__ == "__main__":
    main()