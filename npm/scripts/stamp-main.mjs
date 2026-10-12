// Stamp the release version into npm/package.json before publishing (CI only;
// the checked-in file keeps the development version).
// Usage: node npm/scripts/stamp-main.mjs <version>
import { readFileSync, writeFileSync } from "node:fs";

const version = (process.argv[2] ?? "").replace(/^v/, "");
if (!/^\d+\.\d+\.\d+/.test(version)) {
  console.error("usage: stamp-main.mjs <version> (e.g. 0.1.0)");
  process.exit(2);
}

const pkg = JSON.parse(readFileSync("npm/package.json", "utf8"));
pkg.version = version;
for (const name of Object.keys(pkg.optionalDependencies)) {
  pkg.optionalDependencies[name] = version;
}
writeFileSync("npm/package.json", `${JSON.stringify(pkg, null, 2)}\n`);
