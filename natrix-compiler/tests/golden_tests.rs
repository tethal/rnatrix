use natrix_compiler::ast::Interpreter;
use natrix_compiler::ctx::CompilerContext;
use natrix_compiler::parser::parse;
use natrix_compiler::token::{TokenType, Tokenizer};
use natrix_runtime::ctx::RuntimeContext;
use std::fmt::Write;
use std::path::Path;
use test_utils::{datatest_stable, run_golden_test};

fn test_tokenizer(path: &Path) -> test_utils::TestResult {
    run_golden_test(path, |input| {
        let mut ctx = CompilerContext::default();
        let source_id = ctx.sources.add_from_string(input);
        let mut tokenizer = Tokenizer::new(&mut ctx, source_id);
        let mut result = String::new();
        loop {
            let token = match tokenizer.next_token() {
                Ok(token) => token,
                Err(error) => {
                    writeln!(result, "{}", error.display_with(&ctx.sources)).unwrap();
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
        let mut ctx = CompilerContext::default();
        let source_id = ctx.sources.add_from_string(input);
        match parse(&mut ctx, source_id) {
            Ok(ast) => format!("{:?}", ast.debug_with(&ctx)),
            Err(error) => format!("{}", error.display_with(&ctx.sources)),
        }
    })
}

fn test_ast_interpreter(path: &Path) -> test_utils::TestResult {
    run_golden_test(path, |input| {
        let mut ctx = CompilerContext::default();
        let source_id = ctx.sources.add_from_string(input);
        let program = match parse(&mut ctx, source_id) {
            Ok(program) => program,
            Err(error) => {
                return format!("{}", error.display_with(&ctx.sources));
            }
        };
        let mut rt = RuntimeContext::with_capture();
        let mut interpreter = Interpreter::new(&mut ctx, &mut rt);
        let result = interpreter.run(program, vec![]);
        let mut output = rt.take_output();
        if let Err(error) = result {
            writeln!(output, "{}", error.display_with(&ctx.sources)).unwrap();
        }
        output
    })
}

const INPUT_PATTERN: &str = r".*\.nx$";

datatest_stable::harness! {
    { test = test_tokenizer, root = "../tests/tokenizer", pattern = INPUT_PATTERN },
    { test = test_parser, root = "../tests/parser", pattern = INPUT_PATTERN },
    { test = test_ast_interpreter, root = "../tests/ast_interpreter", pattern = INPUT_PATTERN },
}
