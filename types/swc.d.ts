/** Import Map fllow the spec: https://wicg.github.io/import-maps/ */
export interface ImportMap {
  imports?: Record<string, string>;
  scopes?: Record<string, Record<string, string>>;
}

export interface Runtime {
  runtime: string;
}

/** delopment options. */
export interface DevOptions {
  /** hot module replacement, default is disabled. */
  hmr?: Runtime;
  /** enable react/preact refresh, default is disabled. */
  refresh?: Runtime & { preact?: boolean };
}

/** transform options for swc. */
export interface SWCTransformOptions {
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

export interface SWCTransformResult {
  readonly code: string;
  readonly map?: string;
  readonly deps?: DependencyDescriptor[];
}

export interface DependencyDescriptor {
  readonly specifier: string;
  readonly resolvedUrl: string;
  readonly loc?: { start: number; end: number };
  readonly dynamic?: boolean;
}

export function transform(options: { code: string; filename: string } & SWCTransformOptions): SWCTransformResult;
