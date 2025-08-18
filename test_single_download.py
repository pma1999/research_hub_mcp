#!/usr/bin/env python3
"""
Test script to test a single paper download with proper session handling
"""

import json
import subprocess
import sys
import time

def test_single_download(doi: str, title: str):
    """Test downloading a single paper with proper session handling"""
    print(f"Testing download for: {title}")
    print(f"DOI: {doi}")
    print("-" * 80)
    
    # Start the MCP server
    process = subprocess.Popen(
        ["./target/release/rust-research-mcp", "--log-level", "debug"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        cwd="/Users/ladvien/sci_hub_mcp"
    )
    
    try:
        # Send initialization request
        init_request = {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {"tools": {}},
                "clientInfo": {"name": "test-client", "version": "1.0"}
            }
        }
        
        init_json = json.dumps(init_request) + "\n"
        print(f"Sending init: {init_json.strip()}")
        process.stdin.write(init_json.encode())
        process.stdin.flush()
        
        # Read init response
        init_response_line = process.stdout.readline().decode().strip()
        print(f"Init response: {init_response_line}")
        
        if not init_response_line:
            print("❌ No init response received")
            return False
            
        try:
            init_response = json.loads(init_response_line)
        except json.JSONDecodeError as e:
            print(f"❌ Failed to parse init response: {e}")
            return False
        
        if "error" in init_response:
            print(f"❌ Init failed: {init_response['error']}")
            return False
        
        print("✅ Initialization successful")
        
        # Send tools/list request directly (as in working e2e test)
        tools_request = {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        }
        
        tools_json = json.dumps(tools_request) + "\n"
        print(f"Sending tools list: {tools_json.strip()}")
        process.stdin.write(tools_json.encode())
        process.stdin.flush()
        
        # Read tools response
        tools_response_line = process.stdout.readline().decode().strip()
        print(f"Tools response: {tools_response_line}")
        
        if not tools_response_line:
            print("❌ No tools response received")
            return False
            
        try:
            tools_response = json.loads(tools_response_line)
        except json.JSONDecodeError as e:
            print(f"❌ Failed to parse tools response: {e}")
            return False
        
        if "error" in tools_response:
            print(f"❌ Tools list failed: {tools_response['error']}")
            return False
        
        # Check available tools
        tools = tools_response.get("result", {}).get("tools", [])
        tool_names = [tool.get("name") for tool in tools]
        print(f"Available tools: {tool_names}")
        
        if "download_paper" not in tool_names:
            print("❌ download_paper tool not available")
            return False
        
        print("✅ download_paper tool found")
        
        # Send download request
        download_request = {
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "download_paper",
                "arguments": {
                    "doi": doi
                }
            }
        }
        
        download_json = json.dumps(download_request) + "\n"
        print(f"Sending download: {download_json.strip()}")
        process.stdin.write(download_json.encode())
        process.stdin.flush()
        
        # Read download response with longer timeout
        print("Waiting for download response...")
        
        # Use a longer timeout for downloads
        import select
        ready, _, _ = select.select([process.stdout], [], [], 30)  # 30 second timeout
        
        if ready:
            download_response_line = process.stdout.readline().decode().strip()
            print(f"Download response: {download_response_line}")
        else:
            print("❌ Download timeout - no response within 30 seconds")
            return False
            
        if not download_response_line:
            print("❌ No download response received")
            return False
            
        try:
            download_response = json.loads(download_response_line)
        except json.JSONDecodeError as e:
            print(f"❌ Failed to parse download response: {e}")
            return False
        
        if "error" in download_response:
            print(f"❌ Download failed: {download_response['error']}")
            return False
        
        # Check download result
        result = download_response.get("result", {})
        content = result.get("content", [])
        
        if not content:
            print("❌ No content in download response")
            return False
        
        result_text = content[0].get("text", "")
        print(f"Download result: {result_text[:200]}...")
        
        if "success" in result_text.lower() or "downloaded" in result_text.lower():
            print("✅ Download appears successful!")
            return True
        else:
            print(f"❌ Download failed: {result_text}")
            return False
        
    except Exception as e:
        print(f"❌ Exception during test: {e}")
        return False
    finally:
        # Clean up
        try:
            process.terminate()
            process.wait(timeout=3)
        except subprocess.TimeoutExpired:
            process.kill()
        except:
            pass
        
        # Capture any stderr output for debugging
        stderr_output = ""
        if process.stderr:
            try:
                stderr_output = process.stderr.read().decode()
                if stderr_output.strip():
                    print(f"Server stderr: {stderr_output}")
            except:
                pass

if __name__ == "__main__":
    # Test with just one paper first
    success = test_single_download(
        "10.14445/22312803/ijctt-v73i1p111", 
        "Agentic Retrieval-Augmented Generation: Advancing AI-Driven Information Retrieval and Processing"
    )
    
    if success:
        print("\n🎉 Test completed successfully!")
    else:
        print("\n💥 Test failed!")
        
    sys.exit(0 if success else 1)