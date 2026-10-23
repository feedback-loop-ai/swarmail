#!/usr/bin/env python3
"""Apply the tooling record's exact patch bytes to its clean source commit."""

import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys


class Refusal(Exception):
    """An input or Git operation cannot establish the recorded source."""


def strict_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise Refusal(f"duplicate JSON key {key!r}")
        result[key] = value
    return result


def non_json_constant(value):
    raise Refusal(f"non-JSON constant {value!r}")


def git(source, *arguments, data=None):
    result = subprocess.run(
        ["git", "-C", str(source), *arguments],
        input=data,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if result.returncode:
        detail = result.stderr.decode("utf-8", errors="replace").strip()
        raise Refusal(f"git {' '.join(arguments)} failed: {detail}")
    return result.stdout


def recorded_patches(bundle, entries):
    if not isinstance(entries, list):
        raise Refusal("brokkr.patches must be an array")
    patches = []
    seen = set()
    for entry in entries:
        if not isinstance(entry, dict) or set(entry) != {"path", "sha256"}:
            raise Refusal("each patch must contain exactly path and sha256")
        relative, expected = entry["path"], entry["sha256"]
        if (
            not isinstance(relative, str)
            or "\\" in relative
            or "\x00" in relative
            or any(part in ("", ".", "..") for part in relative.split("/"))
            or len(relative.split("/")) < 2
            or relative.split("/")[0] != "tooling-patches"
            or Path(relative).is_absolute()
        ):
            raise Refusal("patch path must stay beneath tooling-patches without traversal")
        if relative in seen:
            raise Refusal(f"duplicate patch path {relative!r}")
        seen.add(relative)
        if not isinstance(expected, str) or not re.fullmatch(r"[0-9a-f]{64}", expected):
            raise Refusal(f"patch {relative!r} needs a lowercase sha256")
        allowed = bundle / "tooling-patches"
        if bundle.resolve(strict=True) != bundle or allowed.resolve(strict=True) != allowed:
            raise Refusal("patch directory must not escape through a symlink")
        path = (bundle / relative).resolve(strict=True)
        if not path.is_relative_to(allowed) or not path.is_file():
            raise Refusal(f"patch {relative!r} escapes tooling-patches or is not a file")
        # Read once. Both Git invocations below receive this same verified
        # buffer over stdin, rather than reopening an alterable pathname.
        contents = path.read_bytes()
        if hashlib.sha256(contents).hexdigest() != expected:
            raise Refusal(f"patch {relative!r} sha256 does not match the tooling record")
        patches.append((relative, expected, contents))
    return patches


def apply(source):
    bundle = Path(__file__).resolve().parent.parent / "bundles" / "self"
    record = json.loads(
        (bundle / "tooling.json").read_bytes(),
        object_pairs_hook=strict_object,
        parse_constant=non_json_constant,
    )
    if not isinstance(record, dict) or not isinstance(record.get("brokkr"), dict):
        raise Refusal("tooling record must contain a brokkr object")
    tooling = record["brokkr"]
    expected = tooling.get("sha")
    if not isinstance(expected, str) or not re.fullmatch(r"[0-9a-f]{40}", expected):
        raise Refusal("brokkr.sha must be a lowercase 40-character commit SHA")
    source = source.resolve(strict=True)
    if not source.is_dir():
        raise Refusal("source checkout must be a directory")
    top = Path(git(source, "rev-parse", "--show-toplevel").decode().strip()).resolve()
    if top != source:
        raise Refusal("source argument must name the checkout root")
    actual = git(source, "rev-parse", "HEAD").decode().strip()
    if actual != expected:
        raise Refusal(f"source HEAD {actual} does not match recorded {expected}")
    if git(source, "status", "--porcelain=v1", "--untracked-files=no"):
        raise Refusal("source tracked tree or index is dirty")
    # Validate every entry and digest before the first source mutation.
    patches = recorded_patches(bundle, tooling.get("patches", []))
    applied = []
    for relative, digest, contents in patches:
        try:
            git(source, "apply", "--check", "-", data=contents)
            git(source, "apply", "-", data=contents)
        except Refusal as error:
            raise Refusal(f"patch {relative!r}: {error}") from error
        applied.append({"path": relative, "sha256": digest})
    print(json.dumps({"base_sha": actual, "applied_patches": applied}, sort_keys=True))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path, help="clean checkout of the recorded brokkr commit")
    arguments = parser.parse_args()
    try:
        apply(arguments.source)
    except (Refusal, OSError, ValueError, UnicodeError) as error:
        print(f"apply-brokkr-patches: refusing: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
