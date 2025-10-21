use std::fmt::Write;
pub struct RuntimeContext {
    output: Option<String>,
}

impl RuntimeContext {
    pub fn new() -> Self {
        Self { output: None }
    }

    pub fn with_capture() -> Self {
        Self {
            output: Some(String::new()),
        }
    }

    pub fn write(&mut self, value: &str) {
        match &mut self.output {
            Some(output) => writeln!(output, "{}", value).unwrap(),
            None => println!("{}", value),
        }
    }

    pub fn take_output(self) -> String {
        self.output
            .expect("Runtime was not configured to capture output")
    }
}
