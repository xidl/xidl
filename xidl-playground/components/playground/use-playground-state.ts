import {
  compressToEncodedURIComponent,
  decompressFromEncodedURIComponent,
} from 'lz-string';
import { useCallback, useEffect, useRef, useState } from 'react';
import {
  DEFAULT_IDL,
  DEFAULT_LANG,
  DEFAULT_METADATA,
  DEFAULT_SKIP_CLIENT,
  DEFAULT_SKIP_SERVER,
  LANG_OPTIONS,
  WASM_BASE,
} from '@/components/playground/constants';
import type {
  OutputFile,
  PropItem,
  WasmModule,
} from '@/components/playground/types';
import { buildProps, parseUrlBoolean } from '@/components/playground/utils';

type UsePlaygroundStateResult = {
  lang: string;
  enableMetadata: boolean;
  loading: boolean;
  copySuccess: boolean;
  idl: string;
  formatting: boolean;
  skipClient: boolean;
  skipServer: boolean;
  propsOpen: boolean;
  propItems: PropItem[];
  files: OutputFile[];
  selectedTab: string;
  error: string;
  setEnableMetadata: (value: boolean) => void;
  setIdl: (value: string) => void;
  setSkipClient: (value: boolean) => void;
  setSkipServer: (value: boolean) => void;
  setPropsOpen: (value: boolean) => void;
  setSelectedFile: (value: string) => void;
  clearError: () => void;
  handleLangChange: (lang: string) => void;
  handleOpenGithub: () => void;
  handleShare: () => void;
  runGenerate: () => Promise<void>;
  runFormat: () => Promise<void>;
  addProp: () => void;
  updateProp: (id: string, field: 'key' | 'value', value: string) => void;
  removeProp: (id: string) => void;
};

export function usePlaygroundState(): UsePlaygroundStateResult {
  const [lang, setLang] = useState(DEFAULT_LANG);
  const [skipClient, setSkipClient] = useState(DEFAULT_SKIP_CLIENT);
  const [skipServer, setSkipServer] = useState(DEFAULT_SKIP_SERVER);
  const [enableMetadata, setEnableMetadata] = useState(DEFAULT_METADATA);
  const [idl, setIdl] = useState(DEFAULT_IDL);
  const [propItems, setPropItems] = useState<PropItem[]>([
    { id: 'prop-1', key: '', value: '' },
  ]);
  const [files, setFiles] = useState<OutputFile[]>([]);
  const [selectedFile, setSelectedFile] = useState('');
  const [error, setError] = useState('');
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

    const metadata = parseUrlBoolean(params.get('meta'));
    if (metadata !== null) {
      setEnableMetadata(metadata);
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
    const timer = window.setTimeout(updateUrl, 250);
    return () => window.clearTimeout(timer);
  }, [updateUrl]);

  useEffect(() => {
    if (files.length > 0 && !selectedFile) {
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

    // @ts-expect-error: Emscripten factory signature is dynamic
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

      const pointer = generate(payload);
      const resultText = module.UTF8ToString(pointer);
      free(pointer);

      const result = JSON.parse(resultText);
      if (result.error) {
        throw new Error(result.error);
      }

      const outFiles = (result.files || []) as OutputFile[];
      setFiles(outFiles);
      if (outFiles.length > 0) {
        setSelectedFile(outFiles[0].path);
      }
    } catch (runError: any) {
      setError(runError?.message ?? 'Generation failed');
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

      const payload = JSON.stringify({ source: idl });
      const pointer = format(payload);
      const resultText = module.UTF8ToString(pointer);
      free(pointer);

      const result = JSON.parse(resultText);
      if (result.error) {
        throw new Error(result.error);
      }
      if (typeof result.formatted !== 'string') {
        throw new Error('Invalid format response');
      }

      setIdl(result.formatted);
    } catch (formatError: any) {
      setError(formatError?.message ?? 'Format failed');
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
      .catch(copyError => {
        console.error('Failed to copy URL:', copyError);
        setError('Failed to copy URL to clipboard');
      });
  }, []);

  const handleOpenGithub = useCallback(() => {
    if (typeof window === 'undefined') return;
    window.open('https://github.com/xidl/xidl', '_blank');
  }, []);

  const handleLangChange = useCallback((nextLang: string) => {
    setLang(nextLang);
    setFiles([]);
  }, []);

  const addProp = useCallback(() => {
    setPropItems(items => [
      ...items,
      { id: `prop-${Date.now()}`, key: '', value: '' },
    ]);
  }, []);

  const updateProp = useCallback(
    (id: string, field: 'key' | 'value', value: string) => {
      setPropItems(items =>
        items.map(item =>
          item.id === id ? { ...item, [field]: value } : item,
        ),
      );
    },
    [],
  );

  const removeProp = useCallback((id: string) => {
    setPropItems(items => items.filter(item => item.id !== id));
  }, []);

  const clearError = useCallback(() => {
    setError('');
  }, []);

  return {
    lang,
    enableMetadata,
    loading,
    copySuccess,
    idl,
    formatting,
    skipClient,
    skipServer,
    propsOpen,
    propItems,
    files,
    selectedTab,
    error,
    setEnableMetadata,
    setIdl,
    setSkipClient,
    setSkipServer,
    setPropsOpen,
    setSelectedFile,
    clearError,
    handleLangChange,
    handleOpenGithub,
    handleShare,
    runGenerate,
    runFormat,
    addProp,
    updateProp,
    removeProp,
  };
}
