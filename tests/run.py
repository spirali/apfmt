#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.10"
# dependencies = ["rich>=13"]
# ///
"""
Test harness for afmt.

For every .rs file in tests/sources:
  1. format it with `rustfmt` (reference output); if `<name>.expected.rs` exists
     next to it, that file is used as the reference instead (for cases where
     afmt intentionally differs from rustfmt)
  2. format it with `afmt` and compare with the reference
  3. format the afmt output again with `afmt` and check idempotency

Usage:
  uv run tests/run.py                  # run all tests
  uv run tests/run.py basic            # only tests whose path contains "basic"
  uv run tests/run.py 'basic/*main*'   # glob filters are supported too
  uv run tests/run.py --list           # list tests
  uv run tests/run.py -v basic         # for failures also print input, afmt and reference output
"""

import argparse
import difflib
import fnmatch
import shutil
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor, as_completed
from dataclasses import dataclass, field
from pathlib import Path

from rich.console import Console
from rich.progress import (
    BarColumn,
    MofNCompleteColumn,
    Progress,
    TextColumn,
    TimeElapsedColumn,
)
from rich.syntax import Syntax
from rich.text import Text

EXPECTED_SUFFIX = ".expected.rs"

TESTS_DIR = Path(__file__).resolve().parent
ROOT_DIR = TESTS_DIR.parent
SOURCES_DIR = TESTS_DIR / "sources"
CURRENT_DIR = TESTS_DIR / "current"
DEFAULT_AFMT = ROOT_DIR / "target" / "debug" / "afmt"

console = Console()


@dataclass
class Failure:
    kind: str
    message: str
    diff: list[str] = field(default_factory=list)


@dataclass
class Result:
    name: str
    workdir: Path
    failures: list[Failure] = field(default_factory=list)
    outputs: dict[str, str] = field(default_factory=dict)

    @property
    def ok(self) -> bool:
        return not self.failures


class CommandError(Exception):
    pass


def run_cmd(cmd: list[str], stdin: str | None, timeout: float) -> str:
    try:
        proc = subprocess.run(
            cmd, input=stdin, capture_output=True, text=True, timeout=timeout
        )
    except subprocess.TimeoutExpired:
        raise CommandError(f"`{' '.join(cmd)}` timed out after {timeout}s")
    if proc.returncode != 0:
        raise CommandError(
            f"`{' '.join(cmd)}` exited with code {proc.returncode}\n{proc.stderr.rstrip()}"
        )
    return proc.stdout


def run_rustfmt(code: str, args) -> str:
    # stdin mode prints only the formatted code to stdout
    return run_cmd([args.rustfmt, "--edition", args.edition], stdin=code, timeout=args.timeout)


def run_afmt(code: str, args) -> str:
    # like rustfmt, afmt reads from stdin when no file is given
    return run_cmd([str(args.afmt)], stdin=code, timeout=args.timeout)


def make_diff(expected: str, actual: str, a_name: str, b_name: str) -> list[str]:
    return list(
        difflib.unified_diff(
            expected.splitlines(keepends=True),
            actual.splitlines(keepends=True),
            fromfile=a_name,
            tofile=b_name,
        )
    )


def expected_path_for(source: Path) -> Path:
    return source.with_name(source.name.removesuffix(".rs") + EXPECTED_SUFFIX)


def run_test(source: Path, args) -> Result:
    name = source.relative_to(SOURCES_DIR).as_posix()
    result = Result(name, CURRENT_DIR / name.removesuffix(".rs"))
    result.outputs["input.rs"] = code = source.read_text()

    expected_path = expected_path_for(source)
    if expected_path.exists():
        expected_name = "expected.rs"
        expected = expected_path.read_text()
    else:
        expected_name = "rustfmt.rs"
        try:
            expected = run_rustfmt(code, args)
        except CommandError as e:
            result.failures.append(Failure("rustfmt error", str(e)))
            return result
    result.outputs[expected_name] = expected

    try:
        actual = run_afmt(code, args)
    except CommandError as e:
        result.failures.append(Failure("afmt error", str(e)))
        return result
    result.outputs["afmt.rs"] = actual

    if actual != expected:
        result.failures.append(
            Failure(
                "mismatch",
                f"afmt output differs from {expected_name}",
                make_diff(expected, actual, expected_name, "afmt.rs"),
            )
        )
        return result

    try:
        second = run_afmt(actual, args)
    except CommandError as e:
        result.failures.append(Failure("afmt error (2nd pass)", str(e)))
        return result
    result.outputs["afmt2.rs"] = second

    if second != actual:
        result.failures.append(
            Failure(
                "not idempotent",
                "afmt(afmt(input)) differs from afmt(input)",
                make_diff(actual, second, "afmt.rs", "afmt2.rs"),
            )
        )
    return result


def write_outputs(result: Result) -> None:
    result.workdir.mkdir(parents=True, exist_ok=True)
    for filename, content in result.outputs.items():
        (result.workdir / filename).write_text(content)


def clean_current_dir() -> None:
    if not CURRENT_DIR.is_dir():
        return
    for p in CURRENT_DIR.iterdir():
        if p.is_dir() and not p.is_symlink():
            shutil.rmtree(p)


def print_diff(lines: list[str], max_lines: int) -> None:
    shown = lines if max_lines <= 0 else lines[:max_lines]
    for line in shown:
        line = line.rstrip("\n")
        if line.startswith(("+++", "---")):
            style = "bold"
        elif line.startswith("@@"):
            style = "cyan"
        elif line.startswith("+"):
            style = "green"
        elif line.startswith("-"):
            style = "red"
        else:
            style = ""
        console.print(Text(line, style=style))
    if len(shown) < len(lines):
        console.print(f"[dim]... {len(lines) - len(shown)} more diff lines[/dim]")


def print_source(title: str, code: str) -> None:
    console.print(f"[bold]{title}:[/bold]", highlight=False)
    console.print(Syntax(code, "rust", line_numbers=True, theme="ansi_dark", word_wrap=False))

REPORT_ORDER = ("input.rs", "afmt.rs", "expected.rs", "rustfmt.rs", "afmt2.rs")

def report_failure(result: Result, max_diff_lines: int, verbose: bool) -> None:
    console.rule(f"[bold red]FAIL[/bold red] {result.name}", align="left")
    if verbose:
        for filename in REPORT_ORDER:
            if filename in result.outputs:
                print_source(filename, result.outputs[filename])
    for f in result.failures:
        console.print(f"[bold red]{f.kind}[/bold red]: {f.message}", highlight=False)
        if f.diff:
            print_diff(f.diff, max_diff_lines)
    console.print("[bold]Files:[/bold]")
    for p in sorted(result.workdir.iterdir()):
        console.print(f"  {p}", highlight=False)
    console.print()


def collect_sources(filters: list[str]) -> list[Path]:
    sources = sorted(p for p in SOURCES_DIR.rglob("*.rs") if not p.name.endswith(EXPECTED_SUFFIX))
    if not filters:
        return sources

    def matches(path: Path) -> bool:
        rel = path.relative_to(SOURCES_DIR).as_posix()
        for f in filters:
            if any(c in f for c in "*?["):
                if fnmatch.fnmatch(rel, f) or fnmatch.fnmatch(rel.removesuffix(".rs"), f):
                    return True
            elif f in rel:
                return True
        return False

    return [s for s in sources if matches(s)]


def build_afmt(release: bool) -> None:
    cmd = ["cargo", "build", "-p", "afmt"]
    if release:
        cmd.append("--release")
    console.print(f"[dim]$ {' '.join(cmd)}[/dim]")
    proc = subprocess.run(cmd, cwd=ROOT_DIR, capture_output=True, text=True)
    if proc.returncode != 0:
        console.print(proc.stderr, highlight=False, markup=False)
        console.print("[bold red]Build failed[/bold red]")
        sys.exit(2)


def main() -> None:
    parser = argparse.ArgumentParser(description="afmt test harness")
    parser.add_argument(
        "filters",
        nargs="*",
        help="run only tests whose path (relative to tests/sources) contains "
        "the given substring or matches the given glob",
    )
    parser.add_argument("--list", action="store_true", help="list selected tests and exit")
    parser.add_argument(
        "-v",
        "--verbose",
        action="store_true",
        help="for failed tests print input, afmt output and reference output before the diff",
    )
    parser.add_argument("--afmt", type=Path, help=f"afmt binary (default: {DEFAULT_AFMT})")
    parser.add_argument("--rustfmt", default="rustfmt", help="rustfmt binary")
    parser.add_argument("--edition", default="2024", help="edition passed to rustfmt")
    parser.add_argument("--no-build", action="store_true", help="do not run cargo build")
    parser.add_argument("--release", action="store_true", help="build and use release afmt")
    parser.add_argument("-j", "--jobs", type=int, default=None, help="parallel jobs")
    parser.add_argument("--timeout", type=float, default=10.0, help="per-command timeout (s)")
    parser.add_argument(
        "--max-diff-lines", type=int, default=200, help="truncate diffs (0 = unlimited)"
    )
    args = parser.parse_args()

    sources = collect_sources(args.filters)
    if args.list:
        names = [s.relative_to(SOURCES_DIR).as_posix() for s in sources]
        width = max((len(n) for n in names), default=0)
        for name, s in zip(names, sources):
            if expected_path_for(s).exists():
                reference = Text("expected.rs", style="bold cyan")
            else:
                reference = Text("rustfmt", style="dark_orange")
            console.print(Text(f"{name:<{width}}  ") + reference, soft_wrap=True)
        return
    if not sources:
        console.print("[yellow]No tests selected[/yellow]")
        sys.exit(1)

    if args.afmt is None:
        args.afmt = ROOT_DIR / "target" / (
        "release" if args.release else "debug") / "afmt"
        if not args.no_build:
            build_afmt(args.release)
    if not args.afmt.exists():
        console.print(f"[bold red]afmt binary not found:[/bold red] {args.afmt}")
        sys.exit(2)
    if shutil.which(args.rustfmt) is None:
        console.print(f"[bold red]rustfmt not found:[/bold red] {args.rustfmt}")
        sys.exit(2)

    clean_current_dir()
    results: list[Result] = []

    with Progress(
        TextColumn("[progress.description]{task.description}"),
        BarColumn(),
        MofNCompleteColumn(),
        TextColumn("[red]{task.fields[failed]} failed"),
        TimeElapsedColumn(),
        console=console,
        transient=False,
    ) as progress:
        task = progress.add_task("Testing", total=len(sources), failed=0)
        failed = 0
        with ThreadPoolExecutor(max_workers=args.jobs) as pool:
            futures = {pool.submit(run_test, s, args): s for s in sources}
            for fut in as_completed(futures):
                result = fut.result()
                results.append(result)
                if not result.ok:
                    write_outputs(result)
                    failed += 1
                    progress.console.print(f"[red]FAIL[/red] {result.name}", highlight=False)
                progress.update(task, advance=1, failed=failed)

    results.sort(key=lambda r: r.name)
    failures = [r for r in results if not r.ok]

    console.print()
    for r in failures:
        report_failure(r, args.max_diff_lines, args.verbose)

    passed = len(results) - len(failures)
    if failures:
        console.print(
            f"[bold red]{len(failures)} failed[/bold red], [green]{passed} passed[/green]"
        )
        console.print(f"Results directory: {CURRENT_DIR}", highlight=False)
        if not args.verbose:
            names = " ".join(r.name for r in failures)
            console.print(
                f"[dim]Hint: run `uv run tests/run.py -v {names}` to also see the input, "
                "afmt output and reference output[/dim]",
                highlight=False,
                soft_wrap=True,
            )
        sys.exit(1)

    console.print(f"[bold green]All {passed} tests passed[/bold green]")


if __name__ == "__main__":
    main()
