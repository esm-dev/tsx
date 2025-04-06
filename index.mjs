import initWasm, { initSync as initWasmSync, transform as wasmTransform } from "./pkg/tsx.js";

export function transform(options) {
  const { filename, code, importMap } = options;
  if (typeof filename !== "string" || filename === "") {
    throw new Error("filename is required");
  }
  if (typeof code === "string") {
    options.code = new TextEncoder().encode(code);
  }
  if (!(options.code instanceof Uint8Array)) {
    throw new Error("code is required");
  }
  if (importMap !== undefined && !(typeof importMap === "object" && importMap !== null && !Array.isArray(importMap))) {
    throw new Error("invalid importMap");
  }
  return wasmTransform(options);
}

export function initSync(module) {
  return initWasmSync({ module });
}

export async function init(module_or_path) {
  const importUrl = import.meta.url;
  if (!module_or_path && importUrl.startsWith("file://") && globalThis.Deno) {
    const wasmUrl = new URL("./pkg/tsx_bg.wasm", importUrl);
    const wasmBytes = await Deno.readFile(wasmUrl);
    initWasmSync({ module: wasmBytes });
    return;
  }
  const esmshBaseUrl = "https://esm.sh/@esm.sh/tsx@";
  if (!module_or_path && importUrl.startsWith(esmshBaseUrl)) {
    const version = importUrl.slice(esmshBaseUrl.length).split("/", 1)[0];
    module_or_path = esmshBaseUrl + version + "/pkg/tsx_bg.wasm";
  }
  return initWasm(module_or_path ? { module_or_path } : undefined);
}

export default init;
