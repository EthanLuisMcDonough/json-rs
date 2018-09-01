use super::{CheckedStack, IntoJson, JsonValue, ObjArrItem, ObjArrStack, PendingItem};

#[derive(PartialEq, Debug)]
pub struct ObjectStack {
    inner: Vec<ObjArrItem>,
}

impl ObjectStack {
    pub fn new() -> Self {
        Self { inner: vec![] }
    }
}

impl From<ObjectStack> for PendingItem {
    fn from(stack: ObjectStack) -> PendingItem {
        PendingItem::ObjArr(Box::new(stack))
    }
}

impl IntoJson for ObjectStack {
    fn into_json(mut self: Box<Self>) -> Result<JsonValue, ()> {
        use self::ObjArrItem::*;
        use std::collections::HashMap;

        match self.peek() {
            Some(Comma) => Err(()),
            _ => {
                let mut dict = HashMap::new();

                while self.inner.len() > 0 {
                    let mut s = shift_multi(&mut self.inner, 4);
                    match s.as_slice() {
                        [Key(_), Colon, Item(_), Comma] | [Key(_), Colon, Item(_)] => {
                            if let (Key(k), _, Item(v)) = (s.remove(0), s.remove(0), s.remove(0)) {
                                dict.insert(k, v);
                            }
                        }
                        _ => return Err(()),
                    }
                }

                Ok(JsonValue::Object(dict))
            }
        }
    }
}

impl CheckedStack<ObjArrItem, ()> for ObjectStack {
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
                Key(s) | Item(JsonValue::Text(s)) => Ok(self.inner.push(Key(s))),
                _ => Err(()),
            },
            Some(Item(_)) => match item {
                Comma => Ok(self.inner.push(item)),
                _ => Err(()),
            },
            Some(Colon) => match item {
                Item(_) => Ok(self.inner.push(item)),
                _ => Err(()),
            },
            Some(Key(_)) => match item {
                Colon => Ok(self.inner.push(item)),
                _ => Err(()),
            },
        }
    }

    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl ObjArrStack for ObjectStack {
    fn get_delimiter(&self, c: char) -> Option<ObjArrItem> {
        Some(match c {
            ',' => ObjArrItem::Comma,
            ':' => ObjArrItem::Colon,
            _ => return None,
        })
    }

    fn is_end_char(&self, c: char) -> bool {
        c == '}'
    }

    fn next_must_be_key(&self) -> bool {
        use self::ObjArrItem::*;

        match self.peek() {
            Some(Comma) | None => true,
            _ => false,
        }
    }
}

fn shift_multi<T>(v: &mut Vec<T>, count: usize) -> Vec<T> {
    let mut ret = vec![];
    while v.len() > 0 && ret.len() < count {
        ret.push(v.remove(0));
    }
    ret
}

#[cfg(test)]
mod tests {
    struct ShiftTest<T> {
        original: Vec<T>,
        remove_count: usize,
        changed: Vec<T>,
        spliced: Vec<T>,
    }

    impl<T> ShiftTest<T>
    where
        T: ::std::fmt::Debug + PartialEq,
    {
        fn run_test(mut self) {
            use super::shift_multi;

            let new = shift_multi(&mut self.original, self.remove_count);
            assert_eq!(self.original, self.changed);
            assert_eq!(new, self.spliced);
        }
    }

    #[test]
    fn shift_multi() {
        let tests = vec![
            ShiftTest {
                original: vec![1, 2, 3, 4, 5, 6, 7],
                remove_count: 4,
                changed: vec![5, 6, 7],
                spliced: vec![1, 2, 3, 4],
            },
            ShiftTest {
                original: vec![10, 6, 9, 20],
                remove_count: 10,
                changed: vec![],
                spliced: vec![10, 6, 9, 20],
            },
            ShiftTest {
                original: vec![],
                remove_count: 3,
                changed: vec![],
                spliced: vec![],
            },
        ];

        for test in tests.into_iter() {
            test.run_test();
        }
    }

    #[test]
    fn object_push() {
        use super::{
            CheckedStack, JsonValue,
            ObjArrItem::{self, *},
            ObjectStack,
        };

        let tests: Vec<(ObjectStack, ObjArrItem, Result<(), ()>)> = vec![
            (
                ObjectStack { inner: vec![] },
                Item(JsonValue::Text("aaaa".to_string())),
                Ok(()),
            ),
            (
                ObjectStack {
                    inner: vec![Key("k".to_string())],
                },
                Colon,
                Ok(()),
            ),
            (
                ObjectStack {
                    inner: vec![Key("vvv#@".to_string()), Colon],
                },
                Item(JsonValue::Boolean(true)),
                Ok(()),
            ),
            (
                ObjectStack { inner: vec![] },
                Item(JsonValue::Boolean(true)),
                Err(()),
            ),
            (
                ObjectStack {
                    inner: vec![Key("thing".to_string())],
                },
                Item(JsonValue::Null),
                Err(()),
            ),
            (
                ObjectStack {
                    inner: vec![
                        Key("1_q_2".to_string()),
                        Colon,
                        Item(JsonValue::Text("vskjjlds".to_string())),
                    ],
                },
                Comma,
                Ok(()),
            ),
            (
                ObjectStack {
                    inner: vec![
                        Key("1_q_2".to_string()),
                        Colon,
                        Item(JsonValue::Text("vskjjlds".to_string())),
                        Comma,
                    ],
                },
                Key("eeeakse".to_string()),
                Ok(()),
            ),
            (
                ObjectStack {
                    inner: vec![
                        Key("thing".to_string()),
                        Colon,
                        Item(JsonValue::Null),
                        Comma,
                    ],
                },
                Comma,
                Err(()),
            ),
            (
                ObjectStack {
                    inner: vec![Key("hhhhhh".to_string()), Colon, Item(JsonValue::Null)],
                },
                Colon,
                Err(()),
            ),
        ];

        for (mut stack, value, result) in tests.into_iter() {
            assert_eq!(stack.push(value), result);
        }
    }

    #[test]
    fn object_into_json() {
        use super::{IntoJson, JsonValue, ObjArrItem::*, ObjectStack};
        use std::collections::HashMap;

        let tests = vec![
            (
                vec![
                    Key("j23O@".to_string()),
                    Colon,
                    Item(JsonValue::Boolean(true)),
                    Comma,
                    Key("ffff".to_string()),
                    Colon,
                    Item(JsonValue::Text("aaaa".to_string())),
                    Comma,
                    Key("qqqqq".to_string()),
                    Colon,
                    Item(JsonValue::Object({
                        let mut map = HashMap::new();
                        map.insert("d29".to_string(), JsonValue::Number(10f64));
                        map.insert("0000e".to_string(), JsonValue::Null);;
                        map
                    })),
                    Comma,
                    Key("arr".to_string()),
                    Colon,
                    Item(JsonValue::Array(vec![
                        JsonValue::Number(10f64),
                        JsonValue::Number(20f64),
                        JsonValue::Number(0.2332),
                    ])),
                ],
                Ok(JsonValue::Object({
                    let mut map = HashMap::new();
                    map.insert("j23O@".to_string(), JsonValue::Boolean(true));
                    map.insert("ffff".to_string(), JsonValue::Text("aaaa".to_string()));
                    map.insert(
                        "qqqqq".to_string(),
                        JsonValue::Object({
                            let mut map = HashMap::new();
                            map.insert("d29".to_string(), JsonValue::Number(10f64));
                            map.insert("0000e".to_string(), JsonValue::Null);;
                            map
                        }),
                    );
                    map.insert(
                        "arr".to_string(),
                        JsonValue::Array(vec![
                            JsonValue::Number(10f64),
                            JsonValue::Number(20f64),
                            JsonValue::Number(0.2332),
                        ]),
                    );
                    map
                })),
            ),
            (vec![], Ok(JsonValue::Object(HashMap::new()))),
            (vec![Key("aaaa".to_string())], Err(())),
            (vec![Key("aaaa".to_string()), Colon], Err(())),
            (
                vec![
                    Key("ppppp".to_string()),
                    Colon,
                    Item(JsonValue::Boolean(false)),
                    Comma,
                ],
                Err(()),
            ),
        ];

        for (inner, result) in tests.into_iter() {
            let mut stack = ObjectStack { inner };
            assert_eq!(Box::new(stack).into_json(), result);
        }
    }
}
