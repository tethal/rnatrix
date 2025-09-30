# Project Overview

## What This Project Is

A Rust learning project focused on building a complete programming language implementation from scratch. The goal is to
learn Rust concepts in a meaningful way by progressing from high-level safe code to low-level `unsafe` concepts through
practical compiler/interpreter construction.

**Primary Goal:** Learn Rust deeply through hands-on implementation
**Secondary Goal:** Understand language implementation (parsers, compilers, VMs, type systems, code generation)

## Learning Philosophy

Start with high-level abstractions (AST interpreter) and gradually progress to low-level systems programming (`unsafe`,
memory management, x64 code generation). Learn Rust features when they become necessary for the current implementation
phase, not all at once.

## Project Progression

The project will evolve through several phases:

1. **Tree-Walking Interpreter** - Direct AST evaluation, learn Rust basics
2. **Bytecode VM** - Compilation to bytecode, stack-based VM
3. **Type System** - Optional static type annotations, type checking
4. **Native Code Generation** - x64 compilation for typed code
5. **Advanced Types** - Generics, inference, sophisticated type features (stretch goal)

See [[roadmap]] for detailed breakdown of each phase.

## Language Design Decisions

### Gradually Typed System

Starting with a dynamically typed language that will gain optional static type annotations later (similar to
TypeScript's approach but simpler).

**Rationale:**

- Easier learning curve - get interpreter working without type system complexity
- Learn Rust ownership/borrowing with straightforward structures first
- Can still generate x64 code for statically-typed subset later
- Avoids early complexity: variance, contravariance, generic types, etc.
- Real-world relevant: gradual typing is an active PL research area

### Reference Counting Initially

Using `Rc<>` (and `Rc<RefCell<>>` where needed) for garbage collection in early phases. Will accept memory leaks from
reference cycles.

**Rationale:**

- Focus on getting interpreter working, not GC complexity
- Simple and idiomatic Rust for shared ownership
- Natural learning progression
- Will upgrade to proper tracing GC later (Phase 4+)

### Stack-Based Bytecode VM

Phase 2 will use stack-based bytecode rather than register-based.

**Rationale:**

- Simpler to implement and reason about
- Good learning progression from tree-walking
- Industry standard (Python, JVM, Lua, etc.)
- Easier to extend with new instructions

### Script Files, Not REPL

The interpreter/compiler will run existing script files, not provide an interactive REPL.

**Rationale:**

- Simpler implementation - no incremental parsing/evaluation
- Focus on compiler pipeline, not interactive tooling
- Easier testing with file-based test suite

### AST Representation

**Immutable trees with `Box<Expr>` for indirection:**

```rust
enum Expr {
    Literal(f64),
    Binary { op: BinaryOp, left: Box<Expr>, right: Box<Expr> },
    // ...
}
```

**Design decisions:**

- **Immutable AST** - Tree-walking interpreter consumes and potentially rebuilds trees (constant folding, optimization).
  Rust's move semantics make this efficient by reusing unchanged subtrees.
- **Separate representations per phase** - Following rustc's approach: immutable AST → typed IR (later phases). Type
  information lives in separate structures, not mixed into AST.
- **Consistent indirection** - Always use `Box<Expr>` for expression children, even when not strictly required for
  recursion. Provides uniform representation and easier refactoring.
- **Direct `Box`, no abstraction** - Use `Box<Expr>` directly rather than `type ExprPtr = Box<Expr>` or custom wrappers.
  Keep it simple; can refactor to arena allocation later if needed.

**Rationale:**

- Learn Rust's ownership and move semantics through practical use
- Immutability simplifies reasoning about transformations
- Matches production compiler patterns (rustc uses similar approach)
- Easy to evolve - can add arena allocation or custom smart pointers later
