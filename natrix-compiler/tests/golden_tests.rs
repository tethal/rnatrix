use natrix_compiler::analyze::analyze;
use natrix_compiler::ast::Interpreter as AstInterpreter;
use natrix_compiler::bc::compiler::compile;
use natrix_compiler::ctx::CompilerContext;
use natrix_compiler::error::SourceResult;
use natrix_compiler::parser::parse;
use natrix_compiler::src::SourceId;
use natrix_compiler::token::{TokenType, Tokenizer};
use natrix_runtime::bc::{Bytecode, Interpreter as BcInterpreter};
use natrix_runtime::ctx::RuntimeContext;
use std::fmt::Write;
use std::path::Path;
use test_utils::{datatest_stable, run_golden_test, run_golden_test_variant};

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
    run_golden_test_variant(path, "ast", |input| {
        let mut ctx = CompilerContext::default();
        let source_id = ctx.sources.add_from_string(input);
        let program = match parse(&mut ctx, source_id) {
            Ok(program) => program,
            Err(error) => {
                return format!("{}", error.display_with(&ctx.sources));
            }
        };
        let mut rt = RuntimeContext::with_capture();
        let mut interpreter = AstInterpreter::new(&mut ctx, &mut rt);
        let result = interpreter.run(program, vec![]);
        let mut output = rt.take_output();
        if let Err(error) = result {
            writeln!(output, "{}", error.display_with(&ctx.sources)).unwrap();
        }
        output
    })
}

fn compile_to_bc(ctx: &mut CompilerContext, source_id: SourceId) -> SourceResult<Bytecode> {
    let program = parse(ctx, source_id)?;
    let hir = analyze(&ctx, &program)?;
    compile(&ctx, &hir)
}

fn test_bc_interpreter(path: &Path) -> test_utils::TestResult {
    run_golden_test_variant(path, "bc", |input| {
        let mut ctx = CompilerContext::default();
        let source_id = ctx.sources.add_from_string(input);
        let bc = match compile_to_bc(&mut ctx, source_id) {
            Ok(bc) => bc,
            Err(error) => {
                return format!("{}", error.display_with(&ctx.sources));
            }
        };

        let mut rt = RuntimeContext::with_capture();
        let mut interpreter = BcInterpreter::new(&mut rt);
        let result = interpreter.run(&bc, vec![]);
        let mut output = rt.take_output();
        if let Err(error) = result {
            writeln!(output, "{:?}", error).unwrap();
        }
        output
    })
}

const INPUT_PATTERN: &str = r".*\.nx$";

datatest_stable::harness! {
    { test = test_tokenizer, root = "../tests/tokenizer", pattern = INPUT_PATTERN },
    { test = test_parser, root = "../tests/parser", pattern = INPUT_PATTERN },
    { test = test_ast_interpreter, root = "../tests/ast_interpreter", pattern = INPUT_PATTERN },
    { test = test_ast_interpreter, root = "../tests/common_interpreter", pattern = INPUT_PATTERN },
    { test = test_bc_interpreter, root = "../tests/common_interpreter", pattern = INPUT_PATTERN },
    { test = test_bc_interpreter, root = "../tests/bc_interpreter", pattern = INPUT_PATTERN },
}
