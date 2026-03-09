export const DEFAULT_IDL = `interface HelloWorld {
  @post(path = "/hello")
  void sayHello(
    in string name
  );
};`;

export const DEFAULT_LANG = 'rust';
export const DEFAULT_METADATA = true;
export const DEFAULT_SKIP_CLIENT = false;
export const DEFAULT_SKIP_SERVER = false;

export const LANG_OPTIONS = [
  { value: 'typed_ast', label: 'Typed AST' },
  { value: 'hir', label: 'HIR' },
  { value: 'c', label: 'C' },
  { value: 'cpp', label: 'C++' },
  { value: 'rust', label: 'Rust' },
  { value: 'rust-axum', label: 'Rust Axum' },
  { value: 'rust-jsonrpc', label: 'Rust JsonRPC' },
  { value: 'openapi', label: 'OpenAPI' },
  { value: 'openrpc', label: 'OpenRPC' },
];

export function isValidLanguage(lang: string): boolean {
  return LANG_OPTIONS.some(option => option.value === lang);
}

export const BASE_PATH = (process.env.NEXT_PUBLIC_BASE_PATH ?? '').replace(
  /\/$/,
  '',
);
export const WASM_BASE = `${BASE_PATH}/wasm`;
