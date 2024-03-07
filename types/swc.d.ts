export type HmrOptions = {
  runtime: string;
  reactRefresh?: boolean;
  reactRefreshRuntime?: string;
};

export type MinifierOptions = {
  compress?: boolean;
  keepNames?: boolean;
};

export type SWCTransformOptions = {
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
};

export type SWCTransformResult = {
  readonly code: string;
  readonly map?: string;
  readonly deps?: DependencyDescriptor[];
};

export type DependencyDescriptor = {
  readonly specifier: string;
  readonly resolvedUrl: string;
  readonly loc?: { start: number; end: number };
  readonly dynamic?: boolean;
};

export function transform(
  filename: string,
  code: string,
  options: SWCTransformOptions,
): SWCTransformResult;
