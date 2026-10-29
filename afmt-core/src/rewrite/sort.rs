//! Version sorting of names, as the Style Guide defines it ("Sorting")

use std::cmp::Ordering;

/// A maximal run of ASCII digits, a `_`, or a run of other characters
enum Chunk<'a> {
    Underscore,
    Number(&'a str),
    Text(&'a str),
}

fn chunks<'a>(s: &'a str) -> impl Iterator<Item = Chunk<'a>> {
    let mut rest = s;
    std::iter::from_fn(move || {
        let first = rest.chars().next()?;
        let run = |end: Option<usize>| end.unwrap_or(rest.len());
        let (len, chunk): (usize, fn(&'a str) -> Chunk<'a>) = match first {
            '_' => (1, |_| Chunk::Underscore),
            '0'..='9' => (run(rest.find(|c: char| !c.is_ascii_digit())), Chunk::Number),
            _ => (
                run(rest.find(|c: char| c == '_' || c.is_ascii_digit())),
                Chunk::Text,
            ),
        };
        let (text, tail) = rest.split_at(len);
        rest = tail;
        Some(chunk(text))
    })
}

/// Characters that can start an `UpperCamelCase` identifier sort before lowercase ones
fn compare_text(a: &str, b: &str) -> Ordering {
    let key = |c: char| (c.is_lowercase(), c);
    a.chars().map(key).cmp(b.chars().map(key))
}

/// `x8` < `x16`, `_` first, uppercase before lowercase; on a tie, more leading zeros first
pub(crate) fn version_cmp(a: &str, b: &str) -> Ordering {
    let mut leading_zeros = Ordering::Equal;
    let mut a_chunks = chunks(a);
    let mut b_chunks = chunks(b);
    loop {
        let (a, b) = match (a_chunks.next(), b_chunks.next()) {
            (None, None) => return leading_zeros,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(a), Some(b)) => (a, b),
        };
        let order = match (a, b) {
            (Chunk::Underscore, Chunk::Underscore) => Ordering::Equal,
            (Chunk::Underscore, _) => Ordering::Less,
            (_, Chunk::Underscore) => Ordering::Greater,
            (Chunk::Number(a), Chunk::Number(b)) => {
                let a_value = a.trim_start_matches('0');
                let b_value = b.trim_start_matches('0');
                let order = a_value.len().cmp(&b_value.len()).then(a_value.cmp(b_value));
                if order == Ordering::Equal && leading_zeros == Ordering::Equal {
                    // More leading zeros first
                    leading_zeros = b.len().cmp(&a.len());
                }
                order
            }
            (Chunk::Number(a) | Chunk::Text(a), Chunk::Number(b) | Chunk::Text(b)) => {
                compare_text(a, b)
            }
        };
        if order != Ordering::Equal {
            return order;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::version_cmp;

    fn sorted(names: &[&'static str]) -> Vec<&'static str> {
        let mut names = names.to_vec();
        names.sort_by(|a, b| version_cmp(a, b));
        names
    }

    #[test]
    fn numbers_by_value() {
        assert_eq!(sorted(&["x16", "x8", "x1"]), ["x1", "x8", "x16"]);
        assert_eq!(
            sorted(&["a10", "a2", "a02", "a1"]),
            ["a1", "a02", "a2", "a10"]
        );
        assert_eq!(
            sorted(&["1_010_001", "1_000_000"]),
            ["1_000_000", "1_010_001"]
        );
    }

    #[test]
    fn underscore_first() {
        assert_eq!(sorted(&["a1", "a_b", "ab"]), ["a_b", "a1", "ab"]);
        assert_eq!(sorted(&["aaaaa", "aaa_a"]), ["aaa_a", "aaaaa"]);
        assert_eq!(sorted(&["__", "_"]), ["_", "__"]);
        assert_eq!(sorted(&["foo_", "foo"]), ["foo", "foo_"]);
        assert_eq!(sorted(&["Ab", "_a"]), ["_a", "Ab"]);
    }

    #[test]
    fn uppercase_first() {
        assert_eq!(
            sorted(&["b", "aa", "aA", "a", "B", "AA", "A"]),
            ["A", "AA", "B", "a", "aA", "aa", "b"]
        );
        assert_eq!(
            sorted(&["x86_128", "X86_128", "x86_64", "X86_64"]),
            ["X86_64", "X86_128", "x86_64", "x86_128"]
        );
        assert_eq!(sorted(&["u8", "a", "U8"]), ["U8", "a", "u8"]);
    }

    #[test]
    fn prefix_first() {
        assert_eq!(
            sorted(&["applesauce", "apple", ""]),
            ["", "apple", "applesauce"]
        );
        assert_eq!(sorted(&["xxx", "x7x"]), ["x7x", "xxx"]);
    }
}
