import { transform as swc } from "./pkg/esm_compiler.js";

export function transform({ filename, code, ...options }) {
  if (typeof filename !== "string" || filename === "") {
    throw new Error("filename is required");
  }
  if (typeof code !== "string") {
    throw new Error("code is required");
  }
  const { importMap } = options;
  if (importMap !== undefined && !(typeof importMap === "object" && importMap !== null && !Array.isArray(importMap))) {
    throw new Error("invalid importMap");
  }
  return swc(filename, code, options);
}
