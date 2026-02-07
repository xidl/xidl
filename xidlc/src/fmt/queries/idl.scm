; Basic IDL formatting rules.

(comment) @comment

(definition) @append-newline @append-newline
(member) @append-newline
(export) @append-newline

(annotation_appl) @append-newline
(param_attribute) @append-space
(const_dcl (const_type) @append-space)
(member (type_spec) @append-space)
(param_dcl (type_spec) @append-space)
(op_dcl (op_type_spec) @append-space)
(readonly_attr_spec (type_spec) @append-space)
(attr_spec (type_spec) @append-space)
(readonly_attr_spec "attribute" @append-space)
(attr_spec "attribute" @append-space)

(preproc_include (keyword_include) @append-space)
(preproc_call (preproc_directive) @append-space (preproc_arg))
(preproc_define "#define" @append-space)

(module_dcl "module" @append-space)
(template_module_dcl "module" @append-space)
(template_module_inst "module" @append-space)
(struct_def "struct" @append-space)
(struct_forward_dcl "struct" @append-space)
(union_def "union" @append-space (identifier) @append-space "switch" @append-space)
(union_forward_dcl "union" @append-space)
(interface_forward_dcl (interface_kind) @append-space)
(interface_header (interface_kind) @append-space)
(except_dcl "exception" @append-space)
(enum_dcl "enum" @append-space)
(bitmask_dcl "bitmask" @append-space)
(bitset_dcl "bitset" @append-space)
(typedef_dcl "typedef" @append-space)
(const_dcl "const" @append-space)
(readonly_attr_spec "readonly" @append-space)
(raises_expr "raises" @prepend-space)
(get_excep_expr "getraises" @prepend-space)
(set_excep_expr "setraises" @prepend-space)

(module_dcl "{" @append-newline @add-ident "}" @dec-ident)
(template_module_dcl "{" @append-newline @add-ident "}" @dec-ident)
(struct_def "{" @append-newline @add-ident "}" @dec-ident)
(union_def "{" @append-newline @add-ident "}" @dec-ident)
(interface_def "{" @append-newline @add-ident "}" @dec-ident)
(except_dcl "{" @append-newline @add-ident "}" @dec-ident)
(enum_dcl "{" @append-newline @add-ident "}" @prepend-newline @dec-ident)
(bitset_dcl "{" @append-newline @add-ident "}" @dec-ident)
(bitmask_dcl "{" @append-newline @add-ident "}" @prepend-newline @dec-ident)

("{" @prepend-space)
("," @append-space)
(":" @append-space)
("=" @prepend-space @append-space)
(case_label "case" @append-space)
(case (case_label) @append-newline @add-ident (element_spec) ";" @append-newline @dec-ident)
(bitmask_dcl "," @append-newline)
(enum_dcl "," @append-newline)

(op_dcl "(" @append-newline @add-ident (parameter_dcls) ")" @prepend-newline @dec-ident)
(parameter_dcls "," @append-newline)
(type_declarator (simple_type_spec) @append-space)
(type_declarator (template_type_spec) @append-space)
(type_declarator (constr_type_dcl) @append-space)
(element_spec (type_spec) @append-space)
(bitfield) @append-newline
(bitfield (bitfield_spec) @append-space)

(formal_parameter (formal_parameter_type) @append-space)
(template_module_ref "alias" @append-space)
(template_module_ref ">" @append-space)
(template_module_inst ">" @append-space)
