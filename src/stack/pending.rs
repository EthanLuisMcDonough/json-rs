use super::{IntoJson, JsonValue, SimpleStack};

const BOOL_STRS: &'static [&'static str] = &["true", "false"];
const NULL_STRS: &[&'static str] = &["null"];

pub trait PendingStack<C>: IntoJson {
    fn push(&mut self, c: C) -> Result<bool, C>;
}

#[derive(PartialEq, Debug)]
struct StringMatcherStack {
    inner_string: String,
    matchers: &'static [&'static str],
}

impl StringMatcherStack {
    fn new(matchers: &'static [&'static str]) -> Self {
        Self {
            matchers,
            inner_string: String::new(),
        }
    }

    fn push(&mut self, c: char) -> Result<bool, char> {
        let concatenated = format!("{}{}", self.inner_string, c);
        if self.matchers.iter().any(|s| s.starts_with(&concatenated)) {
            self.inner_string.push(c);
            Ok(self.matchers.contains(&&*self.inner_string))
        } else {
            Err(c)
        }
    }

    fn into_string(self) -> Result<String, ()> {
        let matchers = self.matchers;
        Some(self.inner_string)
            .filter(|s| matchers.contains(&s.as_str()))
            .ok_or(())
    }
}

#[derive(PartialEq, Debug)]
pub struct BoolStack {
    matcher_stack: StringMatcherStack,
}

impl SimpleStack for BoolStack {}

impl BoolStack {
    pub fn new() -> Self {
        Self {
            matcher_stack: StringMatcherStack::new(BOOL_STRS),
        }
    }

    pub fn init_true() -> Self {
        let mut s = Self::new();
        s.matcher_stack.inner_string.push('t');
        s
    }

    pub fn init_false() -> Self {
        let mut s = Self::new();
        s.matcher_stack.inner_string.push('f');
        s
    }
}

impl IntoJson for BoolStack {
    fn into_json(self: Box<Self>) -> Result<JsonValue, ()> {
        self.matcher_stack
            .into_string()
            .and_then(|s| s.parse().map(JsonValue::Boolean).map_err(::std::mem::drop))
    }
}

impl PendingStack<char> for BoolStack {
    fn push(&mut self, c: char) -> Result<bool, char> {
        self.matcher_stack.push(c)
    }
}

#[derive(PartialEq, Debug)]
pub struct NullStack {
    matcher_stack: StringMatcherStack,
}

impl SimpleStack for NullStack {}

impl NullStack {
    pub fn new() -> Self {
        Self {
            matcher_stack: StringMatcherStack::new(NULL_STRS),
        }
    }

    pub fn init_n() -> Self {
        let mut stack = Self::new();
        stack.matcher_stack.inner_string.push('n');
        stack
    }
}

impl IntoJson for NullStack {
    fn into_json(self: Box<Self>) -> Result<JsonValue, ()> {
        self.matcher_stack.into_string().map(|_| JsonValue::Null)
    }
}

impl PendingStack<char> for NullStack {
    fn push(&mut self, c: char) -> Result<bool, char> {
        self.matcher_stack.push(c)
    }
}

#[derive(PartialEq, Debug)]
enum EscapeType {
    SimpleChar(char),
    Unicode(String),
}

#[derive(PartialEq, Debug)]
struct EscapeSequence {
    inner: Option<EscapeType>,
}

impl EscapeSequence {
    fn new() -> Self {
        Self { inner: None }
    }

    fn push(&mut self, c: char) -> Result<bool, char> {
        Ok(if self.inner.is_none() {
            self.inner = Some(if c == 'u' {
                EscapeType::Unicode(String::new())
            } else {
                EscapeType::SimpleChar(match c {
                    'n' => '\n',
                    't' => '\t',
                    'r' => '\r',
                    'b' => '\x08',
                    'f' => '\x0C',
                    '\\' | '"' => c,
                    _ => return Err(c),
                })
            });
            c != 'u'
        } else {
            match &mut self.inner {
                Some(EscapeType::Unicode(ref mut s)) if c.is_digit(16) && s.len() < 4 => {
                    s.push(c);
                    s.len() == 4
                }
                _ => return Err(c),
            }
        })
    }

    fn into_char(self) -> Result<char, ()> {
        use std::{char::from_u32, mem::drop};
        match self.inner {
            Some(EscapeType::Unicode(ref s)) if s.len() == 4 => u32::from_str_radix(s, 16)
                .map_err(drop)
                .and_then(|code| from_u32(code).ok_or(())),
            Some(EscapeType::SimpleChar(c)) => Ok(c),
            _ => Err(()),
        }
    }
}

#[derive(PartialEq, Debug)]
pub struct TextStack {
    inner: String,
    completed: bool,
    escape: Option<EscapeSequence>,
}

impl SimpleStack for TextStack {}

impl TextStack {
    pub fn new() -> Self {
        Self {
            inner: String::new(),
            completed: false,
            escape: None,
        }
    }
}

impl IntoJson for TextStack {
    fn into_json(self: Box<Self>) -> Result<JsonValue, ()> {
        let completed = self.completed;
        Some(JsonValue::Text(self.inner))
            .filter(|_| completed)
            .ok_or(())
    }
}

impl PendingStack<char> for TextStack {
    fn push(&mut self, c: char) -> Result<bool, char> {
        if let Some(mut seq) = self.escape.take() {
            if seq.push(c)? {
                self.inner.push(seq.into_char().map_err(|()| c)?);
            } else {
                self.escape = Some(seq);
            }
            Ok(false)
        } else {
            match c {
                '"' if !self.completed => {
                    self.completed = true;
                    Ok(true)
                }
                '\\' if !self.completed => {
                    self.escape = Some(EscapeSequence::new());
                    Ok(false)
                }
                _ if !self.completed && !c.is_control() => {
                    self.inner.push(c);
                    Ok(false)
                }
                _ => Err(c),
            }
        }
    }
}

#[derive(PartialEq, Debug)]
enum NumPosition {
    Whole,
    IntoDecimal,
    Decimal,
    Exponent,
}

#[derive(PartialEq, Debug)]
pub struct NumberStack {
    position: NumPosition,
    positive: bool,
    whole: String,
    decimal: String,
    exponent: String,
}

impl NumberStack {
    pub fn new() -> Self {
        Self {
            position: NumPosition::Whole,
            positive: true,
            whole: String::new(),
            decimal: String::new(),
            exponent: String::new(),
        }
    }

    pub fn stringify(&self) -> Result<String, ()> {
        use self::NumPosition::*;

        if (self.position == Whole && self.whole.is_empty())
            || (self.position == Decimal && self.decimal.is_empty())
            || (self.position == Exponent && self.exponent.is_empty())
        {
            Err(())
        } else {
            let mut stringified = String::new();

            if !self.positive {
                stringified.push('-');
            }

            stringified.push_str(&self.whole);

            if !self.decimal.is_empty() {
                stringified.push('.');
                stringified.push_str(&self.decimal);
            }

            if !self.exponent.is_empty() {
                stringified.push('e');
                stringified.push_str(&self.exponent);
            }

            Ok(stringified)
        }
    }

    pub fn can_push(&self, c: char) -> bool {
        use self::NumPosition::*;

        match self.position {
            Whole => {
                (c == '-' && self.whole.is_empty() && self.positive)
                    || (c == '0' && self.whole.is_empty())
                    || (c.is_digit(10)
                        && ((c != '0' && self.whole.is_empty()) || self.whole.len() > 0))
                    || (self.whole.len() > 0 && c.eq_ignore_ascii_case(&'e'))
                    || (c == '.' && self.whole.len() > 0)
            }
            Exponent => c.is_digit(10) || ((c == '+' || c == '-') && self.exponent.is_empty()),
            Decimal => c.is_digit(10) || (self.decimal.len() > 0 && c.eq_ignore_ascii_case(&'e')),
            IntoDecimal => c == '.',
        }
    }
}

impl IntoJson for NumberStack {
    fn into_json(self: Box<Self>) -> Result<JsonValue, ()> {
        self.stringify()
            .and_then(|s| s.parse().map(JsonValue::Number).map_err(::std::mem::drop))
    }
}

impl PendingStack<char> for NumberStack {
    fn push(&mut self, c: char) -> Result<bool, char> {
        use self::NumPosition::*;

        match self.position {
            Whole if c == '-' && self.whole.is_empty() && self.positive => self.positive = false,
            Whole if c == '0' && self.whole.is_empty() => {
                self.whole.push(c);
                self.position = IntoDecimal
            }
            Whole
                if c.is_digit(10)
                    && ((self.whole.is_empty() && c != '0') || self.whole.len() > 0) =>
            {
                self.whole.push(c)
            }
            IntoDecimal if c == '.' => self.position = Decimal,
            Whole if c == '.' && self.whole.len() > 0 => self.position = Decimal,
            Decimal if c.is_digit(10) => self.decimal.push(c),
            Decimal if self.decimal.len() > 0 && c.eq_ignore_ascii_case(&'e') => {
                self.position = Exponent
            }
            Whole if self.whole.len() > 0 && c.eq_ignore_ascii_case(&'e') => {
                self.position = Exponent
            }
            Exponent if c.is_digit(10) || ((c == '+' || c == '-') && self.exponent.is_empty()) => {
                self.exponent.push(c)
            }
            _ => return Err(c),
        }

        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::PendingStack;

    #[test]
    fn bool_stack() {
        use super::BoolStack;

        let tests = vec![
            ("tru", 'e', Ok(true)),
            ("", 't', Ok(false)),
            ("fa", 's', Err('s')),
            ("f", 'a', Ok(false)),
            ("", '#', Err('#')),
            ("fals", '3', Err('3')),
            ("false", ' ', Err(' ')),
            ("true", 'e', Err('e')),
            ("fals", 'e', Ok(true)),
        ];

        for (existing, insert, result) in tests.into_iter() {
            let mut stack = BoolStack::new();
            stack.matcher_stack.inner_string = existing.to_string();
            assert_eq!(stack.push(insert), result);
        }
    }

    #[test]
    fn null_stack() {
        use super::NullStack;

        let tests = vec![
            ("nul", 'l', Ok(true)),
            ("nu", 'l', Ok(false)),
            ("n", 'u', Ok(false)),
            ("", 'n', Ok(false)),
            ("", 'u', Err('u')),
            ("null", 'l', Err('l')),
            ("nu", 'u', Err('u')),
        ];

        for (existing, insert, result) in tests.into_iter() {
            let mut stack = NullStack::new();
            stack.matcher_stack.inner_string = existing.to_string();
            assert_eq!(stack.push(insert), result);
        }
    }

    #[test]
    fn escape_stack() {
        use super::EscapeSequence;
        let tests = vec![
            ("f", Some(Ok('\x0C')), None),
            ("na", None, Some(Err('a'))),
            ("", Some(Err(())), None),
            ("u0000", Some(Ok('\x00')), None),
            ("\\", Some(Ok('\\')), None),
            ("\"", Some(Ok('"')), None),
            ("k", None, Some(Err('k'))),
            ("u00cD", Some(Ok('Í')), None),
            ("u023", Some(Err(())), None),
            ("u02N2", None, Some(Err('N'))),
        ];

        for (insert, success, error) in tests.into_iter() {
            let mut seq = EscapeSequence::new();
            insert
                .chars()
                .map(|c| seq.push(c))
                .skip_while(|r| r.is_ok())
                .next()
                .map(|e| {
                    assert_eq!(Some(e), error);
                })
                .unwrap_or_else(|| {
                    assert_eq!(Some(seq.into_char()), success);
                });
        }
    }

    #[test]
    fn text_stack() {
        use super::TextStack;
        let success_tests = vec!["athlekns", "jtld \\\" \\u2023 ", "sdlk {} "];

        for st in success_tests.into_iter() {
            let mut seq = TextStack::new();
            for c in st.chars() {
                assert_eq!(seq.push(c), Ok(false));
            }
            assert_eq!(seq.push('"'), Ok(true));
        }

        let bad_tests = vec![
            ("thign \\u2c0 mm", Err(' ')),
            ("\\ dd", Err(' ')),
            ("mmm \" thing", Err(' ')),
            ("control \t ch", Err('\t')),
        ];
        for (st, error) in bad_tests.into_iter() {
            let mut seq = TextStack::new();
            assert_eq!(
                st.chars()
                    .map(|c| seq.push(c))
                    .skip_while(|r| r.is_ok())
                    .next()
                    .filter(|&e| e == error),
                Some(error)
            );
        }
    }

    #[test]
    fn number_stack_push() {
        use super::NumberStack;

        let tests = vec![
            ("102.", 'e', Err('e')),
            ("20", '.', Ok(false)),
            ("0", '9', Err('9')),
            ("-3.", '2', Ok(false)),
            ("-3.230e+", '9', Ok(false)),
            ("", '-', Ok(false)),
            ("2302e", '.', Err('.')),
            ("-", '-', Err('-')),
            ("-0", '0', Err('0')),
            ("0", '.', Ok(false)),
            ("4.23e", '-', Ok(false)),
            ("2302E0", '.', Err('.')),
            ("2.33e0", '0', Ok(false)),
            ("4e2", '.', Err('.')),
        ];

        for (st, push_in, res) in tests.into_iter() {
            let mut stack = NumberStack::new();
            for c in st.chars() {
                stack.push(c).unwrap();
            }
            assert_eq!(stack.push(push_in), res);
        }
    }
}
