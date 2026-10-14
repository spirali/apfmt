#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.10"
# dependencies = ["rich>=13"]
# ///
"""
Test harness for afmt.

`test`: for every .rs file in tests/sources:
  1. format it with `rustfmt` (reference output); if `<name>.expected.rs` exists
     next to it, that file is used as the reference instead (for cases where
     afmt intentionally differs from rustfmt)
  2. format it with `afmt` and compare with the reference
  3. format the afmt output again with `afmt` and check idempotency
  Unsupported syntax fails a test, except in `<name>.unsupported.rs`: there afmt runs with
  `--unsupported warn`, must report some unsupported syntax (otherwise the test should be
  renamed), and `<name>.unsupported.expected.rs` is required as the reference (it checks what
  afmt keeps, e.g. attributes that rustfmt would drop).

`compare`: show afmt and rustfmt output side by side for selected tests.
  The comparison is always against rustfmt; `<name>.expected.rs` files are ignored.
  Outputs are shown unaligned, line N next to line N, with changed lines highlighted
  (with --align, lines are aligned by inserting blank lines and differing lines are highlighted).

Usage:
  uv run tests/run.py test                  # run all tests
  uv run tests/run.py test basic            # only tests whose path contains "basic"
  uv run tests/run.py test 'basic/*main*'   # glob filters are supported too
  uv run tests/run.py test --list           # list tests
  uv run tests/run.py test -v basic         # for failures also print input, afmt and reference output

  uv run tests/run.py compare calls         # side by side for tests whose path contains "calls"
  uv run tests/run.py compare --list calls  # list selected tests
  uv run tests/run.py compare -a calls      # also show tests where outputs are equal
  uv run tests/run.py compare -A calls      # align outputs by inserting blank lines

`bench`: measure how long the release build of afmt takes to format selected tests
  (tests in tests/sources/benchmarks when no filter is given). Every file is formatted
  --warmup times without measuring, then --runs times; the report shows the mean,
  standard deviation, min, median and max of the wall time and the mean CPU time
  (user + sys) per file and for all files together. After the timed runs, every file is
  formatted once more under GNU time (`time` in PATH) to measure the peak memory (the
  maximum resident set size; for all files together, the largest of the files); without
  GNU time, the memory is not reported. With --compare, rustfmt formats the same files
  the same number of times and the report compares the two.

  uv run tests/run.py bench                 # benchmark tests/sources/benchmarks
  uv run tests/run.py bench -n 20 calls     # 20 runs over tests whose path contains "calls"
  uv run tests/run.py bench -c              # also measure rustfmt and compare
  uv run tests/run.py bench -o bench.json   # also store the results as JSON
"""

import argparse
import datetime
import difflib
import fnmatch
import json
import math
import platform
import resource
import shutil
import statistics
import subprocess
import sys
import tempfile
import time
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
from rich.table import Table
from rich.text import Text

EXPECTED_SUFFIX = ".expected.rs"
# Tests of syntax that afmt copies unchanged (see `run_test`)
UNSUPPORTED_SUFFIX = ".unsupported.rs"

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


def run_cmd(
    cmd: list[str], stdin: str | None, timeout: float, cwd: Path | None = None
) -> tuple[str, str]:
    """The standard output and the standard error of a successful command"""
    try:
        proc = subprocess.run(
            cmd,
            input=stdin,
            capture_output=True,
            text=True,
            timeout=timeout,
            cwd=cwd,
            check=False,
        )
    except subprocess.TimeoutExpired:
        raise CommandError(f"`{' '.join(cmd)}` timed out after {timeout}s")
    if proc.returncode != 0:
        raise CommandError(
            f"`{' '.join(cmd)}` exited with code {proc.returncode}\n{proc.stderr.rstrip()}"
        )
    return proc.stdout, proc.stderr


def run_rustfmt(code: str, args) -> str:
    # stdin mode prints only the formatted code to stdout; in stdin mode rustfmt looks
    # up its config from the cwd, so run it from the repo root to never pick up
    # tests/sources/rustfmt.toml (which disables formatting)
    stdout, _ = run_cmd(
        [args.rustfmt, "--edition", args.edition],
        stdin=code,
        timeout=args.timeout,
        cwd=ROOT_DIR,
    )
    return stdout


# What afmt reports on stderr for each piece of syntax it copies unchanged
UNSUPPORTED_MESSAGE = "not supported yet"


def is_unsupported_test(source: Path) -> bool:
    return source.name.endswith(UNSUPPORTED_SUFFIX)


def afmt_options(unsupported: bool) -> list[str]:
    """Unsupported syntax fails a test instead of being copied unchanged (afmt's default is
    `warn`), except with `unsupported`: then it is copied and reported on stderr"""
    return ["--unsupported", "warn" if unsupported else "error"]


def run_afmt(code: str, args, unsupported: bool) -> tuple[str, str]:
    """afmt's output and stderr, see `afmt_options`"""
    return run_cmd(
        [str(args.afmt), *afmt_options(unsupported), "--result-check"],
        stdin=code,
        timeout=args.timeout,
    )


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

    unsupported = is_unsupported_test(source)
    expected_path = expected_path_for(source)
    if expected_path.exists():
        expected_name = "expected.rs"
        expected = expected_path.read_text()
    elif unsupported:
        result.failures.append(
            Failure(
                "missing reference",
                f"{expected_path.name} is required next to a {UNSUPPORTED_SUFFIX} test",
            )
        )
        return result
    else:
        expected_name = "rustfmt.rs"
        try:
            expected = run_rustfmt(code, args)
        except CommandError as e:
            result.failures.append(Failure("rustfmt error", str(e)))
            return result
    result.outputs[expected_name] = expected

    try:
        actual, stderr = run_afmt(code, args, unsupported)
    except CommandError as e:
        result.failures.append(Failure("afmt error", str(e)))
        return result
    result.outputs["afmt.rs"] = actual

    if unsupported and UNSUPPORTED_MESSAGE not in stderr:
        result.failures.append(
            Failure(
                "no unsupported syntax",
                f"afmt supports all of it now: rename the test to drop "
                f"`{UNSUPPORTED_SUFFIX.removesuffix('.rs')}`",
            )
        )
        return result

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
        second, _ = run_afmt(actual, args, unsupported)
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
    console.print(
        Syntax(code, "rust", line_numbers=True, theme="ansi_dark", word_wrap=False)
    )


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
    sources = sorted(
        p for p in SOURCES_DIR.rglob("*.rs") if not p.name.endswith(EXPECTED_SUFFIX)
    )
    if not filters:
        return sources

    def matches(path: Path) -> bool:
        rel = path.relative_to(SOURCES_DIR).as_posix()
        for f in filters:
            if any(c in f for c in "*?["):
                if fnmatch.fnmatch(rel, f) or fnmatch.fnmatch(
                    rel.removesuffix(".rs"), f
                ):
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
    proc = subprocess.run(
        cmd, cwd=ROOT_DIR, capture_output=True, text=True, check=False
    )
    if proc.returncode != 0:
        console.print(proc.stderr, highlight=False, markup=False)
        console.print("[bold red]Build failed[/bold red]")
        sys.exit(2)


DIFF_STYLE = "on #3a1d1d"
DIFF_STYLE_RIGHT = "on #1d3a1d"
EMPTY_STYLE = "on grey15"


def split_lines(code: str) -> list[str]:
    """The lines of `code` split on "\\n" only, like `highlight_lines` (`str.splitlines` also splits on
    `\\r`, form feeds, `\\u2028`, ...)"""
    lines = code.split("\n")
    # a trailing newline produces one extra empty line
    if code.endswith("\n"):
        lines = lines[:-1]
    return lines


def highlight_lines(code: str) -> list[Text]:
    syntax = Syntax("", "rust", theme="ansi_dark")
    lines = syntax.highlight(code).split("\n", allow_blank=True)
    # the highlighted text ends with an extra empty line; one line per line of `split_lines`
    return lines[: len(split_lines(code))]


def side_by_side(title: str, left: str, right: str, raw: bool) -> Table:
    left_lines = split_lines(left)
    right_lines = split_lines(right)
    left_hl = highlight_lines(left)
    right_hl = highlight_lines(right)

    table = Table(
        title=title,
        title_justify="left",
        title_style="bold",
        show_lines=False,
        expand=True,
        pad_edge=False,
        padding=(0, 1),
    )
    table.add_column("", justify="right", style="dim", no_wrap=True)
    table.add_column("afmt", ratio=1, overflow="fold")
    table.add_column("", justify="right", style="dim", no_wrap=True)
    table.add_column("rustfmt", ratio=1, overflow="fold")

    def cell(lines: list[Text], idx: int, style: str | None) -> Text:
        text = lines[idx].copy()
        if style:
            text.stylize(style)
        return text

    matcher = difflib.SequenceMatcher(a=left_lines, b=right_lines, autojunk=False)
    if raw:
        # no alignment: row N shows line N of both outputs, changed lines are highlighted
        left_changed: set[int] = set()
        right_changed: set[int] = set()
        for tag, i1, i2, j1, j2 in matcher.get_opcodes():
            if tag != "equal":
                left_changed.update(range(i1, i2))
                right_changed.update(range(j1, j2))
        for k in range(max(len(left_hl), len(right_hl))):
            row = []
            for lines, changed, style in (
                (left_hl, left_changed, DIFF_STYLE),
                (right_hl, right_changed, DIFF_STYLE_RIGHT),
            ):
                if k < len(lines):
                    row += [str(k + 1), cell(lines, k, style if k in changed else None)]
                else:
                    row += ["", Text("")]
            table.add_row(*row)
        return table

    for tag, i1, i2, j1, j2 in matcher.get_opcodes():
        if tag == "equal":
            for i, j in zip(range(i1, i2), range(j1, j2)):
                table.add_row(
                    str(i + 1),
                    cell(left_hl, i, None),
                    str(j + 1),
                    cell(right_hl, j, None),
                )
            continue
        n = max(i2 - i1, j2 - j1)
        for k in range(n):
            i, j = i1 + k, j1 + k
            if i < i2:
                lno, lcell = str(i + 1), cell(left_hl, i, DIFF_STYLE)
            else:
                lno, lcell = "", Text("", style=EMPTY_STYLE, end="")
            if j < j2:
                rno, rcell = str(j + 1), cell(right_hl, j, DIFF_STYLE_RIGHT)
            else:
                rno, rcell = "", Text("", style=EMPTY_STYLE, end="")
            table.add_row(lno, lcell, rno, rcell)
    return table


def list_sources(sources: list[Path], show_reference: bool) -> None:
    names = [s.relative_to(SOURCES_DIR).as_posix() for s in sources]
    if not show_reference:
        for name in names:
            console.print(name, highlight=False)
        return
    width = max((len(n) for n in names), default=0)
    for name, s in zip(names, sources):
        if expected_path_for(s).exists():
            reference = Text("expected.rs", style="bold cyan")
        else:
            reference = Text("rustfmt", style="dark_orange")
        console.print(Text(f"{name:<{width}}  ") + reference, soft_wrap=True)


def prepare_tools(args, need_rustfmt: bool = True) -> None:
    """Builds afmt (unless disabled) and checks that the formatters exist."""
    if args.afmt is None:
        args.afmt = (
            ROOT_DIR / "target" / ("release" if args.release else "debug") / "afmt"
        )
        if not args.no_build:
            build_afmt(args.release)
    if not args.afmt.exists():
        console.print(f"[bold red]afmt binary not found:[/bold red] {args.afmt}")
        sys.exit(2)
    if need_rustfmt and shutil.which(args.rustfmt) is None:
        console.print(f"[bold red]rustfmt not found:[/bold red] {args.rustfmt}")
        sys.exit(2)


def cmd_test(args, sources: list[Path]) -> None:
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
                    progress.console.print(
                        f"[red]FAIL[/red] {result.name}", highlight=False
                    )
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
                f"[dim]Hint: run `uv run tests/run.py test -v {names}` to also see the input, "
                "afmt output and reference output[/dim]",
                highlight=False,
                soft_wrap=True,
            )
        sys.exit(1)

    console.print(f"[bold green]All {passed} tests passed[/bold green]")


def cmd_compare(args, sources: list[Path]) -> None:
    same = 0
    for source in sources:
        name = source.relative_to(SOURCES_DIR).as_posix()
        code = source.read_text()
        errors = []
        try:
            afmt_out, _ = run_afmt(code, args, is_unsupported_test(source))
        except CommandError as e:
            afmt_out = ""
            errors.append(f"afmt error: {e}")
        try:
            rustfmt_out = run_rustfmt(code, args)
        except CommandError as e:
            rustfmt_out = ""
            errors.append(f"rustfmt error: {e}")

        if not errors and afmt_out == rustfmt_out:
            same += 1
            if not args.all:
                continue
            title = f"{name} [green](identical)[/green]"
        else:
            title = f"{name} [red](differs)[/red]"
        console.print(side_by_side(title, afmt_out, rustfmt_out, not args.align))
        for err in errors:
            console.print(Text(err, style="bold red"))
        console.print()

    if same and not args.all:
        console.print(
            f"[dim]{same} of {len(sources)} selected tests identical (use -a to show)[/dim]"
        )


BENCH_DEFAULT_FILTERS = ["benchmarks/"]


@dataclass
class Samples:
    """Times of the runs of one tool over one file, in seconds."""

    wall: list[float] = field(default_factory=list)
    cpu: list[float] = field(default_factory=list)

    def add(self, other: "Samples") -> "Samples":
        """The run-by-run sum of two series of the same length."""
        return Samples(
            [a + b for a, b in zip(self.wall, other.wall)],
            [a + b for a, b in zip(self.cpu, other.cpu)],
        )


@dataclass
class Stats:
    mean: float
    stdev: float
    min: float
    median: float
    max: float
    cpu_mean: float

    @classmethod
    def of(cls, samples: Samples) -> "Stats":
        wall = samples.wall
        return cls(
            mean=statistics.fmean(wall),
            stdev=statistics.stdev(wall) if len(wall) > 1 else 0.0,
            min=min(wall),
            median=statistics.median(wall),
            max=max(wall),
            cpu_mean=statistics.fmean(samples.cpu),
        )

    @property
    def rel_stdev(self) -> float:
        return self.stdev / self.mean if self.mean else 0.0


@dataclass
class BenchFile:
    name: str
    code: str
    samples: dict[str, Samples] = field(default_factory=dict)
    # the peak memory of every tool in bytes, None if it was not measured
    peak_rss: dict[str, int | None] = field(default_factory=dict)

    @property
    def bytes(self) -> int:
        return len(self.code.encode())

    @property
    def lines(self) -> int:
        return len(split_lines(self.code))


def bench_commands(args) -> dict[str, list[str]]:
    """The measured command of every tool; the same commands as `test` uses, without
    afmt's --result-check unless it is asked for."""
    afmt = [str(args.afmt), *afmt_options(False)]
    if args.result_check:
        afmt.append("--result-check")
    commands = {"afmt": afmt}
    if args.compare:
        commands["rustfmt"] = [args.rustfmt, "--edition", args.edition]
    return commands


def timed_run(cmd: list[str], code: bytes, timeout: float) -> tuple[float, float]:
    """Runs `cmd` with `code` on stdin; returns the wall time and the CPU time (user + sys)
    of the child in seconds. Runs must not overlap: the CPU time is the difference of
    the usage of all finished children."""
    before = resource.getrusage(resource.RUSAGE_CHILDREN)
    start = time.perf_counter()
    try:
        proc = subprocess.run(
            cmd,
            input=code,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
            timeout=timeout,
            cwd=ROOT_DIR,
            check=False,
        )
    except subprocess.TimeoutExpired:
        raise CommandError(f"`{' '.join(cmd)}` timed out after {timeout}s")
    wall = time.perf_counter() - start
    after = resource.getrusage(resource.RUSAGE_CHILDREN)
    if proc.returncode != 0:
        raise CommandError(
            f"`{' '.join(cmd)}` exited with code {proc.returncode}\n"
            f"{proc.stderr.decode(errors='replace').rstrip()}"
        )
    cpu = (after.ru_utime - before.ru_utime) + (after.ru_stime - before.ru_stime)
    return wall, cpu


def find_gnu_time() -> str | None:
    """The GNU time binary, if there is one in PATH. The peak memory is measured with it
    and not by Python: on Linux, a child's maximum resident set size includes the memory
    of the process it was forked from (Python's) before `exec`, while GNU time is small."""
    path = shutil.which("time")
    if path is None or "GNU" not in (tool_version([path, "--version"]) or ""):
        return None
    return path


def peak_memory(gnu_time: str, cmd: list[str], code: bytes, timeout: float) -> int:
    """Runs `cmd` with `code` on stdin under GNU time; returns its peak memory (maximum
    resident set size) in bytes."""
    with tempfile.NamedTemporaryFile(mode="r") as report:
        try:
            proc = subprocess.run(
                [gnu_time, "--format=%M", f"--output={report.name}", *cmd],
                input=code,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.PIPE,
                timeout=timeout,
                cwd=ROOT_DIR,
                check=False,
            )
        except subprocess.TimeoutExpired:
            raise CommandError(f"`{' '.join(cmd)}` timed out after {timeout}s")
        if proc.returncode != 0:
            raise CommandError(
                f"`{' '.join(cmd)}` exited with code {proc.returncode}\n"
                f"{proc.stderr.decode(errors='replace').rstrip()}"
            )
        # in KiB
        return int(report.read().split()[-1]) * 1024


def format_bytes(size: int | None) -> str:
    if size is None:
        return "–"
    if size >= 1024**3:
        return f"{size / 1024**3:.2f} GiB"
    if size >= 1024**2:
        return f"{size / 1024**2:.1f} MiB"
    return f"{size / 1024:.0f} KiB"


def format_seconds(seconds: float) -> str:
    if seconds >= 1:
        return f"{seconds:.3f} s"
    if seconds >= 1e-3:
        return f"{seconds * 1e3:.2f} ms"
    return f"{seconds * 1e6:.0f} µs"


def format_ratio(ratio: float, better: str = "faster", worse: str = "slower") -> Text:
    """`ratio` is afmt / rustfmt (time or memory); below 1 afmt is `better`."""
    if ratio <= 1:
        return Text(f"{ratio:.2f}× ({1 / ratio:.2f}× {better})", style="green")
    return Text(f"{ratio:.2f}× ({ratio:.2f}× {worse})", style="red")


def format_memory_ratio(ratio: float | None) -> Text:
    return Text("–") if ratio is None else format_ratio(ratio, "less", "more")


def geometric_mean(values: list[float]) -> float:
    return math.exp(statistics.fmean(math.log(v) for v in values))


def stats_row(stats: Stats) -> list[str]:
    return [
        format_seconds(stats.mean),
        format_seconds(stats.stdev),
        f"{stats.rel_stdev:.1%}",
        format_seconds(stats.min),
        format_seconds(stats.median),
        format_seconds(stats.max),
        format_seconds(stats.cpu_mean),
    ]


def stats_table(title: str, rows: list[tuple[str, str, Stats, int | None]]) -> Table:
    """`rows` are the test, the tool, its time statistics and its peak memory."""
    table = Table(title=title, title_justify="left", title_style="bold")
    table.add_column("test", overflow="fold", min_width=16)
    table.add_column("tool", no_wrap=True)
    for column in ("mean", "σ", "σ %", "min", "median", "max", "CPU", "peak mem"):
        table.add_column(column, justify="right", no_wrap=True)
    previous = None
    for name, tool, stats, peak_rss in rows:
        table.add_row(
            name if name != previous else "",
            tool,
            *stats_row(stats),
            format_bytes(peak_rss),
        )
        previous = name
    return table


def git_revision() -> dict[str, str | bool | None]:
    def git(*cmd: str) -> str | None:
        proc = subprocess.run(
            ["git", *cmd], cwd=ROOT_DIR, capture_output=True, text=True, check=False
        )
        return proc.stdout.strip() if proc.returncode == 0 else None

    status = git("status", "--porcelain", "--untracked-files=no")
    return {
        "commit": git("rev-parse", "HEAD"),
        "dirty": bool(status) if status is not None else None,
    }


def tool_version(cmd: list[str]) -> str | None:
    try:
        proc = subprocess.run(
            cmd, capture_output=True, text=True, timeout=10, check=False
        )
    except (OSError, subprocess.TimeoutExpired):
        return None
    return proc.stdout.strip() or None


def stats_json(stats: Stats) -> dict[str, float]:
    return {
        "mean": stats.mean,
        "stdev": stats.stdev,
        "min": stats.min,
        "median": stats.median,
        "max": stats.max,
        "cpu_mean": stats.cpu_mean,
    }


def write_bench_results(
    path: Path,
    args,
    files: list[BenchFile],
    totals: dict[str, Samples],
    total_peak_rss: dict[str, int | None],
) -> None:
    commands = bench_commands(args)
    tools = {"afmt": {"command": commands["afmt"]}}
    if "rustfmt" in commands:
        tools["rustfmt"] = {
            "command": commands["rustfmt"],
            "version": tool_version([args.rustfmt, "--version"]),
        }
    data = {
        "date": datetime.datetime.now().astimezone().isoformat(timespec="seconds"),
        "git": git_revision(),
        "host": {
            "machine": platform.machine(),
            "system": platform.platform(),
            "python": platform.python_version(),
        },
        "runs": args.runs,
        "warmup": args.warmup,
        "unit": "seconds",
        "memory_unit": "bytes",
        "tools": tools,
        "files": [
            {
                "name": f.name,
                "bytes": f.bytes,
                "lines": f.lines,
                "results": {
                    tool: {
                        "wall": s.wall,
                        "cpu": s.cpu,
                        "stats": stats_json(Stats.of(s)),
                        "peak_rss": f.peak_rss[tool],
                    }
                    for tool, s in f.samples.items()
                },
            }
            for f in files
        ],
        "total": {
            tool: {
                "wall": s.wall,
                "cpu": s.cpu,
                "stats": stats_json(Stats.of(s)),
                "peak_rss": total_peak_rss[tool],
            }
            for tool, s in totals.items()
        },
    }
    path.write_text(json.dumps(data, indent=2) + "\n")
    console.print(f"Results written to {path}", highlight=False)


def cmd_bench(args, sources: list[Path]) -> None:
    commands = bench_commands(args)
    files = [
        BenchFile(s.relative_to(SOURCES_DIR).as_posix(), s.read_text()) for s in sources
    ]
    for f in files:
        f.samples = {tool: Samples() for tool in commands}
        f.peak_rss = {tool: None for tool in commands}
    gnu_time = find_gnu_time()
    if gnu_time is None:
        console.print(
            "[yellow]GNU time not found in PATH, the peak memory is not measured[/yellow]"
        )

    console.print(
        f"[bold]Benchmarking[/bold] {len(files)} files, {args.warmup} warmup + {args.runs} runs, "
        f"tools: {', '.join(commands)}",
        highlight=False,
    )
    failed: dict[str, str] = {}
    with Progress(
        TextColumn("[progress.description]{task.description}"),
        BarColumn(),
        MofNCompleteColumn(),
        TimeElapsedColumn(),
        console=console,
        transient=True,
    ) as progress:
        memory_runs = 1 if gnu_time else 0
        task = progress.add_task(
            "Running", total=(args.warmup + args.runs + memory_runs) * len(files)
        )
        # run by run over all files, and the tools one after another on each file,
        # so that a drift of the machine affects all files and tools alike
        for run in range(args.warmup + args.runs):
            progress.update(
                task,
                description="Warmup"
                if run < args.warmup
                else f"Run {run - args.warmup + 1}",
            )
            for f in files:
                if f.name in failed:
                    progress.advance(task)
                    continue
                code = f.code.encode()
                for tool, cmd in commands.items():
                    try:
                        wall, cpu = timed_run(cmd, code, args.timeout)
                    except CommandError as e:
                        failed[f.name] = f"{tool}: {e}"
                        break
                    if run >= args.warmup:
                        f.samples[tool].wall.append(wall)
                        f.samples[tool].cpu.append(cpu)
                progress.advance(task)
        # separately, so that GNU time does not take part in the measured times
        if gnu_time:
            progress.update(task, description="Memory")
            for f in files:
                code = f.code.encode()
                for tool, cmd in commands.items():
                    if f.name in failed:
                        break
                    try:
                        f.peak_rss[tool] = peak_memory(
                            gnu_time, cmd, code, args.timeout
                        )
                    except CommandError as e:
                        failed[f.name] = f"{tool}: {e}"
                progress.advance(task)

    for name, error in failed.items():
        console.print(f"[bold red]FAIL[/bold red] {name}: {error}", highlight=False)
    files = [f for f in files if f.name not in failed]
    if not files:
        console.print("[bold red]No file was formatted successfully[/bold red]")
        sys.exit(1)

    rows = [
        (f.name, tool, Stats.of(s), f.peak_rss[tool])
        for f in files
        for tool, s in f.samples.items()
    ]
    console.print(
        stats_table(
            "Per file (wall time, CPU time = user + sys, peak mem = max RSS)", rows
        )
    )

    totals: dict[str, Samples] = {}
    for tool in commands:
        total = Samples([0.0] * args.runs, [0.0] * args.runs)
        for f in files:
            total = total.add(f.samples[tool])
        totals[tool] = total
    total_stats = {tool: Stats.of(s) for tool, s in totals.items()}
    # the files are formatted one by one, so the peak is the largest of them
    total_peak_rss = {
        tool: max(peaks) if None not in peaks else None
        for tool in commands
        for peaks in [[f.peak_rss[tool] for f in files]]
    }
    console.print(
        stats_table(
            f"All {len(files)} files per run",
            [
                ("total", tool, stats, total_peak_rss[tool])
                for tool, stats in total_stats.items()
            ],
        )
    )

    size = sum(f.bytes for f in files)
    lines = sum(f.lines for f in files)
    afmt_mean = total_stats["afmt"].mean
    console.print(
        f"[bold]afmt throughput:[/bold] {lines / afmt_mean:,.0f} lines/s, "
        f"{size / afmt_mean / 1024:,.0f} KiB/s ({lines} lines, {size / 1024:.1f} KiB per run)",
        highlight=False,
    )

    if "rustfmt" in commands:
        table = Table(
            title="afmt / rustfmt (mean wall time, peak memory)",
            title_justify="left",
            title_style="bold",
        )
        table.add_column("test")
        table.add_column("afmt", justify="right")
        table.add_column("rustfmt", justify="right")
        table.add_column("ratio")
        table.add_column("afmt mem", justify="right")
        table.add_column("rustfmt mem", justify="right")
        table.add_column("mem ratio")
        time_ratios: list[float] = []
        memory_ratios: list[float] = []

        def add_row(
            name: str, stats: dict[str, Stats], peak_rss: dict[str, int | None]
        ) -> None:
            time_ratio = stats["afmt"].mean / stats["rustfmt"].mean
            memory_ratio = None
            if peak_rss["afmt"] is not None and peak_rss["rustfmt"] is not None:
                memory_ratio = peak_rss["afmt"] / peak_rss["rustfmt"]
                memory_ratios.append(memory_ratio)
            time_ratios.append(time_ratio)
            table.add_row(
                name,
                format_seconds(stats["afmt"].mean),
                format_seconds(stats["rustfmt"].mean),
                format_ratio(time_ratio),
                format_bytes(peak_rss["afmt"]),
                format_bytes(peak_rss["rustfmt"]),
                format_memory_ratio(memory_ratio),
            )

        for f in files:
            stats = {tool: Stats.of(s) for tool, s in f.samples.items()}
            add_row(f.name, stats, f.peak_rss)
        time_geomean = geometric_mean(time_ratios)
        memory_geomean = geometric_mean(memory_ratios) if memory_ratios else None
        table.add_section()
        add_row("total", total_stats, total_peak_rss)
        table.add_row(
            "geometric mean of ratios",
            "",
            "",
            format_ratio(time_geomean),
            "",
            "",
            format_memory_ratio(memory_geomean),
        )
        console.print(table)

    if args.output:
        write_bench_results(args.output, args, files, totals, total_peak_rss)
    if failed:
        sys.exit(1)


def main() -> None:
    common = argparse.ArgumentParser(add_help=False)
    common.add_argument(
        "--list", action="store_true", help="list selected tests and exit"
    )
    common.add_argument(
        "--afmt", type=Path, help=f"afmt binary (default: {DEFAULT_AFMT})"
    )
    common.add_argument("--rustfmt", default="rustfmt", help="rustfmt binary")
    common.add_argument("--edition", default="2024", help="edition passed to rustfmt")
    common.add_argument(
        "--no-build", action="store_true", help="do not run cargo build"
    )
    common.add_argument(
        "--release", action="store_true", help="build and use release afmt"
    )
    common.add_argument(
        "--timeout", type=float, default=10.0, help="per-command timeout (s)"
    )
    filters_help = (
        "tests whose path (relative to tests/sources) contains "
        "the given substring or matches the given glob"
    )

    parser = argparse.ArgumentParser(description="afmt test harness")
    commands = parser.add_subparsers(dest="command", required=True)

    test = commands.add_parser(
        "test",
        parents=[common],
        help="compare afmt with the reference and check idempotency",
    )
    test.add_argument("filters", nargs="*", help=f"run only {filters_help}")
    test.add_argument(
        "-v",
        "--verbose",
        action="store_true",
        help="for failed tests print input, afmt output and reference output before the diff",
    )
    test.add_argument("-j", "--jobs", type=int, default=None, help="parallel jobs")
    test.add_argument(
        "--max-diff-lines", type=int, default=200, help="truncate diffs (0 = unlimited)"
    )

    compare = commands.add_parser(
        "compare", parents=[common], help="side-by-side afmt vs rustfmt output"
    )
    compare.add_argument("filters", nargs="+", help=filters_help)
    compare.add_argument(
        "-a",
        "--all",
        action="store_true",
        help="also show tests where outputs are equal",
    )
    compare.add_argument(
        "-A",
        "--align",
        action="store_true",
        help="align the outputs by inserting blank lines and highlight differing lines "
        "(by default outputs are shown as they are, with changed lines highlighted)",
    )

    bench = commands.add_parser(
        "bench",
        parents=[common],
        help="measure the speed of release afmt (and rustfmt)",
    )
    bench.add_argument(
        "filters",
        nargs="*",
        help=f"benchmark only {filters_help} (default: tests in tests/sources/benchmarks)",
    )
    bench.add_argument(
        "-n", "--runs", type=int, default=5, help="measured runs per file (default: 5)"
    )
    bench.add_argument(
        "-w",
        "--warmup",
        type=int,
        default=1,
        help="unmeasured runs per file before measuring (default: 1)",
    )
    bench.add_argument(
        "-c",
        "--compare",
        action="store_true",
        help="also measure rustfmt with the same number of runs and compare",
    )
    bench.add_argument(
        "--result-check",
        action="store_true",
        help="run afmt with --result-check (slower)",
    )
    bench.add_argument(
        "-o", "--output", type=Path, help="store the results as JSON into this file"
    )

    args = parser.parse_args()

    if args.command == "bench":
        if args.runs < 1 or args.warmup < 0:
            parser.error("--runs must be at least 1 and --warmup at least 0")
        # benchmarks measure the optimized build
        args.release = True
        if not args.filters:
            args.filters = BENCH_DEFAULT_FILTERS

    sources = collect_sources(args.filters)
    if args.list:
        # compare always uses rustfmt, so only `test` shows the reference
        list_sources(sources, show_reference=args.command == "test")
        return
    if not sources:
        console.print("[yellow]No tests selected[/yellow]")
        sys.exit(1)

    prepare_tools(args, need_rustfmt=args.command != "bench" or args.compare)
    if args.command == "test":
        cmd_test(args, sources)
    elif args.command == "bench":
        cmd_bench(args, sources)
    else:
        cmd_compare(args, sources)


if __name__ == "__main__":
    main()
