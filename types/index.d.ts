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
  /** The code language, default is using the file extension. */
  lang?: "ts" | "tsx" | "js" | "jsx";
  /** The transform target, default is "esnext". */
  target?: "es2015" | "es2016" | "es2017" | "es2018" | "es2019" | "es2020" | "es2021" | "es2022" | "es2023" | "es2024" | "esnext";
  /** The import map, pass it if the browser does not support import maps. */
  importMap?: ImportMap;
  /**
   * Specifies the JSX import source. By default, it will use `react` or `preact` from the import map.
   *
   * ```json
   * {
   *   "importMap": {
   *     "imports": { "react": "https://esm.sh/react" }
   *   }
   * }
   * ```
   *
   * Alternatively, you can specify it by adding an "@jsxRuntime" entry in the import map.
   *
   * ```json
   * {
   *   "importMap": {
   *     "imports": { "@jsxRuntime": "https://esm.sh/react" }
   *   }
   * }
   * ```
   */
  jsxImportSource?: string;
  /** strip unused code, default is disabled. */
  treeShaking?: boolean;
  /** create source map, default is disabled. */
  sourceMap?: "inline" | "external";
  /** development mode, default is disabled. */
  dev?: DevOptions;
  /**
   * The version map for the module resolver.
   * ```json
   * {
   *   "versionMap": {
   *     "*": "1.0.0",
   *     "/src/main.tsx": "1.0.1"
   *   }
   * }
   * - "*" is the default version for all modules, e.g. `/src/App.tsx?v=1.0.0`.
   * - "/src/main.tsx" is the version for the specific module, e.g. `/src/main.tsx?v=1.0.1`.
   * - http imports will ignore the version map.
   */
  versionMap?: Record<string, string>;
}

/** Transform result. */
export interface TransformResult {
  readonly code: string;
  readonly map?: string;
}

/** Transforms the given code. */
export function transform(options: { filename: string; code: string } & TransformOptions): TransformResult;

/** Instantiates the given `module`, which can either be bytes or a precompiled `WebAssembly.Module`. */
export function initSync(module: BufferSource | WebAssembly.Module): { memory: WebAssembly.Memory };

/** If `module_or_path` is {RequestInfo} or {URL}, makes a request and for everything else, calls `WebAssembly.instantiate` directly. */
export function init(
  module_or_path?: RequestInfo | URL | Response | BufferSource | WebAssembly.Module,
): Promise<{ memory: WebAssembly.Memory }>;

export { init as default };
