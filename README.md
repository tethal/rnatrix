# Natrix Language Implementation

A Rust learning project implementing a dynamically-typed language from scratch. Built to learn Rust through
practical compiler/VM construction, progressing from high-level safe code to low-level systems programming.

## Usage

```bash
# Build and run (uses bytecode VM by default)
cargo run --release -- demos/sieve.nx -- 50

# Use AST interpreter instead
cargo run --release -- --ast demos/sieve.nx -- 50
```

## Current Implementation

**Completed:**

- Tree-walking AST interpreter with full language support
- HIR (High-level IR) with symbol resolution and constant folding
- Bytecode compiler (AST → HIR → Bytecode)
- Stack-based bytecode VM with LEB128-encoded instructions
- Runtime value system with integers, floats, bools, strings, lists
- Functions (first-class), control flow (if/while/break/continue), scopes

**Architecture:**

- `natrix-runtime` - Standalone VM and value representation
- `natrix-compiler` - Parser, AST, HIR, bytecode compiler, AST interpreter

## Notable Design Decisions

### HIR (High-level IR)

Separate semantic representation between AST and codegen. AST represents syntax (includes `Paren`, unresolved names),
HIR represents semantics (resolved symbols with IDs, desugared constructs). Enables backend-independent optimizations
like constant folding of builtin calls.

### Index-Based Source Tracking

Spans store `SourceId` rather than borrowing from `Sources`. Allows AST to exist while adding new sources (e.g.,
imports). Following rustc/clang pattern - compact, `Copy`, no refcounting overhead.

### Opaque Value Type

`Value` hides internal representation behind accessor methods. Enables future optimization (NaN boxing, pointer tagging)
without changing user code. Returns owned `Rc<T>` pointers (not borrows) for heap values.

### Identifier Interning

All identifiers are interned `Name(NonZeroU32)` indices. Makes comparison/hashing cheap, enables efficient symbol
tables. Common compiler pattern (rustc, production compilers).

### Reference Counting for GC

Using `Rc<RefCell<>>` for heap values initially. Simple, idiomatic Rust for learning phase. Accepts memory leaks from
cycles - can upgrade to tracing GC later.

### Explicit Type Conversions

No implicit conversions (Python-style): `"count: " + 42` errors, must use `str(42)`. Exception: `==`/`!=` never error on
type mismatch (returns `false` like JS/Python/Lua).

### AST Interpreter as Reference Implementation

Tree-walker intentionally duplicates logic rather than sharing code with compiler. Independent implementations catch
each other's bugs.

### Stack-Based Bytecode VM

LEB128-encoded variable-width instructions. Flat bytecode layout (functions are labeled offsets). Separate frame
metadata stack.

## Future Directions

**Type System:** Optional type annotations, type inference, gradually-typed semantics

**Native Compilation:** JIT/AOT x64 codegen for typed code, tracing GC, FFI

**Language Features:** Closures, tuples, dicts, for-loops, iterators/generators, structs/classes, methods, exceptions,
modules/imports

**Built-ins:** String slicing/methods, list growth/operations, map/filter/range

**Tooling:** Disassembler, bytecode serialization, standard library, language server

See `bytecode.md` for VM instruction set and calling conventions.
