; Basic TypeScript formatting rules (tree-sitter-typescript).

(statement_block "{" @append-newline @add-ident "}" @prepend-newline @dec-ident)
(object "{" @append-newline @add-ident "}" @prepend-newline @dec-ident)

(interface_body "{" @append-newline @add-ident "}" @dec-ident)
(object_type "{" @append-newline @add-ident "}" @dec-ident)

(array "[" @append-newline @add-ident "]" @prepend-newline @dec-ident)
(type_arguments "<" @append-newline @add-ident ">" @prepend-newline @dec-ident)
(formal_parameters "(" @append-newline @add-ident ")" @prepend-newline @dec-ident)
(arguments "(" @append-newline @add-ident ")" @prepend-newline @dec-ident)

(";" @append-newline)
("," @append-space)
("=>" @prepend-space @append-space)
("=" @prepend-space @append-space)
(":" @prepend-space @append-space)
("|" @prepend-space @append-space)
("&" @prepend-space @append-space)
("?" @prepend-space)
