#!/usr/bin/env python3
import subprocess
import sys

from runtime_state import hold_database_build_lock


def log(message: str) -> None:
    print(message, file=sys.stderr)


def main() -> int:
    if len(sys.argv) < 3:
        print(
            "Usage: with_database_build_lock.py COMPONENT COMMAND [ARG ...]",
            file=sys.stderr,
        )
        return 2

    component = sys.argv[1]
    command = sys.argv[2:]

    with hold_database_build_lock(component, log):
        completed = subprocess.run(command, check=False)
        return completed.returncode


if __name__ == "__main__":
    raise SystemExit(main())
