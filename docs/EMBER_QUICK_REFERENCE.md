# EMBER Quick Reference Card

A one-page reference for EMBER programming.

---

## Stack Operations

| Word | Stack Effect | Description | Example |
|------|--------------|-------------|---------|
| `dup` | `( a -- a a )` | Duplicate top | `5 dup` → `[5, 5]` |
| `drop` | `( a -- )` | Remove top | `5 3 drop` → `[5]` |
| `swap` | `( a b -- b a )` | Swap top two | `5 3 swap` → `[3, 5]` |
| `over` | `( a b -- a b a )` | Copy second to top | `5 3 over` → `[5, 3, 5]` |
| `rot` | `( a b c -- b c a )` | Rotate three | `1 2 3 rot` → `[2, 3, 1]` |

---

## Arithmetic

| Word | Stack Effect | Description | Example |
|------|--------------|-------------|---------|
| `+` | `( a b -- sum )` | Add | `5 3 +` → `[8]` |
| `-` | `( a b -- diff )` | Subtract | `10 3 -` → `[7]` |
| `*` | `( a b -- prod )` | Multiply | `4 5 *` → `[20]` |
| `/` | `( a b -- quot )` | Divide | `20 4 /` → `[5]` |
| `%` | `( a b -- rem )` | Modulo | `10 3 %` → `[1]` |
| `neg` | `( a -- -a )` | Negate | `5 neg` → `[-5]` |
| `abs` | `( a -- \|a\| )` | Absolute | `-5 abs` → `[5]` |

---

## Comparison

| Word | Stack Effect | Description | Example |
|------|--------------|-------------|---------|
| `=` | `( a b -- bool )` | Equal | `5 5 =` → `[true]` |
| `!=` | `( a b -- bool )` | Not equal | `5 3 !=` → `[true]` |
| `<` | `( a b -- bool )` | Less than | `3 5 <` → `[true]` |
| `>` | `( a b -- bool )` | Greater than | `5 3 >` → `[true]` |
| `<=` | `( a b -- bool )` | Less or equal | `3 5 <=` → `[true]` |
| `>=` | `( a b -- bool )` | Greater or equal | `5 5 >=` → `[true]` |

---

## Logic

| Word | Stack Effect | Description | Example |
|------|--------------|-------------|---------|
| `and` | `( a b -- bool )` | Logical AND | `true false and` → `[false]` |
| `or` | `( a b -- bool )` | Logical OR | `true false or` → `[true]` |
| `not` | `( a -- bool )` | Logical NOT | `true not` → `[false]` |

---

## Lists

| Word | Stack Effect | Description | Example |
|------|--------------|-------------|---------|
| `{ }` | `( -- list )` | Empty list | `{ }` → `[{}]` |
| `len` | `( list -- n )` | Length | `{1 2 3} len` → `[3]` |
| `head` | `( list -- elem )` | First element | `{1 2 3} head` → `[1]` |
| `tail` | `( list -- list )` | Rest of list | `{1 2 3} tail` → `[{2 3}]` |
| `nth` | `( list i -- elem )` | Get nth element | `{10 20 30} 1 nth` → `[20]` |
| `append` | `( list elem -- list )` | Add to end | `{1 2} 3 append` → `[{1 2 3}]` |
| `concat` | `( l1 l2 -- list )` | Concatenate | `{1 2} {3 4} concat` → `[{1 2 3 4}]` |
| `map` | `( list quot -- list )` | Transform | `{1 2 3} [2 *] map` → `[{2 4 6}]` |
| `filter` | `( list quot -- list )` | Filter | `{1 2 3 4} [2 % 0 =] filter` → `[{2 4}]` |
| `fold` | `( list init quot -- result )` | Reduce | `{1 2 3} [+] 0 swap fold` → `[6]` |
| `each` | `( list quot -- )` | Iterate | `{1 2 3} [print] each` |

---

## Strings

| Word | Stack Effect | Description | Example |
|------|--------------|-------------|---------|
| `.` | `( s1 s2 -- str )` | Concatenate | `"Hi" " there" .` → `["Hi there"]` |
| `len` | `( str -- n )` | Length | `"hello" len` → `[5]` |
| `chars` | `( str -- list )` | To char list | `"abc" chars` → `[{'a' 'b' 'c'}]` |
| `upper` | `( str -- str )` | Uppercase | `"hi" upper` → `["HI"]` |
| `lower` | `( str -- str )` | Lowercase | `"HI" lower` → `["hi"]` |

---

## Control Flow

| Word | Stack Effect | Description | Example |
|------|--------------|-------------|---------|
| `if` | `( bool then else -- )` | Conditional | `5 0 > ["yes"] ["no"] if` |
| `when` | `( bool quot -- )` | Conditional (no else) | `5 0 > ["positive" print] when` |
| `times` | `( n quot -- )` | Loop n times | `5 ["Hi" print] times` |
| `call` | `( quot -- ... )` | Execute quotation | `5 [2 *] call` → `[10]` |

---

## Combinators

| Word | Stack Effect | Description | Example |
|------|--------------|-------------|---------|
| `dip` | `( a b quot -- a ... )` | Execute under top | `1 2 [10 +] dip` → `[11, 2]` |
| `keep` | `( a quot -- ... a )` | Execute and preserve | `5 [2 *] keep` → `[10, 5]` |
| `bi` | `( a p q -- p(a) q(a) )` | Two operations | `5 [2 *] [3 +] bi` → `[10, 8]` |
| `tri` | `( a p q r -- ... )` | Three operations | `5 [1 +] [2 *] [3 -] tri` → `[6, 10, 2]` |

---

## Word Definition

```ember
; Define a word
def name
    body
end

; Example
def square
    dup *
end

5 square  ; => 25
```

---

## Quotations

```ember
; Create quotation (code as data)
[ dup * ]

; Execute quotation
5 [ dup * ] call  ; => 25

; Pass to higher-order function
{ 1 2 3 } [ dup * ] map  ; => { 1 4 9 }
```

**When to use `[ ]`:**
- ✅ Conditionals: `condition [then] [else] if`
- ✅ Loops: `5 [code] times`
- ✅ Map/filter: `{1 2 3} [2 *] map`
- ✅ Combinators: `5 [2 *] keep`
- ❌ Word definitions: `def square dup * end` (no brackets!)

---

## Modules

```ember
; Define module (mymath.em)
module MyMath
    def square dup * end
    def cube dup dup * * end
end

; Import and use
import "../stdlib/mymath.em"
5 MyMath.square print  ; => 25

; Wildcard import
use MyMath.*
5 square print  ; => 25
```

---

## I/O

| Word | Stack Effect | Description |
|------|--------------|-------------|
| `print` | `( value -- )` | Print value |

---

## Common Patterns

### Square a number
```ember
dup *
```

### Swap and subtract
```ember
swap -
```

### Absolute value
```ember
dup 0 < [ neg ] [ ] if
```

### Sum of list
```ember
[ + ] 0 swap fold
```

### Factorial
```ember
def factorial
    dup 1 =
    [ drop 1 ]
    [ dup 1 - factorial * ]
    if
end
```

### Map over list
```ember
{ 1 2 3 } [ 2 * ] map
```

### Filter evens
```ember
{ 1 2 3 4 5 } [ 2 % 0 = ] filter
```

---

## Stack Effect Notation

```
( before -- after )

Examples:
dup:    ( a -- a a )
swap:   ( a b -- b a )
+:      ( a b -- sum )
if:     ( bool then else -- )
map:    ( list quot -- list )
```

---

## Error Handling

**Stack underflow:**
```
+  ; Error: need 2 values
```

**Type error:**
```
"hello" 5 +  ; Error: can't add string and integer
```

**Division by zero:**
```
10 0 /  ; Error: division by zero
```

---

## Tips & Tricks

1. **Document stack effects:** `; ( a b -- result )`
2. **Keep words small:** One clear purpose each
3. **Use descriptive names:** `circle-area` not `ca`
4. **Test edge cases:** Empty lists, zero, negatives
5. **Quotations vs direct:** Use `[ ]` for control flow, not definitions
6. **Leverage combinators:** `keep`, `bi`, `dip` for cleaner code

---

## Example Programs

### FizzBuzz
```ember
1 100 [
    dup 15 % 0 = [ drop "FizzBuzz" print ]
    [ dup 3 % 0 = [ drop "Fizz" print ]
      [ dup 5 % 0 = [ drop "Buzz" print ]
        [ dup print ] if
      ] if
    ] if
    1 +
] times drop
```

### Fibonacci
```ember
def fib
    dup 2 <
    [ ]
    [ dup 1 - fib swap 2 - fib + ]
    if
end
```

### Sum of squares
```ember
{ 1 2 3 4 5 } [ dup * ] map [ + ] 0 swap fold
```

---

**EMBER v1.0 • https://github.com/yourusername/ember**
