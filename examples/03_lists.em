; examples/03_lists.em
; List operations

; Basic list operations
{ 1 2 3 4 5 } len print                  ; => 5
{ 1 2 3 4 5 } [ 2 * ] map print          ; => { 2 4 6 8 10 }
{ 1 2 3 4 5 } [ 2 % 0 = ] filter print   ; => { 2 4 }
{ 1 2 3 4 5 } 0 [ + ] fold print         ; => 15 (sum)

; List construction
{ } 1 append 2 append 3 append print  ; => { 1 2 3 }
