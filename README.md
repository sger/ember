# Ember 
*A small, experimental, stack-based programming language*

Ember is a **concatenative, stack-based language** inspired by Forth and Joy.  
It is designed to be **simple to compile**, **easy to reason about**, and **fun to experiment with** while building a VM and compiler from first principles.

> ⚠️ Ember is in an **early stage**. The language, VM, and compiler are evolving quickly.

---

## Features

- Stack-based execution model
- Concatenative syntax (everything is a word)
- User-defined words (`def … end`)
- Quotations (`[ ... ]`) and conditionals (`if`)
- Recursive functions (used instead of loops for now)
- Modules and qualified calls (`Module.word`)
- Ahead-of-time compilation to bytecode
- Simple bytecode virtual machine
- Clear runtime errors (stack underflow, undefined words)

---

## Example

```ember
def gcd
    dup 0 = [
        drop
    ] [
        swap over %
        gcd
    ] if
end

48 18 gcd print   ; => 6
```

```
Compiling examples/gcd.em...
✓ Compiled 1 words

=== BYTECODE PROGRAM ===

════════════════════════════════════════
 main
 5 instructions
════════════════════════════════════════
0000   PUSH        48
0001   PUSH        18
0002   CALL_WORD   "gcd"
0003   PRINT       ; ( value -- )
0004   RETURN

════════════════════════════════════════
 gcd
 11 instructions
════════════════════════════════════════
0000   DUP
0001   PUSH        0
0002   EQ
0003   JUMP_FALSE  +3 ↓ (→ 0006)
0004   DROP
0005   JUMP        +5 ↓ (→ 0010)
      ┌──────────────────────────────────
0006 ► SWAP
0007   OVER
0008   MOD
0009   CALL_WORD   "gcd"
      ┌──────────────────────────────────
0010 ► RETURN


Executing...
6
```

---

## Concepts

### Stack-Based
All data lives on a single stack.  
Words consume values from the stack and push results back.

```ember
5 dup * print   ; 25
```

### Words
Words are functions defined using `def`:

```ember
def double
    dup +
end
```

### Quotations
Quotations are anonymous code blocks:

```ember
[ dup * ]
```

They are executed via control words like `if` (and `call` when used explicitly).

### Conditionals
```ember
dup 0 < [ neg ] [ ] if
```

---

## Modules

```ember
module Player
    def create 100 end
    def damage swap - end
end

Player.create print
```

Modules provide namespacing and organization.

---

## Getting Started

### Build
```bash
cargo build
```

### Run a program
```bash
cargo run -- examples/01_basics.em
```

### Disassemble bytecode (debugging)
```bash
cargo run -- examples/01_basics.em --disasm
```

---

## License

MIT
