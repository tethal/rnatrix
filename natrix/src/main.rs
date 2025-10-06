use natrix_core::src::Sources;
use natrix_core::token::{TokenType, Tokenizer};

fn main() {
    let mut sources = Sources::new();
    let source_id = sources.add_from_string("1 + 2");
    let mut tokenizer = Tokenizer::new(sources.get_by_id(source_id));
    loop {
        let token = tokenizer.advance();
        println!("{:?}: {:?}", token, tokenizer.lexeme(&token));
        if token.tt == TokenType::Eof {
            break;
        }
    }
}
