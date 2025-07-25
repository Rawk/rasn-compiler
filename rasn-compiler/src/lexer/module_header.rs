use crate::{
    input::{context_boundary, Input},
    intermediate::*,
};
use nom::{
    branch::alt,
    bytes::complete::{tag, take_until},
    character::complete::{char, one_of},
    combinator::{into, map, not, opt, peek, recognize, value},
    multi::{many0, many1, separated_list1},
    sequence::{delimited, pair, preceded, separated_pair, terminated},
    Parser,
};

use super::{
    common::{identifier, skip_ws, skip_ws_and_comments, value_reference},
    error::ParserResult,
    in_braces, into_inner,
    object_identifier::object_identifier_value,
};

pub fn module_header(input: Input<'_>) -> ParserResult<'_, ModuleHeader> {
    skip_ws_and_comments(into((
        identifier,
        opt(skip_ws(definitive_identification)),
        skip_ws_and_comments(delimited(
            tag(DEFINITIONS),
            opt(environments),
            skip_ws_and_comments(pair(tag(ASSIGN), skip_ws_and_comments(tag(BEGIN)))),
        )),
        context_boundary(opt(exports)),
        context_boundary(opt(imports)),
    )))
    .parse(input)
}

fn definitive_identification(input: Input<'_>) -> ParserResult<'_, DefinitiveIdentifier> {
    into(pair(object_identifier_value, opt(iri_value))).parse(input)
}

fn iri_value(input: Input<'_>) -> ParserResult<'_, &str> {
    into_inner(skip_ws_and_comments(delimited(
        tag("\"/"),
        recognize(separated_list1(char('/'), unicode_label)),
        char('"'),
    )))
    .parse(input)
}

fn unicode_label(input: Input<'_>) -> ParserResult<'_, &str> {
    skip_ws_and_comments(into_inner(recognize(many1(one_of(
        "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz01234567890-_~.",
    )))))
    .parse(input)
}

fn exports(input: Input<'_>) -> ParserResult<'_, Exports> {
    skip_ws_and_comments(delimited(
        tag(EXPORTS),
        skip_ws(alt((
            value(Exports::All, tag(ALL)),
            into(separated_list1(
                skip_ws(char(COMMA)),
                skip_ws(alt((parameterized_identifier, identifier))),
            )),
        ))),
        char(SEMICOLON),
    ))
    .parse(input)
}

fn imports(input: Input<'_>) -> ParserResult<'_, Vec<Import>> {
    skip_ws_and_comments(delimited(
        tag(IMPORTS),
        skip_ws_and_comments(many0(import)),
        skip_ws_and_comments(char(SEMICOLON)),
    ))
    .parse(input)
}

fn parameterized_identifier(input: Input<'_>) -> ParserResult<'_, &str> {
    terminated(identifier, tag("{}")).parse(input)
}

fn global_module_reference(input: Input<'_>) -> ParserResult<'_, GlobalModuleReference> {
    into(skip_ws_and_comments(pair(
        identifier,
        alt((
            map(
                skip_ws_and_comments(object_identifier_value),
                AssignedIdentifier::ObjectIdentifierValue,
            ),
            map(
                skip_ws_and_comments(separated_pair(identifier, char(DOT), value_reference)),
                |(mod_ref, val_ref)| {
                    AssignedIdentifier::ExternalValueReference(ExternalValueReference {
                        module_reference: mod_ref.to_owned(),
                        value_reference: val_ref.to_owned(),
                    })
                },
            ),
            map(
                skip_ws_and_comments(pair(
                    value_reference,
                    skip_ws(recognize(in_braces(take_until("}")))),
                )),
                |(v, p)| AssignedIdentifier::ParameterizedValue {
                    value_reference: v.to_owned(),
                    actual_parameter_list: p.inner().to_owned(),
                },
            ),
            map(
                skip_ws_and_comments(terminated(
                    value_reference,
                    not(skip_ws_and_comments(alt((
                        peek(value((), tag(FROM))),
                        peek(value((), char(COMMA))),
                    )))),
                )),
                |v| AssignedIdentifier::ValueReference(v.to_owned()),
            ),
            value(
                AssignedIdentifier::Empty,
                not(skip_ws_and_comments(alt((
                    peek(value((), tag(FROM))),
                    peek(value((), char(COMMA))),
                )))),
            ),
        )),
    )))
    .parse(input)
}

fn import(input: Input<'_>) -> ParserResult<'_, Import> {
    into(skip_ws_and_comments(pair(
        separated_list1(
            skip_ws(char(COMMA)),
            skip_ws(alt((parameterized_identifier, identifier))),
        ),
        preceded(
            skip_ws_and_comments(tag(FROM)),
            skip_ws_and_comments(pair(
                global_module_reference,
                opt(into_inner(skip_ws_and_comments(alt((
                    tag(WITH_SUCCESSORS),
                    tag(WITH_DESCENDANTS),
                ))))),
            )),
        ),
    )))
    .parse(input)
}

fn environments(
    input: Input<'_>,
) -> ParserResult<
    '_,
    (
        Option<EncodingReferenceDefault>,
        TaggingEnvironment,
        ExtensibilityEnvironment,
    ),
> {
    (
        opt(skip_ws_and_comments(into(terminated(
            identifier,
            into_inner(skip_ws(tag(INSTRUCTIONS))),
        )))),
        skip_ws_and_comments(map(
            opt(terminated(
                into_inner(alt((tag(AUTOMATIC), tag(IMPLICIT), tag(EXPLICIT)))),
                skip_ws(tag(TAGS)),
            )),
            |m| match m {
                Some(AUTOMATIC) => TaggingEnvironment::Automatic,
                Some(EXPLICIT) => TaggingEnvironment::Explicit,
                _ => TaggingEnvironment::Implicit,
            },
        )),
        skip_ws_and_comments(map(opt(tag(EXTENSIBILITY_IMPLIED)), |m| {
            if m.is_some() {
                ExtensibilityEnvironment::Implied
            } else {
                ExtensibilityEnvironment::Explicit
            }
        })),
    )
        .parse(input)
}

#[cfg(test)]
mod tests {
    use std::vec;

    use crate::lexer::module_header::*;

    #[test]
    fn parses_a_module_header() {
        assert_eq!(module_header(r#"--! @options: no-fields-header

    ETSI-ITS-CDD {itu-t (0) identified-organization (4) etsi (0) itsDomain (5) wg1 (1) 102894 cdd (2) major-version-3 (3) minor-version-1 (1)}

    DEFINITIONS AUTOMATIC TAGS ::=

    BEGIN
    "#.into()).unwrap().1,
    ModuleHeader {name:"ETSI-ITS-CDD".into(),module_identifier:Some(DefinitiveIdentifier::DefinitiveOID(ObjectIdentifierValue(vec![ObjectIdentifierArc{name:Some("itu-t".into()),number:Some(0)},ObjectIdentifierArc{name:Some("identified-organization".into()),number:Some(4)},ObjectIdentifierArc{name:Some("etsi".into()),number:Some(0)},ObjectIdentifierArc{name:Some("itsDomain".into()),number:Some(5)},ObjectIdentifierArc{name:Some("wg1".into()),number:Some(1)},ObjectIdentifierArc{name:None,number:Some(102894)},ObjectIdentifierArc{name:Some("cdd".into()),number:Some(2)},ObjectIdentifierArc{name:Some("major-version-3".into()),number:Some(3)},ObjectIdentifierArc{name:Some("minor-version-1".into()),number:Some(1)}]))),encoding_reference_default:None,tagging_environment:crate::intermediate::TaggingEnvironment::Automatic,extensibility_environment:crate::intermediate::ExtensibilityEnvironment::Explicit, imports: vec![], exports: None }
  )
    }

    #[test]
    fn parses_a_module_header_with_imports() {
        assert_eq!(module_header(r#"CPM-PDU-Descriptions { itu-t (0) identified-organization (4) etsi (0) itsDomain (5) wg1 (1) ts (103324) cpm (1) major-version-1 (1) minor-version-1(1)}

        DEFINITIONS AUTOMATIC TAGS ::=

        BEGIN

        IMPORTS

        ItsPduHeader, MessageRateHz, MessageSegmentationInfo, OrdinalNumber1B,  ReferencePosition, StationType, TimestampIts
        FROM ETSI-ITS-CDD {itu-t (0) identified-organization (4) etsi (0) itsDomain (5) wg1 (1) ts (102894) cdd (2) major-version-3 (3) minor-version-1 (1)}
        WITH SUCCESSORS

        OriginatingRsuContainer, OriginatingVehicleContainer
        FROM CPM-OriginatingStationContainers {itu-t (0) identified-organization (4) etsi (0) itsDomain (5) wg1 (1) ts (103324) originatingStationContainers (2) major-version-1 (1) minor-version-1(1)}
        WITH SUCCESSORS;
    "#.into()).unwrap().1,
    ModuleHeader { name: "CPM-PDU-Descriptions".into(), module_identifier: Some(DefinitiveIdentifier::DefinitiveOID(ObjectIdentifierValue(vec![ObjectIdentifierArc { name: Some("itu-t".into()), number: Some(0) }, ObjectIdentifierArc { name: Some("identified-organization".into()), number: Some(4) }, ObjectIdentifierArc { name: Some("etsi".into()), number: Some(0) }, ObjectIdentifierArc { name: Some("itsDomain".into()), number: Some(5) }, ObjectIdentifierArc { name: Some("wg1".into()), number: Some(1) }, ObjectIdentifierArc { name: Some("ts".into()), number: Some(103324) }, ObjectIdentifierArc { name: Some("cpm".into()), number: Some(1) }, ObjectIdentifierArc { name: Some("major-version-1".into()), number: Some(1) }, ObjectIdentifierArc { name: Some("minor-version-1".into()), number: Some(1) }]))), encoding_reference_default: None, tagging_environment: TaggingEnvironment::Automatic, extensibility_environment: ExtensibilityEnvironment::Explicit, imports: vec![Import { types: vec!["ItsPduHeader".into(), "MessageRateHz".into(), "MessageSegmentationInfo".into(), "OrdinalNumber1B".into(), "ReferencePosition".into(), "StationType".into(), "TimestampIts".into()], global_module_reference: GlobalModuleReference { module_reference: "ETSI-ITS-CDD".into(), assigned_identifier: AssignedIdentifier::ObjectIdentifierValue(ObjectIdentifierValue(vec![ObjectIdentifierArc { name: Some("itu-t".into()), number: Some(0) }, ObjectIdentifierArc { name: Some("identified-organization".into()), number: Some(4) }, ObjectIdentifierArc { name: Some("etsi".into()), number: Some(0) }, ObjectIdentifierArc { name: Some("itsDomain".into()), number: Some(5) }, ObjectIdentifierArc { name: Some("wg1".into()), number: Some(1) }, ObjectIdentifierArc { name: Some("ts".into()), number: Some(102894) }, ObjectIdentifierArc { name: Some("cdd".into()), number: Some(2) }, ObjectIdentifierArc { name: Some("major-version-3".into()), number: Some(3) }, ObjectIdentifierArc { name: Some("minor-version-1".into()), number: Some(1) }]))}, with: Some(With::Successors) }, Import { types: vec!["OriginatingRsuContainer".into(), "OriginatingVehicleContainer".into()], global_module_reference: GlobalModuleReference { module_reference: "CPM-OriginatingStationContainers".into(), assigned_identifier: AssignedIdentifier::ObjectIdentifierValue(ObjectIdentifierValue(vec![ObjectIdentifierArc { name: Some("itu-t".into()), number: Some(0) }, ObjectIdentifierArc { name: Some("identified-organization".into()), number: Some(4) }, ObjectIdentifierArc { name: Some("etsi".into()), number: Some(0) }, ObjectIdentifierArc { name: Some("itsDomain".into()), number: Some(5) }, ObjectIdentifierArc { name: Some("wg1".into()), number: Some(1) }, ObjectIdentifierArc { name: Some("ts".into()), number: Some(103324) }, ObjectIdentifierArc { name: Some("originatingStationContainers".into()), number: Some(2) }, ObjectIdentifierArc { name: Some("major-version-1".into()), number: Some(1) }, ObjectIdentifierArc { name: Some("minor-version-1".into()), number: Some(1) }]))}, with: Some(With::Successors) }], exports: None } )
    }

    #[test]
    fn parses_iri_value() {
        assert_eq!(module_header(r#"CMSCKMKeyManagement {itu-t recommendation(0) x(24) cms-profile(894) module(0) cKMKeyManagement(1) version1(1)}
        "/ITU-T/Recommendation/X/CMS-Profile/Module/CKMKeyManagement/Version1"
        DEFINITIONS ::=
        BEGIN
        EXPORTS ALL;
        IMPORTS
        ALGORITHM,AlgorithmIdentifier{}
        FROM AlgorithmInformation-2009
        {iso(1) identified-organization(3) dod(6) internet(1) security(5)
        mechanisms(5) pkix(7) id-mod(0) id-mod-algorithmInformation-02(58)} WITH DESCENDANTS;"#.into()).unwrap().1,
        ModuleHeader {
            name: "CMSCKMKeyManagement".into(),
            module_identifier: Some(DefinitiveIdentifier::DefinitiveOIDandIRI {
                oid: ObjectIdentifierValue(vec![
                    ObjectIdentifierArc { name: Some("itu-t".into()), number: None },
                    ObjectIdentifierArc { name: Some("recommendation".into()), number: Some(0) },
                    ObjectIdentifierArc { name: Some("x".into()), number: Some(24) },
                    ObjectIdentifierArc { name: Some("cms-profile".into()), number: Some(894) },
                    ObjectIdentifierArc { name: Some("module".into()), number: Some(0) },
                    ObjectIdentifierArc { name: Some("cKMKeyManagement".into()), number: Some(1) },
                    ObjectIdentifierArc { name: Some("version1".into()), number: Some(1) },
                ]),
                iri: "ITU-T/Recommendation/X/CMS-Profile/Module/CKMKeyManagement/Version1".into()
            }),
            encoding_reference_default: None,
            tagging_environment: TaggingEnvironment::Implicit,
            extensibility_environment: ExtensibilityEnvironment::Explicit,
            imports: vec![
                Import {
                    types: vec!["ALGORITHM".into(), "AlgorithmIdentifier".into()],
                    global_module_reference: GlobalModuleReference {
                        module_reference: "AlgorithmInformation-2009".into(),
                        assigned_identifier: AssignedIdentifier::ObjectIdentifierValue(ObjectIdentifierValue(vec![
                            ObjectIdentifierArc { name: Some("iso".into()), number: Some(1) },
                            ObjectIdentifierArc { name: Some("identified-organization".into()), number: Some(3) },
                            ObjectIdentifierArc { name: Some("dod".into()), number: Some(6) },
                            ObjectIdentifierArc { name: Some("internet".into()), number: Some(1) },
                            ObjectIdentifierArc { name: Some("security".into()), number: Some(5) },
                            ObjectIdentifierArc { name: Some("mechanisms".into()), number: Some(5) },
                            ObjectIdentifierArc { name: Some("pkix".into()), number: Some(7) },
                            ObjectIdentifierArc { name: Some("id-mod".into()), number: Some(0) },
                            ObjectIdentifierArc { name: Some("id-mod-algorithmInformation-02".into()), number: Some(58) },
                        ]))
                    },
                    with: Some(With::Descendants) }
            ],
            exports: Some(Exports::All)
        })
    }

    #[test]
    fn parses_imports() {
        assert_eq!(
            imports(
                r#"IMPORTS
            DomainParameters
            FROM ANSI-X9-42
            {iso(1) member-body(2) us(840) ansi-x942(10046) module(5) 1}
            ECDomainParameters
            FROM ANSI-X9-62
            {iso(1) member-body(2) us(840) 10045 modules(0) 2};"#
                    .into()
            )
            .unwrap()
            .1,
            vec![
                Import {
                    types: vec!["DomainParameters".into()],
                    global_module_reference: GlobalModuleReference {
                        module_reference: "ANSI-X9-42".into(),
                        assigned_identifier: AssignedIdentifier::ObjectIdentifierValue(
                            ObjectIdentifierValue(vec![
                                ObjectIdentifierArc {
                                    name: Some("iso".into()),
                                    number: Some(1)
                                },
                                ObjectIdentifierArc {
                                    name: Some("member-body".into()),
                                    number: Some(2)
                                },
                                ObjectIdentifierArc {
                                    name: Some("us".into()),
                                    number: Some(840)
                                },
                                ObjectIdentifierArc {
                                    name: Some("ansi-x942".into()),
                                    number: Some(10046)
                                },
                                ObjectIdentifierArc {
                                    name: Some("module".into()),
                                    number: Some(5)
                                },
                                ObjectIdentifierArc {
                                    name: None,
                                    number: Some(1)
                                },
                            ])
                        )
                    },
                    with: None
                },
                Import {
                    types: vec!["ECDomainParameters".into()],
                    global_module_reference: GlobalModuleReference {
                        module_reference: "ANSI-X9-62".into(),
                        assigned_identifier: AssignedIdentifier::ObjectIdentifierValue(
                            ObjectIdentifierValue(vec![
                                ObjectIdentifierArc {
                                    name: Some("iso".into()),
                                    number: Some(1)
                                },
                                ObjectIdentifierArc {
                                    name: Some("member-body".into()),
                                    number: Some(2)
                                },
                                ObjectIdentifierArc {
                                    name: Some("us".into()),
                                    number: Some(840)
                                },
                                ObjectIdentifierArc {
                                    name: None,
                                    number: Some(10045)
                                },
                                ObjectIdentifierArc {
                                    name: Some("modules".into()),
                                    number: Some(0)
                                },
                                ObjectIdentifierArc {
                                    name: None,
                                    number: Some(2)
                                },
                            ])
                        )
                    },
                    with: None
                }
            ]
        )
    }

    #[test]
    fn global_module_reference_empty_assigned_identifier() {
        assert_eq!(
            global_module_reference(r#" EMPTY-assigned-ID next-module-import, "#.into()).unwrap(),
            (
                Input::from(" next-module-import, ").with_line_column_and_offset(1, 19, 18),
                GlobalModuleReference {
                    module_reference: "EMPTY-assigned-ID".to_owned(),
                    assigned_identifier: AssignedIdentifier::Empty
                }
            )
        )
    }

    #[test]
    fn global_module_reference_val_ref_assigned_identifier() {
        assert_eq!(
            global_module_reference(r#" VALref-assigned-ID valref next-module-import,"#.into())
                .unwrap()
                .1,
            GlobalModuleReference {
                module_reference: "VALref-assigned-ID".to_owned(),
                assigned_identifier: AssignedIdentifier::ValueReference("valref".to_owned())
            }
        )
    }

    #[test]
    fn global_module_reference_ext_val_ref_assigned_identifier() {
        assert_eq!(
            global_module_reference(
                r#" ext-VALref-assigned-ID MODULE-ref.valref next-module-import,"#.into()
            )
            .unwrap()
            .1,
            GlobalModuleReference {
                module_reference: "ext-VALref-assigned-ID".to_owned(),
                assigned_identifier: AssignedIdentifier::ExternalValueReference(
                    ExternalValueReference {
                        module_reference: "MODULE-ref".to_owned(),
                        value_reference: "valref".to_owned()
                    }
                )
            }
        )
    }

    #[test]
    fn issue_4_imports() {
        assert_eq!(
            imports(
                r#"IMPORTS

            Criticality,
            Presence,
            PrivateIE-ID,
            ProtocolExtensionID,
            ProtocolIE-ID
        FROM NGAP-CommonDataTypes

            maxPrivateIEs,
            maxProtocolExtensions,
            maxProtocolIEs
        FROM NGAP-Constants;"#
                    .into()
            )
            .unwrap()
            .1,
            vec![
                Import {
                    types: vec![
                        "Criticality".to_owned(),
                        "Presence".to_owned(),
                        "PrivateIE-ID".to_owned(),
                        "ProtocolExtensionID".to_owned(),
                        "ProtocolIE-ID".to_owned(),
                    ],
                    global_module_reference: GlobalModuleReference {
                        module_reference: "NGAP-CommonDataTypes".to_owned(),
                        assigned_identifier: AssignedIdentifier::Empty
                    },
                    with: None
                },
                Import {
                    types: vec![
                        "maxPrivateIEs".to_owned(),
                        "maxProtocolExtensions".to_owned(),
                        "maxProtocolIEs".to_owned()
                    ],
                    global_module_reference: GlobalModuleReference {
                        module_reference: "NGAP-Constants".to_owned(),
                        assigned_identifier: AssignedIdentifier::Empty
                    },
                    with: None
                }
            ]
        );
    }
}
