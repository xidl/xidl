// Keep the runtime value, wire name, and compiler declaration in one table.
// The compiler parses the resulting IDL using its ordinary declaration parser.
macro_rules! content_types {
    ($($(#[$doc:meta])* $variant:ident => $media:literal),+ $(,)?) => {
        /// The supported representations of an xidl HTTP response union.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
        pub enum ContentType {
            $($(#[$doc])* #[serde(rename = $media)] $variant),+
        }

        impl ContentType {
            /// The ordinary IDL declaration used to resolve HTTP discriminators.
            /// This declaration describes the shared runtime type, not a user model.
            pub const IDL: &'static str = concat!(
                "enum ContentType {\n",
                $("    @rename(\"", $media, "\") ", stringify!($variant), ",\n",)+
                "};\n",
            );

            /// Returns the member name used in IDL case labels and generated variants.
            pub const fn idl_name(self) -> &'static str {
                match self { $(Self::$variant => stringify!($variant)),+ }
            }

            /// Resolves an IDL member name after the compiler has checked its scope.
            pub fn from_idl_name(name: &str) -> Option<Self> {
                match name { $(stringify!($variant) => Some(Self::$variant),)+ _ => None }
            }

            /// Returns the HTTP media type, without parameters.
            pub const fn as_str(self) -> &'static str {
                match self { $(Self::$variant => $media),+ }
            }
        }
    };
}

content_types! {
    /// JSON-encoded structured data.
    Json => "application/json",
    /// An uninterpreted byte sequence.
    OctetStream => "application/octet-stream",
    /// Plain text.
    Text => "text/plain",
}

#[cfg(test)]
mod tests;
