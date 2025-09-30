use natrix_core::transform;
use std::path::Path;
use test_utils::{datatest_stable, run_golden_test};

fn test_transform(path: &Path) -> test_utils::TestResult {
    run_golden_test(path, |input| transform(input))
}

const INPUT_PATTERN: &str = r".*\.nx$";

datatest_stable::harness! {
    { test = test_transform, root = "../tests/transform", pattern = INPUT_PATTERN },
}
