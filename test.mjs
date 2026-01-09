import { readFile } from "node:fs/promises";
import { CompressionStream } from "node:stream/web";
import init, { transform } from "./pkg/tsx.js";

async function load() {
  const wasmData = await readFile(new URL("./pkg/tsx_bg.wasm", import.meta.url));
  await init({ module_or_path: wasmData });

  let gzSize = 0;
  await new Response(wasmData).body.pipeThrough(new CompressionStream("gzip")).pipeTo(
    new WritableStream({
      write(chunk) {
        gzSize += chunk.byteLength;
      },
    }),
  );

  console.log(
    `%c✔ wasm loaded: ${(wasmData.byteLength / 1024 / 1024).toFixed(2)}MB (gzip: ${Math.ceil(gzSize / 1024)}KB)`,
    "color: green;",
  );
}

async function test() {
  const enc = new TextEncoder();
  const dec = new TextDecoder();

  const source = `
    import { createRoot } from "react-dom/client"
    import App from "./App.tsx"

    createRoot(document.getElementById("app")).render(<App />)
  `;
  const ret = transform({
    filename: "/source.tsx",
    code: enc.encode(source),
    importMap: {
      "$src": "file:///index.html",
      "imports": {
        "react/": "https://esm.sh/react@18/",
        "react-dom/": "https://esm.sh/react-dom@18/",
      },
    },
    sourceMap: "inline",
  });
  const code = dec.decode(ret.code);
  if (!code.includes(`import { jsx as _jsx } from "https://esm.sh/react@18/jsx-runtime"`)) {
    console.log(code);
    throw new Error("jsx-runtime not imported");
  }
  if (!code.includes(`import { createRoot } from "https://esm.sh/react-dom@18/client"`)) {
    console.log(code);
    throw new Error("'react-dom/client' not resolved");
  }
  if (!code.includes(`import App from "/App.tsx"`)) {
    console.log(code);
    throw new Error("'/App.tsx' not resolved");
  }
  if (!code.includes(`_jsx(App`)) {
    console.log(code);
    throw new Error("jsx not transformed");
  }
  if (!code.includes("//# sourceMappingURL=data:application/json;charset=utf-8;base64,")) {
    console.log(code);
    throw new Error("source map not generated");
  }

  // catch syntax error
  try {
    const source = `export App() {}`;
    transform({
      filename: "/source.ts",
      code: enc.encode(source),
    });
  } catch (error) {
    if (error.message !== "Expected '{', got 'ident' at /source.ts:1:7") {
      throw error;
    }
  }

  // use `lang` option
  {
    const ret = transform({
      filename: "/index",
      code: enc.encode(`const s: string = "hello"`),
      lang: "ts",
    });
    const code = dec.decode(ret.code);
    if (!code.includes(`const s = "hello";`)) {
      console.log(code);
      throw new Error("lang option not working");
    }
  }

  console.log("%c✔ test passed", "color: green;");
}

if (import.meta.main || process.argv[1] === new URL(import.meta.url).pathname) {
  await load();
  await test();
}
