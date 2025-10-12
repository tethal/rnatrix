use natrix_core::ast_interpreter::eval;
use natrix_core::ctx::CompilerContext;
use natrix_core::error::NxResult;
use natrix_core::parser::parse;
use natrix_core::value::Value;

fn parse_and_eval(ctx: &mut CompilerContext, src: &str) -> NxResult<Value> {
    let source_id = ctx.sources.add_from_string(src);
    let expr = dbg!(parse(ctx, source_id)?);
    eval(&expr)
}

fn main() {
    let mut ctx = CompilerContext::default();
    let result = parse_and_eval(&mut ctx, "   ( 42 + a )  ");
    match result {
        Ok(value) => println!("Result: {}", value),
        Err(err) => println!("{}", err.display_with(&ctx.sources)),
    }
}
