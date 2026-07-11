#!/usr/bin/env python3
import json
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
GATES = ROOT / ".github" / "swift-cutover-gates.json"
ALLOWED_STATUS = {"pending", "passed", "blocked"}


def fail(message):
    print(f"swift cutover gate failed: {message}", file=sys.stderr)
    sys.exit(1)


def main():
    if not GATES.exists():
        fail(f"missing {GATES.relative_to(ROOT)}")

    try:
        data = json.loads(GATES.read_text())
    except json.JSONDecodeError as error:
        fail(f"invalid JSON: {error}")

    if data.get("schemaVersion") != 1:
        fail("schemaVersion must be 1")
    if data.get("targetBranch") != "swift-main":
        fail("targetBranch must be swift-main")

    allowed = data.get("rootPackageMoveAllowed")
    if not isinstance(allowed, bool):
        fail("rootPackageMoveAllowed must be true or false")

    prerequisites = data.get("rootMovePrerequisites")
    if not isinstance(prerequisites, list) or not prerequisites:
        fail("rootMovePrerequisites must be a non-empty list")

    seen = set()
    for gate in prerequisites:
        if not isinstance(gate, dict):
            fail("each prerequisite must be an object")

        gate_id = gate.get("id")
        status = gate.get("status")
        evidence = gate.get("evidence")

        if not gate_id or gate_id in seen:
            fail("each prerequisite needs a unique id")
        seen.add(gate_id)

        if status not in ALLOWED_STATUS:
            fail(f"{gate_id} has invalid status {status!r}")
        if not isinstance(evidence, str) or not evidence.strip():
            fail(f"{gate_id} needs evidence")

    root_package = ROOT / "Package.swift"
    if root_package.exists() and not allowed:
        fail("Package.swift is at repo root while rootPackageMoveAllowed is false")

    if allowed:
        if not root_package.exists():
            fail("rootPackageMoveAllowed is true but Package.swift is not at repo root")

        pending = [gate["id"] for gate in prerequisites if gate["status"] != "passed"]
        if pending:
            fail("root package move allowed with incomplete gates: " + ", ".join(pending))

    print("swift cutover gates OK")


if __name__ == "__main__":
    main()
