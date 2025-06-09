# Simple Calculator Program
# Demonstrates basic arithmetic operations

# Addition: 5 + 3 = 8
push 5
push 3
add
print

# Subtraction: 10 - 4 = 6
push 10
push 4
sub
print

# Multiplication: 7 * 6 = 42
push 7
push 6
mul
print

# Division: 20 / 5 = 4
push 20
push 5
div
print

# Modulo: 17 % 5 = 2
push 17
push 5
mod
print

# More complex calculation: (10 + 5) * 2 = 30
push 10
push 5
add
push 2
mul
print

# Demonstration of dup and swap
# Calculate (7 - 3) * (7 + 3)
push 7  # a
push 3  # b

# Compute a - b
dup     # b, b
swap    # b, b
push 7  # b, b, a
swap    # b, a, b
sub     # b, (a-b)

# Compute a + b
swap    # (a-b), b
push 7  # (a-b), b, a
add     # (a-b), (a+b)

# Multiply (a-b) * (a+b)
mul     # (a-b)*(a+b)
print   # should print 40 (7-3)*(7+3) = 4*10 = 40

# Exit program
halt 