; examples/09_practical.em
; Practical algorithms

; Find maximum in a list
def max-list [
    dup head swap tail swap
    [ over over > [ ] [ swap ] if drop ]
    fold
] end
{ 3 7 2 9 1 5 } max-list print  ; => 9

; Check if list contains element
def contains [
    swap                    ; value, list
    [ over = ] map          ; value, [list of bools]
    [ or ] false swap fold  ; Fold OR over the booleans
    swap drop               ; Drop the value
] end
{ 1 2 3 4 5 } 3 contains print  ; => true
{ 1 2 3 4 5 } 7 contains print  ; => false

; Reverse a list
def rev [
    { } swap [ swap append ] fold
] end
{ 1 2 3 4 5 } rev print  ; => { 5 4 3 2 1 }
