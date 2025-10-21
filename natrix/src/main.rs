use natrix_compiler::analyze::analyze;
use natrix_compiler::ast::Interpreter as AstInterpreter;
use natrix_compiler::bc::compiler::compile;
use natrix_compiler::ctx::CompilerContext;
use natrix_compiler::error::{AttachErrSpan, SourceResult};
use natrix_compiler::parser::parse;
use natrix_runtime::bc::Interpreter as BcInterpreter;
use natrix_runtime::ctx::RuntimeContext;
use natrix_runtime::value::Value;

fn parse_and_eval(ctx: &mut CompilerContext, src: &str, arg: i64) -> SourceResult<()> {
    let source_id = ctx
        .sources
        .add_from_file(src)
        .expect("Unable to load source file");
    let ast = parse(ctx, source_id)?;
    println!("{:?}", ast.debug_with(&ctx));

    let hir = analyze(&ctx, &ast)?;
    println!("{:?}", hir.debug_with(&ctx));

    let bc = compile(ctx, &hir)?;
    println!("BC: {:02X?}", bc.code);

    let mut rt = RuntimeContext::new();
    let mut interpreter = AstInterpreter::new(ctx, &mut rt);
    println!(
        "AST result: {}",
        interpreter.run(ast, vec![Value::from_int(arg)])?
    );

    let mut interpreter = BcInterpreter::new(&mut rt);
    println!(
        "BC result: {}",
        interpreter
            .run(&bc, vec![Value::from_int(arg)])
            .err_at(hir.span)?
    );
    Ok(())
}

fn main() {
    let mut ctx = CompilerContext::default();
    let result = parse_and_eval(&mut ctx, "demos/hanoi.nx", 14);
    if let Err(err) = result {
        println!("{}", err.display_with(&ctx.sources));
    }
}
