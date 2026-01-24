; examples/08_recursion.em
; Recursive functions

; Fibonacci
def fib [
    dup 2 <
    [ ]
    [ dup 1 - fib swap 2 - fib + ]
    if
] end

10 fib print  ; => 55

; Sum of list (recursive)
def sum-list [
    dup { } =
    [ drop 0 ]
    [ dup head swap tail sum-list + ]
    if
] end

{ 1 2 3 4 5 } sum-list print  ; => 15
