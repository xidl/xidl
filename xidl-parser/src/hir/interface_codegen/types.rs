use super::*;

pub fn idl_type_spec(ty: &TypeSpec) -> String {
    match ty {
        TypeSpec::SimpleTypeSpec(simple) => match simple {
            SimpleTypeSpec::IntegerType(value) => idl_integer_type(value),
            SimpleTypeSpec::FloatingPtType => "double".to_string(),
            SimpleTypeSpec::CharType => "char".to_string(),
            SimpleTypeSpec::WideCharType => "wchar".to_string(),
            SimpleTypeSpec::Boolean => "boolean".to_string(),
            SimpleTypeSpec::AnyType => "any".to_string(),
            SimpleTypeSpec::ObjectType => "Object".to_string(),
            SimpleTypeSpec::ValueBaseType => "ValueBase".to_string(),
            SimpleTypeSpec::ScopedName(value) => scoped_name_to_idl(value),
        },
        TypeSpec::TemplateTypeSpec(template) => match template {
            TemplateTypeSpec::SequenceType(seq) => {
                let elem = idl_type_spec(&seq.ty);
                match &seq.len {
                    Some(len) => format!("sequence<{}, {}>", elem, render_const_expr(&len.0)),
                    None => format!("sequence<{}>", elem),
                }
            }
            TemplateTypeSpec::StringType(string) => match &string.bound {
                Some(bound) => format!("string<{}>", render_const_expr(&bound.0)),
                None => "string".to_string(),
            },
            TemplateTypeSpec::WideStringType(string) => match &string.bound {
                Some(bound) => format!("wstring<{}>", render_const_expr(&bound.0)),
                None => "wstring".to_string(),
            },
            TemplateTypeSpec::FixedPtType(fixed) => format!(
                "fixed<{}, {}>",
                render_const_expr(&fixed.integer.0),
                render_const_expr(&fixed.fraction.0)
            ),
            TemplateTypeSpec::MapType(map) => {
                let key = idl_type_spec(&map.key);
                let value = idl_type_spec(&map.value);
                match &map.len {
                    Some(len) => format!("map<{}, {}, {}>", key, value, render_const_expr(&len.0)),
                    None => format!("map<{}, {}>", key, value),
                }
            }
            TemplateTypeSpec::TemplateType(template) => {
                let args = template
                    .args
                    .iter()
                    .map(idl_type_spec)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}<{}>", template.ident, args)
            }
        },
    }
}

pub fn idl_integer_type(value: &IntegerType) -> String {
    match value {
        IntegerType::Char => "int8".to_string(),
        IntegerType::UChar => "uint8".to_string(),
        IntegerType::U8 => "uint8".to_string(),
        IntegerType::U16 => "unsigned short".to_string(),
        IntegerType::U32 => "unsigned long".to_string(),
        IntegerType::U64 => "unsigned long long".to_string(),
        IntegerType::I8 => "int8".to_string(),
        IntegerType::I16 => "short".to_string(),
        IntegerType::I32 => "long".to_string(),
        IntegerType::I64 => "long long".to_string(),
    }
}

pub fn scoped_name_to_idl(name: &ScopedName) -> String {
    let mut out = String::new();
    if name.is_root {
        out.push_str("::");
    }
    out.push_str(&name.name.join("::"));
    out
}

pub fn qualified_exception_name(name: &ScopedName, modules: &[String]) -> String {
    if name.is_root {
        return name.name.join("::");
    }
    if modules.is_empty() {
        return name.name.join("::");
    }
    let mut all = modules.to_vec();
    all.extend(name.name.iter().cloned());
    all.join("::")
}

pub fn render_const_expr(expr: &ConstExpr) -> String {
    fn render_or(expr: &OrExpr) -> String {
        match expr {
            OrExpr::XorExpr(value) => render_xor(value),
            OrExpr::OrExpr(left, right) => {
                format!("({} | {})", render_or(left), render_xor(right))
            }
        }
    }

    fn render_xor(expr: &XorExpr) -> String {
        match expr {
            XorExpr::AndExpr(value) => render_and(value),
            XorExpr::XorExpr(left, right) => {
                format!("({} ^ {})", render_xor(left), render_and(right))
            }
        }
    }

    fn render_and(expr: &AndExpr) -> String {
        match expr {
            AndExpr::ShiftExpr(value) => render_shift(value),
            AndExpr::AndExpr(left, right) => {
                format!("({} & {})", render_and(left), render_shift(right))
            }
        }
    }

    fn render_shift(expr: &ShiftExpr) -> String {
        match expr {
            ShiftExpr::AddExpr(value) => render_add(value),
            ShiftExpr::LeftShiftExpr(left, right) => {
                format!("({} << {})", render_shift(left), render_add(right))
            }
            ShiftExpr::RightShiftExpr(left, right) => {
                format!("({} >> {})", render_shift(left), render_add(right))
            }
        }
    }

    fn render_add(expr: &AddExpr) -> String {
        match expr {
            AddExpr::MultExpr(value) => render_mult(value),
            AddExpr::AddExpr(left, right) => {
                format!("({} + {})", render_add(left), render_mult(right))
            }
            AddExpr::SubExpr(left, right) => {
                format!("({} - {})", render_add(left), render_mult(right))
            }
        }
    }

    fn render_mult(expr: &MultExpr) -> String {
        match expr {
            MultExpr::UnaryExpr(value) => render_unary(value),
            MultExpr::MultExpr(left, right) => {
                format!("({} * {})", render_mult(left), render_unary(right))
            }
            MultExpr::DivExpr(left, right) => {
                format!("({} / {})", render_mult(left), render_unary(right))
            }
            MultExpr::ModExpr(left, right) => {
                format!("({} % {})", render_mult(left), render_unary(right))
            }
        }
    }

    fn render_unary(expr: &UnaryExpr) -> String {
        match expr {
            UnaryExpr::UnaryExpr(op, expr) => {
                let op = match op {
                    UnaryOperator::Add => "+",
                    UnaryOperator::Sub => "-",
                    UnaryOperator::Not => "~",
                };
                format!("({}{})", op, render_primary(expr))
            }
            UnaryExpr::PrimaryExpr(value) => render_primary(value),
        }
    }

    fn render_primary(expr: &PrimaryExpr) -> String {
        match expr {
            PrimaryExpr::ScopedName(value) => scoped_name_to_idl(value),
            PrimaryExpr::Literal(value) => render_literal(value),
            PrimaryExpr::ConstExpr(value) => render_const_expr(value),
        }
    }

    fn render_literal(value: &Literal) -> String {
        match value {
            Literal::IntegerLiteral(lit) => match lit {
                IntegerLiteral::BinNumber(value) => value.clone(),
                IntegerLiteral::OctNumber(value) => value.clone(),
                IntegerLiteral::DecNumber(value) => value.clone(),
                IntegerLiteral::HexNumber(value) => value.clone(),
            },
            Literal::FloatingPtLiteral(lit) => {
                let sign = lit
                    .sign
                    .as_ref()
                    .map(|value| value.0.as_str())
                    .unwrap_or("");
                format!("{}{}.{}", sign, lit.integer.0, lit.fraction.0)
            }
            Literal::CharLiteral(value) => value.clone(),
            Literal::WideCharacterLiteral(value) => value.clone(),
            Literal::StringLiteral(value) => value.clone(),
            Literal::WideStringLiteral(value) => value.clone(),
            Literal::BooleanLiteral(value) => value.to_ascii_lowercase(),
        }
    }

    render_or(&expr.0)
}
