use super::*;

pub fn idl_type_spec(ty: &TypeSpec) -> String {
    match ty {
        TypeSpec::IntegerType(value) => idl_integer_type(value),
        TypeSpec::FloatingPtType => "double".to_string(),
        TypeSpec::CharType => "char".to_string(),
        TypeSpec::WideCharType => "wchar".to_string(),
        TypeSpec::Boolean => "boolean".to_string(),
        TypeSpec::AnyType => "any".to_string(),
        TypeSpec::ObjectType => "Object".to_string(),
        TypeSpec::ValueBaseType => "ValueBase".to_string(),
        TypeSpec::ScopedName(value) => scoped_name_to_idl(value),
        TypeSpec::SequenceType(seq) => {
            let elem = idl_type_spec(&seq.ty);
            match &seq.len {
                Some(len) => format!("sequence<{}, {}>", elem, render_const_expr(&len.0)),
                None => format!("sequence<{}>", elem),
            }
        }
        TypeSpec::StringType(string) => match &string.bound {
            Some(bound) => format!("string<{}>", render_const_expr(&bound.0)),
            None => "string".to_string(),
        },
        TypeSpec::WideStringType(string) => match &string.bound {
            Some(bound) => format!("wstring<{}>", render_const_expr(&bound.0)),
            None => "wstring".to_string(),
        },
        TypeSpec::FixedPtType(fixed) => format!(
            "fixed<{}, {}>",
            render_const_expr(&fixed.integer.0),
            render_const_expr(&fixed.fraction.0)
        ),
        TypeSpec::MapType(map) => {
            let key = idl_type_spec(&map.key);
            let value = idl_type_spec(&map.value);
            match &map.len {
                Some(len) => format!("map<{}, {}, {}>", key, value, render_const_expr(&len.0)),
                None => format!("map<{}, {}>", key, value),
            }
        }
        TypeSpec::TemplateType(template) => {
            let args = template
                .args
                .iter()
                .map(idl_type_spec)
                .collect::<Vec<_>>()
                .join(", ");
            format!("{}<{}>", template.ident, args)
        }
    }
}

pub fn idl_integer_type(value: &IntegerType) -> String {
    match value {
        IntegerType::Char => "int8".to_string(),
        IntegerType::UChar | IntegerType::Octet => "uint8".to_string(),
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
    fn render_expr(expr: &ConstExpr) -> String {
        match expr {
            ConstExpr::ScopedName(value) => scoped_name_to_idl(value),
            ConstExpr::Literal(value) => render_literal(value),
            ConstExpr::UnaryExpr(op, value) => {
                let op = match op {
                    UnaryOperator::Add => "+",
                    UnaryOperator::Sub => "-",
                    UnaryOperator::Not => "~",
                };
                format!("({op}{})", render_expr(value))
            }
            ConstExpr::BinaryExpr(op, left, right) => {
                let op = match op {
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
                };
                format!("({} {op} {})", render_expr(left), render_expr(right))
            }
        }
    }

    fn render_literal(value: &Literal) -> String {
        match value {
            Literal::IntegerLiteral(lit) => lit.0.clone(),
            Literal::FloatingPtLiteral(lit) => {
                let sign = lit.sign.as_ref().map(IntegerSign::as_str).unwrap_or("");
                format!("{}{}.{}", sign, lit.integer.0, lit.fraction.0)
            }
            Literal::CharLiteral(value) => value.clone(),
            Literal::WideCharacterLiteral(value) => value.clone(),
            Literal::StringLiteral(value) => value.clone(),
            Literal::WideStringLiteral(value) => value.clone(),
            Literal::BooleanLiteral(value) => value.to_string(),
        }
    }

    render_expr(expr)
}
