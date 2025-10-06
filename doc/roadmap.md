# Project Roadmap

See [[README]] for project overview and design decisions.

## Phase 1: Tree-Walking Interpreter

**Goal:** Build a working interpreter that directly evaluates AST nodes from script files

### Implementation Steps

- [X] Project structure setup - libs with compiler/interpreter, CLI exe
- [X] Framework for golden file based testing
- [X] **Value representation (primitives)** - Integers, floats, bools, nil. Opaque `Value` type with accessor methods.
- [X] **Display for values** - Debug printing to test value representation.
- [X] **Source representation** - SourceId-based design, Cursor for tokenization, Span for AST, location tracking
- [X] **Tokenizer** - Literals, operators, parentheses for expressions.
- [ ] **Parser (expressions only)** - Binary ops, unary ops, grouping. AST output.
- [ ] **Evaluator (expressions)** - Tree-walking evaluation of expressions, returns `Value`.
- [ ] **Strings** - Extend `Value`, add string literals and concatenation.
- [ ] **Variables (let bindings)** - Add statements to parser, environment for name→value mapping.
- [ ] **Lists** - List literals, indexing operations.
- [ ] **Functions (first-class)** - Lambda syntax, closures, function calls.
- [ ] **Builtin functions** - Print, length, etc.
- [ ] **Control flow** - If/else, while loops.

### Rust Learning Focus

- Enums for AST and internal value representation
- Pattern matching for evaluation (via accessor methods)
- **Index-based references** - Understanding when IDs work better than lifetimes/refcounting
- **Strategic lifetime usage** - Using lifetimes for temporary structures (Cursor) but not persistent ones (Span)
- `Rc<RefCell<>>` for environments/scopes and runtime heap values
- Basic error handling
- Ownership and borrowing with tree structures

### Memory Management Strategy

- **Sources/Spans:** Index-based design with append-only collection (see [[README#Source Representation and Spans]])
- **Runtime values:** Just `Rc<>` for heap values, accept memory leaks from cycles

---

## Phase 2: Bytecode VM

**Goal:** Compile to bytecode and execute on a stack-based virtual machine

### New Features

- [ ] Bytecode instruction set design
- [ ] Compiler (AST → bytecode)
- [ ] Stack-based VM
- [ ] Better performance than tree-walking

### Rust Learning Focus

- Vectors as stacks
- Instruction encoding
- First `unsafe` code for performance
- More sophisticated enum usage
- Debugging bytecode execution

### Deliverables

- Bytecode instruction definitions
- Bytecode compiler
- VM with stack and instruction pointer
- Disassembler for debugging
- Performance comparison with Phase 1

### GC Strategy

Upgrade to `Rc<RefCell<>>` with better reference management

---

## Phase 3: Type Annotations & Checking

**Goal:** Add optional static types and type checker

### New Features

- [ ] Type annotation syntax (`:` syntax like `x: int`)
- [ ] Type checker as separate pass
- [ ] Type inference for simple cases
- [ ] Typed function signatures

#### Ideas to be discussed

- [ ] Primitive types: `int`, `float`, `bool`, `string`
- [ ] Function types: `fn(T1, T2) -> R`
- [ ] Simple collection types: `array<T>`

### Rust Learning Focus

- Complex AST traversal algorithms
- Symbol tables (index-based or with appropriate lifetimes)
- Type representation and unification
- Multi-pass compilation
- Choosing the right ownership strategy for different data structures

### Deliverables

- Extended AST with type annotations
- Type checker implementation
- Type error messages
- Examples of gradually-typed programs
- Optimizations for statically-typed code paths

### Note

Keep it simple - no generics, no variance, no structural subtyping yet. Start with invariant generics only.

---

## Phase 4: x64 Code Generation

**Goal:** Compile fully-typed functions to native x64 machine code

### New Features

- [ ] JIT or AOT compilation to x64
- [ ] Only compile statically-typed functions
- [ ] Fall back to interpreter/VM for dynamic code
- [ ] FFI for calling compiled code
- [ ] Real garbage collector

### Rust Learning Focus

- Heavy `unsafe` code
- FFI and C interop
- Raw pointers and memory layout
- Working with assembler libraries

### Deliverables

- Code generator
- Runtime support for compiled code
- Proper GC (mark-sweep or mark-compact)
- Benchmarks comparing interpreted vs compiled

### Memory Management Strategy

- Implement or integrate real tracing GC - required for native code
- **Optional:** Consider switching source references to GC'd pointers if beneficial, but index-based approach may remain
  appropriate

### Tools to Consider

- `cranelift-jit` - production-quality code generator
- `dynasm-rs` - runtime assembler
- `rust-gc` - existing GC implementation
- Custom mark-sweep collector (better for learning)

---

## Phase 5: Advanced Types (Optional)

**Goal:** Add sophisticated type system features

### New Features

- [ ] Generic functions and types
- [ ] Traits/interfaces
- [ ] Better type inference (Hindley-Milner)
- [ ] Variance annotations
- [ ] Structural typing for objects/records

### Rust Learning Focus

- Advanced type system algorithms
- Unification and constraint solving
- Generic programming patterns in Rust

### Note

This phase is genuinely hard PL theory. Consider it a stretch goal.

---

## Milestones

### Milestone 1: "Hello, World"

Can run basic programs with functions and variables

### Milestone 2: "Fibonacci"

Can compute recursive functions efficiently via bytecode

### Milestone 3: "Typed Fibonacci"

Can type-check and optimize statically-typed recursive functions

### Milestone 4: "Native Fibonacci"

Can compile to x64 and run at native speed

---

## Future Possibilities

After Phase 5, could explore:

- Concurrency primitives
- Module system
- Standard library
- Package manager
- Language server protocol (IDE support)
- Self-hosting (compiler written in itself)

Don't think about these until much later!
