use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::char,
    combinator::{into, opt},
    error::context,
    multi::many0,
    sequence::{separated_pair, terminated, tuple},
    IResult,
};

use crate::{
    input::Input,
    intermediate::{types::*, *},
};

use super::{constraint::constraint, error::ParserResult, *};

pub fn choice_value(input: Input<'_>) -> ParserResult<'_, ASN1Value> {
    map(
        skip_ws_and_comments(separated_pair(identifier, char(':'), asn1_value)),
        |(id, val)| ASN1Value::Choice {
            type_name: None,
            variant_name: id.to_owned(),
            inner_value: Box::new(val),
        },
    )(input)
}

/// Tries to parse the named alternative an ASN1 CHOICE
///
/// *`input` - [Input]-wrapped string slice to be matched against
///
/// `selection_type_choice` will try to match a CHOICE selection type in the `input` string.
/// ```ignore
/// // An example of selection type notation for the following CHOICE...
/// Example-choice ::= CHOICE {
///         alt1 Type1,
///         alt2 Type2,
///         alt3 Type3
/// }
/// // ... is the following assignment
/// Type-3-alias ::= alt3 < Example-choice
/// ```
/// If the match succeeds, the lexer will consume the match and return the remaining string
/// and a wrapped `Choice` value representing the ASN1 declaration. If the defined CHOICE
/// contains anonymous members, these nested members will be represented as
/// structs within the same global scope.
/// If the match fails, the lexer will not consume the input and will return an error.
pub fn selection_type_choice(input: Input<'_>) -> ParserResult<'_, ASN1Type> {
    map(
        into(separated_pair(
            skip_ws_and_comments(value_identifier),
            skip_ws_and_comments(char(LEFT_CHEVRON)),
            skip_ws_and_comments(title_case_identifier),
        )),
        ASN1Type::ChoiceSelectionType,
    )(input)
}

/// Tries to parse an ASN1 CHOICE
///
/// *`input` - [Input]-wrapped string slice to be matched against
///
/// `choice` will try to match an CHOICE declaration in the `input` string.
/// If the match succeeds, the lexer will consume the match and return the remaining string
/// and a wrapped `Choice` value representing the ASN1 declaration. If the defined CHOICE
/// contains anonymous members, these nested members will be represented as
/// structs within the same global scope.
/// If the match fails, the lexer will not consume the input and will return an error.
pub fn choice(input: Input<'_>) -> ParserResult<'_, ASN1Type> {
    context(
        "ChoiceType",
        map(
            preceded(
                skip_ws_and_comments(tag(CHOICE)),
                in_braces(tuple((
                    many0(terminated(
                        skip_ws_and_comments(choice_option),
                        optional_comma,
                    )),
                    opt(terminated(
                        extension_marker,
                        opt(skip_ws_and_comments(char(COMMA))),
                    )),
                    opt(map(
                        many0(alt((
                            map(
                                terminated(skip_ws_and_comments(choice_option), optional_comma),
                                |extension| vec![extension],
                            ),
                            terminated(
                                in_brackets(in_brackets(many1(terminated(
                                    skip_ws_and_comments(choice_option),
                                    optional_comma,
                                )))),
                                optional_comma,
                            ),
                        ))),
                        |extensions| extensions.into_iter().flatten().collect(),
                    )),
                ))),
            ),
            |m| ASN1Type::Choice(m.into()),
        ),
    )(input)
}

fn choice_option(input: Input<'_>) -> ParserResult<'_, ChoiceOption> {
    into(tuple((
        skip_ws_and_comments(identifier),
        opt(asn_tag),
        skip_ws_and_comments(asn1_type),
        opt(skip_ws_and_comments(constraint)),
    )))(input)
}

#[cfg(test)]
mod tests {
    use crate::{
        intermediate::{
            types::{Choice, ChoiceOption, ChoiceSelectionType},
            ASN1Type, DeclarationElsewhere,
        },
        lexer::{choice::selection_type_choice, choice_value, ASN1Value},
    };

    use crate::lexer::choice;

    #[test]
    fn parses_extensible_choice() {
        assert_eq!(
            choice(
                r#"CHOICE
    {normal NULL,
    high NULL,
    ...,
    medium NULL }"#
                    .into()
            )
            .unwrap()
            .1,
            ASN1Type::Choice(Choice {
                extensible: Some(2),
                options: vec![
                    ChoiceOption {
                        is_recursive: false,
                        name: "normal".into(),
                        tag: None,
                        ty: ASN1Type::Null,
                        constraints: vec![]
                    },
                    ChoiceOption {
                        is_recursive: false,
                        name: "high".into(),
                        tag: None,
                        ty: ASN1Type::Null,
                        constraints: vec![]
                    },
                    ChoiceOption {
                        is_recursive: false,
                        name: "medium".into(),
                        tag: None,
                        ty: ASN1Type::Null,
                        constraints: vec![]
                    }
                ],
                constraints: vec![]
            })
        )
    }

    #[test]
    fn parses_selection_type_choice() {
        assert_eq!(
            selection_type_choice("localDistinguishedName < ObjectInstance".into())
                .unwrap()
                .1,
            ASN1Type::ChoiceSelectionType(ChoiceSelectionType {
                choice_name: "ObjectInstance".into(),
                selected_option: "localDistinguishedName".into()
            })
        )
    }

    #[test]
    fn parses_extension_groups() {
        assert_eq!(
            ASN1Type::Choice(Choice {
                extensible: Some(1,),
                options: vec![
                    ChoiceOption {
                        is_recursive: false,
                        name: "glc".into(),
                        tag: None,
                        ty: ASN1Type::ElsewhereDeclaredType(DeclarationElsewhere {
                            parent: None,
                            identifier: "GeographicLocationContainer".into(),
                            constraints: vec![],
                        },),
                        constraints: vec![],
                    },
                    ChoiceOption {
                        is_recursive: false,
                        name: "avc".into(),
                        tag: None,
                        ty: ASN1Type::ElsewhereDeclaredType(DeclarationElsewhere {
                            parent: None,
                            identifier: "AutomatedVehicleContainer".into(),
                            constraints: vec![],
                        },),
                        constraints: vec![],
                    },
                    ChoiceOption {
                        is_recursive: false,
                        name: "rsc".into(),
                        tag: None,
                        ty: ASN1Type::ElsewhereDeclaredType(DeclarationElsewhere {
                            parent: None,
                            identifier: "RoadSurfaceContainer".into(),
                            constraints: vec![],
                        },),
                        constraints: vec![],
                    },
                    ChoiceOption {
                        is_recursive: false,
                        name: "isc".into(),
                        tag: None,
                        ty: ASN1Type::ElsewhereDeclaredType(DeclarationElsewhere {
                            parent: None,
                            identifier: "InfrastructureSupportContainer".into(),
                            constraints: vec![],
                        },),
                        constraints: vec![],
                    },
                ],
                constraints: vec![],
            },),
            choice(
                r#"CHOICE {
            glc	    GeographicLocationContainer,
            ...,	-- original extension indicator of V1
         [[ 
            avc	    AutomatedVehicleContainer,	
            rsc	    RoadSurfaceContainer ]], -- Extension in V2
            isc      InfrastructureSupportContainer  -- Extension in V3.1
         }"#
                .into()
            )
            .unwrap()
            .1
        )
    }

    #[test]
    fn constructed_choice_value() {
        assert_eq!(
            choice_value(
                r#"equalityMatch: { attributeDesc "ABCDLMYZ", assertionValue 'A2'H }"#.into()
            )
            .unwrap()
            .1,
            ASN1Value::Choice {
                type_name: None,
                variant_name: "equalityMatch".into(),
                inner_value: Box::new(ASN1Value::SequenceOrSet(vec![
                    (
                        Some("attributeDesc".into()),
                        Box::new(ASN1Value::String("ABCDLMYZ".into())),
                    ),
                    (
                        Some("assertionValue".into()),
                        Box::new(ASN1Value::BitString(vec![
                            true, false, true, false, false, false, true, false
                        ],)),
                    ),
                ])),
            },
        )
    }

    #[test]
    fn nested_choice_value() {
        assert_eq!(
            choice_value(r#"not:equalityMatch: "ABCDLMYZ""#.into())
                .unwrap()
                .1,
            ASN1Value::Choice {
                type_name: None,
                variant_name: "not".into(),
                inner_value: Box::new(ASN1Value::Choice {
                    type_name: None,
                    variant_name: "equalityMatch".into(),
                    inner_value: Box::new(ASN1Value::String("ABCDLMYZ".into()))
                }),
            },
        )
    }

    #[test]
    fn nested_constructed_choice_value() {
        assert_eq!(
            choice_value(
                r#"not:equalityMatch: { attributeDesc "ABCDLMYZ", assertionValue 'A2'H }"#.into()
            )
            .unwrap()
            .1,
            ASN1Value::Choice {
                type_name: None,
                variant_name: "not".into(),
                inner_value: Box::new(ASN1Value::Choice {
                    type_name: None,
                    variant_name: "equalityMatch".into(),
                    inner_value: Box::new(ASN1Value::SequenceOrSet(vec![
                        (
                            Some("attributeDesc".into()),
                            Box::new(ASN1Value::String("ABCDLMYZ".into())),
                        ),
                        (
                            Some("assertionValue".into()),
                            Box::new(ASN1Value::BitString(vec![
                                true, false, true, false, false, false, true, false
                            ],)),
                        ),
                    ])),
                })
            },
        )
    }
}
