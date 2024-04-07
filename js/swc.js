import { transform as swc } from "../pkg/esm_compiler.js";

export function transform({ filename, code, importMap, ...options }) {
  if (typeof filename !== "string" || filename.length === 0) {
    throw new Error("filename is required");
  }
  if (typeof code !== "string") {
    throw new Error("code is required");
  }
  if (importMap) {
    if (typeof importMap === "object" && !Array.isArray(importMap)) {
      options.importMap = JSON.stringify(importMap);
    } else if (typeof importMap === "string" && importMap.startsWith("{") && importMap.endsWith("}")) {
      options.importMap = importMap;
    } else {
      throw new Error("invalid importMap");
    }
  }
  return swc(filename, code, options);
}
