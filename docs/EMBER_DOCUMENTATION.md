# EMBER Programming Language Documentation

**Version:** 1.0  
**Last Updated:** January 2026

EMBER is a modern concatenative (stack-based) programming language inspired by Forth and Factor, written in Rust. It features:
- Stack-based evaluation
- Quotations (first-class functions)
- Module system
- Bytecode compilation
- Type inference
- Interactive REPL

---

## Table of Contents

1. [Getting Started](#getting-started)
2. [Basic Concepts](#basic-concepts)
3. [Data Types](#data-types)
4. [Stack Operations](#stack-operations)
5. [Arithmetic](#arithmetic)
6. [Comparison & Logic](#comparison--logic)
7. [Control Flow](#control-flow)
8. [Quotations](#quotations)
9. [Word Definitions](#word-definitions)
10. [Lists](#lists)
11. [Strings](#strings)
12. [Combinators](#combinators)
13. [Module System](#module-system)
14. [Standard Library](#standard-library)
15. [Error Handling](#error-handling)
16. [Best Practices](#best-practices)

---

## Getting Started

### Installation

```bash
git clone https://github.com/yourusername/ember
cd ember
cargo build --release
```

### Running EMBER

```bash
# Run a file
ember program.em

# Interactive REPL
ember

# Show bytecode disassembly
ember program.em --disasm

# Compile to bytecode cache
ember program.em --compile
```

### Your First Program

```ember
; hello.em
"Hello, EMBER!" print
```

Run it:
```bash
ember hello.em
```

---

## Basic Concepts

### Stack-Based Evaluation

EMBER uses a **stack** to store values. Operations consume values from the stack and push results back.

```ember
5 3 +   ; Push 5, push 3, add them → stack: [8]
```

**Stack visualization:**
```
5       ; Stack: [5]
3       ; Stack: [5, 3]
+       ; Stack: [8]
```

### Postfix Notation

EMBER uses **postfix** (Reverse Polish Notation):

```
Traditional:  (5 + 3) * 2
EMBER:        5 3 + 2 *
```

### Comments

```ember
; Single-line comment

; Multi-line comments
; are just multiple
; single-line comments
```

---

## Data Types

### Integers

```ember
42
-17
0
1000
```

### Floats

```ember
3.14
-0.5
2.71828
```

### Strings

```ember
"Hello, World!"
"String with \"quotes\""
"Multi-word string"
```

### Booleans

```ember
true
false
```

### Lists

```ember
{ }              ; Empty list
{ 1 2 3 }        ; List of integers
{ "a" "b" "c" }  ; List of strings
{ 1 "two" 3.0 }  ; Mixed types
```

### Quotations

```ember
[ 2 * ]          ; Code block (quotation)
[ dup + ]        ; Another quotation
```

---

## Stack Operations

### Basic Stack Manipulation

```ember
; dup: ( a -- a a )
; Duplicates top of stack
5 dup        ; Stack: [5, 5]

; drop: ( a -- )
; Removes top of stack
5 3 drop     ; Stack: [5]

; swap: ( a b -- b a )
; Swaps top two items
5 3 swap     ; Stack: [3, 5]

; over: ( a b -- a b a )
; Copies second item to top
5 3 over     ; Stack: [5, 3, 5]

; rot: ( a b c -- b c a )
; Rotates top three items
1 2 3 rot    ; Stack: [2, 3, 1]
```

### Stack Visualization Examples

```ember
; Example: Compute (5 + 3) * 2
5 3 +        ; Stack: [8]
2 *          ; Stack: [16]

; Example: Swap and subtract
10 3 swap -  ; Stack: [-7]  (3 - 10)

; Example: Duplicate and multiply (square)
5 dup *      ; Stack: [25]
```

---

## Arithmetic

### Basic Operations

```ember
; Addition: ( a b -- sum )
5 3 +        ; => 8

; Subtraction: ( a b -- difference )
10 3 -       ; => 7

; Multiplication: ( a b -- product )
4 5 *        ; => 20

; Division: ( a b -- quotient )
20 4 /       ; => 5

; Modulo: ( a b -- remainder )
10 3 %       ; => 1

; Negation: ( a -- -a )
5 neg        ; => -5

; Absolute value: ( a -- |a| )
-5 abs       ; => 5
```

### Type Coercion

```ember
; Integer + Float → Float
5 3.0 +      ; => 8.0

; Float + Integer → Float
3.14 2 +     ; => 5.14
```

---

## Comparison & Logic

### Comparison Operators

```ember
; Equal: ( a b -- bool )
5 5 =        ; => true
5 3 =        ; => false

; Not equal: ( a b -- bool )
5 3 !=       ; => true

; Less than: ( a b -- bool )
3 5 <        ; => true

; Greater than: ( a b -- bool )
5 3 >        ; => true

; Less than or equal: ( a b -- bool )
3 5 <=       ; => true

; Greater than or equal: ( a b -- bool )
5 5 >=       ; => true
```

### Logical Operations

```ember
; AND: ( a b -- bool )
true true and    ; => true
true false and   ; => false

; OR: ( a b -- bool )
true false or    ; => true
false false or   ; => false

; NOT: ( a -- bool )
true not         ; => false
false not        ; => true
```

---

## Control Flow

### Conditional Execution

```ember
; if: ( bool then-quot else-quot -- )
; Executes one of two quotations based on condition

5 0 >
    [ "Positive" print ]
    [ "Non-positive" print ]
if

; Example: Absolute value
def my-abs
    dup 0 <
    [ neg ]
    [ ]
    if
end

-5 my-abs print  ; => 5
```

### When (Conditional without else)

```ember
; when: ( bool quot -- )
; Executes quotation only if condition is true

5 0 >
    [ "It's positive!" print ]
when
```

### Loops

```ember
; times: ( n quot -- )
; Executes quotation n times

5 [ "Hello" print ] times
; Prints "Hello" 5 times

; Example: Count from 1 to 5
1
5 [ dup print 1 + ] times
drop
```

---

## Quotations

### What are Quotations?

Quotations are **code blocks** - pieces of code treated as data.

```ember
[ 2 * ]          ; A quotation that doubles a number
```

### Creating Quotations

```ember
; Simple quotation
[ dup + ]

; Multi-operation quotation
[ dup * 2 + ]

; Nested quotations
[ 5 [ 2 * ] call + ]
```

### Executing Quotations

```ember
; call: ( quot -- ... )
; Executes a quotation

5 [ 2 * ] call   ; => 10

; Example with nested quotations
[ [ 1 2 + ] call 5 * ] call  ; => 15
```

### Quotations vs Direct Code

```ember
; WRONG - this defines a word that pushes a quotation
def double [ 2 * ] end
5 double         ; Stack: [5, [2 *]]  ❌

; CORRECT - this defines a word that executes
def double 2 * end
5 double         ; Stack: [10]  ✓
```

### When to Use Quotations

✅ **Use quotations `[ ]` for:**
- Conditionals: `condition [ then ] [ else ] if`
- Loops: `5 [ code ] times`
- Higher-order functions: `{ 1 2 3 } [ 2 * ] map`
- Combinators: `5 [ 2 * ] keep`

❌ **Don't use quotations for:**
- Word definitions: `def square dup * end`
- Constants: `def pi 3.14159 end`
- Immediate execution: Just write the code directly

---

## Word Definitions

### Defining Words

```ember
; Basic syntax: def name body end
def square
    dup *
end

; Usage
5 square print  ; => 25
```

### Multi-line Definitions

```ember
def factorial
    dup 1 =
    [ drop 1 ]
    [ dup 1 - factorial * ]
    if
end

5 factorial print  ; => 120
```

### Words with Stack Effects

```ember
; Document stack effects with comments
; ( before -- after )

def square  ; ( n -- n² )
    dup *
end

def swap-subtract  ; ( a b -- b-a )
    swap -
end
```

### Recursive Words

```ember
def countdown
    dup 0 >
    [
        dup print
        1 -
        countdown
    ]
    [ drop ]
    if
end

5 countdown  ; Prints: 5 4 3 2 1
```

---

## Lists

### Creating Lists

```ember
{ }              ; Empty list
{ 1 2 3 }        ; List with elements
{ 1 2 3 4 5 }    ; Larger list
```

### List Operations

```ember
; len: ( list -- length )
{ 1 2 3 } len    ; => 3

; head: ( list -- first-element )
{ 1 2 3 } head   ; => 1

; tail: ( list -- remaining-list )
{ 1 2 3 } tail   ; => { 2 3 }

; nth: ( list index -- element )
{ 10 20 30 } 1 nth  ; => 20

; append: ( list element -- list )
{ 1 2 3 } 4 append  ; => { 1 2 3 4 }

; concat: ( list1 list2 -- combined-list )
{ 1 2 } { 3 4 } concat  ; => { 1 2 3 4 }
```

### List Processing

```ember
; map: ( list quot -- new-list )
; Apply quotation to each element
{ 1 2 3 } [ 2 * ] map  ; => { 2 4 6 }

; filter: ( list quot -- filtered-list )
; Keep elements where quotation returns true
{ 1 2 3 4 5 } [ 2 % 0 = ] filter  ; => { 2 4 }

; fold: ( list init quot -- result )
; Reduce list with operation
{ 1 2 3 4 } [ + ] 0 swap fold  ; => 10

; each: ( list quot -- )
; Execute quotation for each element
{ 1 2 3 } [ print ] each
```

### Building Lists

```ember
; Build a list of squares
{ }
5 [ dup dup * swap append swap 1 + ] times
drop
; => { 1 4 9 16 25 }

; Using map (easier!)
{ 1 2 3 4 5 } [ dup * ] map
; => { 1 4 9 16 25 }
```

---

## Strings

### String Operations

```ember
; String concatenation: ( str1 str2 -- combined )
"Hello" " World" .  ; => "Hello World"

; String length: ( str -- length )
"Hello" len  ; => 5

; Convert to list of characters
"abc" chars  ; => { 'a' 'b' 'c' }

; String to uppercase
"hello" upper  ; => "HELLO"

; String to lowercase
"HELLO" lower  ; => "hello"
```

### String Examples

```ember
; Greeting
"Hello, " "EMBER!" . print
; => "Hello, EMBER!"

; Reverse a string
"hello" chars reverse  ; Reverse character list
```

---

## Combinators

Combinators are higher-order functions that manipulate quotations and the stack.

### dip

```ember
; dip: ( x quot -- x )
; Execute quotation under top of stack

1 2 [ 3 + ] dip
; Stack: [4, 2]
; (3 + 1 = 4, then 2 is restored)
```

### keep

```ember
; keep: ( x quot -- ... x )
; Execute quotation on x, then restore x

5 [ 2 * ] keep
; Stack: [10, 5]
; (5 * 2 = 10, then 5 is restored)
```

### bi

```ember
; bi: ( x p q -- p(x) q(x) )
; Apply two quotations to the same value

5 [ 2 * ] [ 3 + ] bi
; Stack: [10, 8]
; (5 * 2 = 10, 5 + 3 = 8)
```

### tri

```ember
; tri: ( x p q r -- p(x) q(x) r(x) )
; Apply three quotations to the same value

5 [ 1 + ] [ 2 * ] [ 3 - ] tri
; Stack: [6, 10, 2]
; (5 + 1 = 6, 5 * 2 = 10, 5 - 3 = 2)
```

### Combinator Examples

```ember
; Calculate area and perimeter of rectangle
def rect-info  ; ( width height -- area perimeter )
    [ * ]                    ; area quotation
    [ + 2 * ]                ; perimeter quotation
    bi
end

5 3 rect-info
; Stack: [15, 16]  (area=15, perimeter=16)
```

---

## Module System

### Defining Modules

```ember
; math.em
module Math

def pi 3.14159265359 end

def square dup * end

def circle-area
    dup * pi *
end

end
```

### Importing Modules

```ember
; Import a module
import "math.em"

; Use qualified names
Math.pi print           ; => 3.14159265359
5 Math.square print     ; => 25
```

### Wildcard Imports

```ember
; Import all names from module
import "math.em"
use Math.*

; Now use without prefix
pi print       ; => 3.14159265359
5 square print ; => 25
```

### Module Organization

```
project/
├── main.em          ; Your program
├── stdlib/          ; Standard library modules
│   ├── math.em
│   ├── string.em
│   └── list.em
└── mylib/           ; Your custom modules
    └── utils.em
```

---

## Standard Library

### Math Module

```ember
import "../stdlib/math.em"

Math.pi          ; 3.14159265359
Math.e           ; 2.71828182846

5 Math.square           ; => 25
4 Math.cube             ; => 64
10 Math.double          ; => 20
3 Math.triple           ; => 9

5 Math.circle_area      ; => 78.54
4 Math.is_even          ; => true
-5 Math.sign            ; => -1
```

---

## Error Handling

### Common Errors

**Stack Underflow:**
```ember
+  ; Error: stack underflow, expected 2 values, found 0
```

**Type Error:**
```ember
"hello" 5 +
; Error: type error: cannot add string and integer
```

**Division by Zero:**
```ember
10 0 /
; Error: division by zero
```

**Undefined Word:**
```ember
unknown-word
; Error: undefined word: unknown-word
```

### Error Messages

EMBER provides detailed error messages with:
- Error type and description
- File name and location
- Source code context
- Helpful suggestions

Example:
```
❌ Runtime Error: type error: cannot add string and integer
  --> examples/test.em:1:11
   1 | "hello" 5 +
     |           ^

💡 Help: Addition works on numbers, but got string and integer
```

---

## Best Practices

### 1. Document Stack Effects

```ember
; Good: Clear documentation
def circle-area  ; ( radius -- area )
    dup * Math.pi *
end

; Bad: No documentation
def circle-area
    dup * Math.pi *
end
```

### 2. Use Descriptive Names

```ember
; Good
def calculate-average
    sum swap len /
end

; Bad
def avg
    + swap len /
end
```

### 3. Keep Words Small

```ember
; Good: Small, focused words
def square dup * end
def sum-of-squares [ square ] map sum end

; Bad: One giant word
def sum-of-squares
    [ dup * ] map
    0 swap
    [ + ] fold
end
```

### 4. Use Quotations Appropriately

```ember
; Good: Quotations for control flow
5 0 > [ "positive" ] [ "negative" ] if

; Bad: Quotations in definitions
def square [ dup * ] end  ; Wrong!
def square dup * end      ; Correct!
```

### 5. Leverage Combinators

```ember
; Good: Using combinators
5 [ 2 * ] [ 3 + ] bi

; Acceptable but verbose
5 dup 2 * swap 3 +
```

### 6. Handle Edge Cases

```ember
; Good: Handles empty list
def safe-head
    dup len 0 >
    [ head ]
    [ drop 0 ]
    if
end

; Bad: Crashes on empty list
def unsafe-head
    head
end
```

### 7. Test Your Code

```ember
; Create test cases
def test-square
    5 square 25 = [ "PASS" ] [ "FAIL" ] if print
end

test-square  ; => "PASS"
```

---

## Quick Reference

### Stack Operations
```ember
dup     ; ( a -- a a )
drop    ; ( a -- )
swap    ; ( a b -- b a )
over    ; ( a b -- a b a )
rot     ; ( a b c -- b c a )
```

### Arithmetic
```ember
+       ; ( a b -- sum )
-       ; ( a b -- difference )
*       ; ( a b -- product )
/       ; ( a b -- quotient )
%       ; ( a b -- remainder )
neg     ; ( a -- -a )
abs     ; ( a -- |a| )
```

### Comparison
```ember
=       ; ( a b -- bool )
!=      ; ( a b -- bool )
<       ; ( a b -- bool )
>       ; ( a b -- bool )
<=      ; ( a b -- bool )
>=      ; ( a b -- bool )
```

### Logic
```ember
and     ; ( a b -- bool )
or      ; ( a b -- bool )
not     ; ( a -- bool )
```

### Lists
```ember
len     ; ( list -- length )
head    ; ( list -- first )
tail    ; ( list -- rest )
nth     ; ( list i -- elem )
append  ; ( list elem -- list )
concat  ; ( list1 list2 -- list )
map     ; ( list quot -- list )
filter  ; ( list quot -- list )
fold    ; ( list init quot -- result )
each    ; ( list quot -- )
```

### Strings
```ember
.       ; ( str1 str2 -- combined )
len     ; ( str -- length )
chars   ; ( str -- list )
upper   ; ( str -- uppercase )
lower   ; ( str -- lowercase )
```

### Control Flow
```ember
if      ; ( bool then else -- )
when    ; ( bool quot -- )
times   ; ( n quot -- )
call    ; ( quot -- ... )
```

### Combinators
```ember
dip     ; ( x quot -- x )
keep    ; ( x quot -- ... x )
bi      ; ( x p q -- p(x) q(x) )
tri     ; ( x p q r -- p(x) q(x) r(x) )
```

### I/O
```ember
print   ; ( value -- )
```

---

## Example Programs

See the `/examples` directory for complete programs:

- `01_basics.em` - Basic operations
- `02_quotations.em` - Working with quotations
- `03_lists.em` - List manipulation
- `04_strings.em` - String operations
- `05_control_flow.em` - Conditionals and loops
- `06_word_definitions.em` - Defining words
- `07_combinators.em` - Advanced combinators
- `08_recursion.em` - Recursive algorithms
- `09_practical.em` - Practical algorithms
- `10_modules.em` - Module system

---

## Resources

- **GitHub:** https://github.com/yourusername/ember
- **Examples:** `/examples` directory
- **Standard Library:** `/stdlib` directory
- **Issue Tracker:** GitHub Issues

---

## License

MIT License - See LICENSE file for details

---

**Happy EMBER Programming! 🔥**
