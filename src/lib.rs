use pest::Parser;
#[allow(unused_imports)]
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "query.pest"]
pub struct QueryParser;

#[derive(Debug)]
struct Segment {
    kind: SegmentType,
    selectors: Vec<Selector>,
}

#[derive(Debug)]
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

pub fn interpret_query(
    _input: serde_json::Value,
    query: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let segments = parse_to_segments(query);
    println!("{segments:?}");
    Ok(())
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
}
