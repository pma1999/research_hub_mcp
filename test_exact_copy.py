#!/usr/bin/env python3
"""
Test script that exactly copies the working e2e test approach
"""

import subprocess
import time

def test_exact_copy():
    """Test using the exact same approach as the working e2e test"""
    print("Testing with exact e2e test pattern...")
    print("-" * 80)
    
    # Start the MCP server exactly like the e2e test
    process = subprocess.Popen(
        ["./target/release/rust-research-mcp", "--log-level", "error"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        cwd="/Users/ladvien/sci_hub_mcp"
    )
    
    try:
        # Send exact same init as e2e test
        init_line = r'{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{"tools":{}},"clientInfo":{"name":"test","version":"1.0"}}}' + "\n"
        print(f"Sending init: {init_line.strip()}")
        process.stdin.write(init_line.encode())
        process.stdin.flush()
        
        # Read init response
        init_response = process.stdout.readline().decode().strip()
        print(f"Init response: {init_response}")
        
        if not init_response or "rust-research-mcp" not in init_response:
            print("❌ Init failed")
            return False
        
        print("✅ Init successful")
        
        # Send exact same tools/list as e2e test
        tools_line = r'{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' + "\n"
        print(f"Sending tools: {tools_line.strip()}")
        process.stdin.write(tools_line.encode())
        process.stdin.flush()
        
        # Read tools response
        tools_response = process.stdout.readline().decode().strip()
        print(f"Tools response: {tools_response}")
        
        if not tools_response:
            print("❌ No tools response")
            return False
        
        if "search_papers" in tools_response and "download_paper" in tools_response:
            print("✅ Tools list successful")
            
            # Try a download
            download_line = r'{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"download_paper","arguments":{"doi":"10.14445/22312803/ijctt-v73i1p111"}}}' + "\n"
            print(f"Sending download: {download_line.strip()}")
            process.stdin.write(download_line.encode())
            process.stdin.flush()
            
            # Wait for download response with timeout
            print("Waiting for download response...")
            time.sleep(5)  # Give it some time
            
            download_response = process.stdout.readline().decode().strip()
            print(f"Download response: {download_response}")
            
            if download_response:
                print("✅ Got download response!")
                return True
            else:
                print("❌ No download response")
                return False
        else:
            print(f"❌ Tools list failed or incomplete: {tools_response}")
            return False
        
    except Exception as e:
        print(f"❌ Exception: {e}")
        return False
    finally:
        # Kill process after short delay
        time.sleep(0.1)
        try:
            process.kill()
            process.wait()
        except:
            pass
        
        # Print stderr for debugging
        stderr = process.stderr.read().decode() if process.stderr else ""
        if stderr.strip():
            print(f"Server stderr: {stderr}")

if __name__ == "__main__":
    success = test_exact_copy()
    
    if success:
        print("\n🎉 Test completed successfully!")
    else:
        print("\n💥 Test failed!")
    
    exit(0 if success else 1)