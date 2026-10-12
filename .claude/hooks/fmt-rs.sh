#!/usr/bin/env bash
# PostToolUse hook: cargo fmt whenever a .rs file was just edited/created.
# Best-effort: formatting failure never blocks; a malformed payload surfaces (exit 1).
# exec propagates python's exit code (0 ok, 1 malformed payload).
exec python -c '
import json, sys, os, subprocess
try:
    data = json.load(sys.stdin)
except json.JSONDecodeError as e:
    print(f"fmt-rs hook: malformed payload ({e})", file=sys.stderr)
    sys.exit(1)
path = (data.get("tool_input") or {}).get("file_path", "")
if path.endswith(".rs"):
    manifest = os.path.join(os.environ.get("CLAUDE_PROJECT_DIR", "."), "Cargo.toml")
    subprocess.run(["cargo", "fmt", "--manifest-path", manifest])
'
