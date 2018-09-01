use super::JsonValue;

pub mod array;
pub mod object;
pub mod pending;

use self::pending::PendingStack;
use std::fmt::Debug;

pub trait CheckedStack<T, E> {
    fn peek(&self) -> Option<&T>;
    fn peek_mut(&mut self) -> Option<&mut T>;
    fn pop(&mut self) -> Option<T>;
    fn push(&mut self, e: T) -> Result<(), E>;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

pub trait SimpleStack: PendingStack<char> + IntoJson + Debug {}

pub trait IntoJson {
    fn into_json(self: Box<Self>) -> Result<JsonValue, ()>;
}

pub trait ObjArrStack: IntoJson + Debug + CheckedStack<ObjArrItem, ()> {
    fn get_delimiter(&self, c: char) -> Option<ObjArrItem>;
    fn is_end_char(&self, c: char) -> bool;
    fn next_must_be_key(&self) -> bool;
}

#[derive(Debug)]
pub enum PendingItem {
    ObjArr(Box<ObjArrStack>),
    Simple(Box<SimpleStack>),
    Number(pending::NumberStack),
    FinalizedJsonValue(JsonValue),
}

#[derive(PartialEq, Debug)]
pub enum ObjArrItem {
    Colon,
    Comma,
    Item(JsonValue),
    Key(String),
}

impl From<JsonValue> for ObjArrItem {
    fn from(item: JsonValue) -> Self {
        ObjArrItem::Item(item)
    }
}

impl From<JsonValue> for PendingItem {
    fn from(item: JsonValue) -> PendingItem {
        PendingItem::FinalizedJsonValue(item)
    }
}

impl From<Box<SimpleStack>> for PendingItem {
    fn from(stack: Box<SimpleStack>) -> Self {
        PendingItem::Simple(stack)
    }
}

impl From<Box<ObjArrStack>> for PendingItem {
    fn from(stack: Box<ObjArrStack>) -> Self {
        PendingItem::ObjArr(stack)
    }
}

#[derive(Debug)]
pub struct StackCounter {
    inner: String,
    in_string: bool,
    escape: bool,
}

impl StackCounter {
    pub fn new() -> Self {
        Self {
            inner: String::new(),
            in_string: false,
            escape: false,
        }
    }

    pub fn push(&mut self, c: char) -> Result<(), ()> {
        let res = match c {
            '{' | '[' if !self.in_string => Ok(self.inner.push(c)),
            '}' | ']' if !self.in_string => self
                .inner
                .pop()
                .filter(|last| match last {
                    '[' => c == ']',
                    '{' => c == '}',
                    _ => false,
                })
                .map(::std::mem::drop)
                .ok_or(()),
            _ if self.escape => Ok(self.escape = false),
            '\\' if self.in_string => Ok(self.escape = true),
            '"' => Ok(self.in_string = !self.in_string),
            _ => Ok(()),
        };
        res
    }

    pub fn level(&self) -> usize {
        self.inner.len()
    }

    pub fn in_string(&self) -> bool {
        self.in_string
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn stack_counter() {
        use super::StackCounter;

        let tests = vec![
            (r#" [ [ " thing \" ] "#, 2, true),
            (r#""#, 0, false),
            (r#" { "thing": 20, "other": [], "fi": [[], [ "#, 3, false),
            (r#"[{"e": []}]"#, 0, false),
            (r#"[ "aaaa", "bbbb", " \" "#, 1, true),
            (r#"[ "aaaa", "bbbb", " \\" "#, 1, false),
            (r#"{ "aa"#, 1, true),
        ];

        for (st, level, in_string) in tests.into_iter() {
            let mut counter = StackCounter::new();
            for c in st.chars() {
                counter.push(c).unwrap();
            }
            assert_eq!(counter.in_string(), in_string);
            assert_eq!(counter.level(), level);
        }
    }

    #[test]
    fn stack_counter_err_push() {
        use super::StackCounter;

        let tests = vec![("[", '}'), ("[[]", '}'), ("{{", ']'), (r#" [ "\"{" "#, '}')];

        for (st, ch) in tests.into_iter() {
            let mut counter = StackCounter::new();
            for c in st.chars() {
                counter.push(c).unwrap();
            }
            assert!(counter.push(ch).is_err());
        }
    }
}
