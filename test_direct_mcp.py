#!/usr/bin/env python3
"""
Simple test script to verify MCP protocol works correctly.
This mimics how Claude Desktop would interact with the rust-research-mcp server.
"""

import json
import subprocess
import sys
import time

def send_request(process, request):
    """Send a JSON-RPC request to the MCP server"""
    request_json = json.dumps(request) + "\n"
    print(f"-> {request_json.strip()}")
    process.stdin.write(request_json)
    process.stdin.flush()

def read_response(process):
    """Read a JSON-RPC response from the MCP server"""
    try:
        line = process.stdout.readline()
        if line:
            response = json.loads(line.strip())
            print(f"<- {json.dumps(response, indent=2)}")
            return response
        return None
    except json.JSONDecodeError as e:
        print(f"Failed to decode JSON: {e}")
        print(f"Raw line: {line}")
        return None

def test_mcp_protocol():
    """Test the MCP protocol with rust-research-mcp server"""
    print("🚀 Starting rust-research-mcp server test")
    
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
        # Step 1: Initialize the server
        print("\n📋 Step 1: Initialize server")
        init_request = {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "roots": {"listChanged": True},
                    "sampling": {}
                },
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                }
            }
        }
        send_request(process, init_request)
        
        # Read initialize response
        init_response = read_response(process)
        if not init_response or "error" in init_response:
            print(f"❌ Initialize failed: {init_response}")
            return False
        
        # Step 2: Send initialized notification
        print("\n📋 Step 2: Send initialized notification")
        initialized_notification = {
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }
        send_request(process, initialized_notification)
        
        # Step 3: List tools
        print("\n🔧 Step 3: List available tools")
        list_tools_request = {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        }
        send_request(process, list_tools_request)
        
        # Read tools response
        tools_response = read_response(process)
        if not tools_response or "error" in tools_response:
            print(f"❌ List tools failed: {tools_response}")
            return False
        
        print(f"✅ Found {len(tools_response.get('result', {}).get('tools', []))} tools")
        
        # Step 4: Test download_paper tool
        print("\n📥 Step 4: Test download_paper tool")
        download_request = {
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "download_paper",
                "arguments": {
                    "doi": "10.1038/nature12373",
                    "filename": "test_paper.pdf"
                }
            }
        }
        send_request(process, download_request)
        
        # Read download response
        download_response = read_response(process)
        if not download_response:
            print("❌ No download response received")
            return False
        
        if "error" in download_response:
            print(f"⚠️ Download failed (expected for this test): {download_response['error']}")
        else:
            print(f"✅ Download response: {download_response}")
        
        print("\n🎉 MCP protocol test completed successfully!")
        return True
        
    except Exception as e:
        print(f"❌ Test failed with exception: {e}")
        return False
    
    finally:
        # Clean up
        try:
            process.terminate()
            process.wait(timeout=5)
        except:
            process.kill()

if __name__ == "__main__":
    success = test_mcp_protocol()
    sys.exit(0 if success else 1)