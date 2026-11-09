use super::*;

fn commit(root: &str, id: &str, parents: &[&str]) -> ScopedCommit {
    ScopedCommit {
        root: root.into(),
        commit: Commit {
            id: id.into(),
            parents: parents.iter().map(|id| (*id).into()).collect(),
            subject: id.into(),
            author: String::new(),
            date: String::new(),
            timestamp: 0,
            refs: String::new(),
            graph: GraphRow::default(),
        },
    }
}

#[test]
fn identical_hashes_in_clones_keep_separate_graph_lanes_across_pages() {
    let mut first = vec![
        commit("backend", "tip", &["base"]),
        commit("front", "tip", &["base"]),
    ];
    let lanes = assign_graph(&mut first, Vec::new());
    assert_eq!(lanes.len(), 2);
    assert_ne!(first[0].commit.graph.lane, first[1].commit.graph.lane);
    assert_ne!(first[0].key(), first[1].key());
    let mut second = vec![commit("front", "base", &[]), commit("backend", "base", &[])];
    assert!(assign_graph(&mut second, lanes).is_empty());
    assert_eq!(second[0].commit.graph.lane, 1);
    assert_eq!(second[0].commit.graph.outgoing, vec![(0, 0)]);
    assert_eq!(second[1].commit.graph.lane, 0);
    assert!(second[1].commit.graph.outgoing.is_empty());
}
