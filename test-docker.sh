#!/bin/bash

# Test script for Docker deployment
# This script builds and tests the Docker container locally before Smithery deployment

set -e

echo "🔨 Building Docker image..."
docker build -t rust-research-mcp:test .

echo ""
echo "✅ Build successful!"
echo ""

echo "🚀 Starting container..."
CONTAINER_ID=$(docker run -d \
  -p 3000:3000 \
  -e DOWNLOAD_DIR=/data \
  -e LOG_LEVEL=debug \
  -v "$(pwd)/test-downloads:/data" \
  rust-research-mcp:test)

echo "Container ID: $CONTAINER_ID"
echo ""

# Wait for server to start
echo "⏳ Waiting for server to start..."
sleep 5

echo ""
echo "🧪 Testing health endpoint..."
curl -s http://localhost:3000/mcp | jq .

echo ""
echo ""
echo "🧪 Testing MCP initialize..."
curl -s -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "initialize",
    "params": {
      "protocolVersion": "2024-11-05",
      "capabilities": {},
      "clientInfo": {
        "name": "test",
        "version": "1.0.0"
      }
    }
  }' | jq .

echo ""
echo ""
echo "🧪 Testing tools/list..."
curl -s -X POST http://localhost:3000/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 2,
    "method": "tools/list",
    "params": {}
  }' | jq .

echo ""
echo ""
echo "📊 Container logs:"
echo "===================="
docker logs "$CONTAINER_ID" | tail -20

echo ""
echo ""
echo "🛑 Stopping container..."
docker stop "$CONTAINER_ID"
docker rm "$CONTAINER_ID"

echo ""
echo "✅ All tests completed successfully!"
echo ""
echo "📦 Image size:"
docker images rust-research-mcp:test --format "{{.Size}}"

echo ""
echo "🎉 Ready for Smithery deployment!"

