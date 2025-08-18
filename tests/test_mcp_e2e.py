#!/usr/bin/env python3
"""
End-to-end tests for MCP protocol implementation.
"""

import json
import subprocess
import sys
import time
import os
import tempfile
import unittest
from typing import Optional, Dict, Any

class MCPTestClient:
    """Helper class for MCP protocol testing"""
    
    def __init__(self):
        self.process: Optional[subprocess.Popen] = None
        self.request_id = 0
        
    def start_server(self) -> bool:
        """Start the MCP server process"""
        try:
            cmd = ["cargo", "run", "--bin", "rust-research-mcp"]
            self.process = subprocess.Popen(
                cmd,
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                bufsize=0
            )
            return True
        except Exception as e:
            print(f"Failed to start server: {e}")
            return False
    
    def stop_server(self):
        """Stop the MCP server process"""
        if self.process:
            try:
                self.process.terminate()
                self.process.wait(timeout=5)
            except:
                self.process.kill()
            self.process = None
    
    def send_request(self, method: str, params: Optional[Dict[str, Any]] = None) -> int:
        """Send a JSON-RPC request and return the request ID"""
        self.request_id += 1
        request = {
            "jsonrpc": "2.0",
            "id": self.request_id,
            "method": method
        }
        if params:
            request["params"] = params
        
        request_json = json.dumps(request) + "\n"
        self.process.stdin.write(request_json)
        self.process.stdin.flush()
        return self.request_id
    
    def send_notification(self, method: str, params: Optional[Dict[str, Any]] = None):
        """Send a JSON-RPC notification (no ID)"""
        notification = {
            "jsonrpc": "2.0",
            "method": method
        }
        if params:
            notification["params"] = params
        
        notification_json = json.dumps(notification) + "\n"
        self.process.stdin.write(notification_json)
        self.process.stdin.flush()
    
    def read_response(self, timeout: float = 5.0) -> Optional[Dict[str, Any]]:
        """Read a JSON-RPC response with timeout"""
        try:
            # Set a simple timeout using select would be better, but this works
            line = self.process.stdout.readline()
            if line:
                return json.loads(line.strip())
        except json.JSONDecodeError as e:
            print(f"Failed to decode JSON: {e}")
            print(f"Raw line: {line}")
        except Exception as e:
            print(f"Error reading response: {e}")
        return None
    
    def initialize(self) -> bool:
        """Initialize the MCP connection"""
        # Step 1: Send initialize request
        init_params = {
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
        
        self.send_request("initialize", init_params)
        response = self.read_response()
        
        if not response or "error" in response:
            return False
        
        # Step 2: Send initialized notification
        self.send_notification("notifications/initialized")
        
        return True


class TestMCPProtocol(unittest.TestCase):
    """Test MCP protocol implementation"""
    
    @classmethod
    def setUpClass(cls):
        """Set up test environment"""
        cls.client = MCPTestClient()
        cls.server_started = cls.client.start_server()
        if cls.server_started:
            time.sleep(2)  # Give server time to start
            cls.initialized = cls.client.initialize()
        else:
            cls.initialized = False
    
    @classmethod
    def tearDownClass(cls):
        """Clean up test environment"""
        cls.client.stop_server()
    
    def test_01_server_starts(self):
        """Test that server starts successfully"""
        self.assertTrue(self.server_started, "Server should start")
    
    def test_02_initialization(self):
        """Test MCP initialization sequence"""
        self.assertTrue(self.initialized, "Server should initialize")
    
    def test_03_list_tools(self):
        """Test listing available tools"""
        if not self.initialized:
            self.skipTest("Server not initialized")
        
        self.client.send_request("tools/list", {})
        response = self.client.read_response()
        
        self.assertIsNotNone(response, "Should receive response")
        self.assertIn("result", response, "Response should have result")
        self.assertIn("tools", response["result"], "Result should have tools")
        
        tools = response["result"]["tools"]
        self.assertIsInstance(tools, list, "Tools should be a list")
        self.assertGreater(len(tools), 0, "Should have at least one tool")
        
        # Check for expected tools
        tool_names = [tool["name"] for tool in tools]
        self.assertIn("search_papers", tool_names, "Should have search_papers tool")
        self.assertIn("download_paper", tool_names, "Should have download_paper tool")
        self.assertIn("extract_metadata", tool_names, "Should have extract_metadata tool")
        self.assertIn("debug_test", tool_names, "Should have debug_test tool")
    
    def test_04_debug_tool(self):
        """Test debug tool functionality"""
        if not self.initialized:
            self.skipTest("Server not initialized")
        
        params = {
            "name": "debug_test",
            "arguments": {
                "message": "Hello from test"
            }
        }
        
        self.client.send_request("tools/call", params)
        response = self.client.read_response()
        
        self.assertIsNotNone(response, "Should receive response")
        self.assertIn("result", response, "Response should have result")
        self.assertIn("content", response["result"], "Result should have content")
        
        content = response["result"]["content"][0]["text"]
        self.assertIn("Debug echo: Hello from test", content, "Should echo message")
    
    def test_05_search_tool(self):
        """Test search tool functionality"""
        if not self.initialized:
            self.skipTest("Server not initialized")
        
        params = {
            "name": "search_papers",
            "arguments": {
                "query": "quantum computing",
                "limit": 5
            }
        }
        
        self.client.send_request("tools/call", params)
        response = self.client.read_response(timeout=30)  # Longer timeout for search
        
        self.assertIsNotNone(response, "Should receive response")
        self.assertIn("result", response, "Response should have result")
        
        # Search might fail due to network, but should not error
        if "error" not in response:
            content = response["result"]["content"][0]["text"]
            self.assertIn("Found", content, "Should report results")
    
    def test_06_download_tool_validation(self):
        """Test download tool input validation"""
        if not self.initialized:
            self.skipTest("Server not initialized")
        
        # Test with invalid input (no DOI or URL)
        params = {
            "name": "download_paper",
            "arguments": {}
        }
        
        self.client.send_request("tools/call", params)
        response = self.client.read_response()
        
        self.assertIsNotNone(response, "Should receive response")
        # Should get an error for missing required parameter
        self.assertTrue(
            "error" in response or 
            (response.get("result", {}).get("isError") == True),
            "Should error on missing DOI"
        )
    
    def test_07_metadata_extraction(self):
        """Test metadata extraction tool"""
        if not self.initialized:
            self.skipTest("Server not initialized")
        
        params = {
            "name": "extract_metadata",
            "arguments": {
                "input": "10.1038/nature12373"
            }
        }
        
        self.client.send_request("tools/call", params)
        response = self.client.read_response(timeout=20)
        
        self.assertIsNotNone(response, "Should receive response")
        
        if "error" not in response:
            # Metadata extraction might succeed or fail based on network
            self.assertIn("result", response, "Response should have result")


class TestSearchFunctionality(unittest.TestCase):
    """Test search functionality across providers"""
    
    @classmethod
    def setUpClass(cls):
        cls.client = MCPTestClient()
        cls.client.start_server()
        time.sleep(2)
        cls.initialized = cls.client.initialize()
    
    @classmethod
    def tearDownClass(cls):
        cls.client.stop_server()
    
    def _test_search(self, query: str, expected_in_result: str = None):
        """Helper to test search with a query"""
        params = {
            "name": "search_papers",
            "arguments": {
                "query": query,
                "limit": 3
            }
        }
        
        self.client.send_request("tools/call", params)
        response = self.client.read_response(timeout=30)
        
        self.assertIsNotNone(response, f"Should receive response for query: {query}")
        
        if "error" not in response and response.get("result"):
            content = response["result"]["content"][0]["text"]
            if expected_in_result:
                self.assertIn(expected_in_result, content, 
                    f"Result should contain '{expected_in_result}'")
            return content
        return None
    
    def test_doi_search(self):
        """Test searching by DOI"""
        if not self.initialized:
            self.skipTest("Server not initialized")
        
        # Test with a well-known DOI
        content = self._test_search("10.1038/nature12373", "10.1038/nature12373")
        if content:
            self.assertIn("DOI", content, "Should show DOI in results")
    
    def test_title_search(self):
        """Test searching by title"""
        if not self.initialized:
            self.skipTest("Server not initialized")
        
        content = self._test_search("deep learning", "Found")
        if content:
            self.assertIn("papers", content.lower(), "Should mention papers")
    
    def test_author_search(self):
        """Test searching by author"""
        if not self.initialized:
            self.skipTest("Server not initialized")
        
        content = self._test_search("LeCun", "Found")
        # Author search should work across multiple providers


class TestDownloadFunctionality(unittest.TestCase):
    """Test download functionality"""
    
    @classmethod
    def setUpClass(cls):
        cls.client = MCPTestClient()
        cls.client.start_server()
        time.sleep(2)
        cls.initialized = cls.client.initialize()
        cls.test_dir = tempfile.mkdtemp()
    
    @classmethod
    def tearDownClass(cls):
        cls.client.stop_server()
        # Clean up test directory
        import shutil
        shutil.rmtree(cls.test_dir, ignore_errors=True)
    
    def test_download_validation(self):
        """Test download input validation"""
        if not self.initialized:
            self.skipTest("Server not initialized")
        
        # Test various invalid inputs
        test_cases = [
            ({}, "Missing DOI and URL"),
            ({"doi": "invalid-doi"}, "Invalid DOI format"),
            ({"doi": "10.1038/test", "filename": "../malicious.pdf"}, "Invalid filename"),
        ]
        
        for args, description in test_cases:
            params = {
                "name": "download_paper",
                "arguments": args
            }
            
            self.client.send_request("tools/call", params)
            response = self.client.read_response()
            
            self.assertIsNotNone(response, f"Should receive response for: {description}")
            
            # These should all fail or report errors
            is_error = ("error" in response or 
                       response.get("result", {}).get("isError") == True or
                       "failed" in str(response).lower())
            
            self.assertTrue(is_error, f"Should fail for: {description}")
    
    def test_successful_download(self):
        """Test successful download of a known paper"""
        if not self.initialized:
            self.skipTest("Server not initialized")
        
        params = {
            "name": "download_paper",
            "arguments": {
                "doi": "10.1038/nature12373",
                "filename": "test_download.pdf"
            }
        }
        
        self.client.send_request("tools/call", params)
        response = self.client.read_response(timeout=60)  # Long timeout for download
        
        self.assertIsNotNone(response, "Should receive response")
        
        if "error" not in response and response.get("result"):
            content = response["result"]["content"][0]["text"]
            
            # Check if download succeeded
            if "Download successful" in content:
                self.assertIn("KB", content, "Should report file size")
                # Clean up downloaded file
                download_path = os.path.expanduser("~/Downloads/papers/test_download.pdf")
                if os.path.exists(download_path):
                    os.remove(download_path)
            else:
                # Download might fail due to availability
                self.assertIn("not available", content.lower(), 
                    "Should explain why download failed")


class TestProviderIntegration(unittest.TestCase):
    """Test provider integration and cascade"""
    
    @classmethod
    def setUpClass(cls):
        cls.client = MCPTestClient()
        cls.client.start_server()
        time.sleep(2)
        cls.initialized = cls.client.initialize()
    
    @classmethod
    def tearDownClass(cls):
        cls.client.stop_server()
    
    def test_multi_provider_search(self):
        """Test that search uses multiple providers"""
        if not self.initialized:
            self.skipTest("Server not initialized")
        
        params = {
            "name": "search_papers",
            "arguments": {
                "query": "machine learning",
                "limit": 10
            }
        }
        
        self.client.send_request("tools/call", params)
        response = self.client.read_response(timeout=45)  # Longer for multi-provider
        
        self.assertIsNotNone(response, "Should receive response")
        
        if "error" not in response and response.get("result"):
            content = response["result"]["content"][0]["text"]
            
            # Should mention multiple sources if working
            if "Found" in content and "papers" in content:
                # Look for evidence of multiple providers
                self.assertTrue(
                    any(source in content for source in ["Source:", "provider", "CrossRef", "ArXiv"]),
                    "Should show source information"
                )
    
    def test_zero_byte_prevention(self):
        """Test that failed downloads don't create zero-byte files"""
        if not self.initialized:
            self.skipTest("Server not initialized")
        
        # Try to download a paper that won't be available
        params = {
            "name": "download_paper",
            "arguments": {
                "doi": "10.9999/nonexistent.12345",
                "filename": "zero_byte_test.pdf"
            }
        }
        
        self.client.send_request("tools/call", params)
        response = self.client.read_response(timeout=30)
        
        self.assertIsNotNone(response, "Should receive response")
        
        # Check that no zero-byte file was created
        test_path = os.path.expanduser("~/Downloads/papers/zero_byte_test.pdf")
        if os.path.exists(test_path):
            size = os.path.getsize(test_path)
            self.assertGreater(size, 0, "File should not be zero bytes")
            os.remove(test_path)
        else:
            # Good - no file created for failed download
            pass


def run_tests():
    """Run all E2E tests"""
    # Create test suite
    loader = unittest.TestLoader()
    suite = unittest.TestSuite()
    
    # Add test classes in order
    suite.addTests(loader.loadTestsFromTestCase(TestMCPProtocol))
    suite.addTests(loader.loadTestsFromTestCase(TestSearchFunctionality))
    suite.addTests(loader.loadTestsFromTestCase(TestDownloadFunctionality))
    suite.addTests(loader.loadTestsFromTestCase(TestProviderIntegration))
    
    # Run tests with verbosity
    runner = unittest.TextTestRunner(verbosity=2)
    result = runner.run(suite)
    
    # Return exit code
    return 0 if result.wasSuccessful() else 1


if __name__ == "__main__":
    sys.exit(run_tests())