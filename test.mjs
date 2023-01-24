import init, { transform } from "./pkg/esm_compiler.js";

const wasmData = await Deno.readFile("./pkg/esm_compiler_bg.wasm");
await init(wasmData);

const result = transform(
  "index.tsx",
  [
    `import React from "react";`,
    `import { renderToString } from "react-dom/server";`,
    `const msg:string = "Hello world";`,
    `renderToString(<p>{msg}</p>)`
  ].join("\n"),
  { importMap: `{ "imports": { "react": "https://esm.sh/react@18", "react-dom/": "https://esm.sh/react-dom@18/" } }` }
);
if (!result.code.includes(`import React from "https://esm.sh/react@18"`)) {
  throw new Error("dep `react` should be replaced");
}
if (!result.code.includes(`import { renderToString } from "https://esm.sh/react-dom@18/server"`)) {
  throw new Error("dep `react-dom` should be replaced");
}
if (!result.code.includes(`React.createElement("p", null, msg)`)) {
  throw new Error("React.createElement should be used");
}
if (result.deps.length !== 2) {
  throw new Error("deps length should be 2");
}
console.log("%c✔ test passed", "color: green;");
