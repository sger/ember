# EMBER Tutorial: From Zero to Hero

A hands-on tutorial to learn EMBER programming step by step.

---

## Lesson 1: The Stack

### What is a Stack?

Think of a stack like a stack of plates. You can only:
- **Push** a plate on top
- **Pop** a plate from the top

EMBER uses a stack to store values.

### Your First Stack Operations

```ember
; Push numbers onto the stack
5
3
```

After these lines, the stack looks like:
```
[5, 3]  (3 is on top)
```

### Stack Manipulation

```ember
; Print the top value
5 3 print    ; Prints: 3  (and pops it)

; Stack is now: [5]
```

**Try this:**
```ember
10 20 30
print  ; => 30
print  ; => 20
print  ; => 10
```

---

## Lesson 2: Basic Math

### Addition

```ember
5 3 +    ; Pops 3 and 5, pushes 8
print    ; => 8
```

Step by step:
1. Push 5 → Stack: `[5]`
2. Push 3 → Stack: `[5, 3]`
3. Add → Pop 3, pop 5, push 8 → Stack: `[8]`
4. Print → Pop 8, display it

### All Operations

```ember
10 3 +   ; => 13  (addition)
10 3 -   ; => 7   (subtraction)
10 3 *   ; => 30  (multiplication)
10 3 /   ; => 3   (division)
10 3 %   ; => 1   (modulo/remainder)
```

**Exercise:** Calculate (5 + 3) × 2

<details>
<summary>Solution</summary>

```ember
5 3 + 2 *
print  ; => 16
```
</details>

---

## Lesson 3: Stack Words

### dup - Duplicate

```ember
5 dup    ; Stack: [5, 5]
*        ; Stack: [25]  (square!)
print    ; => 25
```

### swap - Swap Top Two

```ember
5 3 swap   ; Stack: [3, 5]
-          ; Stack: [-2]  (3 - 5)
print      ; => -2
```

### drop - Remove Top

```ember
5 3 drop   ; Stack: [5]  (3 was removed)
print      ; => 5
```

**Exercise:** Calculate (10 - 3) without using variables

<details>
<summary>Solution</summary>

```ember
10 3 -
print  ; => 7
```

Or with swap:
```ember
3 10 swap -
print  ; => 7
```
</details>

---

## Lesson 4: Conditionals

### The if Word

```ember
; Syntax: condition [ then-branch ] [ else-branch ] if

5 0 >
    [ "Positive!" print ]
    [ "Not positive!" print ]
if
```

Step by step:
1. `5 0 >` → Pushes `true`
2. `if` pops `true` and the two quotations
3. Since `true`, executes first quotation
4. Prints "Positive!"

### More Examples

```ember
; Check if even
10 2 % 0 =
    [ "Even" print ]
    [ "Odd" print ]
if
; => "Even"

; Absolute value
-5 dup 0 <
    [ neg ]
    [ ]
if
print  ; => 5
```

**Exercise:** Write code to print the larger of two numbers

<details>
<summary>Solution</summary>

```ember
def max
    over over >
    [ drop ]
    [ swap drop ]
    if
end

5 3 max print  ; => 5
```

Or simpler with the stack:
```ember
5 3 over over >
    [ drop ]
    [ swap drop ]
if
print  ; => 5
```
</details>

---

## Lesson 5: Word Definitions

### Defining Your Own Words

```ember
def square
    dup *
end

5 square print  ; => 25
```

### Multiple Words

```ember
def square dup * end
def cube dup dup * * end

3 cube print  ; => 27
```

### Words Using Words

```ember
def square dup * end

def sum-of-squares  ; ( a b -- a²+b² )
    square swap square +
end

3 4 sum-of-squares print  ; => 25
```

**Exercise:** Define a word `average` that computes the average of two numbers

<details>
<summary>Solution</summary>

```ember
def average
    + 2 /
end

10 20 average print  ; => 15
```
</details>

---

## Lesson 6: Lists

### Creating Lists

```ember
{ 1 2 3 4 5 }  ; A list
```

### List Operations

```ember
; Length
{ 1 2 3 } len print  ; => 3

; First element (head)
{ 1 2 3 } head print  ; => 1

; Rest of list (tail)
{ 1 2 3 } tail print  ; => { 2 3 }

; Get nth element (0-indexed)
{ 10 20 30 } 1 nth print  ; => 20

; Append
{ 1 2 3 } 4 append print  ; => { 1 2 3 4 }
```

### Map - Transform Every Element

```ember
{ 1 2 3 4 5 } [ 2 * ] map print
; => { 2 4 6 8 10 }
```

This multiplies each element by 2!

### Filter - Keep Matching Elements

```ember
{ 1 2 3 4 5 6 } [ 2 % 0 = ] filter print
; => { 2 4 6 }
```

This keeps only even numbers!

**Exercise:** Create a list of squares from 1 to 5

<details>
<summary>Solution</summary>

```ember
{ 1 2 3 4 5 } [ dup * ] map print
; => { 1 4 9 16 25 }
```
</details>

---

## Lesson 7: Quotations Deep Dive

### What Are Quotations?

Quotations are **code as data**. They're like functions you can pass around.

```ember
[ 2 * ]  ; This is a quotation
```

### Using Quotations

```ember
; Store a quotation
[ 2 * ]

; Apply it with call
5 [ 2 * ] call print  ; => 10
```

### Quotations in Control Flow

```ember
; if needs two quotations
5 0 >
    [ "yes" print ]  ; First quotation
    [ "no" print ]   ; Second quotation
if
```

### Quotations with map/filter

```ember
; map applies quotation to each element
{ 1 2 3 } [ 10 * ] map print
; => { 10 20 30 }

; filter keeps elements where quotation returns true
{ 1 2 3 4 5 } [ 3 > ] filter print
; => { 4 5 }
```

**Exercise:** Use map to convert a list of Celsius temperatures to Fahrenheit

Formula: F = C × 9/5 + 32

<details>
<summary>Solution</summary>

```ember
{ 0 10 20 30 } [ 9 * 5 / 32 + ] map print
; => { 32 50 68 86 }
```
</details>

---

## Lesson 8: Loops

### times - Repeat n Times

```ember
5 [ "Hello!" print ] times
; Prints "Hello!" 5 times
```

### each - Iterate Over List

```ember
{ 1 2 3 } [ print ] each
; Prints:
; 1
; 2
; 3
```

### Building Lists with Loops

```ember
; Create list { 1 2 3 4 5 }
{ }                    ; Start with empty list
1                      ; Counter
5 [
    dup swap append    ; Append counter to list
    swap               ; Bring counter back
    1 +                ; Increment
    swap               ; Bring list back
] times
drop                   ; Drop final counter
print  ; => { 1 2 3 4 5 }
```

**Exercise:** Use times to calculate factorial of 5

<details>
<summary>Solution</summary>

```ember
1              ; Accumulator
5 [
    swap       ; Bring counter to top
    dup        ; Duplicate counter
    rot        ; Bring accumulator to top
    *          ; Multiply
    swap       ; Put accumulator on top
    1 -        ; Decrement counter
    swap       ; Put counter back
] times
drop print     ; => 120
```

Or better, define it recursively:
```ember
def factorial
    dup 1 =
    [ drop 1 ]
    [ dup 1 - factorial * ]
    if
end

5 factorial print  ; => 120
```
</details>

---

## Lesson 9: Recursion

### Simple Recursion

```ember
def countdown
    dup 0 >
    [
        dup print      ; Print current number
        1 -            ; Decrement
        countdown      ; Recursive call
    ]
    [ drop ]
    if
end

5 countdown
; Prints: 5 4 3 2 1
```

### Factorial (Classic Recursion)

```ember
def factorial
    dup 1 =
    [ drop 1 ]
    [ dup 1 - factorial * ]
    if
end

5 factorial print  ; => 120
```

### Fibonacci

```ember
def fib
    dup 2 <
    [ ]
    [
        dup 1 - fib
        swap 2 - fib
        +
    ]
    if
end

10 fib print  ; => 55
```

**Exercise:** Write a recursive sum for a list

<details>
<summary>Solution</summary>

```ember
def sum-list
    dup len 0 =
    [ drop 0 ]
    [
        dup head
        swap tail sum-list
        +
    ]
    if
end

{ 1 2 3 4 5 } sum-list print  ; => 15
```

Or use fold:
```ember
{ 1 2 3 4 5 } [ + ] 0 swap fold print  ; => 15
```
</details>

---

## Lesson 10: Combinators

### keep - Execute and Preserve

```ember
; keep: ( x quot -- result x )
5 [ 2 * ] keep
; Stack: [10, 5]

; Useful for calculations that need the original
5 [ dup * ] keep +  ; => 30  (25 + 5)
```

### dip - Execute Under Top

```ember
; dip: ( a b quot -- a result )
1 2 [ 10 + ] dip
; Stack: [11, 2]
; (10 + 1 = 11, then 2 is restored)
```

### bi - Apply Two Operations

```ember
; bi: ( x p q -- p(x) q(x) )
5 [ 2 * ] [ 3 + ] bi
; Stack: [10, 8]
```

### Real-World Example

```ember
; Calculate area and perimeter of rectangle
def rect-stats
    [ * ]       ; area
    [ + 2 * ]   ; perimeter
    bi
end

5 3 rect-stats
; Stack: [15, 16]
print print  ; => 16, 15
```

---

## Lesson 11: Modules

### Creating a Module

```ember
; mymath.em
module MyMath

def square dup * end
def cube dup dup * * end
def double 2 * end

end
```

### Using a Module

```ember
; main.em
import "../stdlib/mymath.em"

5 MyMath.square print  ; => 25
3 MyMath.cube print    ; => 27
```

### Wildcard Import

```ember
import "../stdlib/mymath.em"
use MyMath.*

5 square print  ; => 25  (no prefix needed!)
```

---

## Final Project: Practical Examples

### 1. FizzBuzz

```ember
def fizzbuzz
    1
    100 [
        dup 15 % 0 =
        [ drop "FizzBuzz" print ]
        [
            dup 3 % 0 =
            [ drop "Fizz" print ]
            [
                dup 5 % 0 =
                [ drop "Buzz" print ]
                [ dup print ]
                if
            ]
            if
        ]
        if
        1 +
    ] times
    drop
end

fizzbuzz
```

### 2. Prime Checker

```ember
def is-prime
    dup 2 <
    [ drop false ]
    [
        dup 2 =
        [ drop true ]
        [
            ; Check divisibility from 2 to sqrt(n)
            ; (Simplified version)
            dup 2 % 0 =
            [ drop false ]
            [ drop true ]  ; Simplified!
            if
        ]
        if
    ]
    if
end

7 is-prime print  ; => true
8 is-prime print  ; => false
```

### 3. Reverse a String

```ember
def reverse-string
    chars
    { } swap
    [ swap append ] fold
end

"hello" reverse-string print  ; => "olleh"
```

---

## Next Steps

1. Read the full [EMBER Documentation](EMBER_DOCUMENTATION.md)
2. Explore the [Standard Library](../stdlib/)
3. Study the [Example Programs](../examples/)
4. Build your own programs!
5. Join the EMBER community

**Happy Coding! 🔥**
