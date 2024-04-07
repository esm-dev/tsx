export interface HmrOptions {
  runtime: string;
  reactRefresh?: boolean;
  reactRefreshRuntime?: string;
}

export interface MinifierOptions {
  compress?: boolean;
  keepNames?: boolean;
}

export interface SWCTransformOptions {
  lang?: "ts" | "tsx" | "js" | "jsx";
  target?:
    | "es2015"
    | "es2016"
    | "es2017"
    | "es2018"
    | "es2019"
    | "es2020"
    | "es2021"
    | "es2022";
  importMap?: string;
  isDev?: boolean;
  hmr?: HmrOptions;
  jsxFactory?: string;
  jsxFragmentFactory?: string;
  jsxImportSource?: string;
  minify?: boolean | MinifierOptions;
  sourceMap?: "inline" | "external";
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
