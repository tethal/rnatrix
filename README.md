# Project Overview

## What This Project Is

A Rust learning project focused on building a complete programming language ("Natrix") implementation from scratch. The
goal is to learn Rust concepts in a meaningful way by progressing from high-level safe code to low-level `unsafe`
concepts through practical compiler/interpreter construction.

**Primary Goal:** Learn Rust deeply through hands-on implementation
**Secondary Goal:** Understand language implementation (parsers, compilers, VMs, type systems, code generation)

## Learning Philosophy

Start with high-level abstractions (AST interpreter) and gradually progress to low-level systems programming (`unsafe`,
memory management, custom tracing GC, x64 code generation). Learn Rust features when they become necessary for the
current implementation phase, not all at once.

## Project Progression

The project will evolve through several phases:

1. **Tree-Walking Interpreter** - Direct AST evaluation, learn Rust basics
2. **Bytecode VM** - Compilation to bytecode, stack-based VM
3. **Type System** - Optional static type annotations, type checking
4. **GC and/or Native Code Generation** - x64 compilation for typed code, tracing GC
5. **Advanced Language Features** - Closures, iterators, generators, generics, ...

See [[roadmap]] for detailed breakdown of each phase.

## Language Design Decisions

### Gradually Typed System

Starting with a dynamically typed language that will gain optional static type annotations later.

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
- Might upgrade to proper tracing GC later (Phase 4+)

**Future GC considerations:**

When implementing tracing GC, pointer copies (e.g., `Gc::clone()`) will have no semantic side effects in a simple
mark-sweep collector with conservative stack scanning. In theory, `Gc<T>` could be `Copy`. However, keeping it
non-Copy (requiring explicit `.clone()`) maintains:

- API consistency with Rust's smart pointer conventions
- Flexibility for future GC improvements (generational GC, write barriers, incremental collection)
- Option to add `Drop` for debugging/write barriers without breaking changes

Alternative approaches like arena-based GC with lifetime-tagged references (`&'gc T`) could provide `Copy` semantics,
worth exploring in Phase 4.

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
- **Minimal boxing** - Use `Box<Expr>` only where required for recursive types (e.g., `Binary`, `Unary`). Non-recursive
  containers (like statement variants) store expressions directly for better performance and cache locality.
- **Direct `Box`, no abstraction** - Use `Box<Expr>` directly rather than `type ExprPtr = Box<Expr>` or custom wrappers.
  Keep it simple; can refactor to arena allocation later if needed.

**Rationale:**

- Learn Rust's ownership and move semantics through practical use
- Immutability simplifies reasoning about transformations
- Matches production compiler patterns (rustc uses similar approach)
- Easy to evolve - can add arena allocation or custom smart pointers later

### Runtime Value Representation

**Opaque `Value` type with accessor methods:**

```rust
pub struct Value(ValueImpl);  // ValueImpl is private

impl Value {
    pub fn from_int(n: i64) -> Self { ... }
    pub fn from_float(n: f64) -> Self { ... }
    pub fn unwrap_int(&self) -> i64 { ... }
    pub fn unwrap_float(&self) -> f64 { ... }
    // ...
}
```

**Design decisions:**

- **Opaque representation** - Internal representation is hidden to allow future optimization (NaN boxing, pointer
  tagging).
- **Accessor methods only** - All value creation and inspection goes through methods, never direct enum matching.
- **Distinct int/float types** - Following Python's approach rather than JavaScript (where everything is a float).
  Requires numeric coercion rules in the evaluator.
- **Heap values return pointers** - Methods like `unwrap_string()` return `Rc<String>` (or `Gc<String>` later), not
  borrowed
  references. This allows values to outlive the original `Value` instance.
- **Start simple** - Initial implementation uses standard Rust enum with `Rc<>` for heap values. Can be replaced with
  NaN-boxed representation later without changing user code.

**Rationale:**

- Provides flexibility for future performance optimizations
- Clean API boundary between value representation and evaluator logic
- Returning owned pointers (not borrowed references) works better with Rust's ownership model when building data
  structures like environments
- Integer/float distinction enables better native code generation later

### String and Type Conversion Design

**String representation and operations:**

- **Immutable heap strings**: `Rc<String>` for shared ownership
- **UTF-8 storage**: Strings are just sequences of bytes, assuming UTF-8
- **Byte-level operations**: `len()` returns byte count, indexing operates on bytes, not Unicode codepoints
- **Concatenation**: Only `string + string` works - no implicit conversion
- **Comparison**: Supports all operators (`==`, `!=`, `<`, `<=`, `>`, `>=`) with lexicographic ordering

**Type conversion philosophy:**

- **Explicit conversions**: Following Python's approach - no implicit conversions
    - `"count: " + 42` → error (must use `"count: " + str(42)`)
    - `"42" + 1` → error (must use `int("42") + 1`)
- **Builtins for conversion**: `str()`, `int()`, `float()`

**Equality special case:**

- `==` and `!=` never error on incompatible types - they return `false`
- Example: `42 == "42"` → `false`, not an error
- Rationale: Matches JavaScript/Python/Lua behavior for dynamic typing
- All other operators (`+`, `-`, `<`, etc.) require compatible types

**Conversion semantics:**

- `int()`: Accepts string (parsed), int (identity), float (truncates towards zero, saturates on overflow, NaN → 0)
- `float()`: Accepts string (parsed), float (identity), int (exact conversion)
- `str()`: Accepts any value, uses `Display` formatting
- Follows Java/modern-Rust cast semantics rather than C undefined behavior

**Rationale:**

- Explicit conversions prevent bugs and make intent clear
- Lenient equality enables flexible comparisons without verbosity
- Documented cast behavior avoids surprises with edge cases (NaN, infinity, overflow)

### Source Representation and Spans

**Index-based design with append-only source collection:**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SourceId(usize);

struct Sources {
    sources: Vec<Source>,  // Append-only collection
}

struct Span {
    source_id: SourceId,  // No lifetime - just an ID
    start: usize,
    end: usize,
}

struct Cursor<'src> {
    source: &'src Source,  // Temporary borrow during tokenization
    offset: usize,
}
```

**Design decisions:**

- **Append-only Sources collection** - `Sources` owns all `Source` objects. Sources are never removed, so IDs remain
  valid forever.
- **Spans store IDs, not references** - `Span` contains `SourceId` rather than borrowing from Sources. This allows AST
  to exist while adding new sources (e.g., from import statements).
- **Cursors use lifetimes** - `Cursor` is temporary (used during tokenization), so it borrows `&Source` directly. This
  provides ergonomic access without explicit ID lookups in hot tokenization loops.
- **Explicit Sources passing** - Methods like `span.start_pos(&sources)` take explicit `&Sources` parameter. No
  global/TLS access initially.
- **Byte offset based spans** - Store start/end byte positions, convert to line:column on demand via binary search on
  precomputed line starts.
- **Eager line calculation** - Compute line start positions when loading source, keep `Source` immutable.

**Rationale:**

- **Handles dynamic source loading** - Can add sources after parsing has started (e.g., discovering imports), because
  Span doesn't borrow from Sources.
- **Compact and Copy** - `Span` is just 3 usizes (24 bytes). Can be `Copy`, works across threads without `Arc`.
- **No refcounting overhead** - Unlike `Rc<Source>`, Spans are just integers. No atomic refcount operations.
- **Production pattern** - Index-based source positions are used by rustc, clang, and other production compilers.
- **Clear ownership** - Sources owns the data, Spans reference by ID. No shared ownership complexity.
- **Append-only = IDs always valid** - Since sources are never removed, a `SourceId` is always valid for the lifetime of
  the `Sources` collection.
- **Future evolution** - Can add TLS access later for nicer Debug formatting. Can switch to `Gc<Source>` in Phase 4 if
  shared ownership proves beneficial.

## Future Improvements

Ideas for later phases or optimizations:

### Source Representation

- **TLS access for Debug formatting** - Store `Sources` in thread-local storage to enable `Span` Debug impl to show
  `file:line:col` instead of offsets
- **Virtual source concatenation** - Treat all sources as one virtual string. A single global offset would determine
  both source and position, eliminating need for separate `source_id` field in `Span`

### Value Representation

- **NaN boxing** - Pack `Value` into 64 bits using NaN-boxing technique for better cache locality
