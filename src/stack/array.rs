use super::{CheckedStack, IntoJson, JsonValue, ObjArrItem, ObjArrStack, PendingItem};

#[derive(PartialEq, Debug)]
pub struct ArrayStack {
    inner: Vec<ObjArrItem>,
}

impl From<ArrayStack> for PendingItem {
    fn from(stack: ArrayStack) -> PendingItem {
        PendingItem::ObjArr(Box::new(stack))
    }
}

impl ArrayStack {
    pub fn new() -> Self {
        Self { inner: vec![] }
    }
}

impl IntoJson for ArrayStack {
    fn into_json(self: Box<Self>) -> Result<JsonValue, ()> {
        match self.peek() {
            Some(ObjArrItem::Comma) => Err(()),
            _ => Ok(JsonValue::Array(
                self.inner
                    .into_iter()
                    .flat_map(|t| match t {
                        ObjArrItem::Item(i) => Some(i),
                        _ => None,
                    })
                    .collect(),
            )),
        }
    }
}

impl CheckedStack<ObjArrItem, ()> for ArrayStack {
    fn peek(&self) -> Option<&ObjArrItem> {
        self.inner.last()
    }

    fn peek_mut(&mut self) -> Option<&mut ObjArrItem> {
        self.inner.last_mut()
    }

    fn pop(&mut self) -> Option<ObjArrItem> {
        self.inner.pop()
    }

    fn push(&mut self, item: ObjArrItem) -> Result<(), ()> {
        use self::ObjArrItem::*;

        match self.peek() {
            Some(Comma) | None => match item {
                Item(_) => Ok(self.inner.push(item)),
                _ => Err(()),
            },
            Some(Item(_)) => match item {
                Comma => Ok(self.inner.push(item)),
                _ => Err(()),
            },
            _ => Err(()),
        }
    }

    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl ObjArrStack for ArrayStack {
    fn get_delimiter(&self, c: char) -> Option<ObjArrItem> {
        Some(ObjArrItem::Comma).filter(|_| c == ',')
    }

    fn is_end_char(&self, c: char) -> bool {
        c == ']'
    }

    fn next_must_be_key(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::{ArrayStack, JsonValue, ObjArrItem::*};

    #[test]
    fn array_into_json() {
        use super::IntoJson;
        use std::collections::HashMap;

        let tests = vec![
            (
                vec![Item(JsonValue::Null)],
                Ok(JsonValue::Array(vec![JsonValue::Null])),
            ),
            (vec![], Ok(JsonValue::Array(vec![]))),
            (vec![Comma], Err(())),
            (
                vec![Item(JsonValue::Object(HashMap::new())), Comma],
                Err(()),
            ),
            (
                vec![
                    Item(JsonValue::Boolean(true)),
                    Comma,
                    Item(JsonValue::Array(vec![])),
                ],
                Ok(JsonValue::Array(vec![
                    JsonValue::Boolean(true),
                    JsonValue::Array(vec![]),
                ])),
            ),
        ];

        for (v, result) in tests.into_iter() {
            assert_eq!(Box::new(ArrayStack { inner: v }).into_json(), result);
        }
    }

    #[test]
    fn array_push() {
        use super::CheckedStack;

        let tests = vec![
            (vec![Item(JsonValue::Null)], JsonValue::Null.into(), Err(())),
            (vec![], Comma, Err(())),
            (vec![JsonValue::Number(10.0).into()], Comma, Ok(())),
            (vec![JsonValue::Boolean(false).into()], Colon, Err(())),
        ];

        for (v, push_in, result) in tests.into_iter() {
            let mut stack = ArrayStack { inner: v };
            assert_eq!(stack.push(push_in), result);
        }
    }
}
