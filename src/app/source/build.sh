#!/bin/bash

# builds the wasm module, nothing fancy

echo "Building TrifledOS Terminal WASM module..."

# bail if wasm-pack missing
if ! command -v wasm-pack &> /dev/null; then
    echo "Error: wasm-pack is not installed."
    echo "Please install it with: curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh"
    exit 1
fi

# nuke old builds
echo "Cleaning previous builds..."
rm -rf pkg/

# do the actual build
echo "Building for web target..."
wasm-pack build --target web --out-dir pkg

# get rid of auto-generated gitignore
rm pkg/.gitignore

# check exit code and give feedback
if [ $? -eq 0 ]; then
    echo "Build successful! Output in pkg/ directory"
    echo ""
    echo "To use in your React app:"
    echo "1. Copy the pkg/ directory to your React project"
    echo "2. Import with: import init, { Terminal } from './pkg/source';"
    echo "3. Initialize with: await init();"
    echo ""
    echo "See README.md for detailed usage instructions."
else
    echo "Build failed!"
    exit 1
fi 