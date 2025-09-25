#!/bin/bash
set -e

echo "Testing tool calls with longer delay..."

{
    echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{"tools":{}},"clientInfo":{"name":"test-client","version":"1.0.0"}}}'
    sleep 1
    echo '{"jsonrpc":"2.0","method":"notifications/initialized"}'
    sleep 1
    echo '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"resolve-library-id","arguments":{"libraryName":"nix"}}}' 
    sleep 5
} | ../target/release/c67-mcp