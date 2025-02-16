use pest::Parser;
#[allow(unused_imports)]
use pest_derive::Parser;
use serde_json::Value;

#[derive(Parser)]
#[grammar = "query.pest"]
pub struct QueryParser;

#[derive(Debug)]
struct Segment {
    kind: SegmentType,
    selectors: Vec<Selector>,
}

#[derive(Debug, PartialEq)]
enum SegmentType {
    Child,
    Descendant,
}

#[derive(Debug)]
enum Selector {
    Name(String),
    Wildcard,
    Index(isize),
    /// start, end, step
    Slice(Option<isize>, Option<isize>, Option<isize>),
}

fn parse_to_segments(query: &str) -> Result<Vec<Segment>, Box<dyn std::error::Error>> {
    let parsed = QueryParser::parse(Rule::jsonpath_query, query)?
        .next()
        .unwrap();

    assert_eq!(parsed.as_rule(), Rule::jsonpath_query);

    let mut normalized_segments: Vec<Segment> = Vec::new();

    for pair in parsed.into_inner() {
        match pair.as_rule() {
            Rule::root_identifier | Rule::EOI => {}
            Rule::segments => {
                let inner_rules = pair.into_inner();
                for rule in inner_rules {
                    assert_eq!(rule.as_rule(), Rule::segment);

                    let segment = rule.into_inner().next().unwrap();

                    let kind = match segment.as_rule() {
                        Rule::child_segment => SegmentType::Child,
                        Rule::descendant_segment => SegmentType::Descendant,
                        _ => unreachable!(),
                    };

                    let selectors = segment
                        .into_inner()
                        .map(|s| match s.as_rule() {
                            Rule::name_selector | Rule::member_name_shorthand => {
                                Selector::Name(trim_quotes(s.as_str()))
                            }
                            Rule::wildcard_selector => Selector::Wildcard,
                            Rule::index_selector => {
                                let inner_rule = s.into_inner().next().unwrap();
                                Selector::Index(inner_rule.as_str().parse().unwrap())
                            }
                            Rule::slice_selector => {
                                let (mut start, mut end, mut step) = (None, None, None);
                                for pair in s.into_inner() {
                                    let value = Some(pair.as_str().parse().unwrap());
                                    match pair.as_rule() {
                                        Rule::start => start = value,
                                        Rule::end => end = value,
                                        Rule::step => step = value,
                                        _ => unreachable!(),
                                    };
                                }
                                Selector::Slice(start, end, step)
                            }
                            _ => unreachable!(),
                        })
                        .collect();

                    let normalized_segment = Segment { kind, selectors };
                    normalized_segments.push(normalized_segment);
                }
            }
            _ => unreachable!(),
        }
    }

    Ok(normalized_segments)
}

fn process_segment(input: Vec<Value>, segment: &Segment) -> Vec<serde_json::Value> {
    // todo: add support for descendant type
    assert_eq!(segment.kind, SegmentType::Child);

    input
        .into_iter()
        .flat_map(|item| {
            segment
                .selectors
                .iter()
                .flat_map(|selector| match selector {
                    Selector::Name(key) => item.get(key).cloned().into_iter().collect(),
                    Selector::Index(idx) => match item.as_array() {
                        Some(arr) if !arr.is_empty() => {
                            let arr_idx = if *idx < 0 {
                                arr.len()
                                    .checked_sub(idx.unsigned_abs())
                                    .unwrap_or(arr.len())
                            } else {
                                *idx as usize
                            };
                            item.get(arr_idx).cloned().into_iter().collect()
                        }
                        _ => vec![],
                    },
                    Selector::Wildcard => match &item {
                        Value::Array(values) => values.clone(),
                        Value::Object(map) => map.values().cloned().collect(),
                        _ => vec![],
                    },
                    Selector::Slice(start, end, step) => match item.as_array() {
                        Some(arr) if !arr.is_empty() => {
                            let step = step.unwrap_or(1);
                            if step == 0 {
                                return vec![];
                            }

                            let (start, end) = if step >= 0 {
                                (start.unwrap_or(0), end.unwrap_or(arr.len() as isize))
                            } else {
                                (
                                    start.unwrap_or((arr.len() - 1) as isize),
                                    end.unwrap_or(-((arr.len() + 1) as isize)),
                                )
                            };

                            let normalize =
                                |i: isize, len: usize| if i >= 0 { i } else { len as isize + i };

                            let bounds = |start: isize, end: isize, step: isize, len: isize| {
                                let (n_start, n_end) =
                                    (normalize(start, len as usize), normalize(end, len as usize));

                                let (lower, upper) = if step >= 0 {
                                    (n_start.max(0).min(len), n_end.max(0).min(len))
                                } else {
                                    (n_end.max(-1).min(len - 1), n_start.max(-1).min(len - 1))
                                };

                                (lower, upper)
                            };

                            let (lower, upper) = bounds(start, end, step, arr.len() as isize);

                            let (lower, upper) = if step >= 0 {
                                (lower as usize, upper as usize)
                            } else {
                                ((lower + 1) as usize, (upper + 1) as usize)
                            };

                            if upper < lower {
                                return vec![];
                            }

                            let itr = arr[lower..upper].iter();

                            if step >= 0 {
                                itr.step_by(step as usize).cloned().collect()
                            } else {
                                itr.rev().step_by(step.unsigned_abs()).cloned().collect()
                            }
                        }
                        _ => vec![],
                    },
                })
                .collect::<Vec<Value>>()
        })
        .collect()
}

pub fn interpret(
    input_s: &str,
    query: &str,
) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>> {
    let segments = parse_to_segments(query)?;
    if let Some(input) = serde_json::from_str(input_s)? {
        Ok(segments.iter().fold(vec![input], process_segment))
    } else {
        Ok(vec![])
    }
}

fn trim_quotes(input: &str) -> String {
    if let Some(stripped) = input
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .or_else(|| input.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')))
    {
        stripped.to_string()
    } else {
        input.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pest::Parser;

    mod parse_valid_queries {
        use super::*;

        macro_rules! parses {
            ($name:ident, $query:expr) => {
                #[test]
                fn $name() {
                    let parsed = QueryParser::parse(Rule::jsonpath_query, $query);
                    assert!(parsed.is_ok(), "Failed to parse: {}", $query);

                    println!("{}", $query);
                    println!("{parsed:#?}");
                }
            };
        }

        parses!(only_root, "$");

        /*
        /  Child Segment
        */

        // named selector
        parses!(named, "$.foo");
        parses!(named_nested, "$.foo['bar']");
        parses!(named_nested_further, "$.foo['bar baz']['k.k']");
        parses!(named_diff_delimiter, r#"$.foo["bar baz"]["k.k"]"#);
        parses!(named_unusual, r#"$["'"]["@"]"#);

        // wildcard selector
        parses!(wildcard_root, "$.*");
        parses!(wildcard_after_named, "$.foo[*]");
        parses!(wildcard_multiple_selection, "$.foo[*, *]");

        // index selector
        parses!(index_positive, "$[1]");
        parses!(index_negative, "$[-2]");
        parses!(index_combined, "$[0][3]");

        // slice selector
        parses!(slice_start_end, "$[1:3]");
        parses!(slice_start_only, "$[5:]");
        parses!(slice_end_only, "$[:4]");
        parses!(slice_with_step, "$[1:5:2]");
        parses!(slice_only_step, "$[::-1]");
    }

    mod fails_invalid_queries {
        use super::*;

        macro_rules! fails_to_parse {
            ($name:ident, $query:expr) => {
                #[test]
                fn $name() {
                    let parsed = QueryParser::parse(Rule::jsonpath_query, $query);
                    assert!(parsed.is_err(), "Successfully parsed: {}", $query);

                    println!("{}", $query);
                    println!("{parsed:#?}");
                }
            };
        }

        fails_to_parse!(missing_root_identifier, ".foo");
        fails_to_parse!(incomplete_segment_after_root, "$.");
        fails_to_parse!(multiple_roots, "$$['foo']");
        fails_to_parse!(dot_followed_by_brackets, "$.['foo']");
        fails_to_parse!(leading_trailing_whitespace, "  $['foo']  ");

        fails_to_parse!(invalid_char_in_member_name, "$.foo@bar");

        // bracketed selection
        fails_to_parse!(unclosed_bracket_in_selection, "$['foo'");
        fails_to_parse!(additional_bracket_after_selection, "$['foo']]");
        fails_to_parse!(comma_without_selector, "$['key',]");
        fails_to_parse!(empty_brackets, "$[]");
        fails_to_parse!(misplaced_colon, "$[:5:2:3]");

        // string literals
        fails_to_parse!(unclosed_single_quote, "$['foo]");
        fails_to_parse!(mismatched_quotes, "$['foo\"]");
        fails_to_parse!(invalid_unicode_escape, "$['\\uZZZZ']");
        fails_to_parse!(invalid_escape_sequence, "$['\\q']");
        fails_to_parse!(unterminated_escape_sequence, "$['\\']");
        fails_to_parse!(unescaped_newline_in_quotes, "$[\"foo\nbar\"]");
    }

    mod interpret_queries {
        use serde_json::json;

        use super::*;

        macro_rules! json_vec {
            ([$($elem:expr),* $(,)?]) => {
                vec![$(json!($elem)),*]
            };
        }

        macro_rules! interprets_to {
            ($name:ident, $input:expr, $query:expr, $expected:expr) => {
                #[test]
                fn $name() {
                    let output = interpret($input, $query);
                    assert!(output.is_ok());
                    assert_eq!($expected, output.unwrap())
                }
            };

            ($name:ident, $input:expr, $query:expr, $expected:expr, unordered) => {
                #[test]
                fn $name() {
                    let output = interpret($input, $query);
                    assert!(output.is_ok());

                    let mut expected = $expected.clone();
                    let mut result = output.unwrap();

                    expected.sort_by_key(|v| serde_json::to_string(v).unwrap());
                    result.sort_by_key(|v| serde_json::to_string(v).unwrap());

                    assert_eq!(expected, result);
                }
            };
        }

        const EMPTY_VEC: Vec<serde_json::Value> = Vec::new();

        interprets_to!(only_root, r#"{"k": "v"}"#, "$", vec![json!({"k": "v"})]);

        // named selector

        const INPUT_1: &str = r#"{
            "o": {"j j": {"k.k": 3}},
            "'": {"@": 2}
        }"#;

        interprets_to!(
            named_basic,
            INPUT_1,
            "$.o",
            vec![json!({"j j": {"k.k": 3}})]
        );

        interprets_to!(named_nested, INPUT_1, "$.o['j j']", vec![json!({"k.k": 3})]);

        interprets_to!(
            named_nested_further,
            INPUT_1,
            "$.o['j j']['k.k']",
            vec![json!(3)]
        );

        interprets_to!(
            named_diff_delimiter,
            INPUT_1,
            r#"$.o["j j"]["k.k"]"#,
            vec![json!(3)]
        );

        interprets_to!(
            named_unusual_member_names,
            INPUT_1,
            r#"$["'"]["@"]"#,
            vec![json!(2)]
        );

        interprets_to!(
            named_multiple,
            INPUT_1,
            r#"$["'", "o"]"#,
            vec![json!({"@": 2}), json!({"j j": { "k.k": 3 }})]
        );

        // wildcard selector

        const INPUT_2: &str = r#"{
            "o": {"j": 1, "k": 2},
            "a": [5, 3]
        }"#;

        interprets_to!(
            wildcard_first,
            INPUT_2,
            "$[*]",
            vec![json!({"j": 1, "k": 2}), json!([5, 3])],
            unordered
        );

        interprets_to!(wildcard_nested, INPUT_2, "$.o[*]", json_vec!([1, 2]));

        interprets_to!(
            wildcard_repeated,
            INPUT_2,
            "$.o[*, *]",
            json_vec!([1, 2, 1, 2])
        );

        interprets_to!(wildcard_for_array, INPUT_2, "$.a[*]", json_vec!([5, 3]));

        // index selector

        const INPUT_3: &str = r#"["a","b"]"#;

        interprets_to!(index_pos, INPUT_3, "$[1]", vec![json!("b")]);

        interprets_to!(index_neg, INPUT_3, "$[-2]", vec![json!("a")]);

        interprets_to!(index_pos_out_of_bound, INPUT_3, "$[2]", EMPTY_VEC);

        interprets_to!(index_neg_out_of_bound, INPUT_3, "$[-3]", EMPTY_VEC);

        // slice selector

        const INPUT_4: &str = r#"["a", "b", "c", "d", "e", "f", "g"]"#;

        interprets_to!(slice_basic, INPUT_4, "$[1:3]", json_vec!(["b", "c"]));

        interprets_to!(slice_pos_step, INPUT_4, "$[1:5:2]", json_vec!(["b", "d"]));

        interprets_to!(slice_neg_step, INPUT_4, "$[5:1:-2]", json_vec!(["f", "d"]));

        interprets_to!(slice_start_only, INPUT_4, "$[5:]", json_vec!(["f", "g"]));

        interprets_to!(slice_end_only, INPUT_4, "$[:3]", json_vec!(["a", "b", "c"]));

        interprets_to!(
            slice_defaults,
            INPUT_4,
            "$[::]",
            json_vec!(["a", "b", "c", "d", "e", "f", "g"])
        );

        interprets_to!(slice_empty_input, r#"[]"#, "$[1:3]", EMPTY_VEC);

        interprets_to!(slice_zero_step, INPUT_4, "$[1:3:0]", EMPTY_VEC);

        interprets_to!(
            slice_reverse,
            INPUT_4,
            "$[::-1]",
            json_vec!(["g", "f", "e", "d", "c", "b", "a"])
        );

        interprets_to!(slice_end_before_start, INPUT_4, "$[4:2]", EMPTY_VEC);

        interprets_to!(
            slice_neg_bounds,
            INPUT_4,
            "$[-4:-1]",
            json_vec!(["d", "e", "f"])
        );

        interprets_to!(slice_out_of_bounds_pos, INPUT_4, "$[8:15]", EMPTY_VEC);

        interprets_to!(slice_out_of_bounds_neg, INPUT_4, "$[-7:-9]", EMPTY_VEC);

        interprets_to!(
            slice_start_out_of_bound,
            INPUT_4,
            "$[-10:]",
            json_vec!(["a", "b", "c", "d", "e", "f", "g"])
        );

        interprets_to!(
            slice_end_out_of_bound,
            INPUT_4,
            "$[:10]",
            json_vec!(["a", "b", "c", "d", "e", "f", "g"])
        );

        interprets_to!(
            slice_start_out_of_bound_end_in_bound,
            INPUT_4,
            "$[-10:-5]",
            json_vec!(["a", "b"])
        );

        interprets_to!(
            slice_start_in_bound_end_out_of_bound,
            INPUT_4,
            "$[5:10]",
            json_vec!(["f", "g"])
        );

        interprets_to!(
            slice_start_and_end_out_of_bound,
            INPUT_4,
            "$[-10:10]",
            json_vec!(["a", "b", "c", "d", "e", "f", "g"])
        );

        interprets_to!(slice_same_start_end, INPUT_4, "$[2:2]", EMPTY_VEC);
    }
}
