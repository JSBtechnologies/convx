# ConvX MCP Server

ConvX includes a built-in [Model Context Protocol](https://modelcontextprotocol.io/) (MCP) server that enables AI assistants to perform local file conversions.

## Quick Setup

### Claude Desktop

Add to `~/Library/Application Support/Claude/claude_desktop_config.json` (macOS) or `%APPDATA%\Claude\claude_desktop_config.json` (Windows):

```json
{
  "mcpServers": {
    "convx": {
      "command": "convx",
      "args": ["mcp"]
    }
  }
}
```

### Cursor

Add to your Cursor MCP settings:

```json
{
  "mcpServers": {
    "convx": {
      "command": "convx",
      "args": ["mcp"]
    }
  }
}
```

### Claude Code

```bash
claude mcp add convx -- convx mcp
```

## Build from Source

If you're developing locally, build the MCP binary directly:

```bash
cargo build --bin convx-mcp --no-default-features
```

Then point your config to the built binary:

```json
{
  "mcpServers": {
    "convx": {
      "command": "/path/to/convx/target/release/convx-mcp",
      "args": []
    }
  }
}
```

## Available Tools

| Tool | Description |
|------|-------------|
| `usage-guide` | Returns a built-in guide for effective ConvX MCP usage |
| `convert_file` | Convert a single file with full option control |
| `batch_convert` | Convert multiple files in one call |
| `get_supported_formats` | List all 53 formats grouped by category |
| `get_conversion_targets` | Get valid output formats for a given input format |
| `can_convert` | Check if a specific conversion path is supported |
| `get_file_info` | Get file metadata (size, format, duration, codecs, resolution) |
| `list_presets` | List all 18 built-in conversion presets |
| `get_preset` | Get detailed settings for a specific preset |
| `check_dependencies` | Verify system dependencies are installed |

### Tool Examples

**Convert a file:**
```
Tool: convert_file
Input: { "input_path": "/Users/me/photo.heic", "output_format": "jpg", "quality": 90 }
```

**Use a preset:**
```
Tool: convert_file
Input: { "input_path": "/Users/me/clip.mp4", "preset": "discord" }
```

**Batch convert:**
```
Tool: batch_convert
Input: { "input_paths": ["/tmp/a.png", "/tmp/b.png"], "output_format": "webp", "quality": 80 }
```

**Check what a format can convert to:**
```
Tool: get_conversion_targets
Input: { "input_format": "csv" }
```

## Verify the Server

Test that the MCP server responds correctly:

```bash
printf '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"0.1"}}}\n' | convx mcp
```

You should see a JSON response containing `result.serverInfo`.

## Troubleshooting

### "No such file or directory"

The MCP client can't find the `convx` binary. Ensure it's in your PATH:

```bash
which convx
# Should output: /usr/local/bin/convx or similar
```

If installed via the desktop app, the binary is symlinked during installation. You may need to restart your terminal.

### "Unexpected token" or framing errors

ConvX auto-detects the MCP transport framing (JSON-RPC newline-delimited). If you see framing errors:

1. Rebuild the binary: `cargo build --release`
2. Fully restart your AI assistant (not just reload)
3. Remove and re-add the MCP server entry in your config

### Dependencies missing

Run `convx check` to verify all system dependencies are installed. The MCP server requires the same dependencies as the CLI.

### Transport

ConvX MCP uses **stdio** transport. It reads JSON-RPC messages from stdin and writes responses to stdout. The server auto-detects whether the client uses Content-Length framing or newline-delimited JSON.
