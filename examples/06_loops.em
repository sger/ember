; examples/06_loops.em
; Iteration with times and each

5 [ "Hello!" print ] times     ; Prints "Hello!" 5 times

{ 1 2 3 4 5 } [ dup print ] each  ; Prints each element

; Building a factorial function
def factorial [
    dup 1 <=
    [ drop 1 ]
    [ dup 1 - factorial * ]
    if
] end

5 factorial print  ; => 120
