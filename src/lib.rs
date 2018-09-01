use std::collections::HashMap;

mod stack;
use self::stack::{IntoJson, PendingItem};

/// A JSON value.
#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    /// A JSON string value.
    Text(String),
    /// A numeric JSON value.
    Number(f64),
    /// A JSON boolean value.
    Boolean(bool),
    /// The JSON null value.
    Null,
    /// A JSON array.
    Array(Vec<JsonValue>),
    /// A JSON object.
    Object(HashMap<String, JsonValue>),
}

impl JsonValue {
    /// Gets a reference to the JSON value at a specific key.
    /// ```
    /// extern crate json_rs;
    /// use json_rs::JsonValue;
    /// use std::collections::HashMap;
    ///
    /// fn main() {
    ///     let json = JsonValue::Object({
    ///         let mut map = HashMap::new();
    ///         map.insert("key".into(), JsonValue::Boolean(true));
    ///         map
    ///     });
    ///     assert_eq!(json.get("key"), Some(&JsonValue::Boolean(true)));
    ///     assert_eq!(json.get("nonexistent"), None);
    ///
    ///     let simple_boolean = JsonValue::Boolean(true);
    ///     assert_eq!(simple_boolean.get("key_of_some_sort"), None);
    /// }
    /// ```
    pub fn get(&self, key: &str) -> Option<&Self> {
        match self {
            JsonValue::Object(map) => map.get(key),
            JsonValue::Array(array) => key.parse().ok().and_then(|i: usize| array.get(i)),
            _ => None,
        }
    }

    /// Gets a mutable reference to the JSON value at a specific key.
    /// ```
    /// extern crate json_rs;
    /// use std::collections::HashMap;
    /// use json_rs::JsonValue;
    ///
    /// fn main() {
    ///     let mut json = JsonValue::Object({
    ///         let mut map = HashMap::new();
    ///         map.insert("number".into(), JsonValue::Number(10f64));
    ///         map
    ///     });
    ///     assert_eq!(json.get("number"), Some(&JsonValue::Number(10f64)));
    ///
    ///     if let Some(JsonValue::Number(value)) = json.get_mut("number") {
    ///         *value += 5.0;
    ///     }
    ///     assert_eq!(json.get("number"), Some(&JsonValue::Number(15f64)));
    /// }
    /// ```
    pub fn get_mut(&mut self, key: &str) -> Option<&mut Self> {
        match self {
            JsonValue::Object(map) => map.get_mut(key),
            JsonValue::Array(array) => key.parse().ok().and_then(move |i: usize| array.get_mut(i)),
            _ => None,
        }
    }

    /// Gets a reference to the JSON value at a specific index.
    /// ```
    /// extern crate json_rs;
    /// use json_rs::JsonValue;
    ///
    /// fn main() {
    ///     let json = JsonValue::Array(vec![
    ///         JsonValue::Null,
    ///         JsonValue::Text("aaaskjw".into()),
    ///         JsonValue::Number(10.0),
    ///         JsonValue::Boolean(true),
    ///     ]);
    ///     assert_eq!(json.get_ind(1), Some(&JsonValue::Text("aaaskjw".into())));
    ///     assert_eq!(json.get_ind(3), Some(&JsonValue::Boolean(true)));
    ///     assert_eq!(json.get_ind(10), None);
    /// }
    /// ```
    pub fn get_ind(&self, ind: usize) -> Option<&Self> {
        match self {
            JsonValue::Object(map) => map.get(&ind.to_string()),
            JsonValue::Array(array) => array.get(ind),
            _ => None,
        }
    }

    /// Gets a mutable reference to the JSON value at a specific index.
    /// ```
    /// extern crate json_rs;
    /// use json_rs::JsonValue;
    ///
    /// fn main() {
    ///     let mut json = JsonValue::Array(vec![JsonValue::Text("abcd".into())]);
    ///     assert_eq!(json.get_ind(0), Some(&JsonValue::Text("abcd".into())));
    ///
    ///     if let Some(JsonValue::Text(st)) = json.get_ind_mut(0) {
    ///         st.push_str("efg");
    ///     }
    ///     assert_eq!(json.get_ind(0), Some(&JsonValue::Text("abcdefg".into())));
    /// }
    /// ```
    pub fn get_ind_mut(&mut self, ind: usize) -> Option<&mut Self> {
        match self {
            JsonValue::Object(map) => map.get_mut(&ind.to_string()),
            JsonValue::Array(array) => array.get_mut(ind),
            _ => None,
        }
    }
}

fn unicode_escape(c: char) -> String {
    format!("\\u{:0>4}", format!("{:x}", c as u32))
}

fn escape_str(text: &str) -> String {
    use std::borrow::Cow;

    format!(
        "\"{}\"",
        text.chars()
            .map(|c| -> Cow<str> {
                match c {
                    '\n' => "\\n".into(),
                    '\t' => "\\t".into(),
                    '\r' => "\\r".into(),
                    '\x08' => "\\b".into(),
                    '\x0C' => "\\f".into(),
                    '\\' => "\\\\".into(),
                    '"' => "\\\"".into(),
                    '\x00'...'\x1F' => unicode_escape(c).into(),
                    _ => c.to_string().into(),
                }
            })
            .collect::<String>()
    )
}

impl ToString for JsonValue {
    /// Serializes a JsonValue
    fn to_string(&self) -> String {
        match self {
            JsonValue::Text(text) => escape_str(text),
            JsonValue::Null => "null".to_string(),
            JsonValue::Boolean(b) => b.to_string(),
            JsonValue::Number(n) => n.to_string(),
            JsonValue::Array(array) => format!(
                "[{}]",
                array
                    .iter()
                    .map(|json| json.to_string())
                    .collect::<Vec<String>>()
                    .join(",")
            ),
            JsonValue::Object(map) => format!(
                "{{{}}}",
                map.iter()
                    .map(|(key, val)| format!("{}:{}", escape_str(key), val.to_string()))
                    .collect::<Vec<String>>()
                    .join(",")
            ),
        }
    }
}

/// Describes all possible errors that could occur while parsing a JSON string
#[derive(Clone, Debug, PartialEq)]
pub enum JsonError {
    /// An unexpected character was found in the JSON
    UnexpectedToken {
        /// The invalid character
        character: char,
        /// The index where the char was found
        location: usize,
    },
    /// Unexpected end of input
    UnexpectedEOF,
}

/// Deserializes a JSON string.
/// ```
/// extern crate json_rs;
/// use json_rs::JsonValue;
/// use std::collections::HashMap;
///
/// fn main() {
///     let json = json_rs::json_parse(
///         r#"{
///             "key": 10,
///             "otherKey": "value",
///             "aaa": [ 1, 2, 3 ]
///         }"#,
///     );
///
///     assert_eq!(
///         json,
///         Ok(JsonValue::Object({
///             let mut map = HashMap::new();
///             map.insert("key".into(), JsonValue::Number(10.0));
///             map.insert("otherKey".into(), JsonValue::Text("value".into()));
///             map.insert(
///                 "aaa".into(),
///                 JsonValue::Array(vec![
///                     JsonValue::Number(1.0),
///                     JsonValue::Number(2.0),
///                     JsonValue::Number(3.0),
///                 ]),
///             );
///             map
///         }))
///     );
/// }
/// ```
pub fn json_parse(json_str: &str) -> Result<JsonValue, JsonError> {
    json_parse_internal(json_str, 0)
}

fn tok_err(c: char, loc: usize) -> JsonError {
    JsonError::UnexpectedToken {
        character: c,
        location: loc,
    }
}

fn json_parse_internal(json_str: &str, mut pos: usize) -> Result<JsonValue, JsonError> {
    use self::stack::{
        array::ArrayStack,
        object::ObjectStack,
        pending::{BoolStack, NullStack, NumberStack, PendingStack, TextStack},
        PendingItem::*,
        StackCounter,
    };
    let mut processing = None;
    let mut counter = StackCounter::new();
    let mut chars = json_str.chars().peekable();

    let mut error_ind = None;
    let mut content_str = String::new();
    let mut next_must_be_quote = false;

    while let Some(c) = chars.next() {
        counter.push(c).map_err(|()| tok_err(c, pos))?;

        let mut last = processing.take();
        match last {
            None => match c {
                '"' => processing = Some(Simple(Box::new(TextStack::new()))),
                '[' => processing = Some(ObjArr(Box::new(ArrayStack::new()))),
                '{' => processing = Some(ObjArr(Box::new(ObjectStack::new()))),
                't' => processing = Some(Simple(Box::new(BoolStack::init_true()))),
                'f' => processing = Some(Simple(Box::new(BoolStack::init_false()))),
                'n' => processing = Some(Simple(Box::new(NullStack::init_n()))),
                '-' | '0'...'9' => {
                    let mut stack = NumberStack::new();
                    stack.push(c).unwrap();
                    processing = Some(if chars.peek().filter(|c| stack.can_push(**c)).is_some() {
                        Number(stack)
                    } else {
                        FinalizedJsonValue(Box::new(stack).into_json().map_err(|_| {
                            chars
                                .peek()
                                .map(|c| tok_err(*c, pos + 1))
                                .unwrap_or(JsonError::UnexpectedEOF)
                        })?)
                    })
                }
                _ if c.is_whitespace() => (),
                _ => return Err(tok_err(c, pos)),
            },
            Some(Simple(mut stack)) => {
                processing = Some(if stack.push(c).map_err(|c| tok_err(c, pos))? {
                    FinalizedJsonValue(stack.into_json().unwrap())
                } else {
                    stack.into()
                })
            }
            Some(ObjArr(mut stack)) => if let Some(delimiter) = stack
                .get_delimiter(c)
                .filter(|_| counter.level() == 1 && !counter.in_string())
            {
                error_ind
                    .take()
                    .filter(|_| content_str.trim().len() > 0)
                    .ok_or(tok_err(c, pos))
                    .and_then(|ind| {
                        json_parse_internal(&content_str, ind).map_err(|e| {
                            if e == JsonError::UnexpectedEOF {
                                tok_err(c, pos)
                            } else {
                                e
                            }
                        })
                    })
                    .and_then(|json| {
                        stack
                            .push(json.into())
                            .and_then(|()| stack.push(delimiter))
                            .map(|_| {
                                processing = Some(stack.into());
                                content_str.clear();
                            })
                            .map_err(|_| tok_err(c, pos))
                    })?
            } else if stack.is_end_char(c) && counter.level() == 0 && !counter.in_string() {
                if let Some(ind) = error_ind.take().filter(|_| content_str.trim().len() > 0) {
                    stack
                        .push(
                            json_parse_internal(&content_str, ind)
                                .map_err(|e| {
                                    if e == JsonError::UnexpectedEOF {
                                        tok_err(c, pos)
                                    } else {
                                        e
                                    }
                                })?
                                .into(),
                        )
                        .map_err(|_| tok_err(c, pos))?;
                }
                processing = Some(PendingItem::FinalizedJsonValue(
                    stack.into_json().map_err(|_| tok_err(c, pos))?,
                ));
                content_str.clear();
            } else {
                if error_ind.is_none() {
                    error_ind = Some(pos);
                    next_must_be_quote = stack.next_must_be_key();
                }

                if next_must_be_quote && !c.is_whitespace() && c != '"' {
                    return Err(tok_err(c, pos));
                }

                content_str.push(c);
                next_must_be_quote = next_must_be_quote && c.is_whitespace();
                processing = Some(stack.into());
            },
            Some(Number(mut stack)) => {
                stack.push(c).map_err(|_| tok_err(c, pos))?;
                processing = Some(if chars.peek().filter(|c| stack.can_push(**c)).is_some() {
                    Number(stack)
                } else {
                    FinalizedJsonValue(Box::new(stack).into_json().map_err(|()| {
                        chars
                            .peek()
                            .map(|c| tok_err(*c, pos + 1))
                            .unwrap_or(JsonError::UnexpectedEOF)
                    })?)
                })
            }
            Some(FinalizedJsonValue(_)) if !c.is_whitespace() => {
                return Err(tok_err(c, pos));
            }
            Some(FinalizedJsonValue(_)) => processing = last,
        }
        pos += 1;
    }
    if let Some(FinalizedJsonValue(value)) = processing {
        Ok(value)
    } else {
        if let Some(ind) = error_ind.filter(|_| content_str.len() > 0) {
            // check for syntax errors in any remaining unparsed content_str
            json_parse_internal(&content_str, ind)?;
        }
        Err(JsonError::UnexpectedEOF)
    }
}

#[cfg(test)]
mod tests;
