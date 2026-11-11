"""Format generated closures at many widths and indentations; report errors, lost comments and output
that changes on a second pass.

Usage: `cargo build --release -p apfmt && python3 tests/sweep.py [path/to/apfmt]`; exits with 1 on a
failure.

Closures in calls, `let` and match arms, with different bodies, with and without braces. A bug that
depends on a line landing in a narrow range of widths may show in no project of `tests/scan.py`.
"""

import collections
import itertools
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

DEFAULT_APFMT = Path(__file__).resolve().parent.parent / "target" / "release" / "apfmt"
APFMT_ARGS = ["--no-config", "--edition", "2024", "--unsupported", "allow"]
TIMEOUT = 60
THREADS = 14
SHOWN_GROUPS = 40
SHOWN_CASES = 6

# The body of the closure; {L} is the line whose width varies
BODIES = {
    "match_tuple": "match (opt_str, opt_path) {{\n(Some(a), Some(b)) => compile(path).map_err(|e| {{ {L} }}).map(|p| g(&p)),\n_ => Ok(None),\n}}",
    "match_call": "match compute(opt_str, opt_path) {{\nSome(a) => {{ {L} }}\n_ => Ok(None),\n}}",
    "match_long_call": "match compute_something_long(first_argument_value, second_argument_value, third_argument_value, fourth) {{\nSome(a) => {{ {L} }}\n_ => Ok(None),\n}}",
    "match_chain": "match self.field.compute(opt_str).unwrap_or_default() {{\nSome(a) => {{ {L} }}\n_ => Ok(None),\n}}",
    "match_try": "match compute(opt_str, opt_path)? {{\nSome(a) => {{ {L} }}\n_ => Ok(None),\n}}",
    "match_comment": "match compute(opt_str) {{\n// c\nSome(a) => {{ {L} }}\n_ => Ok(None), // d\n}}",
    "unsafe": "unsafe {{\nlet a = 1;\n{L}\n}}",
    "record": "Foo {{\nfield: {L},\nother: 2,\n}}",
    "call": "compute(opt_str, {L})",
    "chain": "opt_str.compute({L}).unwrap()",
    "if": "if opt_str {{ {L} }} else {{ 2 }}",
    "async": "async move {{\nlet a = 1;\n{L}\n}}",
}
# What is around the body; {B} is the body
WRAPS = {
    "plain": "{B}",
    "braces": "{{ {B} }}",
    "nested_braces": "{{ {{ {B} }} }}",
    "comment_braces": "{{\n// c\n{B}\n}}",
}
# The statement with the closure; {C} is the wrapped body
CALLS = {
    "hug_two": "broadcast_try_binary(ca, json_path, |opt_str, opt_path| {C})",
    "hug_short": "f(a, |opt_str, opt_path| {C})",
    "hug_wide": 'setup_long_function_name_here("a_rather_long_literal_argument_value_here", |opt_str, opt_path| {C})',
    "single": "f(|opt_str, opt_path| {C})",
    "single_nested": "f(g(|opt_str, opt_path| {C}))",
    "let": "let closure = |opt_str, opt_path| {C};",
    "method": "receiver.method(a, |opt_str, opt_path| {C})",
    "chain_two": "receiver.first().method(a, |opt_str, opt_path| {C}).last()",
    "macro": "m!(a, |opt_str, opt_path| {C})",
    "move_ret": "f(a, move |opt_str, opt_path| -> R {{ {C} }})",
    "arm": "match q {{ A => f(a, b, |opt_str, opt_path| {C}), _ => 1 }}",
}
# The length of the string in the line that varies, and the levels of blocks around the statement
WIDTHS = [1, 20, 30, 34, 38, 42, 46, 50, 54, 58, 62, 66, 70, 74, 78, 90]
DEPTHS = [0, 1, 3, 5]


def source(call: str, wrap: str, body: str, width: int, depth: int) -> str:
    line = f'polars_err!(ComputeError: "{"x" * width}", e)'
    closure = WRAPS[wrap].format(B=BODIES[body].format(L=line))
    statement = CALLS[call].format(C=closure)
    return "fn f() {\n" + "{\n" * depth + statement + "\n" + "}\n" * depth + "}\n"


def count_comments(text: str) -> int:
    return text.count("// c") + text.count("// d")


def check(apfmt: str, case: tuple) -> tuple:
    def format_code(code: str) -> tuple[int, str, str]:
        p = subprocess.run(
            [apfmt, *APFMT_ARGS],
            input=code,
            text=True,
            capture_output=True,
            timeout=TIMEOUT,
            check=False,
        )
        return p.returncode, p.stdout, p.stderr

    code = source(*case)
    rc, first, err = format_code(code)
    if rc != 0:
        return "error", case, err[:160]
    rc, second, err = format_code(first)
    if rc != 0:
        return "second_error", case, err[:160]
    if count_comments(first) != count_comments(code):
        return "lost_comment", case, ""
    if first != second:
        return "not_idempotent", case, ""
    return "ok", case, ""


def main() -> int:
    apfmt = sys.argv[1] if len(sys.argv) > 1 else str(DEFAULT_APFMT)
    cases = list(itertools.product(CALLS, WRAPS, BODIES, WIDTHS, DEPTHS))
    with ThreadPoolExecutor(THREADS) as pool:
        results = list(pool.map(lambda case: check(apfmt, case), cases))
    print(len(cases), "cases:", dict(collections.Counter(r[0] for r in results)))
    failures = [r for r in results if r[0] != "ok"]
    groups = collections.Counter((r[0], *r[1][:3]) for r in failures)
    for group, count in groups.most_common(SHOWN_GROUPS):
        print(count, *group)
    for failure in failures[:SHOWN_CASES]:
        print(failure)
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
