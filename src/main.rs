use std::{error::Error, fs};

use lex::Lexer;
use parse::Parser;

mod lex;
mod parse;

fn main() -> Result<(), Box<dyn Error>> {
    let src = fs::read_to_string("example.newt")?;

    let lex = Lexer::new(&src);

    let program = Parser::parse(lex)?;

    dbg!(program);

    Ok(())
}
