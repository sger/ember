# EMBER Language Documentation

Complete documentation for the EMBER programming language.

---

## 📚 Documentation Files

### 1. [EMBER_DOCUMENTATION.md](EMBER_DOCUMENTATION.md)
**Complete language reference** - Everything you need to know about EMBER
- All language features
- Syntax reference
- Built-in operations
- Standard library
- Module system
- Best practices

**Start here if:** You want comprehensive coverage of all features.

---

### 2. [EMBER_TUTORIAL.md](EMBER_TUTORIAL.md)
**Step-by-step tutorial** - Learn EMBER from scratch
- 11 progressive lessons
- Hands-on examples
- Exercises with solutions
- Final projects

**Start here if:** You're new to EMBER or stack-based languages.

---

### 3. [EMBER_QUICK_REFERENCE.md](EMBER_QUICK_REFERENCE.md)
**One-page cheat sheet** - Quick lookup for syntax and operations
- All operations in table format
- Common patterns
- Stack effect notation
- Quick examples

**Start here if:** You need a quick reminder while coding.

---

## 🚀 Getting Started

### New to EMBER?

1. **Read:** [EMBER_TUTORIAL.md](EMBER_TUTORIAL.md) - Lessons 1-3
2. **Try:** Run the examples in `/examples/01_basics.em`
3. **Practice:** Complete tutorial exercises
4. **Reference:** Keep [EMBER_QUICK_REFERENCE.md](EMBER_QUICK_REFERENCE.md) handy

### Already know stack-based languages?

1. **Skim:** [EMBER_QUICK_REFERENCE.md](EMBER_QUICK_REFERENCE.md)
2. **Check:** [EMBER_DOCUMENTATION.md](EMBER_DOCUMENTATION.md) - Module System section
3. **Explore:** Example programs in `/examples/`

---

## 📖 Documentation Structure

### EMBER_DOCUMENTATION.md
```
├── Getting Started
├── Basic Concepts (Stack, Postfix)
├── Data Types
├── Stack Operations
├── Arithmetic
├── Comparison & Logic
├── Control Flow
├── Quotations (Critical section!)
├── Word Definitions
├── Lists
├── Strings
├── Combinators
├── Module System
├── Standard Library
├── Error Handling
└── Best Practices
```

### EMBER_TUTORIAL.md
```
├── Lesson 1: The Stack
├── Lesson 2: Basic Math
├── Lesson 3: Stack Words
├── Lesson 4: Conditionals
├── Lesson 5: Word Definitions
├── Lesson 6: Lists
├── Lesson 7: Quotations Deep Dive
├── Lesson 8: Loops
├── Lesson 9: Recursion
├── Lesson 10: Combinators
├── Lesson 11: Modules
└── Final Projects
```

### EMBER_QUICK_REFERENCE.md
```
├── Stack Operations (table)
├── Arithmetic (table)
├── Comparison (table)
├── Logic (table)
├── Lists (table)
├── Strings (table)
├── Control Flow (table)
├── Combinators (table)
├── Word Definition (syntax)
├── Quotations (when to use)
├── Modules (syntax)
├── Common Patterns
└── Example Programs
```

---

## 🎯 Learning Path

### Week 1: Fundamentals
- **Day 1-2:** Tutorial Lessons 1-3 (Stack, Math, Stack Words)
- **Day 3-4:** Tutorial Lessons 4-5 (Conditionals, Definitions)
- **Day 5:** Run `/examples/01_basics.em` through `/examples/03_lists.em`
- **Day 6-7:** Practice exercises, build simple programs

### Week 2: Advanced Features
- **Day 1-2:** Tutorial Lessons 6-7 (Lists, Quotations)
- **Day 3-4:** Tutorial Lessons 8-9 (Loops, Recursion)
- **Day 5:** Tutorial Lessons 10-11 (Combinators, Modules)
- **Day 6:** Run `/examples/07_combinators.em` through `/examples/10_modules.em`
- **Day 7:** Build a complete project

### Week 3: Mastery
- Build practical applications
- Create your own modules
- Contribute to standard library
- Help others learn EMBER

---

## 📝 Additional Resources

### Example Programs
Located in `/examples/`:
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

### Standard Library
Located in `/stdlib/`:
- `math.em` - Mathematical functions

---

## 🔑 Key Concepts

### The Stack
Everything in EMBER happens on the **stack**. Understanding the stack is crucial.

### Quotations
**Quotations** `[ ]` are code as data. Know when to use them:
- ✅ Control flow (if, times, map)
- ❌ Word definitions

### Concatenative Style
EMBER is **concatenative** - words compose naturally:
```ember
def square-and-double
    square double
end
```

---

## ❓ Common Questions

**Q: When do I use `[ ]` brackets?**  
A: Use brackets for control flow (if, times, map), not for word definitions. See [EMBER_DOCUMENTATION.md - Quotations](EMBER_DOCUMENTATION.md#quotations)

**Q: How do I debug stack issues?**  
A: Add `print` statements to see stack values. See [EMBER_DOCUMENTATION.md - Error Handling](EMBER_DOCUMENTATION.md#error-handling)

**Q: Can I use variables?**  
A: EMBER is stack-based - you manipulate the stack instead of using variables. See [EMBER_TUTORIAL.md - Lesson 1](EMBER_TUTORIAL.md#lesson-1-the-stack)

**Q: How do I create my own modules?**  
A: See [EMBER_DOCUMENTATION.md - Module System](EMBER_DOCUMENTATION.md#module-system)

---

## 🐛 Getting Help

1. **Check the docs:** Search these documentation files
2. **Run examples:** See `/examples/` for working code
3. **Read error messages:** EMBER provides detailed, helpful errors
4. **Ask the community:** GitHub Issues

---

## 🤝 Contributing

Found an error? Want to add examples? PRs welcome!

1. Fork the repository
2. Add your documentation improvements
3. Submit a pull request

---

## 📄 License

This documentation is part of the EMBER project and is licensed under MIT.

---

**Happy EMBER Programming! 🔥**

---

## Quick Links

- [Full Documentation](EMBER_DOCUMENTATION.md)
- [Tutorial](EMBER_TUTORIAL.md)
- [Quick Reference](EMBER_QUICK_REFERENCE.md)
- [Examples](/examples/)
- [Standard Library](/stdlib/)
- [GitHub Repository](https://github.com/yourusername/ember)
