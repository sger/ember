; examples/07_combinators.em
; Advanced combinators

; dip: execute quotation under top of stack
1 2 [ 3 + ] dip print  ; Stack: 4 2 (prints 2)

; keep: execute quotation preserving value
5 [ 2 * ] keep print   ; Stack: 10 5 (prints 5)

; bi: apply two quotations to same value
5 [ 2 * ] [ 3 + ] bi   ; Stack: 10 8
print print            ; prints 8, then 10

; tri: apply three quotations to same value
5 [ 1 + ] [ 2 * ] [ 3 - ] tri  ; Stack: 6 10 2
print print print      ; prints 2, 10, 6
