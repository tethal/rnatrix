use natrix_core::ast_interpreter::eval;
use natrix_core::parser::parse;
use natrix_core::src::Sources;

fn main() {
    let mut sources = Sources::new();
    let source_id = sources.add_from_string("40 + 2");
    let result = parse(sources.get_by_id(source_id));
    match result {
        Ok(expr) => {
            println!("{:?}", expr.debug_with(&sources));
            let expr = dbg!(expr);
            println!("{:?}", eval(&expr));
        }
        Err(err) => println!(
            "Error: {} at {:?}",
            err.message,
            err.span.unwrap().start_pos(&sources)
        ),
    }
}
