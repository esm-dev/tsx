import { initSync, transform } from "../pkg/esm_compiler.js";
import wasm from "../pkg/esm_compiler_bg.wasm";

initSync(wasm);

export function init() {}
export { transform };
