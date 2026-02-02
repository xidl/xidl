; Basic Rust formatting rules (tree-sitter-rust).

; Block-like structures
(block "{" @append-newline @add-ident "}" @prepend-newline @dec-ident)
(match_block "{" @append-newline @add-ident "}" @prepend-newline @dec-ident)
(declaration_list "{" @append-newline @add-ident "}" @prepend-newline @dec-ident)

; Item bodies
(field_declaration_list "{" @append-newline @add-ident "}" @dec-ident)
(enum_variant_list "{" @append-newline @add-ident "}" @dec-ident)

; Struct literals
(field_initializer_list "{" @append-newline @add-ident "}" @dec-ident)

(";" @append-newline)
("=>" @prepend-space @append-space)
("=" @prepend-space @append-space)

(field_declaration_list "," @append-newline)
(enum_variant_list "," @append-newline)
(field_initializer_list "," @append-newline)
(match_block (match_arm) @append-newline)
