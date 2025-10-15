#!/usr/bin/env node

/**
 * HTTP-to-stdio wrapper for Rust MCP server
 * This bridges Smithery's HTTP transport requirement with the stdio-based Rust server
 */

import { spawn } from 'child_process';
import { createServer } from 'http';
import { parse } from 'url';
import { createInterface } from 'readline';

const PORT = process.env.PORT || 3000;
const DOWNLOAD_DIR = process.env.DOWNLOAD_DIR || '/data';
const LOG_LEVEL = process.env.LOG_LEVEL || 'info';

let rustServer = null;
let pendingRequests = new Map();
let requestIdCounter = 1;

// Start the Rust MCP server as a persistent process
function startRustServer() {
  console.log('Starting Rust MCP server...');
  
  rustServer = spawn('/usr/local/bin/rust-research-mcp', [
    '--download-dir', DOWNLOAD_DIR,
    '--log-level', LOG_LEVEL
  ], {
    stdio: ['pipe', 'pipe', 'pipe']
  });

  // Create readline interface for line-by-line output parsing
  const rl = createInterface({
    input: rustServer.stdout,
    crlfDelay: Infinity
  });

  rl.on('line', (line) => {
    try {
      const response = JSON.parse(line);
      console.log('Received response:', JSON.stringify(response).substring(0, 200));
      
      // Find the matching pending request
      const requestId = response.id;
      if (pendingRequests.has(requestId)) {
        const { resolve } = pendingRequests.get(requestId);
        resolve(response);
        pendingRequests.delete(requestId);
      } else {
        console.warn('Received response for unknown request ID:', requestId);
      }
    } catch (error) {
      console.error('Failed to parse Rust server output:', line);
      console.error('Parse error:', error);
    }
  });

  rustServer.stderr.on('data', (data) => {
    // Log stderr but don't treat as error (might be tracing logs)
    console.error('Rust server stderr:', data.toString());
  });

  rustServer.on('close', (code) => {
    console.error(`Rust server exited with code ${code}`);
    // Reject all pending requests
    for (const [id, { reject }] of pendingRequests) {
      reject(new Error(`Server exited with code ${code}`));
    }
    pendingRequests.clear();
    rustServer = null;
  });

  rustServer.on('error', (error) => {
    console.error('Rust server error:', error);
  });

  console.log('Rust MCP server started successfully');
}

// Send request to Rust server and wait for response
async function sendToRustServer(request, timeoutMs = 30000) {
  if (!rustServer) {
    throw new Error('Rust server not running');
  }

  return new Promise((resolve, reject) => {
    const requestId = request.id || requestIdCounter++;
    request.id = requestId;

    // Store the resolver
    pendingRequests.set(requestId, { resolve, reject });

    // Set timeout
    const timeout = setTimeout(() => {
      if (pendingRequests.has(requestId)) {
        pendingRequests.delete(requestId);
        reject(new Error('Request timeout'));
      }
    }, timeoutMs);

    // Clear timeout on resolution
    const originalResolve = resolve;
    const wrappedResolve = (value) => {
      clearTimeout(timeout);
      originalResolve(value);
    };
    pendingRequests.get(requestId).resolve = wrappedResolve;

    // Send the request
    const requestStr = JSON.stringify(request);
    console.log('Sending to Rust server:', requestStr.substring(0, 200));
    rustServer.stdin.write(requestStr + '\n');
  });
}

// Configuration parsing from query parameters (Smithery format)
function parseConfig(query) {
  return {
    downloadDir: query.DOWNLOAD_DIR || DOWNLOAD_DIR,
    logLevel: query.LOG_LEVEL || LOG_LEVEL
  };
}

// Create HTTP server
const server = createServer(async (req, res) => {
  const { pathname, query: queryString } = parse(req.url, true);
  
  // Only handle /mcp endpoint
  if (pathname !== '/mcp') {
    res.writeHead(404, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ error: 'Not found' }));
    return;
  }

  // Handle different methods
  if (req.method === 'GET') {
    // Health check or info endpoint
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ 
      status: 'ok', 
      server: 'rust-research-mcp',
      transport: 'http-to-stdio-bridge',
      rustServerRunning: rustServer !== null
    }));
    return;
  }
  
  if (req.method === 'POST') {
    // Collect request body
    let body = '';
    req.on('data', chunk => {
      body += chunk.toString();
    });
    
    req.on('end', async () => {
      try {
        // Parse JSON-RPC request
        const jsonRpcRequest = JSON.parse(body);
        
        // Forward to Rust server and wait for response
        const response = await sendToRustServer(jsonRpcRequest);
        
        res.writeHead(200, { 
          'Content-Type': 'application/json',
          'Access-Control-Allow-Origin': '*'
        });
        res.end(JSON.stringify(response));

      } catch (error) {
        console.error('Request handling error:', error);
        res.writeHead(error.message === 'Request timeout' ? 504 : 500, { 
          'Content-Type': 'application/json' 
        });
        res.end(JSON.stringify({ 
          error: error.message === 'Request timeout' ? 'Gateway timeout' : 'Internal server error',
          message: error.message 
        }));
      }
    });
    
    return;
  }
  
  if (req.method === 'DELETE') {
    // Cleanup or shutdown endpoint
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ status: 'ok', message: 'Session closed' }));
    return;
  }
  
  if (req.method === 'OPTIONS') {
    // CORS preflight
    res.writeHead(204, {
      'Access-Control-Allow-Origin': '*',
      'Access-Control-Allow-Methods': 'GET, POST, DELETE, OPTIONS',
      'Access-Control-Allow-Headers': 'Content-Type'
    });
    res.end();
    return;
  }

  // Method not allowed
  res.writeHead(405, { 'Content-Type': 'application/json' });
  res.end(JSON.stringify({ error: 'Method not allowed' }));
});

// Start Rust server before listening
startRustServer();

// Start the HTTP server
server.listen(PORT, () => {
  console.log(`MCP HTTP wrapper listening on port ${PORT}`);
  console.log(`Forwarding to Rust MCP server with config:`, {
    downloadDir: DOWNLOAD_DIR,
    logLevel: LOG_LEVEL
  });
});

// Graceful shutdown
function shutdown() {
  console.log('Shutting down gracefully...');
  server.close(() => {
    console.log('HTTP server closed');
    if (rustServer) {
      rustServer.kill('SIGTERM');
      setTimeout(() => {
        if (rustServer) {
          rustServer.kill('SIGKILL');
        }
        process.exit(0);
      }, 5000);
    } else {
      process.exit(0);
    }
  });
}

process.on('SIGTERM', shutdown);
process.on('SIGINT', shutdown);

