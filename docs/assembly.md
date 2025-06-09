# CPU Assembly Language Documentation

The CPU command provides a simple stack-based virtual machine with a small assembly language that can be used to write and execute programs within the console environment.

## Getting Started

### Creating a new assembly program

```
cpu new myprogram.asm
```

This creates a new file with a simple example program.

### Running an assembly program

```
cpu run myprogram.asm
```

### Getting help

```
cpu help
```

### Viewing detailed documentation

```
cpu docs
```

## Assembly Language Reference

### Basic Concepts

The virtual machine is stack-based, meaning operations work by pushing and popping values from a stack. It also has a simple memory system for storing values.

### Instruction Set

| Instruction    | Description                                                  |
|----------------|--------------------------------------------------------------|
| `push <n>`     | Push a value onto the stack                                  |
| `pop`          | Remove the top value from the stack                          |
| `add`          | Add the top two values on the stack                          |
| `sub`          | Subtract the top value from the second value                 |
| `mul`          | Multiply the top two values                                  |
| `div`          | Divide the second value by the top value                     |
| `mod`          | Calculate the modulus (remainder after division)             |
| `dup`          | Duplicate the top value on the stack                         |
| `swap`         | Swap the top two values on the stack                         |
| `load <addr>`  | Load a value from memory at the specified address            |
| `store <addr>` | Store the top value in memory at the specified address       |
| `jump <addr>`  | Jump to the specified instruction address                    |
| `jumpif <addr>`| Jump if the top value is non-zero                            |
| `jumpifz <addr>`| Jump if the top value is zero                               |
| `cmp`          | Compare top two values: 1 if a>b, 0 if a==b, -1 if a<b       |
| `print`        | Print the top value as a number                              |
| `printchar`    | Print the top value as an ASCII character                    |
| `read`         | Read an integer from input                                   |
| `halt`         | Stop program execution                                       |

### Labels

You can define labels in your code for easier jumps:

```
loop:
  # some code
  jump loop
```

Labels are resolved during assembly.

## Example Programs

Check the `examples/` directory for sample programs:

- `factorial.asm` - Calculates factorial of a number
- `fibonacci.asm` - Generates Fibonacci sequence
- `hello_world.asm` - Prints "Hello, World!"
- `calculator.asm` - Demonstrates arithmetic operations

## Advanced Techniques

### 1. Working with Memory

The VM has 1024 memory cells available. Use `store` and `load` to work with memory:

```
push 42
store 0  # Store 42 at address 0
load 0   # Push 42 back onto the stack
print    # Print 42
```

### 2. Creating Loops

Use labels and conditional jumps to create loops:

```
push 0   # Counter
push 10  # Limit

loop:
  dup
  print  # Print current counter
  
  push 1
  add    # Increment counter
  
  dup    # Duplicate counter for comparison
  push 10 # Compare with limit
  cmp    # Compare counter with limit
  push -1 # -1 means counter < limit
  cmp    # Check if previous result was -1
  jumpifz loop # Jump if counter < limit
  
halt
```

### 3. Implementing Functions

While there's no built-in function support, you can simulate functions using jumps:

```
# Call "function" at label 'square'
push 5   # Argument
jump square

# This is where we return to
retpoint:
print    # Print the result
halt

# Function to square a number
square:
  dup
  mul    # n * n
  jump retpoint  # Return to caller
```

## Limitations

- No floating-point support (integers only)
- No string handling beyond individual characters
- No built-in function call mechanism
- Limited to 1024 memory cells
- No I/O beyond basic numeric input/output 