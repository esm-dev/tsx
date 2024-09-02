export interface HmrOptions {
  runtime: string;
  reactRefresh?: boolean;
  reactRefreshRuntime?: string;
}

export interface ImportMap {
  imports?: Record<string, string>;
  scopes?: Record<string, Record<string, string>>;
}

export interface SWCTransformOptions {
  lang?: "ts" | "tsx" | "js" | "jsx";
  sourceMap?: "inline" | "external";
  importMap?: ImportMap;
  isDev?: boolean;
  hmr?: HmrOptions;
  target?:
    | "es2015"
    | "es2016"
    | "es2017"
    | "es2018"
    | "es2019"
    | "es2020"
    | "es2021"
    | "es2022";
  jsxImportSource?: string;
  minify?: boolean;
  keepNames?: boolean;
  treeShaking?: boolean;
  versionMap?: Record<string, string>;
  globalVersion?: string;
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
