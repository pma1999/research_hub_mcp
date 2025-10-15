# Smithery Deployment Guide

This MCP server is compatible with [Smithery](https://smithery.ai), a platform for deploying Model Context Protocol servers.

## Architecture

This deployment uses a hybrid architecture to bridge Smithery's HTTP requirement with the Rust server's stdio transport:

```
Smithery HTTP → Node.js Wrapper → Rust MCP Server (stdio)
```

### Components

1. **Rust MCP Server** (`rust-research-mcp`)
   - Core server implementation in Rust
   - Communicates via stdio (standard input/output)
   - Handles all research, download, and metadata operations

2. **Node.js HTTP Wrapper** (`mcp-http-wrapper.js`)
   - Bridges HTTP to stdio transport
   - Maintains persistent connection to Rust server
   - Handles request/response mapping
   - Provides health check endpoint

3. **Docker Container**
   - Multi-stage build (Rust + Node.js)
   - Optimized for production deployment
   - Runs on port 3000 by default

## Configuration

The server accepts the following configuration parameters:

- **DOWNLOAD_DIR**: Directory for downloaded papers (default: `/data`)
- **LOG_LEVEL**: Logging level - `trace`, `debug`, `info`, `warn`, `error` (default: `info`)

These are configured through the Smithery UI when deploying the server.

## Deployment

### Via Smithery Registry

1. Visit [Smithery](https://smithery.ai)
2. Search for "rust-research-mcp" or "Ladvien/research_hub_mcp"
3. Click "Deploy"
4. Configure your settings:
   - Set `DOWNLOAD_DIR` to your preferred location (or use default `/data`)
   - Set `LOG_LEVEL` to your preferred verbosity (or use default `info`)
5. Deploy and connect to your AI client

### Manual Docker Build

```bash
# Build the image
docker build -t rust-research-mcp .

# Run the container
docker run -p 3000:3000 \
  -e DOWNLOAD_DIR=/data \
  -e LOG_LEVEL=info \
  -v $(pwd)/downloads:/data \
  rust-research-mcp
```

## Endpoints

The HTTP wrapper exposes the following endpoints:

### `GET /mcp`
Health check endpoint. Returns server status.

**Response:**
```json
{
  "status": "ok",
  "server": "rust-research-mcp",
  "transport": "http-to-stdio-bridge",
  "rustServerRunning": true
}
```

### `POST /mcp`
Main MCP endpoint for JSON-RPC requests. Forwards requests to the Rust server.

**Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/list",
  "params": {}
}
```

### `DELETE /mcp`
Session cleanup endpoint.

### `OPTIONS /mcp`
CORS preflight handler.

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `PORT` | HTTP server port | `3000` |
| `DOWNLOAD_DIR` | Paper download directory | `/data` |
| `LOG_LEVEL` | Logging verbosity | `info` |
| `RUST_LOG` | Rust logger configuration | `info` |

## Files

- **`smithery.yaml`**: Smithery deployment configuration
- **`Dockerfile`**: Multi-stage Docker build
- **`mcp-http-wrapper.js`**: HTTP-to-stdio bridge
- **`package.json`**: Node.js wrapper dependencies
- **`.dockerignore`**: Docker build optimization

## Development

### Testing Locally

1. Build the Rust binary:
   ```bash
   cargo build --release
   ```

2. Run the wrapper:
   ```bash
   export DOWNLOAD_DIR=/tmp/papers
   export LOG_LEVEL=debug
   node mcp-http-wrapper.js
   ```

3. Test the endpoint:
   ```bash
   curl -X POST http://localhost:3000/mcp \
     -H "Content-Type: application/json" \
     -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0.0"}}}'
   ```

### Building Docker Image

```bash
docker build -t rust-research-mcp:latest .
docker run -p 3000:3000 rust-research-mcp:latest
```

## Troubleshooting

### Container fails to start

Check logs for errors:
```bash
docker logs <container-id>
```

Common issues:
- Missing environment variables
- Port already in use
- Insufficient permissions for `/data` directory

### Rust server not responding

The wrapper maintains a persistent connection to the Rust server. If the Rust server crashes, check:

1. Rust server logs (stderr output)
2. Wrapper logs for connection errors
3. System resources (memory, disk space)

### Timeout errors

Default timeout is 30 seconds. For long-running operations (large downloads), the server handles this gracefully but may need adjustment in production.

## Security

- Container runs as non-root user (`mcp`, UID 10001)
- HTTPS-only connections to external services
- Certificate validation enabled
- No sensitive data in environment variables (use Smithery secrets for API keys)

## Performance

- Multi-stage Docker build minimizes image size
- Persistent Rust server process reduces startup overhead
- Async I/O for concurrent request handling
- Request queuing with timeout protection

## Support

For issues, questions, or contributions:
- GitHub: [Ladvien/research_hub_mcp](https://github.com/Ladvien/research_hub_mcp)
- Issues: [GitHub Issues](https://github.com/Ladvien/research_hub_mcp/issues)

## License

GPL-3.0 - See LICENSE file for details

