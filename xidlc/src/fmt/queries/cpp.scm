; Basic C/C++ formatting rules (tree-sitter-cpp).

(comment) @comment

("{" @append-newline @add-ident)
("}" @prepend-newline @dec-ident)
("," @append-space)
("=" @prepend-space @append-space)
("==" @prepend-space @append-space)
("!=" @prepend-space @append-space)
(">" @prepend-space @append-space)
("<" @prepend-space @append-space)
(">=" @prepend-space @append-space)
("<=" @prepend-space @append-space)
("+" @prepend-space @append-space)
("-" @prepend-space @append-space)
("*" @prepend-space @append-space)
("/" @prepend-space @append-space)
(";" @append-newline)
