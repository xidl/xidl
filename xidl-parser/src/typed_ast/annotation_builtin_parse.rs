use super::{
    AutoIdKind, BuiltinAnnotation, DataRepresentationKind, ExtensibilityKind, PlacementKind,
    ServicePlatform, TopicPlatform, TryConstructFailAction, VerbatimLanguage,
};
use crate::parser::FromTreeSitter;
use crate::typed_ast::PositiveIntConst;

pub(crate) fn parse_builtin_annotation<'a>(
    node: tree_sitter::Node<'a>,
    ctx: &mut crate::parser::ParseContext<'a>,
) -> crate::error::ParserResult<BuiltinAnnotation> {
    let concrete = node
        .named_children(&mut node.walk())
        .next()
        .ok_or_else(|| {
            crate::error::ParseError::UnexpectedNode("missing builtin annotation".to_string())
        })?;
    match concrete.kind_id() {
        xidl_parser_derive::node_id!("annotation_appl_id") => Ok(BuiltinAnnotation::Id {
            value: first_child_of_kind(
                concrete,
                xidl_parser_derive::node_id!("integer_literal"),
                ctx,
            )?,
        }),
        xidl_parser_derive::node_id!("annotation_appl_autoid") => Ok(BuiltinAnnotation::AutoId {
            value: optional_child_of_kind(
                concrete,
                xidl_parser_derive::node_id!("autoid_kind"),
                ctx,
            )?,
        }),
        xidl_parser_derive::node_id!("annotation_appl_optional") => {
            Ok(BuiltinAnnotation::Optional {
                value: optional_child_of_kind(
                    concrete,
                    xidl_parser_derive::node_id!("positive_int_const"),
                    ctx,
                )?,
            })
        }
        xidl_parser_derive::node_id!("annotation_appl_position") => {
            Ok(BuiltinAnnotation::Position {
                value: first_child_of_kind(
                    concrete,
                    xidl_parser_derive::node_id!("positive_int_const"),
                    ctx,
                )?,
            })
        }
        xidl_parser_derive::node_id!("annotation_appl_value") => Ok(BuiltinAnnotation::Value {
            value: first_child_of_kind(concrete, xidl_parser_derive::node_id!("const_expr"), ctx)?,
        }),
        xidl_parser_derive::node_id!("annotation_appl_extensibility") => {
            Ok(BuiltinAnnotation::Extensibility {
                kind: first_child_of_kind(
                    concrete,
                    xidl_parser_derive::node_id!("extensibility_kind"),
                    ctx,
                )?,
            })
        }
        xidl_parser_derive::node_id!("annotation_appl_final") => Ok(BuiltinAnnotation::Final),
        xidl_parser_derive::node_id!("annotation_appl_appendable") => {
            Ok(BuiltinAnnotation::Appendable)
        }
        xidl_parser_derive::node_id!("annotation_appl_mutable") => Ok(BuiltinAnnotation::Mutable),
        xidl_parser_derive::node_id!("annotation_appl_key") => Ok(BuiltinAnnotation::Key {
            value: optional_child_of_kind(
                concrete,
                xidl_parser_derive::node_id!("positive_int_const"),
                ctx,
            )?,
        }),
        xidl_parser_derive::node_id!("annotation_appl_must_understand") => {
            Ok(BuiltinAnnotation::MustUnderstand {
                value: optional_child_of_kind(
                    concrete,
                    xidl_parser_derive::node_id!("positive_int_const"),
                    ctx,
                )?,
            })
        }
        xidl_parser_derive::node_id!("annotation_appl_default_literal") => {
            Ok(BuiltinAnnotation::DefaultLiteral)
        }
        xidl_parser_derive::node_id!("annotation_appl_default") => Ok(BuiltinAnnotation::Default {
            value: first_child_of_kind(concrete, xidl_parser_derive::node_id!("const_expr"), ctx)?,
        }),
        xidl_parser_derive::node_id!("annotation_appl_range") => Ok(BuiltinAnnotation::Range {
            min: nested_positive_int(concrete, xidl_parser_derive::node_id!("min_expr"), ctx)?,
            max: nested_positive_int(concrete, xidl_parser_derive::node_id!("max_expr"), ctx)?,
        }),
        xidl_parser_derive::node_id!("annotation_appl_min") => Ok(BuiltinAnnotation::Min {
            value: first_child_of_kind(
                concrete,
                xidl_parser_derive::node_id!("positive_int_const"),
                ctx,
            )?,
        }),
        xidl_parser_derive::node_id!("annotation_appl_max") => Ok(BuiltinAnnotation::Max {
            value: first_child_of_kind(
                concrete,
                xidl_parser_derive::node_id!("positive_int_const"),
                ctx,
            )?,
        }),
        xidl_parser_derive::node_id!("annotation_appl_unit") => Ok(BuiltinAnnotation::Unit {
            value: first_child_of_kind(concrete, xidl_parser_derive::node_id!("const_expr"), ctx)?,
        }),
        xidl_parser_derive::node_id!("annotation_appl_bit_bound") => {
            Ok(BuiltinAnnotation::BitBound {
                value: first_child_of_kind(
                    concrete,
                    xidl_parser_derive::node_id!("positive_int_const"),
                    ctx,
                )?,
            })
        }
        xidl_parser_derive::node_id!("annotation_appl_external") => {
            Ok(BuiltinAnnotation::External {
                value: optional_child_of_kind(
                    concrete,
                    xidl_parser_derive::node_id!("positive_int_const"),
                    ctx,
                )?,
            })
        }
        xidl_parser_derive::node_id!("annotation_appl_nested") => Ok(BuiltinAnnotation::Nested {
            value: optional_child_of_kind(
                concrete,
                xidl_parser_derive::node_id!("positive_int_const"),
                ctx,
            )?,
        }),
        xidl_parser_derive::node_id!("annotation_appl_verbatim") => {
            Ok(BuiltinAnnotation::Verbatim {
                language: field_child("language", concrete, ctx)?,
                placement: field_child("placement", concrete, ctx)?,
                text: field_child_required("text", concrete, ctx)?,
            })
        }
        xidl_parser_derive::node_id!("annotation_appl_service") => Ok(BuiltinAnnotation::Service {
            platform: optional_child_of_kind(
                concrete,
                xidl_parser_derive::node_id!("service_platform"),
                ctx,
            )?,
        }),
        xidl_parser_derive::node_id!("annotation_appl_oneway") => Ok(BuiltinAnnotation::Oneway {
            value: optional_child_of_kind(
                concrete,
                xidl_parser_derive::node_id!("const_expr"),
                ctx,
            )?,
        }),
        xidl_parser_derive::node_id!("annotation_appl_ami") => Ok(BuiltinAnnotation::Ami {
            value: optional_child_of_kind(
                concrete,
                xidl_parser_derive::node_id!("const_expr"),
                ctx,
            )?,
        }),
        xidl_parser_derive::node_id!("annotation_appl_hashid") => Ok(BuiltinAnnotation::HashId {
            value: optional_child_of_kind(
                concrete,
                xidl_parser_derive::node_id!("const_expr"),
                ctx,
            )?,
        }),
        xidl_parser_derive::node_id!("annotation_appl_default_nested") => {
            Ok(BuiltinAnnotation::DefaultNested {
                value: optional_child_of_kind(
                    concrete,
                    xidl_parser_derive::node_id!("const_expr"),
                    ctx,
                )?,
            })
        }
        xidl_parser_derive::node_id!("annotation_appl_ignore_literal_names") => {
            Ok(BuiltinAnnotation::IgnoreLiteralNames {
                value: optional_child_of_kind(
                    concrete,
                    xidl_parser_derive::node_id!("const_expr"),
                    ctx,
                )?,
            })
        }
        xidl_parser_derive::node_id!("annotation_appl_try_construct") => {
            Ok(BuiltinAnnotation::TryConstruct {
                value: optional_child_of_kind(
                    concrete,
                    xidl_parser_derive::node_id!("try_construct_fail_action"),
                    ctx,
                )?,
            })
        }
        xidl_parser_derive::node_id!("annotation_appl_non_serialized") => {
            Ok(BuiltinAnnotation::NonSerialized {
                value: optional_child_of_kind(
                    concrete,
                    xidl_parser_derive::node_id!("boolean_literal"),
                    ctx,
                )?,
            })
        }
        xidl_parser_derive::node_id!("annotation_appl_data_representation") => {
            Ok(BuiltinAnnotation::DataRepresentation {
                kinds: children_of_kind(
                    concrete,
                    xidl_parser_derive::node_id!("data_representation_mask"),
                    ctx,
                )?,
            })
        }
        xidl_parser_derive::node_id!("annotation_appl_topic") => Ok(BuiltinAnnotation::Topic {
            name: field_child("name", concrete, ctx)?,
            platform: field_child("platform", concrete, ctx)?,
        }),
        xidl_parser_derive::node_id!("annotation_appl_choice") => Ok(BuiltinAnnotation::Choice),
        xidl_parser_derive::node_id!("annotation_appl_empty") => Ok(BuiltinAnnotation::Empty),
        xidl_parser_derive::node_id!("annotation_appl_dds_service") => {
            Ok(BuiltinAnnotation::DdsService)
        }
        xidl_parser_derive::node_id!("annotation_appl_dds_request_topic") => {
            Ok(BuiltinAnnotation::DdsRequestTopic {
                name: first_child_of_kind(
                    concrete,
                    xidl_parser_derive::node_id!("const_expr"),
                    ctx,
                )?,
            })
        }
        xidl_parser_derive::node_id!("annotation_appl_dds_reply_topic") => {
            Ok(BuiltinAnnotation::DdsReplyTopic {
                name: first_child_of_kind(
                    concrete,
                    xidl_parser_derive::node_id!("const_expr"),
                    ctx,
                )?,
            })
        }
        _ => Err(crate::error::ParseError::UnexpectedNode(format!(
            "unsupported builtin annotation node: {}",
            concrete.kind()
        ))),
    }
}

fn first_child_of_kind<'a, T: FromTreeSitter<'a>>(
    node: tree_sitter::Node<'a>,
    kind_id: u16,
    ctx: &mut crate::parser::ParseContext<'a>,
) -> crate::error::ParserResult<T> {
    let child = node
        .children(&mut node.walk())
        .find(|child| child.kind_id() == kind_id)
        .ok_or_else(|| {
            crate::error::ParseError::UnexpectedNode(format!("missing child in {}", node.kind()))
        })?;
    T::from_node(child, ctx)
}

fn optional_child_of_kind<'a, T: FromTreeSitter<'a>>(
    node: tree_sitter::Node<'a>,
    kind_id: u16,
    ctx: &mut crate::parser::ParseContext<'a>,
) -> crate::error::ParserResult<Option<T>> {
    node.children(&mut node.walk())
        .find(|child| child.kind_id() == kind_id)
        .map(|child| T::from_node(child, ctx))
        .transpose()
}

fn children_of_kind<'a, T: FromTreeSitter<'a>>(
    node: tree_sitter::Node<'a>,
    kind_id: u16,
    ctx: &mut crate::parser::ParseContext<'a>,
) -> crate::error::ParserResult<Vec<T>> {
    node.children(&mut node.walk())
        .filter(|child| child.kind_id() == kind_id)
        .map(|child| T::from_node(child, ctx))
        .collect()
}

fn field_child<'a, T: FromTreeSitter<'a>>(
    field: &str,
    node: tree_sitter::Node<'a>,
    ctx: &mut crate::parser::ParseContext<'a>,
) -> crate::error::ParserResult<Option<T>> {
    node.child_by_field_name(field)
        .map(|child| T::from_node(child, ctx))
        .transpose()
}

fn field_child_required<'a, T: FromTreeSitter<'a>>(
    field: &str,
    node: tree_sitter::Node<'a>,
    ctx: &mut crate::parser::ParseContext<'a>,
) -> crate::error::ParserResult<T> {
    let child = node.child_by_field_name(field).ok_or_else(|| {
        crate::error::ParseError::UnexpectedNode(format!(
            "missing field {field} in {}",
            node.kind()
        ))
    })?;
    T::from_node(child, ctx)
}

fn nested_positive_int<'a>(
    node: tree_sitter::Node<'a>,
    kind_id: u16,
    ctx: &mut crate::parser::ParseContext<'a>,
) -> crate::error::ParserResult<PositiveIntConst> {
    let outer = node
        .children(&mut node.walk())
        .find(|child| child.kind_id() == kind_id)
        .ok_or_else(|| {
            crate::error::ParseError::UnexpectedNode(format!(
                "missing nested child in {}",
                node.kind()
            ))
        })?;
    first_child_of_kind(
        outer,
        xidl_parser_derive::node_id!("positive_int_const"),
        ctx,
    )
}

macro_rules! parse_enum {
    ($ty:ty, $node:ident, $ctx:ident, { $($text:literal => $value:expr),+ $(,)? }) => {{
        match $ctx.node_text(&$node)?.to_ascii_uppercase().as_str() {
            $($text => Ok($value),)+
            value => Err(crate::error::ParseError::UnexpectedNode(format!(
                "parent: {}, got: {}",
                $node.kind(),
                value
            ))),
        }
    }};
}

impl<'a> FromTreeSitter<'a> for AutoIdKind {
    fn from_node(
        node: tree_sitter::Node<'a>,
        ctx: &mut crate::parser::ParseContext<'a>,
    ) -> crate::error::ParserResult<Self> {
        parse_enum!(Self, node, ctx, { "SEQUENTIAL" => Self::Sequential, "HASH" => Self::Hash })
    }
}

impl<'a> FromTreeSitter<'a> for ExtensibilityKind {
    fn from_node(
        node: tree_sitter::Node<'a>,
        ctx: &mut crate::parser::ParseContext<'a>,
    ) -> crate::error::ParserResult<Self> {
        parse_enum!(Self, node, ctx, { "FINAL" => Self::Final, "APPENDABLE" => Self::Appendable, "MUTABLE" => Self::Mutable })
    }
}

impl<'a> FromTreeSitter<'a> for VerbatimLanguage {
    fn from_node(
        node: tree_sitter::Node<'a>,
        ctx: &mut crate::parser::ParseContext<'a>,
    ) -> crate::error::ParserResult<Self> {
        match ctx.node_text(&node)?.to_ascii_lowercase().as_str() {
            "c" => Ok(Self::C),
            "c++" => Ok(Self::Cpp),
            "java" => Ok(Self::Java),
            "idl" => Ok(Self::Idl),
            "*" => Ok(Self::Any),
            value => Err(crate::error::ParseError::UnexpectedNode(format!(
                "parent: {}, got: {}",
                node.kind(),
                value
            ))),
        }
    }
}

impl<'a> FromTreeSitter<'a> for PlacementKind {
    fn from_node(
        node: tree_sitter::Node<'a>,
        ctx: &mut crate::parser::ParseContext<'a>,
    ) -> crate::error::ParserResult<Self> {
        parse_enum!(Self, node, ctx, {
            "BEGIN_FILE" => Self::BeginFile,
            "BEFORE_DECLARATION" => Self::BeforeDeclaration,
            "BEGIN_DECLARATION" => Self::BeginDeclaration,
            "END_DECLARATION" => Self::EndDeclaration,
            "AFTER_DECLARATION" => Self::AfterDeclaration,
            "END_FILE" => Self::EndFile
        })
    }
}

impl<'a> FromTreeSitter<'a> for ServicePlatform {
    fn from_node(
        node: tree_sitter::Node<'a>,
        ctx: &mut crate::parser::ParseContext<'a>,
    ) -> crate::error::ParserResult<Self> {
        parse_enum!(Self, node, ctx, { "CORBA" => Self::Corba, "DDS" => Self::Dds, "*" => Self::Any })
    }
}

impl<'a> FromTreeSitter<'a> for TryConstructFailAction {
    fn from_node(
        node: tree_sitter::Node<'a>,
        ctx: &mut crate::parser::ParseContext<'a>,
    ) -> crate::error::ParserResult<Self> {
        parse_enum!(Self, node, ctx, { "DISCARD" => Self::Discard, "USE_DEFAULT" => Self::UseDefault, "TRIM" => Self::Trim })
    }
}

impl<'a> FromTreeSitter<'a> for DataRepresentationKind {
    fn from_node(
        node: tree_sitter::Node<'a>,
        ctx: &mut crate::parser::ParseContext<'a>,
    ) -> crate::error::ParserResult<Self> {
        parse_enum!(Self, node, ctx, { "XCDR" => Self::Xcdr, "XCDR1" => Self::Xcdr1, "XML" => Self::Xml, "XCDR2" => Self::Xcdr2 })
    }
}

impl<'a> FromTreeSitter<'a> for TopicPlatform {
    fn from_node(
        node: tree_sitter::Node<'a>,
        ctx: &mut crate::parser::ParseContext<'a>,
    ) -> crate::error::ParserResult<Self> {
        parse_enum!(Self, node, ctx, { "DDS" => Self::Dds, "*" => Self::Any })
    }
}
