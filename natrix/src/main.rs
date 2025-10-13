use natrix_core::ast_interpreter::Interpreter;
use natrix_core::ctx::CompilerContext;
use natrix_core::error::NxResult;
use natrix_core::parser::parse;
use natrix_core::value::Value;

fn parse_and_eval(ctx: &mut CompilerContext, src: &str) -> NxResult<Value> {
    let source_id = ctx.sources.add_from_string(src);
    let program = parse(ctx, source_id)?;
    println!("{:?}", program.debug_with(&ctx));
    let mut interpreter = Interpreter::new(ctx);
    interpreter.run(&program, vec![Value::from_int(35)])
}

fn main() {
    let mut ctx = CompilerContext::default();
    let result = parse_and_eval(&mut ctx, "fun main(a) { print a + 7; }");
    match result {
        Ok(value) => println!("Result: {}", value),
        Err(err) => println!("{}", err.display_with(&ctx.sources)),
    }
}
