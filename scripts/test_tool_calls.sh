#!/bin/bash
set -e

echo "Testing tool calls..."

# Test resolve-library-id first
{
    echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{"tools":{}},"clientInfo":{"name":"test-client","version":"1.0.0"}}}'
    echo '{"jsonrpc":"2.0","method":"notifications/initialized"}'
    echo '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"resolve-library-id","arguments":{"libraryName":"nix"}}}'
    sleep 3
    echo '{"jsonrpc":"2.0","method":"exit"}'
} | ../target/release/c67-mcp