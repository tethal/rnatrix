use natrix_core::src::Sources;
use natrix_core::token::{TokenType, Tokenizer};
use natrix_core::transform;
use std::path::Path;
use test_utils::{datatest_stable, run_golden_test};

fn test_transform(path: &Path) -> test_utils::TestResult {
    run_golden_test(path, |input| transform(input))
}

fn test_tokenizer(path: &Path) -> test_utils::TestResult {
    run_golden_test(path, |input| {
        let mut sources = Sources::default();
        let source_id = sources.add_from_string(input);
        let source = sources.get_by_id(source_id);
        let mut tokenizer = Tokenizer::new(source);
        let mut result = String::new();
        loop {
            let token = tokenizer.advance();
            result.push_str(format!("{:?}: {:?}\n", token, tokenizer.lexeme(&token)).as_str());
            if token.tt == TokenType::Eof {
                break;
            }
        }
        result
    })
}

const INPUT_PATTERN: &str = r".*\.nx$";

datatest_stable::harness! {
    { test = test_transform, root = "../tests/transform", pattern = INPUT_PATTERN },
    { test = test_tokenizer, root = "../tests/tokenizer", pattern = INPUT_PATTERN },
}
