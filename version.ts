/** `VERSION` managed by https://deno.land/x/publish */
export const VERSION = "0.2.0";

/** `prepublish` will be invoked before publish */
export async function prepublish(version: string): Promise<boolean> {
  Deno.chdir(new URL(".", import.meta.url).pathname);
  const toml = await Deno.readTextFile("./Cargo.toml");
  await Deno.writeTextFile(
    "./Cargo.toml",
    toml.replace(/version = "[\d\.]+"/, `version = "${version}"`),
  );
  const ok = await run(["wasm-pack", "build", "--target", "web"]);
  if (!ok) {
    return false;
  }
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
  return true;
}

/** `postpublish` will be invoked after published */
export async function postpublish(version: string) {
  Deno.chdir("./pkg");
  await run(["npm", "publish"]);
}

async function run(cmd: string[]) {
  const p = Deno.run({
    cmd,
    stdout: "inherit",
    stderr: "inherit",
  });
  const status = await p.status();
  p.close();
  return status.success;
}
