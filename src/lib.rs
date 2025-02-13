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
}
