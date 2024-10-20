import initWasm, { initSync as initWasmSync, transform as swc } from "./pkg/esm_tsx.js";

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

export const modUrl = import.meta.url;

export function initSync(module) {
  return initWasmSync({ module });
}

export default function init(module_or_path) {
  return initWasm(module_or_path ? { module_or_path } : undefined);
}
