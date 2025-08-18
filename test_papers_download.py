#!/usr/bin/env python3
"""
Test script to download all the agentic memory research papers.
"""

import json
import subprocess
import sys
import time

# List of papers to download
PAPERS = [
    ("10.14445/22312803/ijctt-v73i1p111", "agentic_retrieval_augmented_generation.pdf"),
    ("10.2139/ssrn.5386744", "domain_specific_llms_rag_agentic.pdf"),
    ("10.18653/v1/2025.acl-long.179", "rag_critic_agentic_workflow.pdf"),
    ("10.2478/ijanmc-2024-0029", "vector_storage_longterm_memory.pdf"),
    ("10.1109/icws62655.2024.00105", "velo_vector_database_framework.pdf"),
    ("10.18653/v1/2025.acl-long.1575", "hiagent_hierarchical_memory.pdf"),
    ("10.1609/aaaiss.v2i1.27688", "memory_matters_llm_agents.pdf"),
    ("10.18653/v1/2025.findings-acl.989", "membench_memory_evaluation.pdf"),
    ("10.1007/s10791-025-09638-7", "max_min_semantic_chunking.pdf"),
    ("10.1109/ainit65432.2025.11036011", "latesplit_chunking.pdf"),
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

def test_paper_downloads():
    """Test downloading all research papers"""
    print("🚀 Starting systematic paper download test")
    
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
        # Initialize the server
        print("📋 Initializing MCP server...")
        init_request = {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {"roots": {"listChanged": True}, "sampling": {}},
                "clientInfo": {"name": "test-client", "version": "1.0.0"}
            }
        }
        send_request(process, init_request)
        
        # Read initialize response
        init_response = read_response(process)
        if not init_response or "error" in init_response:
            print(f"❌ Initialize failed: {init_response}")
            return False
        
        # Send initialized notification
        initialized_notification = {
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }
        send_request(process, initialized_notification)
        
        print("✅ MCP server initialized successfully")
        print(f"📚 Testing download of {len(PAPERS)} research papers...\n")
        
        # Download each paper
        successful_downloads = 0
        failed_downloads = 0
        
        for i, (doi, filename) in enumerate(PAPERS, 1):
            print(f"📥 [{i}/{len(PAPERS)}] Downloading: {doi}")
            
            download_request = {
                "jsonrpc": "2.0",
                "id": i + 10,  # Offset IDs
                "method": "tools/call",
                "params": {
                    "name": "download_paper",
                    "arguments": {
                        "doi": doi,
                        "filename": filename
                    }
                }
            }
            send_request(process, download_request)
            
            # Read download response
            download_response = read_response(process)
            if not download_response:
                print(f"❌ No response for {doi}")
                failed_downloads += 1
                continue
            
            if "error" in download_response:
                print(f"❌ Error: {download_response['error']['message']}")
                failed_downloads += 1
            elif download_response.get("result", {}).get("isError", False):
                content = download_response["result"]["content"][0]["text"]
                print(f"⚠️ Download failed: {content[:100]}...")
                failed_downloads += 1
            else:
                content = download_response["result"]["content"][0]["text"]
                if "Download successful" in content:
                    print(f"✅ Success: {filename}")
                    successful_downloads += 1
                else:
                    print(f"⚠️ Unclear result: {content[:100]}...")
                    failed_downloads += 1
            
            print()  # Add spacing between downloads
        
        # Summary
        print("=" * 60)
        print(f"📊 Download Results Summary:")
        print(f"✅ Successful: {successful_downloads}")
        print(f"❌ Failed: {failed_downloads}")
        print(f"📈 Success Rate: {successful_downloads/len(PAPERS)*100:.1f}%")
        
        return successful_downloads > 0
        
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
    success = test_paper_downloads()
    sys.exit(0 if success else 1)