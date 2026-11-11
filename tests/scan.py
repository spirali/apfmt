"""Run apfmt over every .rs file under the given roots and record what goes wrong, as JSON lines.

Usage: `cargo build --release -p apfmt && python3 tests/scan.py ~/sources/*/ > scan.jsonl`

For each file: `apfmt --result-check` (status `ok`, `error`, `panic`, `syntax-error` or `timeout`),
the syntax copied unchanged, idempotency (a second run), and a comparison with rustfmt (default
config, edition 2024): identical output and the number of lines over 100 columns.
"""

import json
import re
import subprocess
import sys
from concurrent.futures import ProcessPoolExecutor
from pathlib import Path

APFMT = str(Path(__file__).resolve().parent.parent / "target" / "release" / "apfmt")
TIMEOUT = 30
MAX_WIDTH = 100
UNSUPPORTED = re.compile(
    r":(\d+):(\d+): not supported yet: (.*?)(?: \(copied unchanged\))?: (.*)$"
)


def run(cmd, code, timeout=TIMEOUT):
    try:
        p = subprocess.run(
            cmd, check=False, input=code, capture_output=True, timeout=timeout
        )
        return p.returncode, p.stdout, p.stderr.decode(errors="replace")
    except subprocess.TimeoutExpired:
        return None, b"", "timeout"


def long_lines(text: bytes) -> int:
    return sum(
        len(line) > MAX_WIDTH for line in text.decode(errors="replace").splitlines()
    )


def check(path: str) -> dict:
    code = Path(path).read_bytes()
    r = {"path": path, "lines": code.count(b"\n")}
    rc, out, err = run([APFMT, "--result-check"], code)
    if rc is None:
        r["status"] = "timeout"
        return r
    if rc != 0:
        if "panicked" in err:
            r["status"] = "panic"
            m = re.search(r"panicked at (.*?):\n(.*)", err)
            r["detail"] = f"{m.group(1)}: {m.group(2)}" if m else err[-300:]
        elif "Syntax" in err or "syntax error" in err.lower():
            r["status"] = "syntax-error"
            r["detail"] = err[:300]
        else:
            r["status"] = "error"
            r["detail"] = err[:600]
        return r
    r["status"] = "ok"
    unsupported = []
    for line in err.splitlines():
        m = UNSUPPORTED.search(line)
        if m:
            unsupported.append([m.group(3), int(m.group(1)), m.group(4)[:80]])
        elif line.strip():
            r.setdefault("stderr", []).append(line[:200])
    r["unsupported"] = unsupported
    rc2, out2, err2 = run([APFMT, "--unsupported", "allow"], out)
    r["idempotent"] = rc2 == 0 and out2 == out
    if rc2 != 0:
        r["second_error"] = err2[:300]
    rc3, rout, _ = run(
        ["rustfmt", "--edition", "2024", "--emit", "stdout", "--quiet"], code
    )
    if rc3 == 0:
        r["same_as_rustfmt"] = rout == out
        r["long_apfmt"] = long_lines(out)
        r["long_rustfmt"] = long_lines(rout)
        r["long_source"] = long_lines(code)
    else:
        r["rustfmt_failed"] = True
    return r


def main():
    roots = sys.argv[1:]
    files = sorted(
        str(p)
        for root in roots
        for p in Path(root).rglob("*.rs")
        if p.is_file() and "/target/" not in str(p) and "/.git/" not in str(p)
    )
    print(f"{len(files)} files", file=sys.stderr)
    with ProcessPoolExecutor(16) as pool:
        for i, r in enumerate(pool.map(check, files, chunksize=4)):
            print(json.dumps(r), flush=True)
            if i % 500 == 0:
                print(i, file=sys.stderr, flush=True)


if __name__ == "__main__":
    main()
