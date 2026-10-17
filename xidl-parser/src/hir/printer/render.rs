use super::super::*;
use super::IdlPrinter;

impl IdlPrinter {
    pub(super) fn is_doc_annotation(annotation: &Annotation) -> bool {
        matches!(annotation, Annotation::Builtin { name, .. } if name.eq_ignore_ascii_case("doc"))
    }

    pub(super) fn value_annotations(value: &ConstrTypeDcl) -> &[Annotation] {
        match value {
            ConstrTypeDcl::StructForwardDcl(value) => &value.annotations,
            ConstrTypeDcl::StructDcl(value) => &value.annotations,
            ConstrTypeDcl::EnumDcl(value) => &value.annotations,
            ConstrTypeDcl::UnionForwardDcl(value) => &value.annotations,
            ConstrTypeDcl::UnionDef(value) => &value.annotations,
            ConstrTypeDcl::BitsetDcl(value) => &value.annotations,
            ConstrTypeDcl::BitmaskDcl(value) => &value.annotations,
        }
    }

    pub(super) fn render_type(value: &TypeSpec) -> String {
        match value {
            TypeSpec::IntegerType(value) => Self::render_integer_type(value),
            TypeSpec::FloatingPtType => "double".to_string(),
            TypeSpec::CharType => "char".to_string(),
            TypeSpec::WideCharType => "wchar".to_string(),
            TypeSpec::Boolean => "boolean".to_string(),
            TypeSpec::AnyType => "any".to_string(),
            TypeSpec::ObjectType => "Object".to_string(),
            TypeSpec::ValueBaseType => "ValueBase".to_string(),
            TypeSpec::ScopedName(value) => Self::render_scoped(value),
            TypeSpec::SequenceType(value) => format!(
                "sequence<{}{}>",
                Self::render_type(&value.ty),
                value
                    .len
                    .as_ref()
                    .map(|len| format!(", {}", Self::render_expr(&len.0)))
                    .unwrap_or_default()
            ),
            TypeSpec::StringType(value) => {
                Self::render_bounded_type("string", value.bound.as_ref())
            }
            TypeSpec::WideStringType(value) => {
                Self::render_bounded_type("wstring", value.bound.as_ref())
            }
            TypeSpec::FixedPtType(value) => format!(
                "fixed<{}, {}>",
                Self::render_expr(&value.integer.0),
                Self::render_expr(&value.fraction.0)
            ),
            TypeSpec::MapType(value) => format!(
                "map<{}, {}{}>",
                Self::render_type(&value.key),
                Self::render_type(&value.value),
                value
                    .len
                    .as_ref()
                    .map(|len| format!(", {}", Self::render_expr(&len.0)))
                    .unwrap_or_default()
            ),
            TypeSpec::TemplateType(value) => format!(
                "{}<{}>",
                value.ident,
                value
                    .args
                    .iter()
                    .map(Self::render_type)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }

    pub(super) fn render_bounded_type(name: &str, bound: Option<&PositiveIntConst>) -> String {
        match bound {
            Some(bound) => format!("{name}<{}>", Self::render_expr(&bound.0)),
            None => name.to_string(),
        }
    }

    pub(super) fn render_const_type(value: &ConstType) -> String {
        match value {
            ConstType::IntegerType(value) => Self::render_integer_type(value),
            ConstType::FloatingPtType => "double".to_string(),
            ConstType::FixedPtConstType => "fixed".to_string(),
            ConstType::CharType => "char".to_string(),
            ConstType::WideCharType => "wchar".to_string(),
            ConstType::BooleanType => "boolean".to_string(),
            ConstType::OctetType => "octet".to_string(),
            ConstType::StringType(value) => {
                Self::render_bounded_type("string", value.bound.as_ref())
            }
            ConstType::WideStringType(value) => {
                Self::render_bounded_type("wstring", value.bound.as_ref())
            }
            ConstType::ScopedName(value) => Self::render_scoped(value),
            ConstType::SequenceType(value) => {
                Self::render_type(&TypeSpec::SequenceType(value.clone()))
            }
        }
    }

    pub(super) fn render_integer_type(value: &IntegerType) -> String {
        match value {
            IntegerType::Char => "char",
            IntegerType::UChar | IntegerType::U8 => "uint8",
            IntegerType::Octet => "octet",
            IntegerType::U16 => "unsigned short",
            IntegerType::U32 => "unsigned long",
            IntegerType::U64 => "unsigned long long",
            IntegerType::I8 => "int8",
            IntegerType::I16 => "short",
            IntegerType::I32 => "long",
            IntegerType::I64 => "long long",
        }
        .to_string()
    }

    pub(super) fn render_switch_type(value: &SwitchTypeSpec) -> String {
        match value {
            SwitchTypeSpec::IntegerType(value) => Self::render_integer_type(value),
            SwitchTypeSpec::CharType => "char".to_string(),
            SwitchTypeSpec::WideCharType => "wchar".to_string(),
            SwitchTypeSpec::BooleanType => "boolean".to_string(),
            SwitchTypeSpec::ScopedName(value) => Self::render_scoped(value),
            SwitchTypeSpec::OctetType => "octet".to_string(),
        }
    }

    pub(super) fn render_bitfield_type(value: &BitFieldType) -> &'static str {
        match value {
            BitFieldType::Bool => "boolean",
            BitFieldType::Octec => "octet",
            BitFieldType::SignedInt => "short",
            BitFieldType::UnsignedInt => "unsigned short",
        }
    }

    pub(super) fn render_declarator(value: &Declarator) -> String {
        match value {
            Declarator::SimpleDeclarator(value) => value.0.clone(),
            Declarator::ArrayDeclarator(value) => format!(
                "{}{}",
                value.ident,
                value
                    .len
                    .iter()
                    .map(|len| format!("[{}]", Self::render_expr(&len.0)))
                    .collect::<String>()
            ),
        }
    }

    pub(super) fn render_readonly_declarator(value: &ReadonlyAttrDeclarator) -> String {
        match value {
            ReadonlyAttrDeclarator::SimpleDeclarator(values) => format!(
                " {}",
                values
                    .iter()
                    .map(|value| value.0.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            ReadonlyAttrDeclarator::WithRaises { declarator, raises } => {
                format!(
                    " {} raises({})",
                    declarator.0,
                    Self::render_exception_names(&raises.0)
                )
            }
        }
    }

    pub(super) fn render_attr_declarator(value: &AttrDeclarator) -> String {
        match value {
            AttrDeclarator::SimpleDeclarator(values) => format!(
                " {}",
                values
                    .iter()
                    .map(|value| value.0.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            AttrDeclarator::WithRaises { declarator, raises } => match raises {
                AttrRaisesExpr::Case1(get, set) => {
                    let set = set
                        .as_ref()
                        .map(|set| {
                            format!(" setraises({})", Self::render_exception_names(&set.expr.0))
                        })
                        .unwrap_or_default();
                    format!(
                        " {} getraises({}){}",
                        declarator.0,
                        Self::render_exception_names(&get.expr.0),
                        set
                    )
                }
                AttrRaisesExpr::SetExcepExpr(set) => {
                    format!(
                        " {} setraises({})",
                        declarator.0,
                        Self::render_exception_names(&set.expr.0)
                    )
                }
            },
        }
    }

    pub(super) fn render_exception_names(values: &[ScopedName]) -> String {
        values
            .iter()
            .map(Self::render_scoped)
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub(super) fn render_scoped(value: &ScopedName) -> String {
        let prefix = if value.is_root { "::" } else { "" };
        format!("{}{}", prefix, value.name.join("::"))
    }

    pub(super) fn render_expr(value: &ConstExpr) -> String {
        match value {
            ConstExpr::ScopedName(value) => Self::render_scoped(value),
            ConstExpr::Literal(value) => Self::render_literal(value),
            ConstExpr::UnaryExpr(operator, value) => {
                let operand = Self::render_expr(value);
                let operand = if matches!(value.as_ref(), ConstExpr::UnaryExpr(_, _)) {
                    format!("({operand})")
                } else {
                    operand
                };
                format!("{}{operand}", Self::render_unary_operator(operator))
            }
            ConstExpr::BinaryExpr(operator, left, right) => format!(
                "({} {} {})",
                Self::render_expr(left),
                Self::render_binary_operator(operator),
                Self::render_expr(right)
            ),
        }
    }

    pub(super) fn render_literal(value: &Literal) -> String {
        match value {
            Literal::IntegerLiteral(value) => value.0.clone(),
            Literal::FloatingPtLiteral(value) => format!(
                "{}{}.{}",
                value.sign.as_ref().map(|sign| sign.as_str()).unwrap_or(""),
                value.integer.0,
                value.fraction.0
            ),
            Literal::CharLiteral(value) => Self::quoted_literal(value, '\'', false),
            Literal::WideCharacterLiteral(value) => Self::quoted_literal(value, '\'', true),
            Literal::StringLiteral(value) => Self::quoted_literal(value, '"', false),
            Literal::WideStringLiteral(value) => Self::quoted_literal(value, '"', true),
            Literal::BooleanLiteral(value) => value.to_string(),
        }
    }

    pub(super) fn quoted_literal(value: &str, quote_char: char, wide: bool) -> String {
        if wide
            && value.starts_with('L')
            && value.chars().nth(1) == Some(quote_char)
            && value.ends_with(quote_char)
        {
            return value.to_string();
        }
        let value = if value.starts_with(quote_char) && value.ends_with(quote_char) {
            value.to_string()
        } else {
            format!(
                "{quote_char}{}{quote_char}",
                value.replace(quote_char, &format!("\\{quote_char}"))
            )
        };
        if wide { format!("L{value}") } else { value }
    }

    pub(super) fn render_unary_operator(value: &UnaryOperator) -> &'static str {
        match value {
            UnaryOperator::Add => "+",
            UnaryOperator::Sub => "-",
            UnaryOperator::Not => "~",
        }
    }

    pub(super) fn render_binary_operator(value: &BinaryOperator) -> &'static str {
        match value {
            BinaryOperator::Or => "|",
            BinaryOperator::Xor => "^",
            BinaryOperator::And => "&",
            BinaryOperator::LeftShift => "<<",
            BinaryOperator::RightShift => ">>",
            BinaryOperator::Add => "+",
            BinaryOperator::Sub => "-",
            BinaryOperator::Mult => "*",
            BinaryOperator::Div => "/",
            BinaryOperator::Mod => "%",
        }
    }

    pub(super) fn quote(value: &str) -> String {
        // HIR stores lexical inner text, including any existing escapes.
        format!("\"{value}\"")
    }
}
