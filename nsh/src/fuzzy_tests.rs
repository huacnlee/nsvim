use super::*;

#[test]
fn file_name_matches_beat_folder_matches() {
    let items = ["src/editor/motion.rs", "src/mode/other.rs", "tests/motions.rs"];
    let (m, total) = rank("mot", &items, 10);
    assert_eq!(total, 3);
    assert_eq!(items[m[0].index], "src/editor/motion.rs");
}

#[test]
fn chars_must_appear_in_order() {
    assert!(score("tom", "motion.rs").is_none());
    assert_eq!(score("mrs", "m.rs").unwrap().1, vec![0, 2, 3]);
}

/// The first ranking, kept as the reference: one `Vec<char>` per text and a full sort.
mod reference {
    pub fn score(query: &str, text: &str) -> Option<(i64, Vec<usize>)> {
        if query.is_empty() {
            return Some((0, Vec::new()));
        }
        let smart = query.chars().any(|c| c.is_uppercase());
        let norm = |c: char| if smart { c } else { c.to_ascii_lowercase() };
        let t: Vec<char> = text.chars().collect();
        let q: Vec<char> = query.chars().map(norm).filter(|c| !c.is_whitespace()).collect();
        if q.is_empty() {
            return Some((0, Vec::new()));
        }
        let name_start = t.iter().rposition(|c| *c == '/').map_or(0, |i| i + 1);
        let hits = find(&t, &q, name_start, norm).or_else(|| find(&t, &q, 0, norm))?;
        let mut s: i64 = 0;
        let mut prev: Option<usize> = None;
        for &i in &hits {
            s += 10;
            if i >= name_start {
                s += 12;
            }
            let at_word = i == 0 || matches!(t[i - 1], '/' | '_' | '-' | '.' | ' ') || (t[i].is_uppercase() && t[i - 1].is_lowercase());
            if at_word {
                s += 18;
            }
            if let Some(p) = prev {
                if i == p + 1 {
                    s += 16;
                } else {
                    s -= (i - p) as i64 / 2;
                }
            }
            prev = Some(i);
        }
        if hits.first() == Some(&name_start) {
            s += 20;
        }
        s -= (t.len() - name_start) as i64 / 2 + t.len() as i64 / 16;
        Some((s, hits))
    }

    fn find(t: &[char], q: &[char], from: usize, norm: impl Fn(char) -> char) -> Option<Vec<usize>> {
        let mut hits = Vec::with_capacity(q.len());
        let mut j = 0;
        for (i, &c) in t.iter().enumerate().skip(from) {
            if j < q.len() && norm(c) == q[j] {
                hits.push(i);
                j += 1;
            }
        }
        (j == q.len()).then_some(hits)
    }

    pub fn rank(query: &str, items: &[String], limit: usize) -> (Vec<(usize, i64, Vec<usize>)>, usize) {
        let mut out: Vec<(usize, i64, Vec<usize>)> =
            items.iter().enumerate().filter_map(|(i, it)| score(query, it).map(|(s, h)| (i, s, h))).collect();
        let total = out.len();
        out.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| items[a.0].len().cmp(&items[b.0].len())));
        out.truncate(limit);
        (out, total)
    }
}

/// Paths with folders, mixed case, separators, repeats and text outside ASCII.
fn paths(n: usize) -> Vec<String> {
    let names = ["main.rs", "MotionView.tsx", "motion_test.go", "README.md", "über_café.rs", "日本/語.txt", "a-b.c d", "Cargo.toml"];
    (0..n).map(|i| format!("pkg{}/src{}/mod_{}/{}", i % 7, i % 13, i % 5, names[i % names.len()])).collect()
}

#[test]
fn scores_match_the_reference() {
    let items = paths(400);
    for q in ["", " ", "mot", "MV", "mot view", "über", "Ü", "語", "rs", "zz", "pkg3src", "c d", "a-b"] {
        for t in &items {
            assert_eq!(score(q, t), reference::score(q, t), "query {q:?} on {t:?}");
        }
    }
}

#[test]
fn ranks_match_the_reference_on_one_thread_and_several() {
    for n in [50, PARALLEL_FROM + 777] {
        let items = paths(n);
        for q in ["", "mot", "MV", "rs", "über", "zz", "pkg3src"] {
            for limit in [0, 1, 7, 200] {
                let (got, total) = rank(q, &items, limit);
                let (want, want_total) = reference::rank(q, &items, limit);
                assert_eq!(total, want_total, "total for {q:?} over {n}");
                let got: Vec<(usize, i64, Vec<usize>)> = got.into_iter().map(|m| (m.index, m.score, m.hits)).collect();
                assert_eq!(got, want, "ranking for {q:?} over {n}, limit {limit}");
            }
        }
    }
}
