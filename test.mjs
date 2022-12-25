import init, { transform } from "./pkg/esm_compiler.js";

const wasmData = await Deno.readFile("./pkg/esm_compiler_bg.wasm");
await init(wasmData);

const result = transform("index.tsx", `import React from "https://esm.sh/react@18";const num:number = 1; console.log(<em>{num}</em>)`, {});
console.log(result);
