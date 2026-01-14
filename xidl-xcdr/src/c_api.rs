macro_rules! decl_for_primitive {
    ($format:ident, $name:ident, $ctype:expr) => {
        concat!(
            "void ",
            stringify!($format),
            "_serialize_",
            stringify!($name),
            "(",
            $ctype,
            " value);\n",
            "void ",
            stringify!($format),
            "_deserialize_",
            stringify!($name),
            "(",
            $ctype,
            " *value);\n"
        )
    };
}

macro_rules! decls_for_format {
    ($format:ident) => {
        concat!(
            decl_for_primitive!($format, u8, "uint8_t"),
            decl_for_primitive!($format, i8, "int8_t"),
            decl_for_primitive!($format, u16, "uint16_t"),
            decl_for_primitive!($format, i16, "int16_t"),
            decl_for_primitive!($format, u32, "uint32_t"),
            decl_for_primitive!($format, i32, "int32_t"),
            decl_for_primitive!($format, u64, "uint64_t"),
            decl_for_primitive!($format, i64, "int64_t"),
            decl_for_primitive!($format, bool, "bool"),
            decl_for_primitive!($format, f32, "float"),
            decl_for_primitive!($format, f64, "double")
        )
    };
}

pub const C_HEADER: &str = concat!(
    "#ifndef XIDL_XCDR_H\n",
    "#define XIDL_XCDR_H\n\n",
    "#include <stdbool.h>\n",
    "#include <stdint.h>\n\n",
    decls_for_format!(cdr),
    decls_for_format!(plcdr),
    decls_for_format!(cdr3),
    "\n#endif\n"
);

pub fn c_header() -> &'static str {
    C_HEADER
}
