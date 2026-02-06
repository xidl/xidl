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
  'rust',
  'rust-axum',
  'rust-jsonrpc',
  'c',
  'cpp',
  'hir',
  'typed_ast',
];

export const BASE_PATH = (process.env.NEXT_PUBLIC_BASE_PATH ?? '').replace(
  /\/$/,
  '',
);
export const WASM_BASE = `${BASE_PATH}/wasm`;
