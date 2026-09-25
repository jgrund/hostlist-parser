// Copyright (c) 2022 DDN. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found in the LICENSE file.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Pad {
    Literal,
    Natural,
    Width(usize),
}

impl Pad {
    fn width(self) -> usize {
        match self {
            Pad::Width(w) => w,
            Pad::Literal | Pad::Natural => 0,
        }
    }
}

enum Name<'a> {
    Literal(&'a str),
    Numbered(&'a str, &'a str, &'a str, u64),
}

type Groups<'a> = BTreeMap<(&'a str, &'a str, Pad), BTreeSet<u64>>;

/// Folds names into a canonical hostlist expression; a name holding `[`, `]`, `,` or whitespace passes through unchanged.
pub fn fold<I, S>(names: I) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let names: Vec<S> = names.into_iter().collect();

    let xs: Vec<Name> = names.iter().map(|x| split(x.as_ref())).collect();

    let padded: BTreeSet<(&str, &str, usize)> = xs
        .iter()
        .filter_map(|x| match x {
            Name::Numbered(prefix, digits, suffix, _)
                if digits.len() > 1 && digits.starts_with('0') =>
            {
                Some((*prefix, *suffix, digits.len()))
            }
            _ => None,
        })
        .collect();

    let mut groups: Groups = BTreeMap::new();

    for x in xs {
        match x {
            Name::Literal(x) => {
                groups.entry((x, "", Pad::Literal)).or_default();
            }
            Name::Numbered(prefix, digits, suffix, n) => {
                let pad = if padded.contains(&(prefix, suffix, digits.len())) {
                    Pad::Width(digits.len())
                } else {
                    Pad::Natural
                };

                groups.entry((prefix, suffix, pad)).or_default().insert(n);
            }
        }
    }

    groups
        .iter()
        .map(|(&(prefix, suffix, pad), ns)| render(prefix, suffix, pad, ns))
        .collect::<Vec<_>>()
        .join(",")
}

fn split(x: &str) -> Name<'_> {
    if x.contains(|c: char| matches!(c, '[' | ']' | ',') || c.is_whitespace()) {
        return Name::Literal(x);
    }

    let Some(end) = x.rfind(|c: char| c.is_ascii_digit()) else {
        return Name::Literal(x);
    };

    let (head, suffix) = x.split_at(end + 1);

    let (prefix, digits) = head.split_at(head.trim_end_matches(|c: char| c.is_ascii_digit()).len());

    match digits.parse() {
        Ok(n) => Name::Numbered(prefix, digits, suffix, n),
        Err(_) => Name::Literal(x),
    }
}

fn runs(ns: &BTreeSet<u64>) -> Vec<(u64, u64)> {
    let mut xs: Vec<(u64, u64)> = vec![];

    for &n in ns {
        match xs.last_mut() {
            Some((_, b)) if *b + 1 == n => *b = n,
            _ => xs.push((n, n)),
        }
    }

    xs
}

fn render(prefix: &str, suffix: &str, pad: Pad, ns: &BTreeSet<u64>) -> String {
    let w = pad.width();

    let body = runs(ns)
        .into_iter()
        .map(|(a, b)| {
            if a == b {
                format!("{a:0>w$}")
            } else {
                format!("{a:0>w$}-{b:0>w$}")
            }
        })
        .collect::<Vec<_>>()
        .join(",");

    match (pad, ns.len()) {
        (Pad::Literal, _) => prefix.to_string(),
        (_, 1) => format!("{prefix}{body}{suffix}"),
        _ => format!("{prefix}[{body}]{suffix}"),
    }
}

#[cfg(test)]
mod tests {
    use crate::{fold, parse};
    use insta::assert_snapshot;
    use proptest::prelude::*;

    fn name() -> impl Strategy<Value = String> {
        (
            prop::sample::select(vec!["", "n", "oss", "r1n"]),
            prop::option::of((1_usize..=3, 0_u64..=120)),
            prop::sample::select(vec!["", "-ib", "a"]),
        )
            .prop_map(|(prefix, digits, suffix)| match digits {
                Some((w, n)) => format!("{prefix}{n:0>w$}{suffix}"),
                None => format!("{prefix}{suffix}"),
            })
            .prop_filter("empty name", |x| !x.is_empty())
    }

    fn set(xs: &[String]) -> Vec<String> {
        let mut xs = xs.to_vec();

        xs.sort();
        xs.dedup();

        xs
    }

    proptest! {
        #[test]
        fn parse_inverts_fold(xs in prop::collection::vec(name(), 0..=40)) {
            let folded = fold(&xs);

            if xs.is_empty() {
                prop_assert_eq!(folded, "");
            } else {
                let parsed = parse(&folded).map_err(|e| TestCaseError::fail(format!("{folded}: {e}")))?;

                prop_assert_eq!(set(&parsed), set(&xs), "{}", folded);
                prop_assert_eq!(fold(&parsed), folded);
            }
        }
    }

    #[test]
    fn consecutive() {
        assert_snapshot!(fold(["n1", "n2", "n3"]), @"n[1-3]");
    }

    #[test]
    fn gap() {
        assert_snapshot!(fold(["n1", "n2", "n5"]), @"n[1-2,5]");
    }

    #[test]
    fn padded() {
        assert_snapshot!(fold(["n01", "n02", "n03"]), @"n[01-03]");
    }

    #[test]
    fn padded_and_natural() {
        assert_snapshot!(fold(["n1", "n01"]), @"n1,n01");
    }

    #[test]
    fn padded_absorbs_full_width() {
        assert_snapshot!(fold(["n08", "n09", "n10", "n11"]), @"n[08-11]");
    }

    #[test]
    fn padded_crosses_digit_count() {
        assert_snapshot!(fold(["n098", "n099", "n100", "n101"]), @"n[098-101]");
    }

    #[test]
    fn prefixes() {
        assert_snapshot!(fold(["a1", "b1", "a2"]), @"a[1-2],b1");
    }

    #[test]
    fn literal() {
        assert_snapshot!(fold(["foo"]), @"foo");
    }

    #[test]
    fn last_digit_run() {
        assert_snapshot!(fold(["r1n1", "r1n2"]), @"r1n[1-2]");
    }

    #[test]
    fn suffix() {
        assert_snapshot!(fold(["n1-ib", "n2-ib"]), @"n[1-2]-ib");
    }

    #[test]
    fn passthrough() {
        assert_snapshot!(fold(["n[1-3]", "n4"]), @"n4,n[1-3]");
    }

    #[test]
    fn empty() {
        assert_snapshot!(fold(Vec::<String>::new()), @"");
    }

    #[test]
    fn address() {
        assert_snapshot!(fold(["10.0.0.11", "10.0.0.12"]), @"10.0.0.[11-12]");
    }
}
