use nom::{
    branch::alt,
    bytes::complete::tag,
    combinator::{into, map, opt, value},
    sequence::preceded,
    Parser,
};

use crate::{
    input::Input,
    intermediate::{ASN1Type, ASN1Value, BOOLEAN, FALSE, TRUE},
};

use super::{common::skip_ws_and_comments, constraint::constraints, error::ParserResult};

pub fn boolean_value(input: Input<'_>) -> ParserResult<'_, ASN1Value> {
    alt((
        value(ASN1Value::Boolean(true), skip_ws_and_comments(tag(TRUE))),
        value(ASN1Value::Boolean(false), skip_ws_and_comments(tag(FALSE))),
    ))
    .parse(input)
}

/// Tries to parse an ASN1 BOOLEAN
///
/// *`input` - [Input]-wrapped string slice to be matched against
///
/// `boolean` will try to match an BOOLEAN declaration in the `input` string.
/// If the match succeeds, the lexer will consume the match and return the remaining string
/// and an `ASN1Type::Boolean` value representing the ASN1 declaration.
/// If the match fails, the lexer will not consume the input and will return an error.
pub fn boolean(input: Input<'_>) -> ParserResult<'_, ASN1Type> {
    map(
        into(skip_ws_and_comments(preceded(
            tag(BOOLEAN),
            skip_ws_and_comments(opt(constraints)),
        ))),
        ASN1Type::Boolean,
    )
    .parse(input)
}

#[cfg(test)]
mod tests {
    use crate::intermediate::types::Boolean;

    use super::*;

    #[test]
    fn parses_boolean() {
        assert_eq!(
            boolean(" --who would put a comment here?--BOOLEAN".into())
                .unwrap()
                .1,
            ASN1Type::Boolean(Boolean {
                constraints: vec![]
            })
        )
    }
}
