//! Test cases for `picoquictest/splay_test.c`.

#![allow(non_snake_case)]

use crate::splay::SplayTree;

/// C: `splay_test` in `picoquictest/splay_test.c`.
///
/// Insert seven values, check the running min / max after every
/// insert, look up every key 0..15 (with previous-or-equal lookups
/// alongside), then delete the values and re-check the running min /
/// max.  All operations go through [`SplayTree<i32, ()>`]; the C
/// version's intrusive node + four-function-pointer setup
/// disappears.
#[test]
fn splay() {
    let values = [5, 7, 1, 3, 13, 9, 11];
    let values_first = [5, 5, 1, 1, 1, 1, 1];
    let values_last = [5, 7, 7, 7, 13, 13, 13];
    // (key probed, expected previous-or-equal value, -1 means none).
    let previous_test: [i32; 15] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14];
    let previous_value: [i32; 15] = [-1, 1, 1, 3, 3, 5, 5, 7, 7, 9, 9, 11, 11, 13, 13];
    let value2_first = [1, 1, 3, 9, 9, 11, 0];
    let value2_last = [13, 13, 13, 13, 11, 11, 0];

    let mut tree: SplayTree<i32, ()> = SplayTree::new();

    // Insertion + running min / max after each insert.
    for (i, &v) in values.iter().enumerate() {
        let (_token, prev) = tree.insert(v, ()).expect("insert under cap");
        assert!(prev.is_none(), "duplicate insert at i={i}, v={v}");
        let first = tree
            .first()
            .and_then(|t| tree.get_key_value(t))
            .map(|(k, _)| *k);
        let last = tree
            .last()
            .and_then(|t| tree.get_key_value(t))
            .map(|(k, _)| *k);
        assert_eq!(first, Some(values_first[i]), "first after insert i={i}");
        assert_eq!(last, Some(values_last[i]), "last after insert i={i}");
    }

    // Direct lookups: succeed when the key is in `values`, fail otherwise.
    for (i, &probe) in previous_test.iter().enumerate() {
        let token = tree.find(&probe);
        if previous_value[i] == probe {
            let token = token.expect("hit on present key");
            let (k, _) = tree.get_key_value(token).expect("token live");
            assert_eq!(*k, previous_value[i], "find at i={i}, probe={probe}");
        } else {
            assert!(token.is_none(), "false hit at i={i}, probe={probe}");
        }
    }

    // Previous-or-equal lookups: succeed when there's any key ≤ probe.
    for (i, &probe) in previous_test.iter().enumerate() {
        let token = tree.find_previous(&probe);
        if previous_value[i] >= 0 {
            let token = token.expect("previous hit");
            let (k, _) = tree.get_key_value(token).expect("token live");
            assert_eq!(
                *k, previous_value[i],
                "find_previous at i={i}, probe={probe}"
            );
        } else {
            assert!(
                token.is_none(),
                "false previous-hit at i={i}, probe={probe}"
            );
        }
    }

    // Deletion + running min / max after each delete.
    for (i, &v) in values.iter().enumerate() {
        tree.remove_by_key(&v).expect("delete present key");
        if i < 6 {
            let first = tree
                .first()
                .and_then(|t| tree.get_key_value(t))
                .map(|(k, _)| *k);
            let last = tree
                .last()
                .and_then(|t| tree.get_key_value(t))
                .map(|(k, _)| *k);
            assert_eq!(first, Some(value2_first[i]), "first after delete i={i}");
            assert_eq!(last, Some(value2_last[i]), "last after delete i={i}");
        }
    }

    // Tree should be empty.
    assert!(tree.first().is_none(), "tree not empty after all deletes");
    assert!(tree.last().is_none(), "tree not empty after all deletes");
}
