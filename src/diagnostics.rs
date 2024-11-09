use std::{cell::Cell, ops::Range};

use inline_colorization::*;

use source_highlight::SourceHighlight;

mod source_highlight;

pub struct Canary {
    had_error: Cell<bool>,
}

impl Canary {
    pub fn new() -> Canary {
        Canary {
            had_error: Cell::new(false),
        }
    }

    pub fn had_error(&self) -> bool {
        self.had_error.get()
    }

    fn mark(&self) {
        self.had_error.set(true);
    }
}

#[derive(Clone, Copy)]
pub struct Diagnostics<'s> {
    src: &'s str,
    canary: &'s Canary,
}

impl<'s> Diagnostics<'s> {
    pub fn new(src: &'s str, canary: &'s Canary) -> Diagnostics<'s> {
        Diagnostics { src, canary }
    }

    pub fn warning(&self, span: Range<usize>, message: &str) {
        let highlight = SourceHighlight::new(self.src, span.clone());
        println!(
            "{color_blue}-->{color_reset} {style_bold}{color_yellow}warning{color_reset}: {message} at example.newt:{}:{}{style_reset}",
            highlight.start_row() + 1,
            highlight.start_col() + 1,
        );
        println!("{highlight}");
    }

    pub fn warning_with_explanation(&self, span: Range<usize>, title: &str, explanation: &str) {
        let highlight = SourceHighlight::new(self.src, span.clone());
        println!(
            "{color_blue}-->{color_reset} {style_bold}{color_yellow}warning{color_reset}: {title} at example.newt:{}:{}{style_reset}",
            highlight.start_row() + 1,
            highlight.start_col() + 1,
        );
        println!("{highlight}");
        println!(" {color_blue}\\-{color_reset} {color_cyan}{explanation}{color_reset}");
    }

    pub fn error(&self, span: Range<usize>, message: &str) {
        self.canary.mark();
        let highlight = SourceHighlight::new(self.src, span.clone());
        println!(
            "{color_blue}-->{color_reset} {style_bold}{color_red}error{color_reset}: {message} at example.newt:{}:{}{style_reset}",
            highlight.start_row() + 1,
            highlight.start_col() + 1,
        );
        println!("{highlight}");
    }

    pub fn error_with_explanation(&self, span: Range<usize>, title: &str, explanation: &str) {
        self.canary.mark();
        let highlight = SourceHighlight::new(self.src, span.clone());
        println!(
            "{color_blue}-->{color_reset} {style_bold}{color_red}error{color_reset}: {title} at example.newt:{}:{}{style_reset}",
            highlight.start_row() + 1,
            highlight.start_col() + 1,
        );
        println!("{highlight}");
        println!(" {color_blue}\\-{color_reset} {color_cyan}{explanation}{color_reset}");
    }
}
