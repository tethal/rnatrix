pub use datatest_stable;
use std::path::Path;

pub type TestResult = Result<(), Box<dyn std::error::Error>>;

pub fn assert_golden(test_file: &Path, actual: &str, variant: &str) -> TestResult {
    let expected_path = test_file.with_extension("expected");
    let actual_path = test_file.with_extension(&format!("{}actual", variant));

    match std::fs::read_to_string(&expected_path) {
        Ok(expected) => {
            // Expected file exists - compare
            if actual.trim() == expected.trim() {
                // Test passed - clean up any leftover .actual file
                let _ = std::fs::remove_file(&actual_path);
                Ok(())
            } else {
                // Test failed - write .actual file
                std::fs::write(&actual_path, actual)?;
                Err(format!(
                    "Golden file mismatch\nExpected: {}\nActual: {}\n\n{}",
                    expected_path.display(),
                    actual_path.display(),
                    diff(&expected, actual)
                )
                .into())
            }
        }
        Err(_) => {
            // Expected file doesn't exist - create it
            std::fs::write(&expected_path, actual)?;
            // Clean up any leftover .actual file
            let _ = std::fs::remove_file(&actual_path);
            println!("Created golden file: {}", expected_path.display());
            Ok(())
        }
    }
}

fn diff(expected: &str, actual: &str) -> String {
    // TODO: use a diff crate for prettier output
    format!("Expected:\n{}\n\nActual:\n{}", expected, actual)
}

pub fn run_golden_test(path: &Path, transform: fn(&str) -> String) -> TestResult {
    let input = std::fs::read_to_string(path)?;
    let actual = transform(&input);
    assert_golden(path, &actual, "")?;
    Ok(())
}

pub fn run_golden_test_variant(
    path: &Path,
    extension: &str,
    transform: fn(&str) -> String,
) -> TestResult {
    let input = std::fs::read_to_string(path)?;
    let actual = transform(&input);
    assert_golden(path, &actual, &format!("{}.", extension))?;
    Ok(())
}
