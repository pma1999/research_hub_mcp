# Claude Desktop Setup Guide

## 🚨 Important: macOS Permissions Required

Claude Desktop runs in a sandbox for security, which can prevent file downloads. If you see this error:

```
Error: IO error: Read-only file system (os error 30)
```

You need to grant Claude Desktop folder access permissions.

## 📋 Step-by-Step Setup

### 1. Grant Folder Permissions (Required)

**Option A: Enable Downloads Folder Access**
1. Open **System Settings** → **Privacy & Security** → **Files and Folders**
2. Find **Claude** in the application list
3. Enable **Downloads Folder** permission
4. Restart Claude Desktop

**Option B: Use Documents Folder (Recommended)**
- Documents folder typically has fewer restrictions
- Update your config to use `~/Documents/Research-Papers`

### 2. Configure Claude Desktop

Add this to your Claude Desktop config file:

**Location:** `~/Library/Application Support/Claude/claude_desktop_config.json`

```json
{
  "mcpServers": {
    "rust-research-mcp": {
      "command": "/Users/ladvien/.cargo/bin/rust-research-mcp",
      "args": [
        "--config", "/Users/ladvien/sci_hub_mcp/config.toml",
        "--log-level", "info"
      ],
      "env": {
        "RSH_DOWNLOAD_DIRECTORY": "/Users/ladvien/Documents/Research-Papers"
      }
    }
  }
}
```

### 3. Test the Setup

In Claude Desktop, try:
```
Download paper with DOI: 10.1186/s13643-017-0491-x
```

## 🔧 Troubleshooting

### Permission Denied Errors
- **Cause**: Claude Desktop doesn't have folder access
- **Fix**: Grant permissions in System Settings as described above

### Directory Not Found
- **Cause**: Target directory doesn't exist or isn't accessible
- **Fix**: Use Documents folder or create the directory manually:
  ```bash
  mkdir -p ~/Documents/Research-Papers
  ```

### Tool Not Found
- **Cause**: Wrong binary path or installation issue
- **Fix**: Verify installation:
  ```bash
  which rust-research-mcp
  rust-research-mcp --version  # Should show v0.4.2
  ```

### Still Having Issues?

1. **Check logs**: Look in Claude Desktop's logs for detailed error messages
2. **Try alternative directory**: Use `/tmp/papers` as a test:
   ```json
   "RSH_DOWNLOAD_DIRECTORY": "/tmp/papers"
   ```
3. **Verify binary**: Ensure you're using the latest version (v0.4.2)

## 🎯 Recommended Configuration

For the best experience, use this production-ready setup:

```toml
# config.toml
[downloads]
directory = "~/Documents/Research-Papers"
max_concurrent = 3
max_file_size_mb = 100

[logging]
level = "info"
format = "text"
```

## 📱 Testing Your Setup

Once configured, you should see Claude Desktop report:
- ✅ 5 tools available (search, download, metadata, code_search, bibliography)
- ✅ Clear permission error messages if folder access is needed
- ✅ Successful downloads to your configured directory

## 🚀 Available Tools

After setup, you can use:
- **search_papers**: "Find papers on machine learning"
- **download_paper**: "Download DOI 10.1000/example"
- **extract_metadata**: "Extract info from downloaded.pdf"
- **search_code**: "Search for 'def train_model' in papers"
- **generate_bibliography**: "Create BibTeX for these DOIs"

Happy researching! 🔬