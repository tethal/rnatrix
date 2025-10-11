use natrix_core::ast_interpreter::eval;
use natrix_core::error::NxResult;
use natrix_core::parser::parse;
use natrix_core::src::Sources;
use natrix_core::value::Value;

fn parse_and_eval(sources: &mut Sources, src: &str) -> NxResult<Value> {
    let source_id = sources.add_from_string(src);
    let expr = dbg!(parse(sources.get_by_id(source_id))?);
    eval(&expr)
}

fn main() {
    let mut sources = Sources::new();
    let result = parse_and_eval(&mut sources, "   ( 42 + 8 )  ");
    match result {
        Ok(value) => println!("Result: {}", value),
        Err(err) => println!("{}", err.display_with(&sources)),
    }
}
