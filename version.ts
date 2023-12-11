import { test } from "./test.ts";

/** `VERSION` managed by https://deno.land/x/publish */
export const VERSION = "0.3.5";

/** `prepublish` will be invoked before publish */
export async function prepublish(version: string): Promise<boolean> {
  Deno.chdir(new URL(".", import.meta.url).pathname);
  const toml = await Deno.readTextFile("./Cargo.toml");
  await Deno.writeTextFile(
    "./Cargo.toml",
    toml.replace(/version = "[\d\.]+"/, `version = "${version}"`),
  );
  const ok = await run("wasm-pack", "build", "--target", "web");
  if (!ok) {
    return false;
  }
  await test();
  const addonDts = await Deno.readTextFile("./types.d.ts");
  const dts = await Deno.readTextFile("./pkg/esm_compiler.d.ts");
  await Deno.writeTextFile(
    "./pkg/esm_compiler.d.ts",
    dts.replace(
      `swc_options: any): any`,
      `swc_options: SWCOptions): SWCTransformResult`,
    ).replace(
      `lightningcss_config: any): any`,
      `lightningcss_config: LightningCSSConfig): LightningCSSTransformResult`,
    ) + addonDts,
  );
  const wasmStat = await Deno.stat("./pkg/esm_compiler_bg.wasm");
  console.log(
    `wasm size: ${prettyBytes(wasmStat.size)}, gzipped: ${
      prettyBytes(await getGzSize("./pkg/esm_compiler_bg.wasm"))
    }`,
  );
  return true;
}

/** `postpublish` will be invoked after published */
export async function postpublish(version: string) {
  Deno.chdir("./pkg");
  await run("npm", "publish");
}

function prettyBytes(n: number) {
  return (n / 1024 / 1024).toFixed(2) + " MB";
}

async function getGzSize(name: string) {
  const f = await Deno.open(name);
  const c = new CompressionStream("gzip");
  const r = new Response(f.readable).body?.pipeThrough(c).getReader()!;
  let size = 0;
  while (true) {
    const { done, value } = await r.read();
    if (done) {
      break;
    }
    size += value.length;
  }
  return size;
}

async function run(cmd: string, ...args: string[]) {
  const c = new Deno.Command(cmd, {
    args,
    stdout: "inherit",
    stderr: "inherit",
  });
  return await c.spawn().status;
}
