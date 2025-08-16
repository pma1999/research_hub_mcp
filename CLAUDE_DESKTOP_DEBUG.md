# Claude Desktop MCP Debug Guide

## Current Status

✅ **Configurations Created**:
- Primary: `~/Library/Application Support/Claude/claude_desktop_config.json`
- Alternative: `~/.config/claude-desktop/config.json`
- Both include rust-sci-hub-mcp with debug logging enabled

✅ **Claude Desktop Restarted**: All application processes terminated

## Next Steps

### 1. Start Claude Desktop
Open Claude Desktop fresh from Applications folder or Dock.

### 2. Check for MCP Server Loading
Look for the rust-sci-hub-mcp tools in Claude Desktop interface:
- Search tools should appear
- Try asking: "Search for papers about quantum computing"

### 3. Debug if Still Not Working

#### Check Claude Desktop Console (if available)
- Look for MCP server initialization messages
- Check for any rust-sci-hub-mcp startup errors

#### Test MCP Server Manually
```bash
# Test that the server starts correctly
rust-sci-hub-mcp --config ~/.config/rust-sci-hub-mcp/config.toml

# Should show:
# INFO Starting rust-sci-hub-mcp server
# INFO Loading config from: /Users/ladvien/.config/rust-sci-hub-mcp/config.toml
# INFO Loaded configuration: profile=development, schema_version=1.0
# INFO Starting MCP server infrastructure
# INFO Initializing SciHub MCP server handler
# INFO Initialized mirror manager with 3 mirrors
# INFO Server running - waiting for shutdown signal
```

#### Verify File Permissions
```bash
# Check config files are readable
ls -la ~/Library/Application\ Support/Claude/claude_desktop_config.json
ls -la ~/.config/claude-desktop/config.json
ls -la ~/.config/rust-sci-hub-mcp/config.toml

# Check binary is executable
ls -la /opt/homebrew/bin/rust-sci-hub-mcp
```

#### Alternative Configuration (Simplified)
If still not working, try this minimal configuration in `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "rust-sci-hub-mcp": {
      "command": "/opt/homebrew/bin/rust-sci-hub-mcp"
    }
  }
}
```

### 4. Check Claude Desktop Version
Some MCP features require specific Claude Desktop versions. Verify you have a recent version that supports MCP servers.

### 5. Restart Process
If changes are made to configuration:
1. Quit Claude Desktop completely
2. Wait 5 seconds
3. Restart Claude Desktop
4. Test MCP integration

## Expected Behavior

When working correctly, you should be able to:

1. **Ask Claude to search**: "Search for papers about machine learning"
2. **Download papers**: "Download the paper with DOI 10.1038/nature12373"  
3. **Extract metadata**: "Get citation information from downloaded papers"

## Troubleshooting

### Issue: "No tools found" or MCP not working
- Verify Claude Desktop version supports MCP
- Check config file syntax with: `python3 -m json.tool config_file.json`
- Try the simplified configuration above

### Issue: "Binary not found"
- Run: `which rust-sci-hub-mcp`
- Should return: `/opt/homebrew/bin/rust-sci-hub-mcp`
- If not found: `brew reinstall rust-sci-hub-mcp`

### Issue: "Configuration errors"
- Check: `rust-sci-hub-mcp --config ~/.config/rust-sci-hub-mcp/config.toml --help`
- Verify Sci-Hub mirrors are configured (not empty array)

## Files Created/Modified

1. **Primary Config**: `~/Library/Application Support/Claude/claude_desktop_config.json`
   - Added rust-sci-hub-mcp with debug logging
   - Maintained existing bevy_brp server

2. **Alternative Config**: `~/.config/claude-desktop/config.json`  
   - Standalone rust-sci-hub-mcp configuration
   - For Claude Desktop versions that use this location

3. **MCP Server Config**: `~/.config/rust-sci-hub-mcp/config.toml`
   - Working configuration with Sci-Hub mirrors
   - Verified to start server successfully

## Success Indicators

✅ Claude Desktop shows MCP tools in interface
✅ Can ask Claude to search for papers
✅ rust-sci-hub-mcp appears in tool listings
✅ Search, download, and metadata tools are available