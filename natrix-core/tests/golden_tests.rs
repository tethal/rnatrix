use natrix_core::ast_interpreter::eval;
use natrix_core::parser::parse;
use natrix_core::src::Sources;
use natrix_core::token::{TokenType, Tokenizer};
use std::fmt::Write;
use std::path::Path;
use test_utils::{datatest_stable, run_golden_test};

fn test_tokenizer(path: &Path) -> test_utils::TestResult {
    run_golden_test(path, |input| {
        let mut sources = Sources::default();
        let source_id = sources.add_from_string(input);
        let source = sources.get_by_id(source_id);
        let mut tokenizer = Tokenizer::new(source);
        let mut result = String::new();
        loop {
            let token = match tokenizer.next_token() {
                Ok(token) => token,
                Err(error) => {
                    writeln!(result, "{}", error.display_with(&sources)).unwrap();
                    break;
                }
            };
            writeln!(result, "{:?}: {:?}", token, tokenizer.lexeme(&token)).unwrap();
            if token.tt == TokenType::Eof {
                break;
            }
        }
        result
    })
}

fn test_parser(path: &Path) -> test_utils::TestResult {
    run_golden_test(path, |input| {
        let mut sources = Sources::default();
        let source_id = sources.add_from_string(input);
        let source = sources.get_by_id(source_id);
        match parse(source) {
            Ok(ast) => format!("{:?}", ast.debug_with(&sources)),
            Err(error) => format!("{}", error.display_with(&sources)),
        }
    })
}

fn test_ast_interpreter(path: &Path) -> test_utils::TestResult {
    run_golden_test(path, |input| {
        let mut sources = Sources::default();
        let source_id = sources.add_from_string(input);
        let source = sources.get_by_id(source_id);
        let result = parse(source).and_then(|e| eval(&e));
        match &result {
            Ok(value) => format!("{:?}", value),
            Err(error) => format!("{}", error.display_with(&sources)),
        }
    })
}

const INPUT_PATTERN: &str = r".*\.nx$";

datatest_stable::harness! {
    { test = test_tokenizer, root = "../tests/tokenizer", pattern = INPUT_PATTERN },
    { test = test_parser, root = "../tests/parser", pattern = INPUT_PATTERN },
    { test = test_ast_interpreter, root = "../tests/ast_interpreter", pattern = INPUT_PATTERN },
}
