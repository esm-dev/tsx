import { readFile } from "node:fs/promises";
import init, { transform } from "./pkg/esm_compiler.js";

export const test = async () => {
  const wasmData = await readFile(
    new URL("./pkg/esm_compiler_bg.wasm", import.meta.url),
  );
  await init(wasmData);

  // test jsx transform
  {
    const source = `
      import { renderToString } from "react-dom/server";
      const msg:string = "Hello world";
      renderToString(<p>{msg}</p>)
    `;
    const { deps, code } = transform("source.tsx", source, {
      importMap: {
        "imports": {
          "@jsxImportSource": "https://esm.sh/react@18",
          "react-dom/server": "https://esm.sh/react-dom@18/server",
        },
      },
    });
    if (!code.includes(`import { jsx as _jsx } from "https://esm.sh/react@18/jsx-runtime"`)) {
      throw new Error("jsx-runtime not imported");
    }
    if (!code.includes(`import { renderToString } from "https://esm.sh/react-dom@18/server"`)) {
      throw new Error("'react-dom' should be replaced");
    }
    if (!code.includes(`_jsx("p`)) {
      throw new Error("jsx not transformed");
    }
    if (deps?.length !== 2) {
      throw new Error("deps length should be 2");
    }
  }

  console.log("%c✔ test passed", "color: green;");
};

if (process.argv[1] === new URL(import.meta.url).pathname) {
  await test();
}
