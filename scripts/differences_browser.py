"""Generate a self-contained HTML browser of the differences between apfmt and rustfmt.

    python3 docs/differences_browser.py docs/rustfmt-differences.toml -o differences.html

Highlights of an output come from comparing it with the output next to it. `highlight_lines` and
`highlight_chunks` on an output replace them for that output: lines are 1-based, as in
`display_lines`; `start_col` is a 1-based display column and `end_col` is exclusive.
"""

import argparse
import difflib
import html
import json
import re
import sys
import tomllib
import unicodedata
from itertools import accumulate
from pathlib import Path

DEFAULT_MAX_WIDTH = 100
DEFAULT_TAB_SPACES = 4
LEVELS = ("high", "medium", "low")

KEYWORDS = frozenset(
    [
        "as",
        "async",
        "await",
        "break",
        "const",
        "continue",
        "crate",
        "dyn",
        "else",
        "enum",
        "extern",
        "false",
        "fn",
        "for",
        "if",
        "impl",
        "in",
        "let",
        "loop",
        "match",
        "mod",
        "move",
        "mut",
        "pub",
        "ref",
        "return",
        "self",
        "static",
        "struct",
        "super",
        "trait",
        "true",
        "type",
        "union",
        "unsafe",
        "use",
        "where",
        "while",
        "yield",
    ]
)
PRIMITIVES = frozenset(
    [
        "bool",
        "char",
        "f32",
        "f64",
        "i8",
        "i16",
        "i32",
        "i64",
        "i128",
        "isize",
        "str",
        "u8",
        "u16",
        "u32",
        "u64",
        "u128",
        "usize",
    ]
)

RAW_STRING = re.compile(r'(?:b|c)?r(#*)"')
PLAIN_STRING = re.compile(r'(?:b|c)?"')
CHAR = re.compile(r"b?'(?:\\.[^'\n]*|[^\\'\n])'")
LIFETIME = re.compile(r"'[^\W\d]\w*")
ATTRIBUTE = re.compile(r"#!?\[\s*[\w:]+")
METAVAR = re.compile(r"\$[^\W\d]\w*")
NUMBER = re.compile(
    r"0[xob][0-9a-fA-F_]+\w*|\d[\d_]*(?:\.(?![.\w])|\.\d[\d_]*)?(?:[eE][+-]?\d[\d_]*)?\w*"
)
IDENT = re.compile(r"(?:r#)?[^\W\d]\w*")
WORD = re.compile(r"\w+|\s+|.", re.DOTALL)


def block_comment_end(src, start):
    depth, i = 0, start
    while i < len(src):
        if src.startswith("/*", i):
            depth, i = depth + 1, i + 2
        elif src.startswith("*/", i):
            depth, i = depth - 1, i + 2
            if depth == 0:
                return i
        else:
            i += 1
    return len(src)


def string_end(src, start):
    i = start
    while i < len(src):
        if src[i] == "\\":
            i += 2
        elif src[i] == '"':
            return i + 1
        else:
            i += 1
    return len(src)


def is_doc_comment(text):
    if text.startswith("//"):
        return text.startswith("//!") or (
            text.startswith("///") and not text.startswith("////")
        )
    return text.startswith("/*!") or (
        text.startswith("/**") and not text.startswith(("/***", "/**/"))
    )


def ident_class(src, name, end, previous):
    if src.startswith("!", end) and not src.startswith("!=", end):
        return "mac"
    if name in KEYWORDS:
        return "kw"
    if name in PRIMITIVES:
        return "pr"
    if name[0].isupper():
        return "ct" if name.isupper() and len(name) > 1 else "ty"
    rest = src[end : end + 40].lstrip(" ")
    if previous == "fn" or rest.startswith(("(", "::<")):
        return "fn"
    return None


def tokenize(src):
    """Spans (start, end, class) of Rust source. Macro bodies need not parse, so nothing is fatal."""
    spans, i, previous = [], 0, None
    while i < len(src):
        c = src[i]
        if c.isspace():
            i += 1
            continue
        end, cls = None, None
        if src.startswith("//", i):
            newline = src.find("\n", i)
            end = len(src) if newline < 0 else newline
            cls = "doc" if is_doc_comment(src[i:end]) else "com"
        elif src.startswith("/*", i):
            end = block_comment_end(src, i)
            cls = "doc" if is_doc_comment(src[i:end]) else "com"
        elif m := RAW_STRING.match(src, i):
            closing = src.find('"' + m.group(1), m.end())
            end = len(src) if closing < 0 else closing + 1 + len(m.group(1))
            cls = "str"
        elif m := PLAIN_STRING.match(src, i):
            end, cls = string_end(src, m.end()), "str"
        elif m := CHAR.match(src, i):
            end, cls = m.end(), "str"
        elif m := LIFETIME.match(src, i):
            end, cls = m.end(), "lt"
        elif m := ATTRIBUTE.match(src, i):
            end, cls = m.end(), "at"
        elif m := METAVAR.match(src, i):
            end, cls = m.end(), "mv"
        elif c.isdigit() and (m := NUMBER.match(src, i)):
            end, cls = m.end(), "num"
        elif m := IDENT.match(src, i):
            end = m.end()
            cls = ident_class(src, m.group(), end, previous)
            if cls == "mac":
                end += 1
            previous = m.group()
        if end is None:
            i += 1
            continue
        if cls:
            spans.append((i, end, cls))
        if not IDENT.fullmatch(src[i:end]):
            previous = None
        i = end
    return spans


def split_lines(text):
    """Lines without `\\n`; a `\\r` stays, as it is a difference to show. "" has no line, "\\n" one."""
    lines = text.split("\n")
    if lines[-1] == "":
        lines.pop()
    return lines


def syntax_classes(text):
    """For each line, the syntax class of each of its characters."""
    lines = split_lines(text)
    classes = [[None] * len(line) for line in lines]
    starts, offset = [], 0
    for line in lines:
        starts.append(offset)
        offset += len(line) + 1
    number = 0
    for start, end, cls in tokenize(text):
        while number + 1 < len(lines) and starts[number + 1] <= start:
            number += 1
        n = number
        while n < len(lines) and starts[n] < end:
            first = max(start - starts[n], 0)
            last = min(end - starts[n], len(lines[n]))
            classes[n][first:last] = [cls] * (last - first)
            n += 1
    return classes


def char_width(c, tab_spaces):
    if c == "\t":
        return tab_spaces
    if c == "\r" or unicodedata.combining(c):
        return 0
    return 2 if unicodedata.east_asian_width(c) in ("W", "F") else 1


def line_columns(line, tab_spaces):
    """The 1-based display column of each character, and the column after the last one."""
    columns, column = [], 1
    for c in line:
        columns.append(column)
        column += char_width(c, tab_spaces)
    return columns, column


def changed_ranges(a, b):
    """The changed character ranges of two lines, or (None, None) if they have little in common.

    Compares tokens, as marks over single characters of different words are noise.
    """
    tokens_a, tokens_b = WORD.findall(a), WORD.findall(b)
    matcher = difflib.SequenceMatcher(None, tokens_a, tokens_b, autojunk=False)
    if matcher.ratio() < 0.4:
        return None, None
    offsets_a = list(accumulate(map(len, tokens_a), initial=0))
    offsets_b = list(accumulate(map(len, tokens_b), initial=0))
    ranges_a, ranges_b = [], []
    for tag, a1, a2, b1, b2 in matcher.get_opcodes():
        if tag != "equal":
            ranges_a += [(offsets_a[a1], offsets_a[a2])] if a1 < a2 else []
            ranges_b += [(offsets_b[b1], offsets_b[b2])] if b1 < b2 else []
    return ranges_a, ranges_b


def compare(a, b):
    """The rows of two outputs, (line index, marks), each side with just its own lines.

    A mark is None for an unchanged line, or the list of changed character ranges of a changed line
    (empty when the whole line differs and no range is worth marking).
    """
    rows_a, rows_b = [], []
    matcher = difflib.SequenceMatcher(None, a, b, autojunk=False)
    for tag, a1, a2, b1, b2 in matcher.get_opcodes():
        if tag == "equal":
            for offset in range(a2 - a1):
                rows_a.append((a1 + offset, None))
                rows_b.append((b1 + offset, None))
            continue
        for offset in range(max(a2 - a1, b2 - b1)):
            i = a1 + offset if a1 + offset < a2 else None
            j = b1 + offset if b1 + offset < b2 else None
            ranges_a, ranges_b = [], []
            if i is not None and j is not None:
                ranges_a, ranges_b = changed_ranges(a[i], b[j])
                ranges_a, ranges_b = ranges_a or [], ranges_b or []
            if i is not None:
                rows_a.append((i, ranges_a))
            if j is not None:
                rows_b.append((j, ranges_b))
    return rows_a, rows_b


def inline(text):
    """Inline Markdown: code spans, bold, `_emphasis_` and http(s) links; the rest is escaped."""
    parts = re.split(r"(`+)(.+?)\1", text)
    out = []
    for index in range(0, len(parts), 3):
        out.append(inline_text(parts[index]))
        if index + 2 < len(parts):
            out.append(f"<code>{html.escape(parts[index + 2].strip())}</code>")
    return "".join(out)


def inline_text(text):
    text = html.escape(text, quote=False)
    text = re.sub(
        r"\[([^\]]+)\]\((https?://[^)\s]+)\)",
        lambda m: (
            f'<a href="{html.escape(m.group(2))}" rel="noopener">{m.group(1)}</a>'
        ),
        text,
    )
    text = re.sub(r"\*\*(.+?)\*\*", r"<strong>\1</strong>", text)
    return re.sub(r"(?<!\w)_([^_\s][^_]*)_(?!\w)", r"<em>\1</em>", text)


def markdown(text):
    """The block Markdown of the descriptions: paragraphs, bullet lists and tables."""
    out = []
    for block in re.split(r"\n\s*\n", text.strip()):
        lines = block.split("\n")
        if lines[0].startswith("- "):
            items = "".join(f"<li>{inline(line[2:])}</li>" for line in lines)
            out.append(f"<ul>{items}</ul>")
        elif lines[0].startswith("|"):
            out.append(markdown_table(lines))
        else:
            out.append(f"<p>{inline(' '.join(lines))}</p>")
    return "".join(out)


def markdown_table(lines):
    rows = [[c.strip() for c in re.split(r"(?<!\\)\|", line)[1:-1]] for line in lines]
    head = "".join(f"<th>{inline(c)}</th>" for c in rows[0])
    body = "".join(
        "<tr>" + "".join(f"<td>{inline(c)}</td>" for c in row) + "</tr>"
        for row in rows[2:]
    )
    return f'<div class="table"><table><tr>{head}</tr>{body}</table></div>'


def render_line(line, syntax, tab_spaces, max_width, changed, manual, show_whitespace):
    """The HTML of a line; `changed` and `manual` are character ranges, `max_width` may be None."""
    columns, _ = line_columns(line, tab_spaces)
    flags = [set() for _ in line]
    for start, end in changed:
        for k in range(start, end):
            flags[k].add("chg")
    for start, end in manual:
        for k in range(start, end):
            flags[k].add("sel")
    if max_width is not None:
        for k, c in enumerate(line):
            if columns[k] + char_width(c, tab_spaces) - 1 > max_width and char_width(
                c, tab_spaces
            ):
                flags[k].add("ovf")
    code_end = len(line.rstrip(" \t\r"))
    out, k = [], 0
    while k < len(line):
        c = line[k]
        whitespace = c == "\r" or (c in " \t" and (show_whitespace or k >= code_end))
        names = sorted(flags[k] | ({"ws"} if whitespace else {syntax[k]} - {None}))
        run = k + 1
        while run < len(line):
            other = line[run]
            other_whitespace = other == "\r" or (
                other in " \t" and (show_whitespace or run >= code_end)
            )
            other_names = sorted(
                flags[run] | ({"ws"} if other_whitespace else {syntax[run]} - {None})
            )
            if other_names != names:
                break
            run += 1
        text = line[k:run]
        if "ws" in names:
            text = text.replace(" ", "·").replace("\t", "→\t").replace("\r", "␍")
        text = html.escape(text, quote=False)
        out.append(f'<span class="{" ".join(names)}">{text}</span>' if names else text)
        k = run
    return "".join(out)


def render_code(
    text, rows, output, max_width, tab_spaces, show_whitespace, mark_lines=True
):
    lines = split_lines(text)
    syntax = syntax_classes(text)
    shown = output.get("display_lines")
    manual_lines = set(output.get("highlight_lines", []))
    manual_chunks = {}
    for chunk in output.get("highlight_chunks", []):
        manual_chunks.setdefault(chunk["line"], []).append(chunk)
    has_manual = bool(manual_lines or manual_chunks)
    out = []
    for index, ranges in rows:
        number = index + 1
        line = lines[index]
        chunks = []
        columns, _ = line_columns(line, tab_spaces)
        for chunk in manual_chunks.get(number, []):
            chars = [
                k
                for k, col in enumerate(columns)
                if chunk["start_col"] <= col < chunk["end_col"]
            ]
            if chars:
                chunks.append((chars[0], chars[-1] + 1))
        classes = ["l"]
        if has_manual:
            if number in manual_lines:
                classes.append("lsel")
        elif ranges is not None and mark_lines:
            classes.append("lchg")
        if shown and not shown[0] <= number <= shown[1]:
            classes.append("wrap")
        changed = [] if has_manual or ranges is None else ranges
        body = render_line(
            line, syntax[index], tab_spaces, max_width, changed, chunks, show_whitespace
        )
        out.append(
            f'<div class="{" ".join(classes)}"><span class="n">{number}</span>'
            f'<span class="t">{body}</span></div>'
        )
    if not lines:
        out.append(
            '<div class="l none"><span class="n"></span><span class="t">(empty file)</span></div>'
        )
    elif text and not text.endswith("\n"):
        out.append(
            '<div class="l none"><span class="n"></span><span class="t">(no line break at the end)</span></div>'
        )
    return "".join(out)


def count_overflow(text, max_width, tab_spaces):
    return sum(
        line_columns(line, tab_spaces)[1] - 1 > max_width for line in split_lines(text)
    )


def merged_config(sample, output):
    return {**sample.get("config", {}), **output.get("config", {})}


def pane_title(version):
    label = version.get("label", version["id"])
    tool = version["tool"]
    detail = label.removeprefix(tool).strip() if label.startswith(tool) else label
    return (
        f"<strong>{html.escape(tool)}</strong>"
        f' <span class="ver">{html.escape(detail)}</span>'
    )


def render_pane(titles, output, sample, rows, mark_lines=True):
    config = merged_config(sample, output)
    max_width = config.get("max_width", DEFAULT_MAX_WIDTH)
    tab_spaces = config.get("tab_spaces", DEFAULT_TAB_SPACES)
    badges = []
    status = output.get("status", "formatted")
    if status == "gave_up":
        badges.append('<span class="badge warn">left unformatted</span>')
    if status == "error":
        badges.append('<span class="badge err">error</span>')
        code = '<div class="l none"><span class="n"></span><span class="t">(no output)</span></div>'
    else:
        text = output["output"]
        over = count_overflow(text, max_width, tab_spaces)
        if over:
            plural = "line" if over == 1 else "lines"
            badges.append(
                f'<span class="badge over">{over} {plural} over {max_width}</span>'
            )
        code = render_code(
            text,
            rows,
            output,
            max_width,
            tab_spaces,
            sample.get("show_whitespace", False),
            mark_lines,
        )
    note = (
        f'<p class="onote">{inline(output["note"])}</p>' if output.get("note") else ""
    )
    title = " · ".join(titles)
    return (
        f'<section class="pane"><header><span>{title}</span>{"".join(badges)}</header>{note}'
        f'<div class="scroll"><div class="code" style="--mw:{max_width}">{code}</div></div></section>'
    )


def not_run_pane(title):
    return (
        f'<section class="pane"><header><span>{title}</span>'
        '<span class="badge">not run</span></header></section>'
    )


def all_shown_lines_changed(rows, output):
    first, last = output.get("display_lines", (1, len(rows)))
    shown = [ranges for index, ranges in rows if first <= index + 1 <= last]
    return bool(shown) and all(ranges is not None for ranges in shown)


def plain_rows(text):
    return [(k, None) for k in range(len(split_lines(text)))]


def render_pair(sample, by_version, left, right, titles):
    a, b = by_version.get(left), by_version.get(right)
    if a is None or b is None:
        panes = []
        for version, output in ((left, a), (right, b)):
            if output is None:
                panes.append(not_run_pane(titles[version]))
            else:
                rows = plain_rows(output.get("output", ""))
                panes.append(render_pane([titles[version]], output, sample, rows))
        return "".join(panes), False
    if a is b:
        rows = plain_rows(a.get("output", ""))
        return render_pane([titles[left], titles[right]], a, sample, rows), True
    a_text, b_text = a.get("output", ""), b.get("output", "")
    if a.get("status") == "error" or b.get("status") == "error":
        rows_a, rows_b = plain_rows(a_text), plain_rows(b_text)
    else:
        rows_a, rows_b = compare(split_lines(a_text), split_lines(b_text))
    # A background on every line says nothing; the changed text is still marked.
    mark_lines = not (
        all_shown_lines_changed(rows_a, a) or all_shown_lines_changed(rows_b, b)
    )
    return (
        render_pane([titles[left]], a, sample, rows_a, mark_lines)
        + render_pane([titles[right]], b, sample, rows_b, mark_lines)
    ), False


def anchor_link(anchor, what):
    target = html.escape(anchor)
    return f'<a class="anchor" href="#{target}" title="Link to this {what}" aria-label="Link to this {what}">#</a>'


def render_sample(item, position, sample, pairs, titles):
    by_version = {v: o for o in sample["outputs"] for v in o["versions"]}
    views = []
    for k, (left, right) in enumerate(pairs):
        body, same = render_pair(sample, by_version, left, right, titles)
        hidden = " hidden" if k else ""
        cls = "pair one" if same else "pair"
        views.append(
            f'<div class="{cls}" data-left="{left}" data-right="{right}"{hidden}>{body}</div>'
        )
    chips = (
        [f"edition {html.escape(str(sample['edition']))}"]
        if "edition" in sample
        else []
    )
    chips += [
        f"{html.escape(k)} = {html.escape(str(v))}"
        for k, v in sample.get("config", {}).items()
    ]
    meta = "".join(f'<span class="chip">{c}</span>' for c in chips)
    note = f'<p class="note">{inline(sample["note"])}</p>' if sample.get("note") else ""
    source = sample.get("input")
    input_view = ""
    if source is not None:
        opened = " open" if sample.get("show_input", True) else ""
        code = render_code(
            source,
            plain_rows(source),
            {},
            None,
            DEFAULT_TAB_SPACES,
            sample.get("show_whitespace", False),
        )
        input_view = (
            f'<details class="input"{opened}><summary>Input</summary>'
            f'<div class="scroll"><div class="code noruler">{code}</div></div></details>'
        )
    anchor = f"{item['id']}-{position}"
    return (
        f'<article class="sample" id="{html.escape(anchor)}"><h4><span>{inline(sample["title"])}</span>'
        f"{anchor_link(anchor, 'sample')}{meta}</h4>{note}{input_view}{''.join(views)}</article>"
    )


def render_item(item, pairs, titles, names):
    level = item.get("level", "low")
    tags = item.get("tags", [])
    search = " ".join(
        [
            item["id"],
            item["name"],
            item.get("summary", ""),
            item.get("description", ""),
            *tags,
        ]
    )
    tag_html = "".join(f'<span class="tag">{html.escape(t)}</span>' for t in tags)
    parts = [
        (
            f'<section class="item" id="{html.escape(item["id"])}" data-level="{level}"'
            f' data-tags="{html.escape(" ".join(tags))}" data-search="{html.escape(search.lower())}">'
        ),
        f"<header><h3>{inline(item['name'])} {anchor_link(item['id'], 'topic')}</h3>",
        f'<div class="meta"><span class="level {level}">{level}</span>{tag_html}</div>',
    ]
    if item.get("summary"):
        parts.append(f'<p class="summary">{inline(item["summary"])}</p>')
    parts.append("</header>")
    if item.get("rationale"):
        parts.append(
            f'<div class="why"><strong>Why</strong> {inline(item["rationale"])}</div>'
        )
    parts.append(render_details(item, names))
    samples = item.get("samples", [])
    parts += [
        render_sample(item, k + 1, s, pairs, titles) for k, s in enumerate(samples)
    ]
    parts.append("</section>")
    return "".join(parts)


def safe_link(url, text):
    if url.startswith(("https://", "http://")):
        return f'<a href="{html.escape(url)}" rel="noopener">{text}</a>'
    return text


def render_details(item, names):
    rows = []
    guide = item.get("style_guide", {})
    conforms = guide.get("conforms")
    if conforms and conforms != "unspecified":
        link = safe_link(guide.get("url", ""), "Style Guide")
        quote = (
            f"<blockquote>{inline(guide['quote'])}</blockquote>"
            if guide.get("quote")
            else ""
        )
        rows.append(
            f"<dt>{link}</dt><dd>Follows it: <strong>{html.escape(conforms)}</strong>{quote}</dd>"
        )
    elif conforms:
        rows.append("<dt>Style Guide</dt><dd>Does not settle this.</dd>")
    references = item.get("references", [])
    if references:
        links = ", ".join(safe_link(r["url"], inline(r["title"])) for r in references)
        rows.append(f"<dt>References</dt><dd>{links}</dd>")
    related = item.get("related", [])
    if related:
        links = ", ".join(
            f'<a href="#{html.escape(r)}">{inline(names.get(r, r))}</a>'
            for r in related
        )
        rows.append(f"<dt>Related</dt><dd>{links}</dd>")
    description = markdown(item["description"]) if item.get("description") else ""
    facts = f"<dl>{''.join(rows)}</dl>" if rows else ""
    return f'<details class="more"><summary>Details</summary><div class="prose">{description}</div>{facts}</details>'


def version_pairs(versions):
    """Every comparison of the first tool with the second; the newest versions come first."""
    tools = list(dict.fromkeys(v["tool"] for v in versions))
    if len(tools) != 2:
        sys.exit(f"error: expected versions of exactly two tools, got {tools}")
    first = [v["id"] for v in versions if v["tool"] == tools[0]]
    second = [v["id"] for v in versions if v["tool"] == tools[1]]
    return [(a, b) for a in reversed(first) for b in reversed(second)], first, second


def render_select(name, ids, labels):
    if len(ids) < 2:
        return f'<span class="version">{html.escape(labels[ids[0]])}</span>'
    options = "".join(
        f'<option value="{html.escape(v)}">{html.escape(labels[v])}</option>'
        for v in reversed(ids)
    )
    return f'<select data-side="{name}" aria-label="{name} version">{options}</select>'


def render_page(data, title):
    versions = data["versions"]
    items = data["item"]
    labels = {v["id"]: v.get("label", v["id"]) for v in versions}
    titles = {v["id"]: pane_title(v) for v in versions}
    pairs, first, second = version_pairs(versions)
    names = {item["id"]: item["name"] for item in items}
    tags = sorted({t for item in items for t in item.get("tags", [])})
    nav, main = [], []
    for level in LEVELS:
        group = [item for item in items if item.get("level") == level]
        if not group:
            continue
        links = "".join(
            f'<li data-for="{html.escape(i["id"])}"><a href="#{html.escape(i["id"])}">{inline(i["name"])}</a></li>'
            for i in group
        )
        nav.append(
            f'<div class="navgroup" data-level="{level}"><h2>{level.capitalize()} '
            f"<small>{len(group)}</small></h2><ul>{links}</ul></div>"
        )
        main += [render_item(i, pairs, titles, names) for i in group]
    level_chips = "".join(
        f'<button class="fchip level {lv}" data-filter-level="{lv}" aria-pressed="true">{lv}</button>'
        for lv in LEVELS
    )
    tag_chips = "".join(
        f'<button class="fchip" data-filter-tag="{html.escape(t)}" aria-pressed="false">{html.escape(t)}</button>'
        for t in tags
    )
    samples = sum(len(i.get("samples", [])) for i in items)
    versions_html = (
        f'<div class="versions">{render_select("left", first, labels)}'
        f' <span class="dim">vs</span> {render_select("right", second, labels)}</div>'
        if len(pairs) > 1
        else ""
    )
    return (
        PAGE.replace("@TITLE@", html.escape(title))
        .replace("@CSS@", CSS)
        .replace("@JS@", JS)
        .replace("@COUNT@", f"{len(items)} differences, {samples} samples")
        .replace("@VERSIONS@", versions_html)
        .replace("@LEVELS@", level_chips)
        .replace("@TAGS@", tag_chips)
        .replace("@PAIRS@", json.dumps(pairs).replace("</", "<\\/"))
        .replace("@NAV@", "".join(nav))
        .replace("@MAIN@", "".join(main))
    )


PAGE = """<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>@TITLE@</title>
<style>@CSS@</style>
</head>
<body>
<aside id="side">
  <h1>@TITLE@</h1>
  <p class="count" id="count" data-total="@COUNT@">@COUNT@</p>
  @VERSIONS@
  <input id="search" type="search" placeholder="Search ( / )" aria-label="Search differences">
  <div class="filters" role="group" aria-label="Importance">@LEVELS@</div>
  <details class="tagbox"><summary>Tags</summary><div class="filters" role="group" aria-label="Tags">@TAGS@</div></details>
  <nav id="nav">@NAV@</nav>
</aside>
<div id="content">
  <main id="main">@MAIN@<p id="nothing" hidden>No differences match.</p></main>
</div>
<script>const PAIRS = @PAIRS@;
@JS@</script>
</body>
</html>
"""

CSS = """
:root {
  color-scheme: light;
  --bg: #ffffff; --panel: #ffffff; --sunk: #f3f3ef; --line: #dcdcd4; --text: #1f2328; --dim: #646a73;
  --accent: #0b5cad; --high: #a3160b; --medium: #8a5a00; --low: #3c6e47;
  --code-bg: #1f2329; --code-text: #d7dae0; --gutter: #7a828e;
  --chg-line: #2b313b; --chg: #3a4660; --sel-line: #352e4a; --sel: #4f3f75;
  --ovf: #6e2a33; --ovf-text: #ffc2bb; --rule: #e0675e;
  --k: #d7a3ec; --ty: #6fd3d6; --fnc: #7cb9f4; --str: #a3d58a; --num: #e6b07a; --com: #a0a8b4;
  --doc: #a9bdd2; --mac: #f092b8; --at: #dcc66f; --lt: #e6b07a; --pr: #6fd3d6;
}
* { box-sizing: border-box; }
html { scroll-padding-top: 12px; }
body { margin: 0; background: var(--bg); color: var(--text); display: grid;
  grid-template-columns: 300px minmax(0, 1fr); min-height: 100vh;
  font: 15px/1.5 system-ui, -apple-system, "Segoe UI", Roboto, sans-serif; }
a { color: var(--accent); text-decoration: none; }
a:hover { text-decoration: underline; }
code { font: 0.92em ui-monospace, "SF Mono", Menlo, Consolas, "DejaVu Sans Mono", monospace;
  background: var(--sunk); padding: 0 3px; border-radius: 3px; overflow-wrap: anywhere; }
button, select, input { font: inherit; color: inherit; }
button:focus-visible, select:focus-visible, input:focus-visible, a:focus-visible, summary:focus-visible {
  outline: 2px solid var(--accent); outline-offset: 1px; }
.dim { color: var(--dim); }
#side { position: sticky; top: 0; height: 100vh; overflow-y: auto; padding: 16px;
  border-right: 1px solid var(--line); background: var(--panel); }
#side h1 { font-size: 17px; margin: 0; line-height: 1.3; }
.count { color: var(--dim); font-size: 13px; margin: 4px 0 12px; }
#search { width: 100%; padding: 6px 10px; border: 1px solid var(--line); border-radius: 6px; background: var(--bg); }
.filters { display: flex; flex-wrap: wrap; gap: 4px; margin: 10px 0; }
.fchip { border: 1px solid var(--line); background: transparent; border-radius: 12px; padding: 1px 9px;
  font-size: 12.5px; cursor: pointer; color: var(--dim); }
.fchip[aria-pressed="true"] { background: var(--sunk); border-color: var(--dim); color: var(--text); font-weight: 600; }
.tagbox summary { cursor: pointer; font-size: 13px; color: var(--dim); }
#nav h2 { font-size: 12px; text-transform: uppercase; letter-spacing: .05em; color: var(--dim); margin: 16px 0 0; padding-bottom: 4px; border-bottom: 1px solid var(--line); }
#nav small { font-weight: 400; }
#nav ul { list-style: none; margin: 0; padding: 0; }
#nav li { border-bottom: 1px solid var(--line); }
#nav li a { display: block; padding: 7px 8px; font-size: 13px; line-height: 1.35; color: var(--text); }
#nav li a:hover { background: var(--sunk); text-decoration: none; }
#nav li.active a { background: var(--sunk); box-shadow: inset 2px 0 0 var(--accent); }
.version { font-weight: 600; }
.ver { color: var(--dim); font-size: 11.5px; }
.versions { margin-bottom: 12px; font-size: 13px; }
.versions select { width: 100%; max-width: 280px; padding: 2px 4px; background: var(--bg); border: 1px solid var(--line); border-radius: 5px; }
#main { padding: 16px 20px 40px; max-width: 1900px; }
.item h3 { margin: 0; font-size: 18px; line-height: 1.3; font-weight: 650; }
.meta { display: flex; flex-wrap: wrap; gap: 4px; margin: 6px 0; align-items: center; }
.level { font-size: 11.5px; font-weight: 700; text-transform: uppercase; letter-spacing: .04em; }
.level.high { color: var(--high); } .level.medium { color: var(--medium); } .level.low { color: var(--low); }
.fchip.level { text-transform: capitalize; letter-spacing: 0; font-size: 12.5px; }
.fchip.level[aria-pressed="false"] { color: var(--dim); }
.tag, .chip { font-size: 12px; background: var(--sunk); color: var(--dim); border-radius: 10px; padding: 0 8px; }
.summary { margin: 4px 0 0; }
.why { margin: 10px 0; padding: 6px 10px; background: var(--sunk); border-radius: 6px; font-size: 14px; }
.more { margin: 8px 0; font-size: 14px; }
.more > summary, .input > summary { cursor: pointer; color: var(--dim); font-size: 13px; padding: 2px 0; }
.prose { max-width: 90ch; }
.prose p { margin: 8px 0; }
.prose ul { margin: 8px 0; padding-left: 20px; }
.table { overflow-x: auto; }
table { border-collapse: collapse; font-size: 13px; margin: 8px 0; }
th, td { border: 1px solid var(--line); padding: 3px 8px; text-align: left; }
dl { display: grid; grid-template-columns: max-content minmax(0, 1fr); gap: 2px 12px; margin: 8px 0; }
dt { color: var(--dim); } dd { margin: 0; }
blockquote { margin: 4px 0; padding-left: 10px; border-left: 3px solid var(--line); color: var(--dim); }
.sample { margin-top: 24px; padding-top: 16px; border-top: 1px solid var(--line); }
.sample h4 { margin: 0 0 8px; font-size: 14.5px; font-weight: 600; display: flex; flex-wrap: wrap; align-items: baseline; gap: 4px 8px; }
.anchor { color: var(--dim); font-weight: 400; }
@media (hover: hover) {
  .anchor { opacity: 0; }
  h3:hover .anchor, h4:hover .anchor, .anchor:focus-visible { opacity: 1; }
}
.note { margin: 2px 0 6px; font-size: 13.5px; color: var(--dim); }
.onote { margin: 0; padding: 4px 8px; font-size: 13px; color: var(--dim); background: var(--panel); border: 1px solid var(--line); border-width: 0 1px; }
.pair { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 8px; }
.pair[hidden] { display: none; }
.pair.one { grid-template-columns: minmax(0, 1fr); }
.pane { display: flex; flex-direction: column; min-width: 0; }
.pane > header { display: flex; flex-wrap: wrap; gap: 4px 8px; align-items: center; padding: 3px 8px; font-size: 12.5px;
  background: var(--sunk); border: 1px solid var(--line); border-bottom: 0; }
.badge { font-size: 11.5px; border-radius: 10px; padding: 0 7px; border: 1px solid var(--line); color: var(--dim); }
.badge.warn { color: var(--medium); border-color: currentColor; }
.badge.err, .badge.over { color: var(--high); border-color: currentColor; }
.input { margin: 4px 0 8px; }
.scroll { flex: auto; overflow-x: auto; color-scheme: dark; background: var(--code-bg); }
.code { --mw: 100; position: relative; width: max-content; min-width: 100%; padding: 4px 0; color: var(--code-text);
  overflow: clip; /* the ruler past the code must not make it scroll */
  font: 12.5px/1.5 ui-monospace, "SF Mono", Menlo, Consolas, "DejaVu Sans Mono", monospace; tab-size: 4; }
.code::after { content: ""; position: absolute; top: 0; bottom: 0; left: calc((var(--mw) + 5) * 1ch);
  border-left: 1px dashed var(--rule); opacity: .55; pointer-events: none; }
.code.noruler::after { display: none; }
.l { display: block; height: 1.5em; white-space: pre; }
.n { display: inline-block; width: 4ch; margin-right: 1ch; text-align: right; color: var(--gutter); user-select: none; }
.none .t { color: var(--com); font-style: italic; }
.wrap { opacity: .55; }
.lchg { background: var(--chg-line); }
.lsel { background: var(--sel-line); }
.chg { background: var(--chg); border-radius: 2px; }
.sel { background: var(--sel); border-radius: 2px; }
.ovf { background: var(--ovf); color: var(--ovf-text); text-decoration: underline wavy var(--rule) 1px; }
.ws { color: var(--gutter); opacity: .75; }
.kw { color: var(--k); } .pr { color: var(--pr); } .ty { color: var(--ty); } .ct { color: var(--num); }
.fn { color: var(--fnc); } .str { color: var(--str); } .num { color: var(--num); } .com { color: var(--com); font-style: italic; }
.doc { color: var(--doc); font-style: italic; } .mac { color: var(--mac); } .at { color: var(--at); }
.lt { color: var(--lt); } .mv { color: var(--mac); }
#nothing { color: var(--dim); padding: 40px 0; text-align: center; }
[hidden] { display: none !important; }
@media (max-width: 1250px) {
  .pair { grid-template-columns: minmax(0, 1fr); }
}
@media (max-width: 800px) {
  body { display: block; }
  #side { position: static; height: auto; max-height: 55vh; border-right: 0; border-bottom: 1px solid var(--line); }
  #main { padding-left: 16px; padding-right: 16px; }
}
"""

JS = r"""
(function () {
  const $ = (s, r) => (r || document).querySelector(s);
  const $$ = (s, r) => Array.from((r || document).querySelectorAll(s));

  const items = $$('.item');
  const search = $('#search');
  const count = $('#count');
  const nothing = $('#nothing');
  const navItems = new Map($$('#nav li').map((li) => [li.dataset.for, li]));

  let matching = items;
  let current = items[0];

  function revealInNav(li) {
    // scrollIntoView would move the page as well.
    const side = $('#side');
    const box = li.getBoundingClientRect();
    const view = side.getBoundingClientRect();
    if (box.top < view.top) side.scrollTop -= view.top - box.top;
    else if (box.bottom > view.bottom) side.scrollTop += box.bottom - view.bottom;
  }

  function showCurrent() {
    for (const item of items) {
      const shown = item === current && matching.includes(item);
      item.hidden = !shown;
      navItems.get(item.id).classList.toggle('active', shown);
      if (shown) revealInNav(navItems.get(item.id));
    }
  }

  function applyFilters() {
    const words = search.value.toLowerCase().split(/\s+/).filter(Boolean);
    const levels = new Set($$('[data-filter-level][aria-pressed="true"]').map((b) => b.dataset.filterLevel));
    const tags = $$('[data-filter-tag][aria-pressed="true"]').map((b) => b.dataset.filterTag);
    matching = items.filter((item) => {
      const itemTags = item.dataset.tags.split(' ');
      return levels.has(item.dataset.level)
        && tags.every((t) => itemTags.includes(t))
        && words.every((w) => item.dataset.search.includes(w));
    });
    for (const item of items) navItems.get(item.id).hidden = !matching.includes(item);
    for (const group of $$('.navgroup')) group.hidden = !$('li:not([hidden])', group);
    nothing.hidden = matching.length > 0;
    count.textContent = matching.length === items.length
      ? count.dataset.total
      : matching.length + ' of ' + items.length + ' differences';
    if (matching.length && !matching.includes(current)) {
      current = matching[0];
      // Otherwise a link to the topic in the old address would not fire hashchange.
      history.replaceState(null, '', '#' + current.id);
    }
    showCurrent();
  }
  search.addEventListener('input', applyFilters);
  document.addEventListener('click', (e) => {
    const chip = e.target.closest('.fchip');
    if (!chip) return;
    chip.setAttribute('aria-pressed', chip.getAttribute('aria-pressed') === 'true' ? 'false' : 'true');
    applyFilters();
  });
  document.addEventListener('keydown', (e) => {
    if (e.key === '/' && !/^(INPUT|SELECT|TEXTAREA)$/.test(document.activeElement.tagName)) {
      e.preventDefault();
      search.focus();
    } else if (e.key === 'Escape' && document.activeElement === search) {
      search.value = '';
      applyFilters();
      search.blur();
    }
  });

  function openHash() {
    const target = document.getElementById(location.hash.slice(1));
    const item = target && target.closest('.item');
    if (!item) return;
    // A link to something a filter hides shows everything again.
    if (!matching.includes(item)) {
      search.value = '';
      $$('[data-filter-level]').forEach((b) => b.setAttribute('aria-pressed', 'true'));
      $$('[data-filter-tag]').forEach((b) => b.setAttribute('aria-pressed', 'false'));
      applyFilters();
    }
    current = item;
    showCurrent();
    (target === item ? $('#content') : target).scrollIntoView();
  }
  window.addEventListener('hashchange', openHash);

  const selects = $$('select[data-side]');
  function selected(side, fallback) {
    const select = selects.find((s) => s.dataset.side === side);
    return select ? select.value : fallback;
  }
  function applyVersions() {
    const left = selected('left', PAIRS[0][0]);
    const right = selected('right', PAIRS[0][1]);
    for (const pair of $$('.pair')) {
      pair.hidden = pair.dataset.left !== left || pair.dataset.right !== right;
    }
  }
  selects.forEach((s) => s.addEventListener('change', applyVersions));

  // Panes of a comparison scroll together, so the same columns stay in view.
  let syncing = false;
  document.addEventListener('scroll', (e) => {
    const scroller = e.target;
    if (syncing || !scroller.classList || !scroller.classList.contains('scroll')) return;
    const pair = scroller.closest('.pair');
    if (!pair) return;
    syncing = true;
    for (const other of $$('.scroll', pair)) {
      if (other !== scroller) other.scrollLeft = scroller.scrollLeft;
    }
    requestAnimationFrame(() => { syncing = false; });
  }, true);

  applyFilters();
  openHash();
})();
"""


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("input", type=Path, help="the TOML file of differences")
    parser.add_argument(
        "-o",
        "--output",
        type=Path,
        help="the HTML file (default: the input with .html)",
    )
    parser.add_argument(
        "--title", default="apfmt and rustfmt differences", help="the title of the page"
    )
    args = parser.parse_args()
    with args.input.open("rb") as file:
        data = tomllib.load(file)
    output = args.output or args.input.with_suffix(".html")
    output.write_text(render_page(data, args.title), encoding="utf-8", newline="")
    print(
        f"{output}: {output.stat().st_size / 1_000_000:.1f} MB, {len(data['item'])} differences"
    )


if __name__ == "__main__":
    main()
