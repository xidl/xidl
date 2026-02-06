export type OutputFile = {
  path: string;
  content: string;
};

export type PropItem = {
  id: string;
  key: string;
  value: string;
};

export type WasmModule = {
  cwrap: (
    name: string,
    ret: string | null,
    args: string[],
  ) => (...args: any[]) => any;
  UTF8ToString: (ptr: number) => string;
};
