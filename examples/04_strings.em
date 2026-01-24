; examples/04_strings.em
; String manipulation

"Hello" " World" . print         ; => "Hello World"
"Hello World" chars len print          ; => 11
"Hello" upper print              ; => "HELLO"
"WORLD" lower print              ; => "world"

; String operations
"one,two,three" "," split print  ; => { "one" "two" "three" }
{ "a" "b" "c" } "," join print   ; => "a,b,c"
