import { readFile, writeFile } from "node:fs/promises";
import { spawn } from "node:child_process";

async function build() {
  const p = spawn("wasm-pack", ["build", "--target", "web", "--no-pack", "--release"]);
  p.stdout.pipe(process.stdout);
  p.stderr.pipe(process.stderr);
  await new Promise((resolve, reject) => {
    p.on("exit", (code) => {
      if (code === 0) {
        resolve();
      } else {
        reject(new Error(`wasm-pack build failed with code ${code}`));
      }
    });
  });

  const { test } = await import("./test.mjs");
  await test();

  const dts = await readFile("./pkg/esm_compiler.d.ts", "utf8");
  await writeFile(
    "./pkg/esm_compiler.d.ts",
    [
      `import { SWCTransformOptions, SWCTransformResult } from "../types/swc.d.ts";`,
      `import { LightningCSSTransformOptions, LightningCSSTransformResult } from "../types/lightningcss.d.ts";`,
      dts.replace(
        `swc_transform_options: any): any`,
        `swc_transform_options: SWCTransformOptions): SWCTransformResult`,
      ).replace(
        `lightningcss_transform_options: any): any`,
        `lightningcss_transform_options: LightningCSSTransformOptions): LightningCSSTransformResult`,
      ),
    ].join("\n"),
    "utf8",
  );
}

await build();
