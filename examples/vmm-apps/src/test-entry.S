    .global _start
    .type _start, @function
_start:
    mrs   x0, CurrentEL
    mov   x1, #'0'
    add   x0, x0, x1
    hvc   #0
    b     .
.end _start, . - _start

