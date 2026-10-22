// Assemble one npm platform package from a built Rust target directory.
// Usage (CI or local): node npm/scripts/make-platform-package.mjs <rust-target-triple>
// Prints the assembled package directory. Version comes from $VERSION ("v" prefix allowed).
import { copyFileSync, mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { TARGETS } from "./targets.mjs";

const target = process.argv[2];
const t = TARGETS[target];
if (!t) {
  console.error(`unknown target '${target}' — expected one of: ${Object.keys(TARGETS).join(", ")}`);
  process.exit(2);
}
const version = (process.env.VERSION ?? "0.0.0").replace(/^v/, "");

const exe = t.os[0] === "win32" ? "sastql.exe" : "sastql";
// CI builds with --target <triple>; a plain local `cargo build` lands in target/release.
const src = join("target", target, "release", exe);
const fallback = join("target", "release", exe);
const out = join("npm", "pkgs", t.pkg);

mkdirSync(join(out, "bin"), { recursive: true });
try {
  copyFileSync(src, join(out, "bin", exe));
} catch {
  copyFileSync(fallback, join(out, "bin", exe));
}
copyFileSync("LICENSE", join(out, "LICENSE"));
copyFileSync("npm/PLATFORM_README.md", join(out, "README.md"));

const manifest = {
  name: t.pkg,
  version,
  description: `SastQL binary (${target})`,
  license: "SEE LICENSE IN LICENSE",
  // npm cross-checks this against the publishing workflow's repo (OIDC/provenance).
  repository: {
    type: "git",
    url: "git+https://github.com/CLVwp/SastQL.git",
  },
  os: t.os,
  cpu: t.cpu,
  ...(t.libc ? { libc: t.libc } : {}),
};
writeFileSync(join(out, "package.json"), `${JSON.stringify(manifest, null, 2)}\n`);
console.log(out);
