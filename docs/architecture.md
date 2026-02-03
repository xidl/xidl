# Architecture

This document describes the end-to-end flow from IDL source to generated code,
and the main components in this repository. The diagram file is available at
`docs/architecture/architecture.excalidraw` if you prefer a visual map.

## Pipeline overview

```d2
source -> ast: tree-sitter-idl
ast -> typed_ast
typed_ast -> hir
hir -> generator
generator -> c
generator -> cpp
generator -> rust

source: {
  ex: |rust
    @derive(Debug)
    struct A {};
  |
}

ast: {
    ex: |json
        {
          "Node": {
            "kind": "specification",
            "children": [
              {
                "Node": {
                  "kind": "definition",
                  "children": [
                    {
                      "Node": {
                        "kind": "type_dcl",
                        "children": [
                          {
                            "Node": {
                              "kind": "annotation_appl",
                              "children": [
                                {
                                  "Node": {
                                    "kind": "annotation_appl_custom_body",
                                    "children": [
                                      {
                                        "Node": {
                                          "kind": "scoped_name",
                                          "children": [
                                            {
                                              "Node": {
                                                "kind": "identifier",
                                                "children": []
                                              }
                                            }
                                          ]
                                        }
                                      },
                                      {
                                        "Node": {
                                          "kind": "annotation_appl_params",
                                          "children": [
                                            {
                                              "Node": {
                                                "kind": "const_expr",
                                                "children": [
                                                  {
                                                    "Node": {
                                                      "kind": "or_expr",
                                                      "children": [
                                                        {
                                                          "Node": {
                                                            "kind": "xor_expr",
                                                            "children": [
                                                              {
                                                                "Node": {
                                                                  "kind": "and_expr",
                                                                  "children": [
                                                                    {
                                                                      "Node": {
                                                                        "kind": "shift_expr",
                                                                        "children": [
                                                                          {
                                                                            "Node": {
                                                                              "kind": "add_expr",
                                                                              "children": [
                                                                                {
                                                                                  "Node": {
                                                                                    "kind": "mult_expr",
                                                                                    "children": [
                                                                                      {
                                                                                        "Node": {
                                                                                          "kind": "unary_expr",
                                                                                          "children": [
                                                                                            {
                                                                                              "Node": {
                                                                                                "kind": "primary_expr",
                                                                                                "children": [
                                                                                                  {
                                                                                                    "Node": {
                                                                                                      "kind": "scoped_name",
                                                                                                      "children": [
                                                                                                        {
                                                                                                          "Node": {
                                                                                                            "kind": "identifier",
                                                                                                            "children": []
                                                                                                          }
                                                                                                        }
                                                                                                      ]
                                                                                                    }
                                                                                                  }
                                                                                                ]
                                                                                              }
                                                                                            }
                                                                                          ]
                                                                                        }
                                                                                      }
                                                                                    ]
                                                                                  }
                                                                                }
                                                                              ]
                                                                            }
                                                                          }
                                                                        ]
                                                                      }
                                                                    }
                                                                  ]
                                                                }
                                                              }
                                                            ]
                                                          }
                                                        }
                                                      ]
                                                    }
                                                  }
                                                ]
                                              }
                                            }
                                          ]
                                        }
                                      }
                                    ]
                                  }
                                }
                              ]
                            }
                          },
                          {
                            "Node": {
                              "kind": "constr_type_dcl",
                              "children": [
                                {
                                  "Node": {
                                    "kind": "struct_dcl",
                                    "children": [
                                      {
                                        "Node": {
                                          "kind": "struct_def",
                                          "children": [
                                            {
                                              "Node": {
                                                "kind": "identifier",
                                                "children": []
                                              }
                                            }
                                          ]
                                        }
                                      }
                                    ]
                                  }
                                }
                              ]
                            }
                          }
                        ]
                      }
                    }
                  ]
                }
              }
            ]
          }
        }
    |
}

typed_ast: {
    ex:|json
    {
      "Specification": [
        {
          "TypeDcl": {
            "annotations": [
              {
                "AnnotationAppl": {
                  "name": {
                    "ScopedName": {
                      "scoped_name": null,
                      "identifier": { "Identifier": "derive" },
                      "node_text": "derive"
                    }
                  },
                  "params": {
                    "ConstExpr": {
                      "ConstExpr": {
                        "XorExpr": {
                          "AndExpr": {
                            "ShiftExpr": {
                              "AddExpr": {
                                "MultExpr": {
                                  "UnaryExpr": {
                                    "PrimaryExpr": {
                                      "ScopedName": {
                                        "scoped_name": null,
                                        "identifier": { "Identifier": "Debug" },
                                        "node_text": "Debug"
                                      }
                                    }
                                  }
                                }
                              }
                            }
                          }
                        }
                      }
                    }
                  },
                  "is_extend": false,
                  "extra": []
                }
              }
            ],
            "decl": {
              "ConstrTypeDcl": {
                "StructDcl": {
                  "StructDef": {
                    "ident": { "Identifier": "A" },
                    "parent": [],
                    "member": []
                  }
                }
              }
            }
          }
        }
      ]
    }
    |
}

hir: {
    ex: |json
    [
        {
            "TypeDcl": {
            "annotations": [
                {
                "ScopedName": {
                    "name": {
                    "name": [
                        "derive"
                    ],
                    "is_root": false
                    },
                    "params": {
                    "ConstExpr": {
                        "XorExpr": {
                        "AndExpr": {
                            "ShiftExpr": {
                            "AddExpr": {
                                "MultExpr": {
                                "UnaryExpr": {
                                    "PrimaryExpr": {
                                    "ScopedName": {
                                        "name": [
                                        "Debug"
                                        ],
                                        "is_root": false
                                    }
                                    }
                                }
                                }
                            }
                            }
                        }
                        }
                    }
                    }
                }
                }
            ],
            "decl": {
                "ConstrTypeDcl": {
                "StructDcl": {
                    "annotations": [],
                    "ident": "A",
                    "parent": [],
                    "member": []
                }
                }
            }
            }
        }
        ]
    |
}
c: {
    ex:|c
        typedef struct A {

        } A;
    |
}

cpp: {
    ex:|cpp
        struct A {
        };
    |
}

rust: {
    ex:|rust
        #[derive(Debug)]
        pub struct A {
        }
    |
}
```

### What each stage means

- source: raw IDL text.
- ast: tree-sitter-idl parse tree with full syntactic detail.
- typed_ast: AST with typed nodes mapped to Rust enums/structs.
- hir: high-level, flattened structures suited for generation.
- generator: codegen backends that emit C/C++/Rust (or other targets).

## Core crates and responsibilities

- `tree-sitter-idl`: grammar and queries for parsing and editor tooling.
- `xidl-parser`: parsing + typed AST + HIR transformations.
- `xidl-derive`: derive helpers used by typed AST.
- `xidlc`: compiler CLI, driver, and code generation orchestration.
- `xidl-jsonrpc`: JSON-RPC transport types used by the JSON-RPC target.
- `xidl-xcdr`: CDR and related runtime support for generated code.
- `xidl-typeobject`: IDL typeobject implementation.

## xidl compiler flow

The compiler driver coordinates the parse, transform, and generation steps. For
example, a Rust JSON-RPC target flows like this:

```d2
source -> driver: source, lang=rust_jsonrpc
driver -> hir_generator: source=source, lang=hir, target_lang=rust_jsonrpc
hir_generator -> driver: source=hir, lang=rust_jsonrpc
driver -> rust_jsonrpc: source=hir, lang=rust_jsonrpc
rust_jsonrpc -> driver: source=hir, lang=rust, generated=source.rs
driver -> rust_generator: source=hir, lang=rust
rust_generator -> driver: generated=source.rs
driver -> IO: write(source.rs)
```

## Design notes

- typed_ast uses derived helpers for most nodes, and manual impls where needed.
- HIR favors fewer, flatter types to simplify generation.
- Generators should depend on HIR, not on tree-sitter or typed_ast.
