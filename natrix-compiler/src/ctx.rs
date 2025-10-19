use crate::src::Sources;
use crate::token_type::{TokenType, KEYWORDS};
use natrix_runtime::runtime::Builtin;
use std::collections::HashMap;
use std::num::NonZeroU32;

/// Compiler context containing shared infrastructure used throughout the compilation pipeline.
pub struct CompilerContext {
    pub sources: Sources,
    pub interner: Interner,
}

impl CompilerContext {
    pub fn new() -> Self {
        let mut interner = Interner::new();
        for builtin in Builtin::ALL {
            interner.intern(builtin.name());
        }
        Self {
            sources: Sources::new(),
            interner,
        }
    }
}

impl Default for CompilerContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Unique identifier for an interned string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Name(NonZeroU32);

/// String interner for deduplicating identifiers and other string literals.
///
/// Stores each unique string once and returns a lightweight `Name` that can be
/// copied and compared efficiently.
pub struct Interner {
    strings: Vec<Box<str>>,
    // NOTE: This duplicates string storage (once in `strings` Vec, once as HashMap keys).
    // This is safe and simple, but wastes memory. A future optimization could use unsafe
    // code to store raw pointers into `strings` as HashMap keys, eliminating the duplication.
    map: HashMap<Box<str>, Name>,
}

impl Interner {
    pub fn new() -> Self {
        let mut interner = Self {
            strings: Vec::new(),
            map: HashMap::new(),
        };
        for &(kw, _) in KEYWORDS {
            interner.intern(kw);
        }
        interner
    }

    /// Interns a string, returning its interned name.
    pub fn intern(&mut self, s: &str) -> Name {
        if let Some(&sym) = self.map.get(s) {
            return sym;
        }

        let sym = Name(
            NonZeroU32::new(
                u32::try_from(self.strings.len() + 1).expect("too many interned strings"),
            )
            .unwrap(), // safe since (self.strings.len() + 1) is always >= 1
        );
        let boxed: Box<str> = s.into();

        self.strings.push(boxed.clone());
        self.map.insert(boxed, sym);

        sym
    }

    pub fn resolve(&self, sym: Name) -> &str {
        &self.strings[sym.0.get() as usize - 1]
    }

    pub fn resolve_keyword(&self, name: Name) -> Option<TokenType> {
        let idx = name.0.get() as usize - 1;
        KEYWORDS.get(idx).map(|&(_, tt)| tt)
    }

    pub fn lookup(&self, name: &str) -> Option<Name> {
        self.map.get(name).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intern_basic() {
        let mut interner = Interner::new();
        let sym1 = interner.intern("hello");
        let sym2 = interner.intern("world");

        assert_ne!(sym1, sym2);
        assert_eq!(interner.resolve(sym1), "hello");
        assert_eq!(interner.resolve(sym2), "world");
    }

    #[test]
    fn test_intern_deduplication() {
        let mut interner = Interner::new();
        let sym1 = interner.intern("foo");
        let sym2 = interner.intern("bar");
        let sym3 = interner.intern("foo"); // Same as sym1

        assert_eq!(sym1, sym3);
        assert_ne!(sym1, sym2);
        assert_eq!(interner.resolve(sym1), "foo");
        assert_eq!(interner.resolve(sym3), "foo");
    }

    #[test]
    fn test_intern_empty_string() {
        let mut interner = Interner::new();
        let sym = interner.intern("");
        assert_eq!(interner.resolve(sym), "");
    }

    #[test]
    fn test_intern_unicode() {
        let mut interner = Interner::new();
        let sym1 = interner.intern("日本語");
        let sym2 = interner.intern("🦀");

        assert_eq!(interner.resolve(sym1), "日本語");
        assert_eq!(interner.resolve(sym2), "🦀");
    }

    #[test]
    fn test_name_size_optimization() {
        // Name should be 4 bytes (u32)
        assert_eq!(size_of::<Name>(), 4);
        // Option<Name> should also be 4 bytes due to NonZeroU32 optimization
        assert_eq!(size_of::<Option<Name>>(), 4);
    }
}
