#!/usr/bin/env python3
"""
Test script to systematically attempt downloading research papers
using the rust-research-mcp server via JSON-RPC over stdio.
"""

import json
import subprocess
import sys
import time
from typing import Dict, List, Tuple

def send_jsonrpc_request(process, request_id: int, method: str, params: Dict) -> Dict:
    """Send a JSON-RPC request to the MCP server"""
    request = {
        "jsonrpc": "2.0",
        "id": request_id,
        "method": method,
        "params": params
    }
    
    request_json = json.dumps(request) + "\n"
    process.stdin.write(request_json.encode())
    process.stdin.flush()
    
    # Read response
    response_line = process.stdout.readline().decode().strip()
    if response_line:
        try:
            return json.loads(response_line)
        except json.JSONDecodeError as e:
            print(f"Failed to parse response: {response_line}")
            print(f"JSON decode error: {e}")
            return {"error": f"JSON decode error: {e}"}
    return {"error": "No response received"}

def test_single_paper_download(paper: Dict, paper_num: int, total: int) -> Dict:
    """Test downloading a single paper with a fresh server instance"""
    print(f"\nTesting paper {paper_num}/{total}: {paper['title'][:60]}...")
    print(f"DOI: {paper['doi']}")
    
    # Start a fresh MCP server for this test
    try:
        process = subprocess.Popen(
            ["./target/release/rust-research-mcp", "--log-level", "error"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            cwd="/Users/ladvien/sci_hub_mcp"
        )
    except FileNotFoundError:
        return {
            "paper": paper,
            "status": "EXCEPTION",
            "error": "Binary not found",
            "response": None
        }
    
    try:
        # Initialize the server
        init_params = {
            "protocolVersion": "2024-11-05",
            "capabilities": {"tools": {}},
            "clientInfo": {"name": "test-client", "version": "1.0"}
        }
        
        init_response = send_jsonrpc_request(process, 1, "initialize", init_params)
        if "error" in init_response:
            return {
                "paper": paper,
                "status": "FAILED",
                "error": f"Initialization failed: {init_response['error']}",
                "response": init_response
            }
        
        # List available tools first
        tools_response = send_jsonrpc_request(process, 2, "tools/list", {})
        if "error" in tools_response:
            return {
                "paper": paper,
                "status": "FAILED",
                "error": f"Tools list failed: {tools_response['error']}",
                "response": tools_response
            }
        
        # Check if download_paper tool is available
        tools = tools_response.get("result", {}).get("tools", [])
        download_tool_available = any(tool.get("name") == "download_paper" for tool in tools)
        
        if not download_tool_available:
            return {
                "paper": paper,
                "status": "FAILED",
                "error": f"download_paper tool not available. Available tools: {[t.get('name') for t in tools]}",
                "response": tools_response
            }
        
        # Try downloading the paper
        download_params = {
            "doi": paper["doi"]
        }
        
        download_response = send_jsonrpc_request(process, 3, "tools/call", {
            "name": "download_paper",
            "arguments": download_params
        })
        
        if "error" in download_response:
            status = "FAILED"
            error_msg = str(download_response.get("error", "Unknown error"))
            print(f"❌ Download failed: {error_msg}")
        elif "result" in download_response and download_response["result"].get("content"):
            content = download_response["result"]["content"]
            if isinstance(content, list) and len(content) > 0:
                result_content = content[0]
                result_text = result_content.get("text", "")
                if result_content.get("type") == "text" and "success" in result_text.lower():
                    status = "SUCCESS"
                    error_msg = None
                    print("✅ Download successful!")
                else:
                    status = "FAILED"
                    error_msg = result_text
                    print(f"❌ Download failed: {error_msg}")
            else:
                status = "FAILED"
                error_msg = "Invalid response format"
                print(f"❌ Download failed: {error_msg}")
        else:
            status = "FAILED"
            error_msg = "No result in response"
            print(f"❌ Download failed: {error_msg}")
        
        return {
            "paper": paper,
            "status": status,
            "error": error_msg if status == "FAILED" else None,
            "response": download_response
        }
        
    except Exception as e:
        print(f"❌ Exception during download: {e}")
        return {
            "paper": paper,
            "status": "EXCEPTION",
            "error": str(e),
            "response": None
        }
    finally:
        # Clean up the process
        try:
            process.terminate()
            process.wait(timeout=3)
        except subprocess.TimeoutExpired:
            process.kill()
        except:
            pass

def test_paper_downloads():
    """Test downloading each paper systematically"""
    
    papers = [
        {
            "title": "Agentic Retrieval-Augmented Generation: Advancing AI-Driven Information Retrieval and Processing",
            "year": "2025",
            "doi": "10.14445/22312803/ijctt-v73i1p111"
        },
        {
            "title": "Domain-Specific LLMs, Retrieval-Augmented Generation, and Agentic AI: A Unified Architecture for Specialized Intelligence",
            "year": "2025",
            "doi": "10.2139/ssrn.5386744"
        },
        {
            "title": "RAG-Critic: Leveraging Automated Critic-Guided Agentic Workflow for Retrieval Augmented Generation",
            "year": "2025",
            "doi": "10.18653/v1/2025.acl-long.179"
        },
        {
            "title": "Vector Storage Based Long-term Memory Research on LLM",
            "year": "2024",
            "doi": "10.2478/ijanmc-2024-0029"
        },
        {
            "title": "VELO: A Vector Database-Assisted Cloud-Edge Collaborative LLM QoS Optimization Framework",
            "year": "2024",
            "doi": "10.1109/icws62655.2024.00105"
        },
        {
            "title": "HiAgent: Hierarchical Working Memory Management for Solving Long-Horizon Agent Tasks with Large Language Model",
            "year": "2025",
            "doi": "10.18653/v1/2025.acl-long.1575"
        },
        {
            "title": "Memory Matters: The Need to Improve Long-Term Memory in LLM-Agents",
            "year": "2024",
            "doi": "10.1609/aaaiss.v2i1.27688"
        },
        {
            "title": "MemBench: Towards More Comprehensive Evaluation on the Memory of LLM-based Agents",
            "year": "2025",
            "doi": "10.18653/v1/2025.findings-acl.989"
        },
        {
            "title": "Max–Min semantic chunking of documents for RAG application",
            "year": "2025",
            "doi": "10.1007/s10791-025-09638-7"
        },
        {
            "title": "LateSplit: Lightweight Post-Retrieval Chunking for Query-Aligned Text Segmentation in RAG Systems",
            "year": "2025",
            "doi": "10.1109/ainit65432.2025.11036011"
        }
    ]
    
    print("Starting systematic paper download test...")
    print("=" * 80)
    
    # Ensure binary is built
    try:
        # Test if binary exists
        test_process = subprocess.Popen(
            ["./target/release/rust-research-mcp", "--help"],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            cwd="/Users/ladvien/sci_hub_mcp"
        )
        test_process.wait()
    except FileNotFoundError:
        print("Error: Could not find rust-research-mcp binary. Building first...")
        build_result = subprocess.run(["cargo", "build", "--release"], 
                                    cwd="/Users/ladvien/sci_hub_mcp",
                                    capture_output=True, text=True)
        if build_result.returncode != 0:
            print(f"Build failed: {build_result.stderr}")
            return
    
    # Test each paper download with a fresh server instance
    results = []
    
    for i, paper in enumerate(papers):
        result = test_single_paper_download(paper, i+1, len(papers))
        results.append(result)
        
        # Small delay between tests
        time.sleep(1)
        
    # Print summary
    print("\n" + "=" * 80)
    print("DOWNLOAD TEST SUMMARY")
    print("=" * 80)
    
    successful = [r for r in results if r["status"] == "SUCCESS"]
    failed = [r for r in results if r["status"] == "FAILED"]
    exceptions = [r for r in results if r["status"] == "EXCEPTION"]
    
    print(f"Total papers tested: {len(papers)}")
    print(f"Successful downloads: {len(successful)}")
    print(f"Failed downloads: {len(failed)}")
    print(f"Exceptions: {len(exceptions)}")
    
    if failed:
        print(f"\nFailed downloads:")
        for result in failed:
            print(f"  - {result['paper']['doi']}: {result['error']}")
    
    if exceptions:
        print(f"\nExceptions:")
        for result in exceptions:
            print(f"  - {result['paper']['doi']}: {result['error']}")
    
    # Print detailed error patterns
    if failed or exceptions:
        print(f"\nError patterns analysis:")
        error_types = {}
        for result in failed + exceptions:
            error = result['error']
            if error:
                # Simplify error message for pattern matching
                if "timeout" in error.lower():
                    error_type = "Timeout"
                elif "not found" in error.lower() or "404" in error:
                    error_type = "Not Found (404)"
                elif "access denied" in error.lower() or "403" in error:
                    error_type = "Access Denied (403)"
                elif "network" in error.lower() or "connection" in error.lower():
                    error_type = "Network Error"
                elif "parse" in error.lower() or "json" in error.lower():
                    error_type = "Parse Error"
                else:
                    error_type = "Other"
                
                error_types[error_type] = error_types.get(error_type, 0) + 1
        
        for error_type, count in error_types.items():
            print(f"  {error_type}: {count} papers")
    
    # Print detailed responses for debugging
    if any(r.get("response") for r in results):
        print(f"\nDetailed responses (first 3 for debugging):")
        for i, result in enumerate(results[:3]):
            if result.get("response"):
                print(f"  Paper {i+1} ({result['paper']['doi']}):")
                print(f"    Response: {json.dumps(result['response'], indent=2)[:500]}...")
                print()

if __name__ == "__main__":
    test_paper_downloads()