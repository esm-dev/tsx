import { initSync, transform, transformCSS } from "../pkg/esm_compiler.js";
import wasm from "../pkg/esm_compiler_bg.wasm";

initSync(wasm);

export default function init() {}
export { transform, transformCSS };
