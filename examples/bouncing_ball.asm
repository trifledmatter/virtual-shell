# Bouncing Ball Animation (single frame, can be extended for animation)
# Draws a 10x5 box with a ball at (x=4, y=2)
# Box: +--------+
#       |        |
#       |   o    |
#       |        |
#       +--------+

# Print top border
    push 43         # '+'
    printchar
    push 8
:top_loop
    push 45         # '-'
    printchar
    dup
    push 1
    sub
    dup
    jumpifz top_end
    jump top_loop
:top_end
    pop
    push 43         # '+'
    printchar
    push 10         # newline
    printchar

# Print rows
    push 0          # row = 0
:row_loop
    dup
    push 5
    cmp
    push row_end
    jumpifz

    # Print left border
    push 124        # '|'
    printchar

    # Print columns
    push 0          # col = 0
:col_loop
        dup
        push 8
        cmp
        push col_end
        jumpifz

        # Ball at (x=4, y=2)
        dup              # col
        push 4
        cmp
        dup
        push 0
        jumpifz col_check_y
        pop
        push 32         # ' '
        printchar
        jump col_next

:col_check_y
        pop              # remove cmp result
        swap             # stack: row col
        dup
        push 2
        cmp
        push col_ball
        jumpifz
        pop
        push 32         # ' '
        printchar
        jump col_next

:col_ball
        pop
        push 111        # 'o'
        printchar

:col_next
        swap
        push 1
        add
        swap
        jump col_loop

:col_end
    pop

    # Print right border
    push 124        # '|'
    printchar
    push 10         # newline
    printchar

    # Next row
    swap
    push 1
    add
    swap
    jump row_loop

:row_end
    pop

# Print bottom border
    push 43         # '+'
    printchar
    push 8
:bot_loop
    push 45         # '-'
    printchar
    dup
    push 1
    sub
    dup
    jumpifz bot_end
    jump bot_loop
:bot_end
    pop
    push 43         # '+'
    printchar
    push 10
    printchar

halt
