#!/usr/bin/env bash
# PreToolUse hook: Cargo.lock edits and dependency changes need user sign-off.
# CLAUDE.md: lock file committed on purpose (binary), no new dep without justification.
# exec propagates python's exit code: 2 = blocked, 1 = hook error (surfaced, non-blocking).
exec python -c '
import json, sys
try:
    data = json.load(sys.stdin)
except json.JSONDecodeError as e:
    print(f"guard-deps hook: malformed payload ({e})", file=sys.stderr)
    sys.exit(1)
ti = data.get("tool_input") or {}
path = ti.get("file_path", "")
cmd = ti.get("command", "")
if path.endswith("Cargo.lock"):
    print("Cargo.lock is committed on purpose (binary project) - ask the user before touching it.", file=sys.stderr)
    sys.exit(2)
parts = cmd.split()
if any(p == "cargo" and parts[i + 1: i + 2] and parts[i + 1] in ("add", "rm", "remove") for i, p in enumerate(parts)):
    print("Dependency change: CLAUDE.md requires a one-line justification - confirm with the user first.", file=sys.stderr)
    sys.exit(2)
'
