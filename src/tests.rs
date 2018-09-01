use std::collections::HashMap;
use std::ffi::OsString;
use std::mem::drop;

#[test]
fn str_escape() {
    use super::escape_str;

    let tests = [
        ("dlksjd)\t", "dlksjd)\\t"),
        ("aaaa  \u{8}  \n", "aaaa  \\b  \\n"),
        (
            "asj \"as[ldkjasl # \u{2} s",
            "asj \\\"as[ldkjasl # \\u0002 s",
        ),
        ("", ""),
        ("    ", "    "),
        ("\u{C}-  \\ \r +#@* {} \" ", "\\f-  \\\\ \\r +#@* {} \\\" "),
        (" aa-\u{1a}_@", " aa-\\u001a_@"),
    ];

    for (t, res) in tests.iter() {
        assert_eq!(escape_str(t), format!("\"{}\"", res));
    }
}

#[test]
fn parse() {
    use super::{json_parse, JsonError, JsonValue};
    use std::fs::{read, read_dir};

    let results = vec![
        Ok(JsonValue::Object({
            let mut map = HashMap::new();
            map.insert(
                "thing".to_string(),
                JsonValue::Array(vec![
                    JsonValue::Number(10f64),
                    JsonValue::Number(20f64),
                    JsonValue::Number(230e20),
                ]),
            );
            map.insert("mmmmm".to_string(), JsonValue::Object(HashMap::new()));
            map.insert("__1ew".to_string(), JsonValue::Text(",, []".to_string()));
            map
        })),
        Err(JsonError::UnexpectedEOF),
        Ok(JsonValue::Array(vec![
            JsonValue::Number(10.0),
            JsonValue::Text(", \" 2{]0".to_string()),
            JsonValue::Number(30.0),
            JsonValue::Object({
                let mut map = HashMap::new();
                map.insert("f".to_string(), JsonValue::Boolean(false));
                map.insert("t".to_string(), JsonValue::Boolean(true));
                map
            }),
            JsonValue::Object({
                let mut map = HashMap::new();
                map.insert("e}".to_string(), JsonValue::Number(2.0));
                map.insert(
                    "v".to_string(),
                    JsonValue::Array(vec![JsonValue::Null, JsonValue::Array(vec![])]),
                );
                map
            }),
        ])),
        Err(JsonError::UnexpectedToken {
            character: 'I',
            location: 20,
        }),
        Ok(JsonValue::Object({
            let mut map = HashMap::new();
            map.insert("jss".to_string(), JsonValue::Number(-0.30e20));
            map.insert(
                "faa".to_string(),
                JsonValue::Text("\t\u{8}  \u{c} oώo".to_string()),
            );
            map.insert(
                "m,".to_string(),
                JsonValue::Array(vec![JsonValue::Null, JsonValue::Boolean(true)]),
            );
            map
        })),
        Ok(JsonValue::Array(vec![
            JsonValue::Object({
                let mut map = HashMap::new();
                map.insert("title".to_string(), JsonValue::Text("EEEEE".to_string()));
                map.insert(
                    "author".to_string(),
                    JsonValue::Text("Richard V.".to_string()),
                );
                map.insert(
                    "ratings".to_string(),
                    JsonValue::Array(vec![
                        JsonValue::Object({
                            let mut map = HashMap::new();
                            map.insert("stars".to_string(), JsonValue::Number(5f64));
                            map.insert(
                                "message".to_string(),
                                JsonValue::Text("Loved it!".to_string()),
                            );
                            map.insert(
                                "author".to_string(),
                                JsonValue::Text("rater_1".to_string()),
                            );
                            map
                        }),
                        JsonValue::Object({
                            let mut map = HashMap::new();
                            map.insert("stars".to_string(), JsonValue::Number(3.4));
                            map.insert(
                            "message".to_string(),
                            JsonValue::Text("Eh.  This was an okay book.  The fact that the book only used the capital letter \"E\" felt a bit unoriginal.  Nevertheless, I enjoyed some parts of it.".to_string()),
                        );
                            map.insert(
                                "author".to_string(),
                                JsonValue::Text("floof_77".to_string()),
                            );
                            map
                        }),
                    ]),
                );
                map
            }),
            JsonValue::Object({
                let mut map = HashMap::new();
                map.insert(
                    "title".to_string(),
                    JsonValue::Text(
                        "243 Pages of James Complaining About That Dog That Just Won't Shut Up"
                            .to_string(),
                    ),
                );
                map.insert(
                    "author".to_string(),
                    JsonValue::Text("James J. Jimmy".to_string()),
                );
                map.insert(
                    "ratings".to_string(),
                    JsonValue::Array(vec![
                        JsonValue::Object({
                            let mut map = HashMap::new();
                            map.insert("stars".to_string(), JsonValue::Number(5f64));
                            map.insert(
                                "message".to_string(),
                                JsonValue::Text("This book resonated with me on a spiritual level.  5/5, no questions asked.".to_string()),
                            );
                            map.insert(
                                "author".to_string(),
                                JsonValue::Text("floof_77".to_string()),
                            );
                            map
                        }),
                        JsonValue::Object({
                            let mut map = HashMap::new();
                            map.insert("stars".to_string(), JsonValue::Number(0f64));
                            map.insert(
                                "message".to_string(),
                                JsonValue::Text(
                                    "He Was Mean To DOGS That's So Rud de :(:(L(L(".to_string(),
                                ),
                            );
                            map.insert(
                                "author".to_string(),
                                JsonValue::Text("jjjjjj_26".to_string()),
                            );
                            map
                        }),
                    ]),
                );
                map
            }),
        ])),
        Ok(JsonValue::Object({
            let mut map = HashMap::new();
            map.insert(
                "articles".to_string(),
                JsonValue::Array(vec![
                    JsonValue::Text("a".to_string()),
                    JsonValue::Text("the".to_string()),
                    JsonValue::Text("an".to_string()),
                ]),
            );
            map.insert(
                "cc".to_string(),
                JsonValue::Array(vec![
                    JsonValue::Text("and".to_string()),
                    JsonValue::Text("but".to_string()),
                    JsonValue::Text("for".to_string()),
                    JsonValue::Text("nor".to_string()),
                    JsonValue::Text("or".to_string()),
                    JsonValue::Text("so".to_string()),
                    JsonValue::Text("yet".to_string()),
                ]),
            );
            map
        })),
        Err(JsonError::UnexpectedToken {
            character: '[',
            location: 25,
        }),
        Err(JsonError::UnexpectedToken {
            character: ',',
            location: 19,
        }),
        Err(JsonError::UnexpectedToken {
            character: '.',
            location: 24,
        }),
        Err(JsonError::UnexpectedToken {
            character: '0',
            location: 40,
        }),
        Ok(JsonValue::Object({
            let mut map = HashMap::new();
            map.insert(
                "animals".to_string(),
                JsonValue::Array(vec![
                    JsonValue::Text("dog".into()),
                    JsonValue::Text("frog".into()),
                    JsonValue::Text("kangaroo".into()),
                    JsonValue::Text("crab".into()),
                ]),
            );
            map.insert(
                "members".to_string(),
                JsonValue::Object({
                    let mut map = HashMap::new();
                    map.insert("John".into(), JsonValue::Number(1f64));
                    map.insert("Ferris".into(), JsonValue::Number(3f64));
                    map
                }),
            );
            map
        })),
        Err(JsonError::UnexpectedEOF),
        Err(JsonError::UnexpectedToken {
            character: '\n',
            location: 34,
        }),
        Err(JsonError::UnexpectedToken {
            character: 'f',
            location: 6,
        }),
        Ok(JsonValue::Text("as asdlkajd \" \u{c}|\t".into())),
        Ok(JsonValue::Boolean(true)),
        Err(JsonError::UnexpectedToken {
            character: 'e',
            location: 11,
        }),
    ];

    for file in read_dir("./test-json").unwrap().flat_map(|r| r) {
        let index = strip_extension(file.file_name())
            .and_then(|s| s.parse().map_err(drop))
            .map(|i: usize| i - 1)
            .unwrap();
        assert_eq!(
            results[index],
            json_parse(&String::from_utf8(read(file.path()).unwrap()).unwrap())
        )
    }
}

fn strip_extension(s: OsString) -> Result<String, ()> {
    let mut name = s.into_string().map_err(drop)?;
    if let Some(ind) = name.rfind('.') {
        name.truncate(ind);
    }
    Ok(name)
}

#[test]
fn stringify() {
    use super::JsonValue;

    // We can't test stringification on objects with more than one key in this manner since rust's HashMap does not guarantee order
    let tests = vec![
        (
            JsonValue::Array(vec![
                JsonValue::Number(10f64),
                JsonValue::Number(20f64),
                JsonValue::Number(30f64),
            ]),
            "[10,20,30]",
        ),
        (
            JsonValue::Object({
                let mut map = HashMap::new();
                map.insert(
                    "aaaa".into(),
                    JsonValue::Array(vec![
                        JsonValue::Null,
                        JsonValue::Object(HashMap::new()),
                        JsonValue::Boolean(false),
                        JsonValue::Array(vec![JsonValue::Array(vec![]), JsonValue::Number(10e10)]),
                    ]),
                );
                map
            }),
            r#"{"aaaa":[null,{},false,[[],100000000000]]}"#,
        ),
        (JsonValue::Array(vec![]), "[]"),
    ];

    for (value, stringified) in tests.into_iter() {
        assert_eq!(value.to_string(), stringified);
    }
}
