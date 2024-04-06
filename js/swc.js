import { transform as swc } from "../pkg/esm_compiler.js";

export function transform({ filename, code, ...options }) {
  if (typeof filename !== "string" || filename.length === 0) {
    throw new Error("filename is required");
  }
  if (typeof code !== "string") {
    throw new Error("code is required");
  }
  return swc(filename, code, options);
}
