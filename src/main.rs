use std::{error::Error, fs};

use lex::Lexer;
use parse::Parser;
use resolve::Resolver;

mod lex;
mod parse;
mod resolve;

fn main() -> Result<(), Box<dyn Error>> {
    let src = fs::read_to_string("example.newt")?;

    let lex = Lexer::new(&src);

    let mut program = Parser::parse(lex)?;

    Resolver::resolve(&mut program);

    dbg!(&program);

    Ok(())
}
