use std::{error::Error, fs};

use diagnostics::{Canary, Diagnostics};
use lex::Lexer;
use parse::Parser;
use resolve::Resolver;

mod constrain;
mod diagnostics;
mod lex;
mod parse;
mod resolve;
mod table;

fn main() -> Result<(), Box<dyn Error>> {
    let src = fs::read_to_string("example.newt")?;

    let canary = Canary::new();
    let diag = Diagnostics::new(&src, &canary);
    let lex = Lexer::new(diag, &src);
    let mut program = Parser::parse(diag, lex)?;
    Resolver::resolve(diag, &mut program);

    dbg!(&program);

    Ok(())
}
