import { readFile } from "node:fs/promises";
import init, { transform, transformCSS } from "./pkg/esm_compiler.js";

export const test = async () => {
  const wasmData = await readFile(
    new URL("./pkg/esm_compiler_bg.wasm", import.meta.url),
  );
  await init(wasmData);

  // test css transform with css modules and nesting draft
  {
    const source = `
      .foo {
        color: red;

        &.bar {
          color: green
        }
      }
    `;
    const { exports, code } = transformCSS("source.module.css", source, {
      cssModules: true,
      targets: {
        chrome: 95 << 16,
      },
    });
    if (exports.size !== 2) {
      throw new Error("css modules should be enabled");
    }
    if (code.includes(".foo.bar{")) {
      throw new Error("css nesting should be downgraded");
    }
  }

  // test jsx transform
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
          "@jsxImportSource": "https://esm.sh/react@18",
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

  // minify & tree-shaking
  {
    const source = `
      import React from "react"
      let foo = "bar"
    `;
    const { code } = transform("source.tsx", source, {
      importMap: {
        "imports": {
          "react": "https://esm.sh/react@18",
        },
      },
      minify: true,
      treeShaking: true,
    });
    if (code !== `import"https://esm.sh/react@18";`) {
      throw new Error("want minified code, got:"+ JSON.stringify(code));
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
