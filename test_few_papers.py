#!/usr/bin/env python3
"""
Test script to download a few key papers to verify the system works.
"""

import json
import subprocess
import sys

# Test a few key papers
PAPERS = [
    ("10.1038/nature12373", "nature_test.pdf"),  # Known working paper
    ("10.1371/journal.pone.0000308", "plos_open_access.pdf"),  # Open access
    ("10.14445/22312803/ijctt-v73i1p111", "agentic_rag_2025.pdf"),  # Recent paper
]

def send_request(process, request):
    """Send a JSON-RPC request to the MCP server"""
    request_json = json.dumps(request) + "\n"
    process.stdin.write(request_json)
    process.stdin.flush()

def read_response(process):
    """Read a JSON-RPC response from the MCP server"""
    try:
        line = process.stdout.readline()
        if line:
            response = json.loads(line.strip())
            return response
        return None
    except json.JSONDecodeError as e:
        print(f"Failed to decode JSON: {e}")
        return None

def test_papers():
    """Test downloading papers"""
    print("🚀 Testing paper downloads with multi-provider system")
    
    # Start the server process
    cmd = ["cargo", "run", "--bin", "rust-research-mcp"]
    process = subprocess.Popen(
        cmd,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        bufsize=0
    )
    
    try:
        # Initialize
        init_request = {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {"roots": {"listChanged": True}},
                "clientInfo": {"name": "test-client", "version": "1.0.0"}
            }
        }
        send_request(process, init_request)
        read_response(process)  # Ignore response
        
        # Send initialized notification
        send_request(process, {"jsonrpc": "2.0", "method": "notifications/initialized"})
        
        print("✅ Server initialized\n")
        
        # Test each paper
        for i, (doi, filename) in enumerate(PAPERS, 1):
            print(f"📥 [{i}/{len(PAPERS)}] Testing: {doi}")
            
            download_request = {
                "jsonrpc": "2.0",
                "id": i + 10,
                "method": "tools/call",
                "params": {
                    "name": "download_paper",
                    "arguments": {"doi": doi, "filename": filename}
                }
            }
            send_request(process, download_request)
            
            response = read_response(process)
            if response and "result" in response:
                content = response["result"]["content"][0]["text"]
                if "Download successful" in content:
                    print(f"✅ Success: {filename}")
                else:
                    print(f"⚠️ Failed or not available")
                    print(f"   Response: {content[:200]}...")
            else:
                print(f"❌ No valid response")
            
            print()
        
        return True
        
    except Exception as e:
        print(f"❌ Test failed: {e}")
        return False
    
    finally:
        try:
            process.terminate()
            process.wait(timeout=5)
        except:
            process.kill()

if __name__ == "__main__":
    test_papers()