#[allow(unused_imports)]
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "query.pest"]
pub struct QueryParser;

#[cfg(test)]
mod tests {
    use super::*;
    use pest::Parser;

    mod parse_valid_queries {
        use super::*;

        macro_rules! parses {
            ($name:ident, $input:expr) => {
                #[test]
                fn $name() {
                    let parsed = QueryParser::parse(Rule::jsonpath_query, $input);
                    assert!(parsed.is_ok(), "Failed to parse: {}", $input);

                    println!("{}", $input);
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
        parses!(named_further_nesting, "$.foo['bar baz']['k.k']");
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
            ($name:ident, $input:expr) => {
                #[test]
                fn $name() {
                    let parsed = QueryParser::parse(Rule::jsonpath_query, $input);
                    assert!(parsed.is_err(), "Successfully parsed: {}", $input);

                    println!("{}", $input);
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
