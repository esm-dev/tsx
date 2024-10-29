import { readFile } from "node:fs/promises";
import init, { transform } from "./pkg/esm_tsx.js";

async function load() {
  const wasmData = await readFile(
    new URL("./pkg/esm_tsx_bg.wasm", import.meta.url),
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
  const { deps, code } = transform("/source.tsx", source, {
    importMap: {
      "$src": "file:///index.html",
      "imports": {
        "react": "https://esm.sh/react@18",
        "react-dom": "https://esm.sh/react-dom@18",
      },
    },
  });
  if (!code.includes(`import { jsx as _jsx } from "https://esm.sh/react@18/jsx-runtime"`)) {
    throw new Error("jsx-runtime not imported");
  }
  if (!code.includes(`import { createRoot } from "https://esm.sh/react-dom@18/client"`)) {
    throw new Error("'react-dom/client' not resolved");
  }
  if (!code.includes(`import App from "/App.tsx?im=L2luZGV4Lmh0bWw"`)) {
    throw new Error("'/App.tsx' not resolved");
  }
  if (!code.includes(`_jsx(App`)) {
    throw new Error("jsx not transformed");
  }
  if (deps?.length !== 3) {
    throw new Error("deps length should be 3");
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

  console.log("%c✔ test passed", "color: green;");
}

if (import.meta.main || process.argv[1] === new URL(import.meta.url).pathname) {
  await load();
  await test();
}
