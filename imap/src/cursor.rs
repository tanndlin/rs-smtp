use std::borrow::Cow;

use crate::errors::ParseError;

pub struct Cursor<'a> {
    buf: &'a [u8],
    pub pos: usize,
}

impl<'a> Cursor<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    fn peek(&self) -> Option<u8> {
        self.buf.get(self.pos).copied()
    }

    pub fn eat(&mut self, b: u8) -> Result<(), ParseError> {
        if let Some(c) = self.peek()
            && c == b' '
        {
            self.skip_sp();
        }

        if self
            .peek()
            .ok_or_else(|| ParseError::out_of_bytes(self.pos))?
            != b
        {
            return Err(ParseError::unexpected_char(self.pos));
        }
        self.pos += 1;
        Ok(())
    }

    fn skip_sp(&mut self) {
        self.pos += 1;
    }

    pub fn atom(&mut self) -> Result<&'a str, ParseError> {
        if let Some(c) = self.peek()
            && c == b' '
        {
            self.skip_sp();
        }

        let start = self.pos;
        while self.peek().is_some_and(is_atom_char) {
            self.pos += 1;
        }
        if self.pos == start {
            return Err(ParseError::expected_atom(self.pos));
        }
        Ok(str::from_utf8(&self.buf[start..self.pos]).unwrap()) // bytes are all ASCII
    }

    pub fn string(&mut self) -> Result<Cow<'a, str>, ParseError> {
        if let Some(c) = self.peek()
            && c == b' '
        {
            self.skip_sp();
        }

        // --------------------- 3 cases ---------------------
        // A: Is a quoted string
        if self.eat(b'"').is_ok() {
            let start = self.pos;
            while self
                .peek()
                .ok_or_else(|| ParseError::out_of_bytes(self.pos))?
                != b'"'
            {
                self.pos += 1
            }

            let end = self.pos;
            self.eat(b'"')?;

            return Ok(Cow::Borrowed(
                str::from_utf8(&self.buf[start..end]).unwrap(),
            ));
        }

        // B: Is length delimted
        if self.eat(b'{').is_ok() {
            let size = self.number()?;
            if let next = self.peek().ok_or(ParseError::out_of_bytes(self.pos))?
                && next == b'+'
            {
                self.pos += 1;
            }

            self.eat(b'}')?;
            self.eat(b'\r')?;
            self.eat(b'\n')?;

            let start = self.pos;
            self.pos += size as usize;
            return if self.buf.get(self.pos - 1).is_some() {
                Ok(Cow::Borrowed(str::from_utf8(&self.buf[start..self.pos])?))
            } else {
                Err(ParseError::out_of_bytes(start))
            };
        }

        // C: Try as atom
        self.atom().map(Cow::Borrowed)
    }

    pub fn number(&mut self) -> Result<u64, ParseError> {
        if let Some(c) = self.peek()
            && c == b' '
        {
            self.skip_sp();
        }

        if !(self
            .peek()
            .ok_or_else(|| ParseError::out_of_bytes(self.pos))?
            .is_ascii_digit())
        {
            return Err(ParseError::expected_number(self.pos));
        }

        let mut n = 0u64;
        while let Some(c) = self.peek()
            && c.is_ascii_digit()
        {
            n *= 10;
            n += (c - b'0') as u64;
            self.pos += 1;
        }

        Ok(n)
    }

    pub fn paren_list<T>(
        &mut self,
        mut f: impl FnMut(&mut Self) -> Result<T, ParseError>,
    ) -> Result<Vec<T>, ParseError> {
        if let Some(c) = self.peek()
            && c == b' '
        {
            self.skip_sp();
        }

        // Support a single non parenthesized item
        if let next = self
            .peek()
            .ok_or_else(|| ParseError::out_of_bytes(self.pos))?
            && is_atom_char(next)
        {
            return Ok(vec![f(self)?]);
        };

        self.eat(b'(')?;

        let mut items = vec![];
        items.push(f(self)?);

        while self
            .peek()
            .ok_or_else(|| ParseError::out_of_bytes(self.pos))?
            != b')'
        {
            while self
                .peek()
                .ok_or_else(|| ParseError::out_of_bytes(self.pos))?
                != b' '
            {
                self.pos += 1;
            }

            items.push(f(self)?);
        }

        self.eat(b')')?;
        Ok(items)
    }

    pub fn raw(&mut self, amount: usize) -> Vec<u8> {
        let slice = self.buf[self.pos..self.pos + amount].to_vec();
        self.pos += amount;
        slice
    }

    // fetch-att is weird because it needs the ]
    pub fn fetch_att(&mut self) -> Result<&'a str, ParseError> {
        if let Some(c) = self.peek()
            && c == b' '
        {
            self.skip_sp();
        }

        let start = self.pos;

        while self.peek().is_some_and(|b| is_atom_char(b) && b != b'[') {
            self.pos += 1;
        }

        if self.peek() == Some(b'[') {
            self.pos += 1;
            let mut paren_depth = 0u32;
            loop {
                match self
                    .peek()
                    .ok_or_else(|| ParseError::out_of_bytes(self.pos))?
                {
                    b']' if paren_depth == 0 => {
                        self.pos += 1;
                        break;
                    }
                    b'(' => {
                        paren_depth += 1;
                        self.pos += 1;
                    }
                    b')' => {
                        paren_depth = paren_depth
                            .checked_sub(1)
                            .ok_or_else(|| ParseError::unexpected_char(self.pos))?;
                        self.pos += 1;
                    }
                    _ => self.pos += 1,
                }
            }
        }

        if self.peek() == Some(b'<') {
            self.pos += 1;
            loop {
                match self
                    .peek()
                    .ok_or_else(|| ParseError::out_of_bytes(self.pos))?
                {
                    b'>' => {
                        self.pos += 1;
                        break;
                    }
                    _ => self.pos += 1,
                }
            }
        }

        if self.pos == start {
            return Err(ParseError::expected_atom(self.pos));
        }

        Ok(str::from_utf8(&self.buf[start..self.pos]).unwrap()) // bytes are all ASCII
    }
}

fn is_atom_char(b: u8) -> bool {
    matches!(b, 0x21..=0x7E) && !matches!(b, b'(' | b')' | b'{' | b'%' | b'*' | b'"' | b'\\' | b']')
}
