import { readFile, writeFile } from "node:fs/promises";
import { spawn } from "node:child_process";

async function build() {
  const pkgJson = JSON.parse(await readFile("./package.json", "utf8"));
  const cargoToml = await readFile("./Cargo.toml", "utf8");
  await writeFile(
    "./Cargo.toml",
    cargoToml.replace(/version = "[\d\.]+"/, `version = "${pkgJson.version}"`),
    "utf8",
  );

  const p = spawn("wasm-pack", ["build", "--target", "web"]);
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
      dts.replace(
        `swc_transform_options: any): any`,
        `swc_transform_options: SWCTransformOptions): SWCTransformResult`,
      ),
    ].join("\n"),
    "utf8",
  );
}

await build();
