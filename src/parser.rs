use crate::token::Token;

struct Tape<'a> {
    input: &'a [Token<'a>],
    offset: usize,
}

impl<'a> Tape<'a> {
    fn new(input: &'a [Token<'a>]) -> Self {
        Self { input, offset: 0 }
    }

    fn peek<'b>(&'b self) -> &'b Token<'a> {
        &self.input[self.offset]
    }

    fn shift<'b>(&'b mut self) -> &'b Token<'a> {
        let res = &self.input[self.offset];

        if self.offset + 1 < self.input.len() {
            self.offset += 1;
        }

        res
    }
}

pub fn parse(input: &[Token]) -> Result<(), ()> {
    Ok(())
}
