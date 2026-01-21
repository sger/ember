; Factorial - recursive

def factorial
    dup 1 <= [drop 1] [
        dup 1 - factorial *
    ] if
end

; Calculate 10!
10 factorial print      ; 3628800
