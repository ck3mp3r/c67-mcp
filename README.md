# c67-mcp

A Rust implementation of a [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) server for [Context7](https://context7.com/) - providing AI assistants with access to up-to-date documentation for any library or framework.

## Overview

This MCP server enables AI assistants like Claude to search for and retrieve current documentation from Context7's extensive database of libraries and frameworks. Context7 provides fresh, comprehensive documentation that's continuously updated, making it an excellent resource for development assistance.

**Key Features:**
- 🔍 **Library Search**: Find documentation by library/package name
- 📚 **Documentation Retrieval**: Get up-to-date docs with customizable token limits
- 🎯 **Topic Filtering**: Focus on specific aspects like "installation", "hooks", or "routing"
- 🦀 **Rust Performance**: Fast, memory-efficient implementation
- 🔒 **Static Binaries**: Self-contained executables with rustls (no OpenSSL dependency)
- 🌐 **Cross-Platform**: Supports macOS (ARM64/Intel), Linux, and Windows

## Architecture

This is a Rust alternative to the [TypeScript Context7 MCP server](https://github.com/upstash/context7). It implements the MCP protocol to provide two main tools:

1. **`resolve-library-id`**: Search for libraries and get Context7-compatible IDs
2. **`get-library-docs`**: Fetch documentation content for a specific library

## Installation

### Pre-built Binaries

Download the latest release for your platform from the [releases page](https://github.com/ck3mp3r/c67-mcp/releases).

### Using Nix

```bash
# Build from source
nix build github:ck3mp3r/c67-mcp

# Or install to your profile
nix profile install github:ck3mp3r/c67-mcp
```

### From Source

```bash
git clone https://github.com/ck3mp3r/c67-mcp.git
cd c67-mcp
cargo build --release
```

## Usage

### Claude Desktop Configuration

Add this to your Claude Desktop MCP settings (`~/Library/Application Support/Claude/claude_desktop_config.json` on macOS):

```json
{
  "mcpServers": {
    "context7": {
      "command": "/path/to/c67-mcp",
      "args": ["--api-key", "your-context7-api-key-here"]
    }
  }
}
```

For development or testing environments where you need to disable TLS verification, add the `--insecure` flag:

```json
{
  "mcpServers": {
    "context7": {
      "command": "/path/to/c67-mcp",
      "args": ["--api-key", "your-context7-api-key-here", "--insecure"]
    }
  }
}
```

### Command Line Options

```bash
c67-mcp --help
```

**Options:**
- `--api-key <KEY>`: Context7 API key (optional for basic usage)
- `--log-level <LEVEL>`: Set logging level (trace, debug, info, warn, error)
- `--debug`: Enable debug logging
- `--verbose`: Increase verbosity
- `--insecure`: Disable TLS certificate verification (useful for development/testing)

### Example Usage in Claude

Once configured, you can ask Claude:

- *"Look up the latest documentation for React hooks"*
- *"Find documentation for Next.js routing"*
- *"Get installation instructions for Tailwind CSS"*

Claude will use the MCP server to search Context7 and retrieve current documentation.

## API Keys

While Context7 can be used without an API key, having one provides:
- Higher rate limits
- Access to premium documentation
- Better performance

Get your API key from [Context7](https://context7.com/).

## Development

### Prerequisites

- Rust 1.70+ (or use Nix development shell)
- [Optional] Nix for reproducible builds

### Development Shell

```bash
# Enter development environment with all tools
nix develop

# Or use direnv if you have it set up
echo "use flake" > .envrc && direnv allow
```

### Building

```bash
# Development build
cargo build

# Release build
cargo build --release

# Run tests
cargo test

# Run with logging
cargo run -- --debug --api-key your-key-here
```

### Testing

The project includes comprehensive tests:

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test file
cargo test --test integration_tests
```

## Project Structure

```
├── src/
│   ├── main.rs          # CLI interface and startup
│   ├── handler.rs       # Core MCP server implementation
│   └── lib.rs          # Library exports
├── tests/
│   ├── functional_tests.rs    # Basic functionality tests
│   └── integration_tests.rs   # HTTP integration tests with mocks
├── .github/workflows/  # CI/CD pipelines
├── flake.nix          # Nix build configuration
└── devshell.toml      # Development environment setup
```

## Contributing

1. Fork the repository
2. Create a feature branch: `git checkout -b feature-name`
3. Make your changes and add tests
4. Run tests: `cargo test`
5. Run formatting: `cargo fmt`
6. Run linting: `cargo clippy`
7. Submit a pull request

## Inspiration

This project was inspired by and follows patterns from [nu-mcp](https://github.com/ck3mp3r/nu-mcp), another Rust MCP server implementation.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Related Projects

- [Context7 TypeScript MCP](https://github.com/upstash/context7) - Official TypeScript implementation
- [Model Context Protocol](https://modelcontextprotocol.io/) - MCP specification
- [nu-mcp](https://github.com/ck3mp3r/nu-mcp) - Nushell MCP server (architectural inspiration)
