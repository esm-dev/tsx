import { readFile } from "node:fs/promises";
import init, { transform } from "./pkg/tsx.js";

async function load() {
  const wasmData = await readFile(
    new URL("./pkg/tsx_bg.wasm", import.meta.url),
  );
  await init({ module_or_path: wasmData });

  console.log("%c✔ wasm loaded: " + (wasmData.byteLength / 1024 / 1024).toFixed(2) + "MB", "color: green;");
}

async function test() {
  const source = `
    import { createRoot } from "react-dom/client"
    import App from "./App.tsx"

    createRoot(document.getElementById("app")).render(<App />)
  `;
  const { code } = transform("/source.tsx", source, {
    importMap: {
      "$src": "file:///index.html",
      "imports": {
        "react": "https://esm.sh/react@18",
        "react-dom": "https://esm.sh/react-dom@18",
      },
    },
    sourceMap: "inline",
    versionMap: {
      "/App.tsx": "2",
    },
  });
  if (!code.includes(`import { jsx as _jsx } from "https://esm.sh/react@18/jsx-runtime"`)) {
    console.log(code)
    throw new Error("jsx-runtime not imported");
  }
  if (!code.includes(`import { createRoot } from "https://esm.sh/react-dom@18/client"`)) {
    console.log(code)
    throw new Error("'react-dom/client' not resolved");
  }
  if (!code.includes(`import App from "/App.tsx?im=L2luZGV4Lmh0bWw&v=2"`)) {
    console.log(code)
    throw new Error("'/App.tsx' not resolved");
  }
  if (!code.includes(`_jsx(App`)) {
    console.log(code)
    throw new Error("jsx not transformed");
  }
  if (!code.includes(`//# sourceMappingURL=data:application/json;charset=utf-8;base64,`)) {
    console.log(code)
    throw new Error("source map not inlined");
  }

  // catch syntax error
  try {
    const source = `export App() {}`;
    transform("source.tsx", source);
  } catch (error) {
    if (error.message !== "Expected '{', got 'App' at source.tsx:1:7") {
      throw error;
    }
  }

  // use `lang` option
  {
    const { code } = transform("/app.vue", `const s: string = "hello"`, { lang: "ts" });
    if (!code.includes(`const s = "hello";`)) {
      throw new Error("lang option not working");
    }
  }

  console.log("%c✔ test passed", "color: green;");
}

if (import.meta.main || process.argv[1] === new URL(import.meta.url).pathname) {
  await load();
  await test();
}
