// Adversary for the single-line walks over the doc (`Formatter::has_flat_layout` and
// `Formatter::flat_width`).
//
// A closure as the only argument of a method call, nested: each level offers several layouts that
// contain the inner closure before any line break. Without a cache, each level walks the inner doc
// about twice and the time doubles with every level; with the `AltFlatInfo` of `Alt` nodes
// (`Formatter::alt_doc`), each `Alt` is measured once and the walks stop at the `Alt` nodes.
//
// - `cannot_be_flat`: the innermost body is a block with statements, so nothing can be on one line
//   and `has_flat_layout` explores every alternative.
// - `can_be_flat`: the innermost body is `x`; `flat_width` walks every alternative.
//
// Release build, 2026-10-14, one function of that depth per file, best of 3 (including the process
// start); "before": `FlatLimit::MaxWidth` did not measure and nothing was cached, "uncached":
// `MaxWidth` measured with the uncached walk (measured once, process start about 0.11 s):
//
//   depth                         16      20      22      24      26      30      60
//   cannot_be_flat  before      11 ms   23 ms   37 ms   47 ms   63 ms   95 ms  641 ms
//                   uncached   0.11 s  0.61 s  2.22 s
//                   Alt cache    9 ms   23 ms   26 ms   32 ms   43 ms   79 ms  401 ms
//   can_be_flat     before      58 ms  344 ms  1.23 s  4.71 s  18.8 s  >60 s
//                   uncached   0.11 s  0.81 s  2.92 s
//                   Alt cache   23 ms   42 ms   65 ms   72 ms   91 ms  125 ms  785 ms
//
// Both functions below have depth 20.
fn cannot_be_flat() {
    let v = x.all(|a| x.all(|a| x.all(|a| x.all(|a| x.all(|a| x.all(|a| x.all(|a| x.all(|a| x.all(|a| x.all(|a| x.all(|a| x.all(|a| x.all(|a| x.all(|a| x.all(|a| x.all(|a| x.all(|a| x.all(|a| x.all(|a| x.all(|a| { x; y }))))))))))))))))))));
}

fn can_be_flat() {
    let v = x.all(|a| x.all(|a| x.all(|a| x.all(|a| x.all(|a| x.all(|a| x.all(|a| x.all(|a| x.all(|a| x.all(|a| x.all(|a| x.all(|a| x.all(|a| x.all(|a| x.all(|a| x.all(|a| x.all(|a| x.all(|a| x.all(|a| x.all(|a| x))))))))))))))))))));
}
