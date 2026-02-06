'use client';

import {
  Code2,
  FileCode,
  Github,
  Plus,
  Settings,
  Share2,
  Trash2,
  X,
} from 'lucide-react';
import {
  compressToEncodedURIComponent,
  decompressFromEncodedURIComponent,
} from 'lz-string';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { CodeViewer } from '@/components/code-viewer';
import { ThemeToggle } from '@/components/theme-toggle';
import { Button } from '@/components/ui/button';
import { Collapsible } from '@/components/ui/collapsible';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from '@/components/ui/resizable';
import { ScrollArea, ScrollBar } from '@/components/ui/scroll-area';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Separator } from '@/components/ui/separator';
import { Switch } from '@/components/ui/switch';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Textarea } from '@/components/ui/textarea';

const DEFAULT_IDL = `interface HelloWorld {
  @post(path = "/hello")
  void sayHello(
    in string name
  );
};`;
const DEFAULT_LANG = 'rust';
const DEFAULT_METADATA = true;
const DEFAULT_SKIP_CLIENT = false;
const DEFAULT_SKIP_SERVER = false;
const LANG_OPTIONS = [
  'rust',
  'rust-axum',
  'rust-jsonrpc',
  'c',
  'cpp',
  'hir',
  'typed_ast',
];

type OutputFile = {
  path: string;
  content: string;
};

type PropItem = {
  id: string;
  key: string;
  value: string;
};

type WasmModule = {
  cwrap: (
    name: string,
    ret: string | null,
    args: string[],
  ) => (...args: any[]) => any;
  UTF8ToString: (ptr: number) => string;
};

const BASE_PATH = (process.env.NEXT_PUBLIC_BASE_PATH ?? '').replace(/\/$/, '');
const WASM_BASE = `${BASE_PATH}/wasm`;

export function Playground() {
  const [lang, setLang] = useState(DEFAULT_LANG);
  const [skipClient, setSkipClient] = useState(DEFAULT_SKIP_CLIENT);
  const [skipServer, setSkipServer] = useState(DEFAULT_SKIP_SERVER);
  const [enableMetadata, setEnableMetadata] = useState(DEFAULT_METADATA);
  const [idl, setIdl] = useState(DEFAULT_IDL);
  const [propItems, setPropItems] = useState<PropItem[]>([
    { id: 'prop-1', key: '', value: '' },
  ]);
  const [files, setFiles] = useState<OutputFile[]>([]);
  const [selectedFile, setSelectedFile] = useState<string>('');
  const [error, setError] = useState<string>('');
  const [loading, setLoading] = useState(false);
  const [formatting, setFormatting] = useState(false);
  const [copySuccess, setCopySuccess] = useState(false);
  const [propsOpen, setPropsOpen] = useState(false);
  const moduleRef = useRef<WasmModule | null>(null);
  const urlReadyRef = useRef(false);

  const selectedTab = selectedFile || files[0]?.path || 'empty';

  useEffect(() => {
    if (typeof window === 'undefined') return;
    const params = new URLSearchParams(window.location.search);
    const idlParam = params.get('idl');
    if (idlParam) {
      const decoded = decompressFromEncodedURIComponent(idlParam);
      if (typeof decoded === 'string') {
        setIdl(decoded);
      }
    }

    const urlLang = params.get('lang');
    if (urlLang && LANG_OPTIONS.includes(urlLang)) {
      setLang(urlLang);
    }

    const meta = parseUrlBoolean(params.get('meta'));
    if (meta !== null) {
      setEnableMetadata(meta);
    }

    const skipClientParam = parseUrlBoolean(params.get('skip_client'));
    if (skipClientParam !== null) {
      setSkipClient(skipClientParam);
    }

    const skipServerParam = parseUrlBoolean(params.get('skip_server'));
    if (skipServerParam !== null) {
      setSkipServer(skipServerParam);
    }

    urlReadyRef.current = true;
  }, []);

  const updateUrl = useCallback(() => {
    if (typeof window === 'undefined') return;
    const params = new URLSearchParams(window.location.search);

    if (idl !== DEFAULT_IDL) {
      params.set('idl', compressToEncodedURIComponent(idl));
    } else {
      params.delete('idl');
    }

    if (lang !== DEFAULT_LANG) {
      params.set('lang', lang);
    } else {
      params.delete('lang');
    }

    if (enableMetadata !== DEFAULT_METADATA) {
      params.set('meta', enableMetadata ? '1' : '0');
    } else {
      params.delete('meta');
    }

    if (skipClient !== DEFAULT_SKIP_CLIENT) {
      params.set('skip_client', skipClient ? '1' : '0');
    } else {
      params.delete('skip_client');
    }

    if (skipServer !== DEFAULT_SKIP_SERVER) {
      params.set('skip_server', skipServer ? '1' : '0');
    } else {
      params.delete('skip_server');
    }

    const url = new URL(window.location.href);
    url.search = params.toString();
    window.history.replaceState({}, '', url.toString());
  }, [enableMetadata, idl, lang, skipClient, skipServer]);

  useEffect(() => {
    if (!urlReadyRef.current) return;
    const handle = window.setTimeout(updateUrl, 250);
    return () => window.clearTimeout(handle);
  }, [updateUrl]);

  useEffect(() => {
    if (files.length && !selectedFile) {
      setSelectedFile(files[0].path);
    }
  }, [files, selectedFile]);

  const loadWasm = useCallback(async () => {
    if (moduleRef.current) {
      return moduleRef.current;
    }

    await new Promise<void>((resolve, reject) => {
      const existing = document.querySelector('script[data-xidlc]');
      if (existing) {
        resolve();
        return;
      }
      const script = document.createElement('script');
      script.src = `${WASM_BASE}/xidlc.js`;
      script.async = true;
      script.dataset.xidlc = '1';
      script.onload = () => resolve();
      script.onerror = () => reject(new Error('Failed to load xidlc wasm'));
      document.body.appendChild(script);
    });

    const factory = (window as any).xidlcModule as
      | undefined
      | (() => Promise<WasmModule>);
    if (!factory) {
      throw new Error('xidlcModule is not available');
    }
    // @ts-expect-error
    const module = await factory({
      locateFile: (path: string) => `${WASM_BASE}/${path}`,
    });
    moduleRef.current = module;
    return module;
  }, []);

  const runGenerate = useCallback(async () => {
    setLoading(true);
    setError('');
    try {
      const module = await loadWasm();
      const generate = module.cwrap('xidlc_generate_json', 'number', [
        'string',
      ]);
      const free = module.cwrap('xidlc_free_string', null, ['number']);

      const props = buildProps(propItems);
      props.enable_metadata = enableMetadata;
      if (skipClient) props.enable_client = false;
      if (skipServer) props.enable_server = false;

      const payload = JSON.stringify({
        lang,
        idl,
        props,
      });

      const ptr = generate(payload);
      const resultText = module.UTF8ToString(ptr);
      free(ptr);

      const result = JSON.parse(resultText);
      if (result.error) {
        throw new Error(result.error);
      }
      const outFiles = (result.files || []) as OutputFile[];
      setFiles(outFiles);
      if (outFiles.length > 0) {
        setSelectedFile(outFiles[0].path);
      }
    } catch (err: any) {
      setError(err?.message ?? 'Generation failed');
      setFiles([]);
    } finally {
      setLoading(false);
    }
  }, [enableMetadata, idl, lang, loadWasm, propItems, skipClient, skipServer]);

  const runFormat = useCallback(async () => {
    setFormatting(true);
    setError('');
    try {
      const module = await loadWasm();
      const format = module.cwrap('xidlc_format_json', 'number', ['string']);
      const free = module.cwrap('xidlc_free_string', null, ['number']);

      const payload = JSON.stringify({
        source: idl,
      });

      const ptr = format(payload);
      const resultText = module.UTF8ToString(ptr);
      free(ptr);

      const result = JSON.parse(resultText);
      if (result.error) {
        throw new Error(result.error);
      }
      if (typeof result.formatted !== 'string') {
        throw new Error('Invalid format response');
      }
      setIdl(result.formatted);
    } catch (err: any) {
      setError(err?.message ?? 'Format failed');
    } finally {
      setFormatting(false);
    }
  }, [idl, loadWasm]);

  const handleShare = useCallback(() => {
    if (typeof window === 'undefined') return;
    const url = window.location.href;
    navigator.clipboard
      .writeText(url)
      .then(() => {
        setCopySuccess(true);
        setTimeout(() => setCopySuccess(false), 2000);
      })
      .catch(err => {
        console.error('Failed to copy URL:', err);
        setError('Failed to copy URL to clipboard');
      });
  }, []);

  const addProp = () => {
    setPropItems(items => [
      ...items,
      { id: `prop-${Date.now()}`, key: '', value: '' },
    ]);
  };

  const updateProp = (id: string, field: 'key' | 'value', value: string) => {
    setPropItems(items =>
      items.map(item => (item.id === id ? { ...item, [field]: value } : item)),
    );
  };

  const removeProp = (id: string) => {
    setPropItems(items => items.filter(item => item.id !== id));
  };

  return (
    <div className="flex h-screen flex-col bg-background text-foreground">
      {/* Header */}
      <header className="flex h-14 items-center justify-between border-border border-b bg-card px-6 shadow-sm">
        <div className="flex items-center gap-3">
          <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-linear-to-br from-primary to-primary/80 text-primary-foreground shadow-sm">
            <Code2 className="h-4 w-4" />
          </div>
          <div>
            <div className="font-semibold text-sm leading-none">
              XIDL Playground
            </div>
            <div className="mt-0.5 text-muted-foreground text-xs">
              Interactive IDL compiler
            </div>
          </div>
        </div>
        <div className="flex items-center gap-4">
          <div className="flex items-center gap-2">
            <Label className="text-muted-foreground text-xs">Language</Label>
            <Select
              value={lang}
              onValueChange={lang => {
                setLang(lang);
                setFiles([]);
              }}
            >
              <SelectTrigger className="h-8 w-36">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="rust">Rust</SelectItem>
                <SelectItem value="rust-axum">Rust Axum</SelectItem>
                <SelectItem value="rust-jsonrpc">Rust JSON-RPC</SelectItem>
                <SelectItem value="c">C</SelectItem>
                <SelectItem value="cpp">C++</SelectItem>
                <SelectItem value="hir">HIR</SelectItem>
                <SelectItem value="typed_ast">Typed AST</SelectItem>
              </SelectContent>
            </Select>
          </div>
          <Separator orientation="vertical" className="h-6" />
          <div className="flex items-center gap-2">
            <Label className="text-muted-foreground text-xs">Metadata</Label>
            <Switch
              checked={enableMetadata}
              onCheckedChange={setEnableMetadata}
              className="scale-90"
            />
          </div>
          <Button
            onClick={runGenerate}
            disabled={loading}
            className="h-8 gap-1.5 shadow-sm"
          >
            <FileCode className="h-3.5 w-3.5" />
            {loading ? 'Generating...' : 'Generate'}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            onClick={() =>
              window.open('https://github.com/xidl/xidl', '_blank')
            }
            className="gap-2"
          >
            <Github className="h-4 w-4" />
            GitHub
          </Button>
          <Button
            variant="ghost"
            size="sm"
            onClick={handleShare}
            className="gap-2"
          >
            <Share2 className="h-4 w-4" />
            {copySuccess ? 'Copied!' : 'Share'}
          </Button>
          <ThemeToggle />
        </div>
      </header>

      {/* Main Content */}
      <div className="flex-1 overflow-hidden p-4">
        <ResizablePanelGroup
          orientation="horizontal"
          className="h-full rounded-lg border border-border bg-card shadow-sm"
        >
          {/* Left Panel - Editor */}
          <ResizablePanel defaultSize={40} minSize={25}>
            <ResizablePanelGroup orientation="vertical">
              {/* IDL Editor */}
              <ResizablePanel defaultSize={65} minSize={30}>
                <div className="flex h-full flex-col">
                  <div className="flex items-center justify-between border-border border-b bg-muted/30 px-4 py-2.5">
                    <div className="flex items-center gap-2">
                      <FileCode className="h-3.5 w-3.5 text-muted-foreground" />
                      <span className="font-medium text-sm">IDL Editor</span>
                    </div>
                    <div className="flex items-center gap-4">
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={runFormat}
                        disabled={formatting}
                        className="h-7 px-2 text-xs"
                      >
                        {formatting ? 'Formatting…' : 'Format'}
                      </Button>
                      <div className="flex items-center gap-2 text-xs">
                        <span className="text-muted-foreground">
                          skip_client
                        </span>
                        <Switch
                          checked={skipClient}
                          onCheckedChange={setSkipClient}
                          className="scale-75"
                        />
                      </div>
                      <div className="flex items-center gap-2 text-xs">
                        <span className="text-muted-foreground">
                          skip_server
                        </span>
                        <Switch
                          checked={skipServer}
                          onCheckedChange={setSkipServer}
                          className="scale-75"
                        />
                      </div>
                    </div>
                  </div>
                  <div className="flex-1 overflow-hidden p-0">
                    <Textarea
                      value={idl}
                      onChange={e => setIdl(e.target.value)}
                      className="h-full resize-none rounded-none border-none font-mono text-xs leading-relaxed"
                      placeholder="Enter your IDL code here..."
                    />
                  </div>
                </div>
              </ResizablePanel>

              <ResizableHandle withHandle />

              {/* Properties Panel */}
              <ResizablePanel defaultSize={35} minSize={20} className="p-2">
                <Collapsible open={propsOpen} onOpenChange={setPropsOpen}>
                  <div className="flex h-full flex-col border-border">
                    <div className="flex items-center gap-2">
                      <Settings className="h-3.5 w-3.5 text-muted-foreground" />
                      <span className="font-medium text-sm">Properties</span>
                      <span className="rounded-md bg-muted px-1.5 py-0.5 font-mono text-muted-foreground text-xs">
                        {propItems.filter(p => p.key.trim()).length}
                      </span>
                    </div>
                    <div className="flex-1 overflow-hidden">
                      <ScrollArea className="h-full">
                        <div className="space-y-2 p-3">
                          {propItems.map(item => (
                            <div
                              key={item.id}
                              className="flex gap-2 rounded border border-border p-2"
                            >
                              <Input
                                placeholder="key"
                                value={item.key}
                                onChange={e =>
                                  updateProp(item.id, 'key', e.target.value)
                                }
                                className="h-7 flex-1 font-mono text-xs"
                              />
                              <Input
                                placeholder="value"
                                value={item.value}
                                onChange={e =>
                                  updateProp(item.id, 'value', e.target.value)
                                }
                                className="h-7 flex-1 font-mono text-xs"
                              />
                              <Button
                                variant="ghost"
                                size="sm"
                                onClick={() => removeProp(item.id)}
                                className="h-7 w-7 p-0"
                              >
                                <Trash2 className="h-3 w-3" />
                              </Button>
                            </div>
                          ))}
                          <Button
                            variant="outline"
                            size="sm"
                            onClick={addProp}
                            className="h-7 w-full gap-1.5 text-xs"
                          >
                            <Plus className="h-3 w-3" />
                            Add Property
                          </Button>
                        </div>
                      </ScrollArea>
                    </div>
                  </div>
                </Collapsible>
              </ResizablePanel>
            </ResizablePanelGroup>
          </ResizablePanel>

          <ResizableHandle withHandle />

          {/* Right Panel - Output */}
          <ResizablePanel defaultSize={60} minSize={30}>
            <Tabs
              value={selectedTab}
              onValueChange={setSelectedFile}
              className="flex h-full flex-col"
            >
              <div className="flex flex-col border-border border-b bg-muted/30">
                <div className="flex items-center justify-between px-4 py-2.5">
                  <div className="flex items-center gap-2">
                    <FileCode className="h-3.5 w-3.5 text-muted-foreground" />
                    <span className="font-medium text-sm">Output Files</span>
                    <span className="rounded-md bg-muted px-1.5 py-0.5 font-mono text-muted-foreground text-xs">
                      {files.length}
                    </span>
                  </div>
                </div>
                <ScrollArea className="border-border border-t">
                  <TabsList className="h-auto w-full justify-start rounded-none bg-transparent p-0">
                    {files.length === 0 && (
                      <TabsTrigger
                        value="empty"
                        className="rounded-none border-transparent border-b-2 border-none data-[state=active]:border-primary data-[state=active]:bg-transparent"
                      >
                        No files
                      </TabsTrigger>
                    )}
                    {files.map(file => (
                      <TabsTrigger
                        key={file.path}
                        value={file.path}
                        className="group relative rounded-none border-transparent border-b-2 border-none px-3 py-2 data-[state=active]:border-primary data-[state=active]:bg-transparent"
                      >
                        <span className="font-mono text-xs">{file.path}</span>
                      </TabsTrigger>
                    ))}
                  </TabsList>
                  <ScrollBar orientation="horizontal" />
                </ScrollArea>
              </div>
              <div className="flex-1 overflow-hidden">
                {files.length === 0 && (
                  <TabsContent
                    value="empty"
                    className="m-0 flex h-full items-center justify-center"
                  >
                    <div className="text-center">
                      <FileCode className="mx-auto h-12 w-12 text-muted-foreground/50" />
                      <p className="mt-3 font-medium text-muted-foreground text-sm">
                        No output yet
                      </p>
                      <p className="mt-1 text-muted-foreground text-xs">
                        Click Generate to see the results
                      </p>
                    </div>
                  </TabsContent>
                )}
                {files.map(file => (
                  <TabsContent
                    key={file.path}
                    value={file.path}
                    className="m-0 h-full overflow-hidden"
                  >
                    <ScrollArea className="h-full">
                      <div className="p-4">
                        <CodeViewer
                          code={file.content}
                          language={inferLanguage(lang, file.path)}
                        />
                      </div>
                      <ScrollBar orientation="horizontal" />
                    </ScrollArea>
                  </TabsContent>
                ))}
              </div>
            </Tabs>
          </ResizablePanel>
        </ResizablePanelGroup>
      </div>

      {/* Error Toast */}
      {error && (
        <div className="slide-in-from-bottom-2 fixed right-4 bottom-4 max-w-md animate-in">
          <div className="rounded-lg border border-destructive bg-destructive/10 p-4 shadow-lg">
            <div className="flex items-start gap-3">
              <div className="flex-1">
                <p className="font-medium text-sm">Error</p>
                <p className="mt-1 text-muted-foreground text-xs">{error}</p>
              </div>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => setError('')}
                className="h-6 w-6 p-0"
              >
                <X className="h-4 w-4" />
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

function buildProps(items: PropItem[]) {
  const props: Record<string, any> = {};
  for (const item of items) {
    const key = item.key.trim();
    if (!key) continue;
    const raw = item.value.trim();
    if (!raw) {
      props[key] = '';
      continue;
    }
    try {
      props[key] = JSON.parse(raw);
    } catch {
      props[key] = raw;
    }
  }
  return props;
}

function parseUrlBoolean(value: string | null) {
  if (value === null) return null;
  if (value === '1' || value === 'true') return true;
  if (value === '0' || value === 'false') return false;
  return null;
}

function inferLanguage(language: string, path: string | undefined): string {
  if (language === 'hir') {
    return 'hir';
  }
  if (language === 'typed_ast') {
    return 'typed_ast';
  }
  if (!path) return 'text';
  const ext = path.split('.').pop()?.toLowerCase();
  const langMap: Record<string, string> = {
    rs: 'rust',
    c: 'c',
    cpp: 'cpp',
    cc: 'cpp',
    h: 'c',
    hpp: 'cpp',
    json: 'json',
    toml: 'toml',
    yaml: 'yaml',
    yml: 'yaml',
  };
  return langMap[ext || ''] || 'text';
}
