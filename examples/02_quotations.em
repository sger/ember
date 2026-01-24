; examples/02_quotations.em
; Working with quotations (code blocks)

; Simple quotation execution
5 [ dup * ] call print         ; => 25

; Define a word
def square [dup *] end
7 square print                  ; => 49

; Using keep combinator (preserves value)
5 [ dup * ] keep print print   ; => 5 25
