// Rust target triple → npm platform package. Single source of truth for the
// target list: make-platform-package.mjs builds from it, stamp-main.mjs
// derives the launcher's "supported" list from it.
// Adding a target: add it here AND npm/package.json optionalDependencies AND
// the release.yml matrix.
export const TARGETS = {
  "x86_64-pc-windows-msvc": { pkg: "sastql-win32-x64", os: ["win32"], cpu: ["x64"] },
  "x86_64-unknown-linux-gnu": {
    pkg: "sastql-linux-x64-gnu",
    os: ["linux"],
    cpu: ["x64"],
    libc: ["glibc"],
  },
  "x86_64-unknown-linux-musl": {
    pkg: "sastql-linux-x64-musl",
    os: ["linux"],
    cpu: ["x64"],
    libc: ["musl"],
  },
  "aarch64-unknown-linux-gnu": {
    pkg: "sastql-linux-arm64-gnu",
    os: ["linux"],
    cpu: ["arm64"],
    libc: ["glibc"],
  },
  "aarch64-apple-darwin": { pkg: "sastql-darwin-arm64", os: ["darwin"], cpu: ["arm64"] },
  "x86_64-apple-darwin": { pkg: "sastql-darwin-x64", os: ["darwin"], cpu: ["x64"] },
};
