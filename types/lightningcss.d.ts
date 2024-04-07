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

export interface LightningCSSTransformOptions {
  /** Whether to enable minification. */
  minify?: boolean;
  /** Whether to output a source map. */
  sourceMap?: boolean;
  /** The browser targets for the generated code. */
  targets?: Targets;
  /** Features that should always be compiled, even when supported by targets. */
  include?: number;
  /** Features that should never be compiled, even when unsupported by targets. */
  exclude?: number;
  /** Whether to enable parsing various draft syntax. */
  drafts?: Drafts;
  /** Whether to enable various non-standard syntax. */
  nonStandard?: NonStandard;
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
  /**
   * Whether to ignore invalid rules and declarations rather than erroring.
   * When enabled, warnings are returned, and the invalid rule or declaration is
   * omitted from the output code.
   */
  errorRecovery?: boolean;
}

export interface Drafts {
  /** Whether to enable @custom-media rules. */
  customMedia?: boolean;
}

export interface NonStandard {
  /** Whether to enable the non-standard >>> and /deep/ selector combinators used by Angular and Vue. */
  deepSelectorCombinator?: boolean;
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
  readonly exports?: Map<string, CSSModuleExport>;
  /** CSS module references, if `dashedIdents` is enabled. */
  readonly references?: Map<string, DependencyCSSModuleReference>;
  /** `@import` and `url()` dependencies, if enabled. */
  readonly dependencies?: Dependency[];
}

export interface Warning {
  message: string;
  type: string;
  value?: any;
  loc: ErrorLocation;
}

export interface CSSModulesConfig {
  /** The pattern to use when renaming class names and other identifiers. Default is `[hash]_[local]`. */
  pattern?: string;
  /** Whether to rename dashed identifiers, e.g. custom properties. */
  dashedIdents?: boolean;
}

export interface CSSModuleExport {
  /** The local (compiled) name for this export. */
  readonly name: string;
  /** Whether the export is referenced in this file. */
  readonly isReferenced: boolean;
  /** Other names that are composed by this export. */
  readonly composes: CSSModuleReference[];
}

export type CSSModuleReference = LocalCSSModuleReference | GlobalCSSModuleReference | DependencyCSSModuleReference;

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
  /** The placeholder that the import was replaced with. */
  readonly placeholder: string;
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

export interface ErrorLocation extends Location {
  filename: string;
}

export const Features: {
  Nesting: 1;
  NotSelectorList: 2;
  DirSelector: 4;
  LangSelectorList: 8;
  IsSelector: 16;
  TextDecorationThicknessPercent: 32;
  MediaIntervalSyntax: 64;
  MediaRangeSyntax: 128;
  CustomMediaQueries: 256;
  ClampFunction: 512;
  ColorFunction: 1024;
  OklabColors: 2048;
  LabColors: 4096;
  P3Colors: 8192;
  HexAlphaColors: 16384;
  SpaceSeparatedColorNotation: 32768;
  FontFamilySystemUi: 65536;
  DoublePositionGradients: 131072;
  VendorPrefixes: 262144;
  LogicalProperties: 524288;
  Selectors: 31;
  MediaQueries: 448;
  Colors: 64512;
};

export function transform(options: { code: string; filename: string } & LightningCSSTransformOptions): LightningCSSTransformResult;
