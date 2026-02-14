#!/bin/bash
# Build script for Zaco Rust Runtime

set -e

echo "Building Zaco Rust Runtime..."
cargo build --release

echo ""
echo "✓ Static library built at: target/release/libzaco_runtime_rs.a"
echo ""
echo "To link with your program, use:"
echo "  cc -o program program.c target/release/libzaco_runtime_rs.a \\"
echo "     -framework CoreFoundation -framework Security -lpthread -ldl"
echo ""
echo "On Linux, use:"
echo "  cc -o program program.c target/release/libzaco_runtime_rs.a \\"
echo "     -lpthread -ldl -lm"
