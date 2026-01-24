; examples/11_errors.em
; Error handling demonstrations
; NOTE: This file intentionally contains errors for demonstration
; Uncomment one section at a time to see different error messages

; Stack underflow error
; + print  ; Error: not enough values on stack

; Division by zero
; 10 0 / print  ; Error: division by zero

; Type mismatch
; "hello" 5 + print  ; Error: cannot add string and integer

; Out of bounds list access
; { 1 2 3 } 10 nth print  ; Error: index out of bounds

; Empty list operations
; { } head print  ; Error: head of empty list

; Undefined word
; nonexistent print  ; Error: undefined word

; Valid examples demonstrating error recovery with conditionals
def safe-divide [
    dup 0 =
    [ drop drop "Error: division by zero" ]
    [ / to-string ]
    if
] end

10 0 safe-divide print  ; => "Error: division by zero"
10 2 safe-divide print  ; => "5"

; Safe list access
def safe-nth [
    swap dup len rot  ; list list_len index
    dup rot >         ; list index (index > len)
    [ drop drop "Error: index out of bounds" ]
    [ nth to-string ]
    if
] end

{ 1 2 3 } 1 safe-nth print  ; => "2"
{ 1 2 3 } 10 safe-nth print ; => "Error: index out of bounds"
