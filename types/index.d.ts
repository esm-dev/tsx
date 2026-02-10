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
   * import __CREATE_HOT_CONTEXT__ from "hmr_runtime"
   * import.meta.hot = __CREATE_HOT_CONTEXT__(import.meta.url)
   * ```
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
  /** The file name, used for source map and error message. */
  filename: string;
  /** The code to transform. */
  code: string | Uint8Array;
  /** The code language, default is using the file extension. */
  lang?: "ts" | "tsx" | "js" | "jsx";
  /** The transform target, default is "esnext". */
  target?: "es2015" | "es2016" | "es2017" | "es2018" | "es2019" | "es2020" | "es2021" | "es2022" | "es2023" | "es2024" | "esnext";
  /** The import map, pass it if the browser does not support import maps. */
  importMap?: ImportMap;
  /**
   * Specifies the JSX import source. if not specified, it will check the import specifier which ends with `/jsx-runtime` from the import map.
   *
   * For example, the jsx import source will be `preact` with the following import map:
   *
   * ```json
   * {
   *   "importMap": {
   *     "imports": { "preact/jsx-runtime": "https://esm.sh/preact/jsx-runtime" }
   *   }
   * }
   * ```
   */
  jsxImportSource?: string;
  /** minify outputed code, default is disabled. */
  minify?: boolean;
  /** strip unused code, default is disabled. */
  treeShaking?: boolean;
  /** create source map, default is disabled. */
  sourceMap?: "inline" | "external";
  /** development mode, default is disabled. */
  dev?: DevOptions;
}

/** Transform result. */
export interface TransformResult {
  /** The transformed JavaScript code. */
  readonly code: Uint8Array;
  /** The generated source map, if the `sourceMap` option is enabled as `external`. */
  readonly map?: Uint8Array;
}

/** Transforms the given code. */
export function transform(options: TransformOptions): TransformResult;

/** Instantiates the given `module`, which can either be bytes or a precompiled `WebAssembly.Module`. */
export function initSync(module: BufferSource | WebAssembly.Module): { memory: WebAssembly.Memory };

/** If `module_or_path` is {RequestInfo} or {URL}, makes a request and for everything else, calls `WebAssembly.instantiate` directly. */
export function init(
  module_or_path?: RequestInfo | URL | Response | BufferSource | WebAssembly.Module,
): Promise<{ memory: WebAssembly.Memory }>;

export { init as default };
