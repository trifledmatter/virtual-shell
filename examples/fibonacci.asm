# Fibonacci Sequence Generator
# Calculates and prints the first 10 Fibonacci numbers

# Store the number of Fibonacci numbers to generate
push 10
store 0

# Initialize the first two Fibonacci numbers
push 0  # First Fibonacci number
push 1  # Second Fibonacci number

# Print the first Fibonacci number
dup
swap
print

# Print the second Fibonacci number
dup
print

# Initialize counter
push 2  # Already printed 2 numbers
store 1

loop:
  # Calculate next Fibonacci number (a + b)
  dup    # Duplicate b
  swap   # Swap to get a on top
  dup    # Duplicate a
  swap   # Get back to a, b, b
  add    # Add a + b to get next number

  # Print the new Fibonacci number
  dup
  print

  # Check if we should continue
  load 1  # Load counter
  push 1
  add     # Increment counter
  dup
  store 1 # Store updated counter
  
  # Compare counter with target count
  load 0  # Load target count
  swap
  cmp     # Compare counter with target
  push -1
  cmp     # Check if result is -1 (counter < target)
  jumpifz loop  # If counter < target, continue loop

  # Exit program
  halt 