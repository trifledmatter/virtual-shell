# Factorial calculation program
# Calculates the factorial of a number (5! = 120)

# Initialize result to 1
push 1

# Initialize counter to 5
push 5

# Start of the factorial calculation loop
loop:
  # Duplicate the counter to use in multiplication
  dup
  
  # Multiply result by counter (result = result * counter)
  mul
  
  # Decrement counter (counter = counter - 1)
  push 1
  swap
  sub
  
  # Duplicate counter to check if we're done
  dup
  
  # If counter > 0, continue loop
  # Otherwise proceed to end
  push 0
  swap
  cmp
  push 1
  cmp  # This will push 0 if counter > 0
  jumpifz loop
  
  # Remove counter from stack
  pop
  
  # Print result
  print
  
  # End program
  halt 