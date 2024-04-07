import { transform as swc } from "../pkg/esm_compiler.js";

export function transform({ filename, code, ...options }) {
  if (typeof filename !== "string" || filename.length === 0) {
    throw new Error("filename is required");
  }
  if (typeof code !== "string") {
    throw new Error("code is required");
  }
  if (options.importMap) {
    if (typeof options.importMap === "object" && !Array.isArray(options.importMap)) {
      options.importMap = JSON.stringify(options.importMap);
    } else if (typeof options.importMap !== "string") {
      throw new Error("importMap should be an object or a string");
    }
  }
  return swc(filename, code, options);
}
