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
- [X] **Parser (expressions only)** - Binary ops, unary ops, grouping. AST output.
- [X] **Evaluator (expressions)** - Tree-walking evaluation of expressions, returns `Value`.
- [X] **Error reporting** - review all test cases and improve the error messages.
- [X] **Variables (let bindings)** - Add statements to parser, environment for name→value mapping.
- [X] **Functions** - Function declarations, function calls, return.
- [X] **Control flow and scopes** - If/else, while loops, break, continue.
- [X] **Refactor operators** - Move from ast_interpreter to value.
- [X] **Builtin functions** - Refactor print statement to builtin function (called via `Call` mechanism).
- [X] **Strings** - Heap-allocated strings (`Rc<String>`), literals, concatenation, comparison.
- [X] **Lists** - Fixed-size lists (`Rc<RefCell<Vec<Value>>>`), literals, indexing, mutation via `list[i] = value`.
- [X] **Indexing for strings** - with bounds checking
- [X] **len() builtin** - Returns length of strings and lists as integer.

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
- **Runtime values:** `Rc<>` for immutable heap values (strings), `Rc<RefCell<>>` for mutable structures (lists)
- **Reference cycles:** Accepted in Phase 1 (foundation for learning GC in Phase 4)

---

## Phase 2: Bytecode VM

**Goal:** Compile to bytecode and execute on a stack-based virtual machine

### Implementation Steps

**1. HIR Infrastructure**

- [X] Define HIR types (`hir::Expr`, `hir::Stmt`, `hir::Program`)
- [X] Define symbol types (`LocalId`, `GlobalId`, `LocalInfo`, `GlobalInfo`)
- [X] Build analyzer module: scope resolution, symbol table construction
- [X] Entry point: `analyze(ast: &ast::Program) -> SourceResult<hir::Program>`
- [X] Test: Simple programs compile to HIR with correct symbol resolution

**2. Sample optimization Pass**

- [X] Simple constant folding on HIR
- [X] Also fold builtin calls (e.g., `int("42")`)
- [X] Test: Constant expressions fold correctly at HIR level

**3. Bytecode Infrastructure**

- [X] Introduce `BytecodeBuilder`
- [X] Define `Bytecode` structure: `{ code: Vec<u8>, constants: Vec<Value>, globals: Vec<Value> }`
- [X] Implement LEB128 encoding helpers (SLEB128 for signed, ULEB128 for unsigned)
- [X] Implement encoder: `BytecodeBuilder → Vec<u8>`

**4. Simple Bytecode Compiler + VM (minimal language subset)**

- **Language subset:** integers, bools, null, arithmetic, variables, single function (no arguments), return
- [X] Compiler: HIR → BytecodeBuilder for expressions and return statements
- [X] VM: Stack machine with value stack (`Vec<Value>`)
- [X] VM: Instruction dispatch loop for basic opcodes (push, arithmetic, ret and variables)
- [X] Entry point: `execute(bytecode: Bytecode) -> Result<Value>`
- [X] Test: `fn main() { return 2 + 3; }` compiles and executes correctly

**5. Control Flow**

- **Add to language:** if/else, while, break/continue
- [X] Compiler: Jump label tracking
- [X] Compiler: Conditional jumps (jtrue/jfalse), unconditional jumps (jmp)
- [X] VM: Jump instructions (update instruction pointer)
- [X] Test: Loops, conditionals, local variable manipulation, De Morgan

**6. Functions and Calls**

- **Add to language:** function arguments, multiple functions, function calls
- [X] Compiler: Function objects in globals array
- [X] Compiler: Call instruction emission, argument handling
- [X] VM: Frame metadata stack (`Vec<CallFrame>`)
- [X] VM: Frame management (push/pop CallFrame on call/ret, frame pointer tracking)
- [X] Test: Recursive functions (e.g., fibonacci), multiple function calls

**7. Remaining Features (deferred - can be implemented in any order)**

- [X] **Floats** - Constant pool storage, numeric coercion in operators
- [X] **Strings** - Constant pool storage, concatenation, comparison, indexing
- [X] **Lists** - Heap allocation, `make_list`/`get_item`/`set_item` instructions
- [X] **Benchmarks** - Simple benchmark harness for comparing with AST interpreter
- [ ] **Debugging metadata** - Variable name tables, line number tables for stack traces
- [ ] **Disassembler** - Bytecode → human-readable instruction listing with constant pool references

### Rust Learning Focus

- Designing compiler IR (high-level instruction representation)
- Two-pass algorithms (symbol collection, code emission)
- HashMap for symbol tables (constants, globals, labels)
- Byte-level encoding (LEB128, variable-width instructions)
- Frame-based VM architecture (stack + frame pointer)
- Vectors as stacks
- First `unsafe` code for performance (optional - instruction dispatch)
- Understanding memory layout and alignment

### Design Decisions

**Flat bytecode layout:**

- VM always operates on flat `Vec<u8>` from day one
- Functions are just labeled offsets in the bytecode
- Execution starts at offset 0 by convention for single-file scripts

**In-memory execution only (Phase 2 scope):**

- Compiler and VM live in same process
- Constant pool is high-level `Vec<Value>` (with `Rc`, `RefCell`)
- No serialization initially - focus on execution model
- Serialization is optional busywork (step 11 or deferred)

**Bytecode structure:**

```rust
struct Bytecode {
    code: Vec<u8>,          // flat bytecode stream
    constants: Vec<Value>,  // constant pool (high-level values)
}
```

Instructions reference constants by index (1 or 2 bytes depending on pool size).

### Deliverables

- Bytecode instruction definitions and documentation
- Disassembler for debugging
- VM with stack and instruction pointer
- Bytecode compiler (AST → Bytecode)
- Performance comparison with Phase 1
- Test suite demonstrating correctness

### Memory Management Strategy

Continue with `Rc<RefCell<>>` for runtime values. No changes to GC strategy in Phase 2.

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
- [ ] Simple collection types: `list<T>`

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

## Deferred Features (Phase 5+)

Features that are interesting but not on the critical path for learning systems Rust:

### Language Features

- [ ] Tuples - `(1, 2, 3)` for multiple returns
- [ ] Dictionaries/Maps - `{"key": value}`
- [ ] Closures/lambdas - `fn(x) { x + 1 }`, environment capture
- [ ] For loops - `for x in list { }`
- [ ] Iterators/generators
- [ ] User-defined structs/classes with fields
- [ ] Method call syntax - `obj.method(args)`
- [ ] Exception handling - try/catch
- [ ] Modules/imports

### Built-in Functions & Methods

- [ ] More builtins - `map()`, `filter()`, `range()`, `type()`, `chr()`, `ord()`
- [ ] String slicing - `s[1:3]`, `s[:5]`, `s[2:]`
- [ ] String interpolation - `"count: {x}"` or similar
- [ ] String methods - `split()`, `join()`, `upper()`, `lower()`, `trim()`
- [ ] List growth - `push()`, `pop()`, `insert()`, `remove()`, `resize()`
- [ ] List operations - `sort()`, `reverse()`, `concat()`

### Tooling & Infrastructure

- [ ] Bytecode assembler (text format → bytecode for testing VM edge cases)
- [ ] Bytecode serialization/deserialization (save/load compiled bytecode)
- [ ] Standard library
- [ ] Package manager
- [ ] Language server protocol (IDE support)
- [ ] Self-hosting (compiler written in itself)
- [ ] Concurrency primitives

Don't think about these until Phase 1-4 are complete!
