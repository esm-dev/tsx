import { transformCSS } from "./pkg/esm_compiler.js";

export const Features = {
  Nesting: 1,
  NotSelectorList: 2,
  DirSelector: 4,
  LangSelectorList: 8,
  IsSelector: 16,
  TextDecorationThicknessPercent: 32,
  MediaIntervalSyntax: 64,
  MediaRangeSyntax: 128,
  CustomMediaQueries: 256,
  ClampFunction: 512,
  ColorFunction: 1024,
  OklabColors: 2048,
  LabColors: 4096,
  P3Colors: 8192,
  HexAlphaColors: 16384,
  SpaceSeparatedColorNotation: 32768,
  FontFamilySystemUi: 65536,
  DoublePositionGradients: 131072,
  VendorPrefixes: 262144,
  LogicalProperties: 524288,
  Selectors: 31,
  MediaQueries: 448,
  Colors: 64512,
};

export function transform({ filename, code, ...rest }) {
  if (!code) {
    throw new Error("code is required");
  }
  if (code instanceof Uint8Array) {
    code = new TextDecoder().decode(code);
  } else if (typeof code !== "string") {
    code = code.toString();
  }
  return transformCSS(filename, code, rest);
}
