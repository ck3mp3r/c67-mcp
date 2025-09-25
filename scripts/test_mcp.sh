#!/bin/bash
set -e

echo "Testing MCP server communication..."

# Start the server in background
../target/release/c67-mcp &
SERVER_PID=$!

sleep 1

# Function to cleanup
cleanup() {
    kill $SERVER_PID 2>/dev/null || true
}
trap cleanup EXIT

# Test the server by sending messages via stdin
{
    echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{"tools":{}},"clientInfo":{"name":"test-client","version":"1.0.0"}}}'
    sleep 0.5
    echo '{"jsonrpc":"2.0","method":"notifications/initialized"}'
    sleep 0.5
    echo '{"jsonrpc":"2.0","id":2,"method":"tools/list"}'
    sleep 0.5
    echo '{"jsonrpc":"2.0","method":"exit"}'
} | ../target/release/c67-mcp

echo "Server test completed"