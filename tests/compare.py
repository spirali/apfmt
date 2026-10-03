#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.10"
# dependencies = ["rich>=13"]
# ///
"""
Show afmt and rustfmt output side by side for selected tests.

The comparison is always against rustfmt; `<name>.expected.rs` files are ignored.
Lines are aligned and differing lines are highlighted (with --raw, outputs are shown
unaligned, line N next to line N, and changed lines are still highlighted).

Usage:
  uv run tests/compare.py calls              # tests whose path contains "calls"
  uv run tests/compare.py 'basic/*main*'     # glob filters are supported too
  uv run tests/compare.py --list calls       # list selected tests
  uv run tests/compare.py -a calls           # also show tests where outputs are equal
  uv run tests/compare.py -r calls           # raw outputs, no alignment lines inserted
"""

import argparse
import difflib
import shutil
import sys
from pathlib import Path

from rich.syntax import Syntax
from rich.table import Table
from rich.text import Text

from run import (
    ROOT_DIR,
    SOURCES_DIR,
    CommandError,
    build_afmt,
    collect_sources,
    console,
    run_afmt,
    run_rustfmt,
)

DIFF_STYLE = "on #3a1d1d"
DIFF_STYLE_RIGHT = "on #1d3a1d"
EMPTY_STYLE = "on grey15"


def highlight_lines(code: str) -> list[Text]:
    syntax = Syntax("", "rust", theme="ansi_dark")
    lines = syntax.highlight(code).split("\n", allow_blank=True)
    # a trailing newline produces one extra empty line
    if code.endswith("\n") and lines:
        lines = lines[:-1]
    return lines


def side_by_side(title: str, left: str, right: str, raw: bool) -> Table:
    left_lines = left.splitlines()
    right_lines = right.splitlines()
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
                table.add_row(str(i + 1), cell(left_hl, i, None), str(j + 1), cell(right_hl, j, None))
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


def main() -> None:
    parser = argparse.ArgumentParser(description="side-by-side afmt vs rustfmt output")
    parser.add_argument(
        "filters",
        nargs="+",
        help="tests whose path (relative to tests/sources) contains "
        "the given substring or matches the given glob",
    )
    parser.add_argument("--list", action="store_true", help="list selected tests and exit")
    parser.add_argument(
        "-a", "--all", action="store_true", help="also show tests where outputs are equal"
    )
    parser.add_argument(
        "-r",
        "--raw",
        action="store_true",
        help="show outputs as they are, without inserting alignment lines (changed lines are still highlighted)",
    )
    parser.add_argument("--afmt", type=Path, help="afmt binary (default: target/debug/afmt)")
    parser.add_argument("--rustfmt", default="rustfmt", help="rustfmt binary")
    parser.add_argument("--edition", default="2024", help="edition passed to rustfmt")
    parser.add_argument("--no-build", action="store_true", help="do not run cargo build")
    parser.add_argument("--release", action="store_true", help="build and use release afmt")
    parser.add_argument("--timeout", type=float, default=10.0, help="per-command timeout (s)")
    args = parser.parse_args()

    sources = collect_sources(args.filters)
    if args.list:
        for s in sources:
            console.print(s.relative_to(SOURCES_DIR).as_posix(), highlight=False)
        return
    if not sources:
        console.print("[yellow]No tests selected[/yellow]")
        sys.exit(1)

    if args.afmt is None:
        args.afmt = ROOT_DIR / "target" / ("release" if args.release else "debug") / "afmt"
        if not args.no_build:
            build_afmt(args.release)
    if not args.afmt.exists():
        console.print(f"[bold red]afmt binary not found:[/bold red] {args.afmt}")
        sys.exit(2)
    if shutil.which(args.rustfmt) is None:
        console.print(f"[bold red]rustfmt not found:[/bold red] {args.rustfmt}")
        sys.exit(2)

    same = 0
    for source in sources:
        name = source.relative_to(SOURCES_DIR).as_posix()
        code = source.read_text()
        errors = []
        try:
            afmt_out = run_afmt(code, args)
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
        console.print(side_by_side(title, afmt_out, rustfmt_out, args.raw))
        for err in errors:
            console.print(Text(err, style="bold red"))
        console.print()

    if same and not args.all:
        console.print(f"[dim]{same} of {len(sources)} selected tests identical (use -a to show)[/dim]")


if __name__ == "__main__":
    main()
