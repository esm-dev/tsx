/* for swc */

export type HmrOptions = {
  runtimeJsUrl: string;
  reactRefresh?: boolean;
};

export type MinifierOptions = {
  compress?: boolean;
  keepNames?: boolean;
};

export type SWCOptions = {
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
  minify?: MinifierOptions;
  sourceMap?: boolean;
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
  readonly importUrl: string;
  readonly loc?: { start: number; end: number; ctxt: number };
  readonly dynamic?: boolean;
};

/* for lightningcss */

export interface Targets {
  android?: number;
  chrome?: number;
  edge?: number;
  firefox?: number;
  ie?: number;
  ios_saf?: number;
  opera?: number;
  safari?: number;
  samsung?: number;
}

export interface DependencyOptions {
  removeImports: boolean;
}

export interface LightningCSSConfig {
  /** Whether to enable minification. */
  minify?: boolean;
  /** Whether to output a source map. */
  sourceMap?: boolean;
  /** The browser targets for the generated code. */
  targets?: Targets;
  /** Whether to compile this file as a CSS module. */
  cssModules?: boolean | CSSModulesConfig;
  /**
   * Whether to analyze dependencies (e.g. `@import` and `url()`).
   * When enabled, `@import` rules are removed, and `url()` dependencies
   * are replaced with hashed placeholders that can be replaced with the final
   * urls later (after bundling). Dependencies are returned as part of the result.
   */
  analyzeDependencies?: DependencyOptions;
  /**
   * Replaces user action pseudo classes with class names that can be applied from JavaScript.
   * This is useful for polyfills, for example.
   */
  pseudoClasses?: PseudoClasses;
  /**
   * A list of class names, ids, and custom identifiers (e.g. @keyframes) that are known
   * to be unused. These will be removed during minification. Note that these are not
   * selectors but individual names (without any . or # prefixes).
   */
  unusedSymbols?: string[];
}

export interface PseudoClasses {
  hover?: string;
  active?: string;
  focus?: string;
  focusVisible?: string;
  focusWithin?: string;
}

export interface LightningCSSTransformResult {
  /** The transformed code. */
  readonly code: string;
  /** The generated source map, if enabled. */
  readonly map?: string;
  /** CSS module exports, if enabled. */
  readonly exports?: CSSModuleExports;
  /** `@import` and `url()` dependencies, if enabled. */
  readonly dependencies?: Dependency[];
}

export interface CSSModulesConfig {
  /** The pattern to use when renaming class names and other identifiers. Default is `[hash]_[local]`. */
  pattern?: string;
  /** Whether to rename dashed identifiers, e.g. custom properties. */
  dashedIdents?: boolean;
}

export type CSSModuleExports = {
  /** Maps exported (i.e. original) names to local names. */
  readonly [name: string]: CSSModuleExport;
};

export interface CSSModuleExport {
  /** The local (compiled) name for this export. */
  readonly name: string;
  /** Whether the export is referenced in this file. */
  readonly isReferenced: boolean;
  /** Other names that are composed by this export. */
  readonly composes: CSSModuleReference[];
}

export type CSSModuleReference =
  | LocalCSSModuleReference
  | GlobalCSSModuleReference
  | DependencyCSSModuleReference;

export interface LocalCSSModuleReference {
  readonly type: "local";
  /** The local (compiled) name for the reference. */
  readonly name: string;
}

export interface GlobalCSSModuleReference {
  readonly type: "global";
  /** The referenced global name. */
  readonly name: string;
}

export interface DependencyCSSModuleReference {
  readonly type: "dependency";
  /** The name to reference within the dependency. */
  readonly name: string;
  /** The dependency specifier for the referenced file. */
  readonly specifier: string;
}

export type Dependency = ImportDependency | UrlDependency;

export interface ImportDependency {
  readonly type: "import";
  /** The url of the `@import` dependency. */
  readonly url: string;
  /** The media query for the `@import` rule. */
  readonly media: string | null;
  /** The `supports()` query for the `@import` rule. */
  readonly supports: string | null;
  /** The source location where the `@import` rule was found. */
  readonly loc: SourceLocation;
}

export interface UrlDependency {
  readonly type: "url";
  /** The url of the dependency. */
  readonly url: string;
  /** The source location where the `url()` was found. */
  readonly loc: SourceLocation;
  /** The placeholder that the url was replaced with. */
  readonly placeholder: string;
}

export interface SourceLocation {
  /** The file path in which the dependency exists. */
  readonly filePath: string;
  /** The start location of the dependency. */
  readonly start: Location;
  /** The end location (inclusive) of the dependency. */
  readonly end: Location;
}

export interface Location {
  /** The line number (1-based). */
  readonly line: number;
  /** The column number (0-based). */
  readonly column: number;
}
