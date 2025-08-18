#!/usr/bin/env python3
"""
Test script to validate download directory fix works through MCP protocol.
"""

import json
import subprocess
import sys
import os
from pathlib import Path

def test_mcp_download():
    """Test MCP server download functionality"""
    
    # Path to the binary
    binary_path = Path("./target/release/rust-research-mcp").resolve()
    config_path = Path("./config.toml").resolve()
    
    if not binary_path.exists():
        print(f"❌ Binary not found: {binary_path}")
        return False
        
    if not config_path.exists():
        print(f"❌ Config file not found: {config_path}")
        return False
    
    print(f"🔧 Testing MCP server: {binary_path}")
    print(f"📋 Using config: {config_path}")
    
    # Prepare MCP messages
    messages = [
        # Initialize
        {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "test-client", "version": "1.0.0"}
            }
        },
        # List tools
        {
            "jsonrpc": "2.0", 
            "id": 2,
            "method": "tools/list",
            "params": {}
        },
        # Test download
        {
            "jsonrpc": "2.0",
            "id": 3, 
            "method": "tools/call",
            "params": {
                "name": "download_paper",
                "arguments": {
                    "doi": "10.1186/s13643-017-0491-x",
                    "filename": "test_ohat_paper.pdf"
                }
            }
        }
    ]
    
    try:
        # Start the MCP server
        process = subprocess.Popen(
            [str(binary_path), "--config", str(config_path), "--log-level", "debug"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            env={**os.environ, "RSH_DOWNLOAD_DIRECTORY": str(Path.home() / "Downloads" / "Research-Papers")}
        )
        
        # Send messages
        input_data = ""
        for msg in messages:
            input_data += json.dumps(msg) + "\n"
        
        print("📤 Sending MCP messages...")
        stdout, stderr = process.communicate(input=input_data, timeout=30)
        
        print("📥 Server response:")
        print("STDOUT:", stdout)
        if stderr:
            print("STDERR:", stderr)
        
        # Check if download directory was created/used
        download_dir = Path.home() / "Downloads" / "Research-Papers"
        if download_dir.exists():
            print(f"✅ Download directory exists: {download_dir}")
            files = list(download_dir.glob("*.pdf"))
            if files:
                print(f"✅ Found downloaded files: {[f.name for f in files]}")
            else:
                print("ℹ️  No PDF files found (may be expected if download failed)")
        else:
            print(f"⚠️  Download directory doesn't exist: {download_dir}")
        
        return process.returncode == 0
        
    except subprocess.TimeoutExpired:
        print("⏰ Test timed out")
        process.kill()
        return False
    except Exception as e:
        print(f"❌ Test failed with error: {e}")
        return False

if __name__ == "__main__":
    print("🧪 Testing Download Directory Fix")
    print("=" * 50)
    
    success = test_mcp_download()
    
    if success:
        print("✅ Test completed successfully!")
        sys.exit(0)
    else:
        print("❌ Test failed!")
        sys.exit(1)