# Bytecode Specification

This document specifies the bytecode format for the Natrix virtual machine (Phase 2).

## Overview

- **Architecture:** Stack-based VM
- **Instruction encoding:** Variable-width using LEB128
- **Address encoding:** Signed LEB128 (SLEB128) for jump offsets, unsigned LEB128 for indices
- **Value stack:** Separate `Vec<Value>` for runtime values
- **Frame metadata:** Separate `Vec<CallFrame>` tracking return addresses and frame pointers

## Bytecode Structure

```rust
struct Bytecode {
    code: Vec<u8>,          // Flat bytecode stream
    constants: Vec<Value>,  // Constant pool (high-level values)
}
```

Functions are compiled to offsets within the flat `code` array. Entry point is conventionally at offset 0.

## Instruction Set

### Stack Notation

- `...` - existing stack contents
- `a, b` - values (top of stack is rightmost)
- `a -> b` - stack transition: pop `a`, push `b`

### Immediates Notation

- `N` - unsigned LEB128 immediate (index into constant pool, variable table, etc.)
- `offset` - signed LEB128 immediate (relative jump offset)

---

### Constants and Literals

| Opcode       | Immediates | Stack Effect        | Description                           |
|--------------|------------|---------------------|---------------------------------------|
| `push_const` | N          | `... -> ..., const` | Push constant from pool at index N    |
| `push_null`  | -          | `... -> ..., null`  | Push null constant                    |
| `push_true`  | -          | `... -> ..., true`  | Push boolean true                     |
| `push_false` | -          | `... -> ..., false` | Push boolean false                    |
| `push_0`     | -          | `... -> ..., 0`     | Push integer 0                        |
| `push_1`     | -          | `... -> ..., 1`     | Push integer 1                        |
| `push_i8`    | byte       | `... -> ..., int`   | Push signed 8-bit integer (-128..127) |

*Note: Special opcodes for common constants reduce bytecode size and avoid constant pool lookups. Integers outside the
-128..127 range use `push_const`.*

---

### Arithmetic Operators

| Opcode | Stack Effect                      | Description                                  |
|--------|-----------------------------------|----------------------------------------------|
| `add`  | `..., left, right -> ..., result` | Addition (also string/list concatenation)    |
| `sub`  | `..., left, right -> ..., result` | Subtraction                                  |
| `mul`  | `..., left, right -> ..., result` | Multiplication (also string/list repetition) |
| `div`  | `..., left, right -> ..., result` | Division                                     |
| `mod`  | `..., left, right -> ..., result` | Modulo/remainder                             |

All operators perform runtime type checking and coercion (int/float).

---

### Comparison Operators

| Opcode | Stack Effect                    | Description                                     |
|--------|---------------------------------|-------------------------------------------------|
| `eq`   | `..., left, right -> ..., bool` | Equality (returns false for incompatible types) |
| `ne`   | `..., left, right -> ..., bool` | Inequality                                      |
| `lt`   | `..., left, right -> ..., bool` | Less than                                       |
| `le`   | `..., left, right -> ..., bool` | Less than or equal                              |
| `gt`   | `..., left, right -> ..., bool` | Greater than                                    |
| `ge`   | `..., left, right -> ..., bool` | Greater than or equal                           |

String comparisons use lexicographic ordering. Numeric comparisons work across int/float.

---

### Unary Operators

| Opcode | Stack Effect                | Description                     |
|--------|-----------------------------|---------------------------------|
| `neg`  | `..., value -> ..., result` | Numeric negation                |
| `not`  | `..., value -> ..., result` | Logical negation (boolean only) |

---

### Variables

| Opcode      | Immediates | Stack Effect        | Description                  |
|-------------|------------|---------------------|------------------------------|
| `load_var`  | N          | `... -> ..., value` | Load variable at index N     |
| `load_1`    | -          | `... -> ..., value` | Load variable at index 1     |
| `store_var` | N          | `..., value -> ...` | Store to variable at index N |

Variable indices are **relative to the frame pointer** (`fp`):

- Index 0: function object (not normally accessed; reserved for future reflection/introspection features)
- Indices 1..arity: function arguments
- Indices (arity+1)..: local variables

All variables (arguments and locals) use the same addressing scheme with unsigned indices.

*Note: `load_1` is a special opcode for loading the first argument, which is extremely common for `self`/`this` in
method calls and primary data arguments. Saves 1 byte per access.*

---

### Collections

| Opcode      | Immediates | Stack Effect                           | Description                                 |
|-------------|------------|----------------------------------------|---------------------------------------------|
| `make_list` | N          | `..., val0, ..., valN -> ..., list`    | Create list from top N stack values         |
| `get_item`  | -          | `..., collection, index -> ..., value` | Index into list or string                   |
| `set_item`  | -          | `..., list, index, value -> ...`       | Mutate list element (strings not supported) |

---

### Control Flow

| Opcode   | Immediates | Stack Effect       | Description                    |
|----------|------------|--------------------|--------------------------------|
| `jmp`    | offset     | `... -> ...`       | Unconditional relative jump    |
| `jtrue`  | offset     | `..., cond -> ...` | Jump if true (pops condition)  |
| `jfalse` | offset     | `..., cond -> ...` | Jump if false (pops condition) |

Jump offsets are **relative** to the instruction pointer after reading the offset immediate.

---

### Functions

| Opcode | Immediates | Stack Effect                                | Description                     |
|--------|------------|---------------------------------------------|---------------------------------|
| `call` | N          | `..., func, arg0, ..., argN -> ..., result` | Call function with N arguments  |
| `ret`  | -          | `..., value -> ...`                         | Return from function with value |

#### Calling Convention

**Function values:**

```rust

// Interpreter-specific handle to corresponding code (AST node or bytecode offset)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodeHandle(pub usize);

pub struct FunctionObject {
    pub name: Box<str>,
    pub arity: usize,
    pub code_handle: CodeHandle,
}

enum ValueImpl {
    //...
    Function(Rc<FunctionObject>),
    //...
}
```

**Call sequence:**

1. Caller pushes function object onto stack
2. Caller evaluates and pushes N arguments
3. `call N` instruction:
    - Validates `function.arity == N` (runtime arity check)
    - Pushes new `CallFrame { return_ip, prev_fp }` to frame metadata stack
    - Sets `fp` to point to function object on value stack
    - Sets `ip = function.id` (function start address)
4. Callee executes with arguments and locals accessible via `fp + offset`
5. `ret` instruction:
    - Saves return value from top of stack
    - Resets `sp = fp` (discards all args and locals; no copying)
    - Writes return value at `fp` (overwrites function object slot)
    - Pops `CallFrame`, restores `ip` and `fp`

**Implementation note:** Arguments and locals remain in place on the stack - no copying occurs. Variable access uses
`stack[fp + index]`.

**Stack layout during call:**

```
[... caller frame ...]
[func]                  <- fp points here at call time
[arg0]
[arg1]
...
[argN]
[... callee locals ...]
```

Frame metadata stored separately in `Vec<CallFrame>`.

---

### Other

| Opcode | Stack Effect        | Description          |
|--------|---------------------|----------------------|
| `pop`  | `..., value -> ...` | Discard top of stack |

Used for expression statements that don't use their result.

---

## Future Extensions

### Default and Keyword Arguments

**Challenge:** With first-class functions, the caller doesn't know the callee's full signature at compile time.

**Solution:** Extend function values to carry metadata:

```rust
struct FunctionValue {
    name: String,
    arity: usize,           // Minimum required arguments
    max_arity: usize,       // Including optional parameters
    defaults: Vec<Value>,   // Default values for optional params
    param_names: Vec<String>, // For keyword argument matching
    id: FunctionId,
}
```

**Calling convention changes:**

- Caller passes arguments as positional or as key-value pairs
- Callee (or VM runtime) performs **argument shuffling**:
    1. Match provided args (positional and keyword) to parameter names
    2. Fill missing optional params with defaults
    3. Arrange final arguments on stack in correct order
- Possible new opcodes: `call_kwargs <n_pos> <n_kw>` with keyword dict on stack

This deferred to **Phase 5**.

---

### Generators and Coroutines

**Challenge:** Generators must suspend execution (`yield`) and resume later, preserving stack state between calls.

**Problem with flat stack:** When a generator yields, its stack frame cannot remain on the main VM stack (caller would
overwrite it).

**Solution 1: Separate stack per generator (Python-style)**

```rust
struct GeneratorState {
    stack: Vec<Value>,     // Dedicated stack
    frames: Vec<CallFrame>, // Own frame metadata
    ip: usize,
}
```

- First call to generator: allocate `GeneratorState`, copy args, return generator object
- `yield`: save `ip`, return value, keep state alive
- `next(g)`: resume execution on generator's stack
- Pro: Full support for nested calls within generators
- Con: Memory overhead (each generator has separate stack)

**Solution 2: Frame copy to heap (Lua-style)**

```rust
struct GeneratorState {
    frame: Vec<Value>,  // Single frame only
    ip: usize,
}
```

- `yield`: copy current frame to heap, pop from main stack
- `next(g)`: push frame back onto main stack, resume
- Pro: Lower memory overhead
- Con: Cannot call other functions from generator (or requires more complex logic)

**Recommendation:** Solution 1 (separate stack) for full generator support. Use distinct opcodes:

- `call_generator <N>` - allocate generator state
- `yield` - suspend execution
- `resume_generator` - restore generator state and continue

This deferred to **Phase 5**.

---

## Implementation Notes

- **LEB128 encoding:** Variable-width integer encoding. Use signed variant (SLEB128) for jump offsets, unsigned for
  indices.
- **Constant pool:** Stores `Value` objects (including `Function` values). No serialization in Phase 2 (in-memory only).
- **Static calls in Phase 2:** All function calls are statically resolved. Function objects placed in constant pool,
  calls use `push_const <func>` + `call <arity>`.
- **First-class functions in Phase 3+:** Same bytecode format. Variables can hold function values, `load_var` pushes
  function, `call` works identically.