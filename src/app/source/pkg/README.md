# TrifledOS Terminal WebAssembly Module

A virtual terminal with CPU assembly language support, compiled to WebAssembly for use in web applications.

## Features

- Full virtual file system with standard Unix-like commands
- Custom CPU assembly language with stack-based execution
- Environment variables and shell scripting support
- Process management simulation
- File permissions and ownership
- Completely isolated and browser-safe

## Building the WASM Module

### Prerequisites

1. Install Rust and Cargo: https://rustup.rs/
2. Install wasm-pack:
   ```bash
   curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
   ```

### Build Commands

```bash
# Build for web (recommended for React apps)
wasm-pack build --target web --out-dir pkg

# Build for bundlers (webpack, etc.)
wasm-pack build --target bundler --out-dir pkg

# Build for Node.js
wasm-pack build --target nodejs --out-dir pkg
```

## Using in a React Application

### 1. Install the built package

After building, you can either:

- Copy the `pkg` folder to your React project
- Or publish to npm and install normally

### 2. Initialize in your React component

```typescript
import init, { Terminal } from './path-to-pkg/source';

// Initialize WASM module once
await init();

// Create terminal instance
const terminal = new Terminal();
```

### 3. Execute commands

```typescript
const response = terminal.execute_command('ls -la');
if (response.success) {
  console.log(response.output);
} else {
  console.error(response.output);
}
```

### 4. Work with files

```typescript
// List files
const files = terminal.list_files('/home');

// Read a file
const content = terminal.read_file('/home/example.txt');

// Write a file
terminal.write_file('/home/new.txt', 'Hello, World!');
```

### 5. CPU Assembly Language

```typescript
// Get assembly template
const template = get_assembly_template('hello');

// Create an assembly file
terminal.write_file('hello.asm', template);

// Run the assembly program
const result = terminal.execute_command('cpu run hello.asm');
```

## Available Commands

- **File System**: `ls`, `mkdir`, `touch`, `rm`, `cp`, `mv`, `pwd`, `ln`
- **Text Processing**: `echo`, `grep`, `sed`
- **Permissions**: `chmod`, `chown`, `chgrp`
- **Process Management**: `ps`, `kill`, `killall`
- **Environment**: `export`, `env`, `alias`, `unalias`
- **Scripting**: `source`, `set`, `functions`
- **Assembly**: `cpu new`, `cpu run`, `cpu help`

## Example React Component

See `examples/react-terminal/Terminal.tsx` for a complete example of integrating the terminal into a React application.

## Assembly Language Reference

The CPU command provides a simple stack-based assembly language:

```assembly
# Basic arithmetic
push 10
push 20
add
print    # Outputs: 30

# Hello World
push 72  # 'H'
printchar
push 101 # 'e'
printchar
# ... etc

# Loops and conditionals
loop:
  dup
  print
  push 1
  sub
  dup
  jumpif loop
```

See `docs/assembly.md` for complete documentation.

## TypeScript Support

TypeScript declarations are included. Import types:

```typescript
import { Terminal, CommandResponse, FileInfo } from './path-to-pkg/source';
```

## Performance Notes

- The WASM module is optimized for size (`opt-level = "s"`)
- File system operations are all in-memory
- No network or actual file system access
- Suitable for educational and demonstration purposes

## License

MIT 