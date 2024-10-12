import { readFile } from "node:fs/promises";
import init, { transform  } from "./pkg/esm_tsx.js";

export const test = async () => {
  const wasmData = await readFile(
    new URL("./pkg/esm_tsx_bg.wasm", import.meta.url),
  );
  await init({ module_or_path: wasmData });

  {
    const source = `
      import { useState } from "react"
      import { createRoot } from "react-dom/client"

      export default function App() {
        const [msg] = useState<string>("world")
        return <h1>Hello {msg}!</h1>
      }
      createRoot(document.getElementById("app")).render(<App />)
    `;
    const { deps, code } = transform("source.tsx", source, {
      importMap: {
        "imports": {
          "react": "https://esm.sh/react@18",
          "react-dom/": "https://esm.sh/react-dom@18/",
        },
      },
    });
    if (!code.includes(`} from "https://esm.sh/react@18/jsx-runtime"`)) {
      throw new Error("jsx-runtime not imported");
    }
    if (!code.includes(`} from "https://esm.sh/react@18"`)) {
      throw new Error("'react' not resolved");
    }
    if (!code.includes(`} from "https://esm.sh/react-dom@18/client"`)) {
      throw new Error("'react-dom/client' not resolved");
    }
    if (!code.includes(`_jsxs("h1"`)) {
      throw new Error("jsx not transformed");
    }
    if (deps?.length !== 3) {
      throw new Error("deps length should be 3");
    }
  }

  // throw syntax error
  {
    try {
      const source = `export App() {}`;
      transform("source.tsx", source);
    } catch (error) {
      if (error.message !== "Expected '{', got 'App' at source.tsx:1:7") {
        throw error;
      }
    }
  }

  console.log("%c✔ test passed", "color: green;");
};

if (import.meta.main || process.argv[1] === new URL(import.meta.url).pathname) {
  await test();
}
