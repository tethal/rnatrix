pub mod src;
pub mod token;
pub mod value;

pub fn transform(s: &str) -> String {
    s.to_ascii_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = transform("aBcD");
        assert_eq!(result, "ABCD");
    }
}
