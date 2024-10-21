/** Import Map fllow the spec: https://wicg.github.io/import-maps/ */
export interface ImportMap {
  $src?: string;
  imports?: Record<string, string>;
  scopes?: Record<string, Record<string, string>>;
}

/** Delopment options. */
export interface DevOptions {
  /**
   * Enable hot module replacement, default is disabled.
   * ```js
   * // the injected code
   * import __CREATE_WEB_MODULE__ from '@hmr.js'
   * import.meta.hot = __CREATE_HOT_CONTEXT__(import.meta.url)
   */
  hmr?: { runtime: string };
  /**
   * Enable react refresh, default is disabled.
   * to enable it, you need to enable hmr first.
   * The runtime module must export `__REFRESH_RUNTIME__` and `__REFRESH__`.
   */
  refresh?: { runtime: string };
  /**
   * Enable preact refresh, default is disabled.
   * to enable it, you need to enable hmr first.
   * The runtime module must export `__REFRESH_RUNTIME__` and `__REFRESH__`.
   */
  prefresh?: { runtime: string };
  /** Add `__source` props to JSX elements, default is disabled. */
  jsxSource?: { fileName: string };
}

/** Transform options. */
export interface TransformOptions {
  /** The import map, pass it if the browser does not support import maps. */
  importMap?: ImportMap;
  /**
   * The jsx import source, default is "react".
   *
   * Alternative you can set it by adding "@jsxImportSource" entry in the import map.
   * ```json
   * {
   *  "imports": {
   *   "@jsxImportSource": "https://esm.sh/react"
   * }
   * ```
   */
  jsxImportSource?: string;
  /** The transform target, default is "esnext". */
  target?: "es2015" | "es2016" | "es2017" | "es2018" | "es2019" | "es2020" | "es2021" | "es2022" | "es2023" | "es2024" | "esnext";
  /** strip unused code, default is disabled. */
  treeShaking?: boolean;
  /** create source map, default is disabled. */
  sourceMap?: "inline" | "external";
  /** development mode, default is disabled. */
  dev?: DevOptions;
  /** The version map for the module resolver. */
  versionMap?: Record<string, string>;
}

/** Transform result. */
export interface TransformResult {
  readonly code: string;
  readonly map?: string;
  readonly deps?: DependencyDescriptor[];
}

/** Dependency descriptor. */
export interface DependencyDescriptor {
  readonly specifier: string;
  readonly resolvedUrl: string;
  readonly loc?: { start: number; end: number };
  readonly dynamic?: boolean;
}

/** Transforms the given code. */
export function transform(options: { filename: string; code: string } & TransformOptions): TransformResult;

/** Instantiates the given `module`, which can either be bytes or a precompiled `WebAssembly.Module`. */
export function initSync(module: BufferSource | WebAssembly.Module): { memory: WebAssembly.Memory };

/** If `module_or_path` is {RequestInfo} or {URL}, makes a request and for everything else, calls `WebAssembly.instantiate` directly. */
export default function init(
  module_or_path?: RequestInfo | URL | Response | BufferSource | WebAssembly.Module,
): Promise<{ memory: WebAssembly.Memory }>;
