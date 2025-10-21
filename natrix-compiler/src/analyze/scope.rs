use crate::ctx::{CompilerContext, Name};
use crate::error::{err_at, SourceResult};
use crate::hir::{GlobalId, LocalId, LocalInfo, LocalKind};
use crate::src::Span;
use natrix_runtime::value::Builtin;
use std::cell::RefCell;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug, Copy, Clone)]
pub enum Symbol {
    Builtin(Builtin),
    Global(GlobalId),
    Local(LocalId),
}

pub trait Lookup {
    fn symbols(&self) -> &RefCell<HashMap<Name, Symbol>>;
    fn parent(&self) -> Option<&dyn Lookup>;
    fn lookup(&self, ctx: &CompilerContext, name: &Name, name_span: Span) -> SourceResult<Symbol> {
        match self.symbols().borrow().get(name) {
            Some(symbol) => Ok(*symbol),
            None => match self.parent() {
                Some(parent) => parent.lookup(ctx, name, name_span),
                None => err_at(
                    name_span,
                    format!("undeclared variable {:?}", ctx.interner.resolve(*name)),
                ),
            },
        }
    }
}

pub trait LocalScope: Lookup {
    fn create_local(&self, name: Name, name_span: Span, kind: LocalKind) -> LocalId;

    fn declare(
        &self,
        ctx: &CompilerContext,
        name: Name,
        name_span: Span,
        kind: LocalKind,
    ) -> SourceResult<LocalId> {
        match self.symbols().borrow_mut().entry(name) {
            Entry::Vacant(e) => {
                let id = self.create_local(name, name_span, kind);
                e.insert(Symbol::Local(id));
                Ok(id)
            }
            Entry::Occupied(_) => err_at(
                name_span,
                format!(
                    "symbol {} already defined in this scope",
                    ctx.interner.resolve(name)
                ),
            ),
        }
    }
}

pub struct BuiltinScope {
    symbols: RefCell<HashMap<Name, Symbol>>,
}

impl BuiltinScope {
    pub fn new(ctx: &CompilerContext) -> Rc<Self> {
        let mut symbols: HashMap<Name, Symbol> = HashMap::new();
        for builtin in Builtin::ALL {
            symbols.insert(
                ctx.interner.lookup(builtin.name()).unwrap(),
                Symbol::Builtin(*builtin),
            );
        }
        Rc::new(BuiltinScope {
            symbols: RefCell::new(symbols),
        })
    }
}

impl Lookup for BuiltinScope {
    fn symbols(&self) -> &RefCell<HashMap<Name, Symbol>> {
        &self.symbols
    }

    fn parent(&self) -> Option<&dyn Lookup> {
        None
    }
}

pub struct GlobalScope {
    parent: Rc<BuiltinScope>,
    symbols: RefCell<HashMap<Name, Symbol>>,
}

impl GlobalScope {
    pub fn new(ctx: &CompilerContext) -> Rc<Self> {
        Rc::new(GlobalScope {
            parent: BuiltinScope::new(ctx),
            symbols: RefCell::new(HashMap::new()),
        })
    }

    pub fn declare(
        &self,
        ctx: &CompilerContext,
        name: Name,
        name_span: Span,
        id: GlobalId,
    ) -> SourceResult<()> {
        match self.symbols.borrow_mut().entry(name) {
            Entry::Vacant(e) => {
                e.insert(Symbol::Global(id));
                Ok(())
            }
            Entry::Occupied(_) => err_at(
                name_span,
                format!(
                    "symbol {} already defined in this scope",
                    ctx.interner.resolve(name)
                ),
            ),
        }
    }
}

pub struct FunctionScope {
    parent: Rc<GlobalScope>,
    locals: RefCell<Vec<LocalInfo>>,
    symbols: RefCell<HashMap<Name, Symbol>>,
}

impl FunctionScope {
    pub fn new(parent: Rc<GlobalScope>) -> Rc<FunctionScope> {
        Rc::new(FunctionScope {
            parent,
            locals: RefCell::new(Vec::new()),
            symbols: RefCell::new(HashMap::new()),
        })
    }

    pub fn take_locals(&self) -> Vec<LocalInfo> {
        self.locals.take()
    }
}

pub struct BlockScope {
    parent: Rc<dyn LocalScope>,
    symbols: RefCell<HashMap<Name, Symbol>>,
}

impl BlockScope {
    pub fn new(parent: Rc<dyn LocalScope>) -> Rc<BlockScope> {
        Rc::new(BlockScope {
            parent,
            symbols: RefCell::new(HashMap::new()),
        })
    }
}

macro_rules! impl_lookup {
    ($type:ty) => {
        impl Lookup for $type {
            fn symbols(&self) -> &RefCell<HashMap<Name, Symbol>> {
                &self.symbols
            }

            fn parent(&self) -> Option<&dyn Lookup> {
                Some(&*self.parent)
            }
        }
    };
}

impl_lookup!(GlobalScope);
impl_lookup!(FunctionScope);
impl_lookup!(BlockScope);

impl LocalScope for FunctionScope {
    fn create_local(&self, name: Name, name_span: Span, kind: LocalKind) -> LocalId {
        let id = LocalId(self.locals.borrow().len());
        self.locals
            .borrow_mut()
            .push(LocalInfo::new(id, name, name_span, kind));
        id
    }
}

impl LocalScope for BlockScope {
    fn create_local(&self, name: Name, name_span: Span, kind: LocalKind) -> LocalId {
        self.parent.create_local(name, name_span, kind)
    }
}
