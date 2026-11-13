use super::*;

fn file(name: &str, directory: &str) -> FileCandidate {
    FileCandidate {
        name: name.to_owned(),
        directory: directory.to_owned(),
        path: PathBuf::from(format!("/{directory}/{name}")),
    }
}

#[test]
fn score_rejects_empty_queries_missing_subsequences_and_case_mismatches() {
    assert_eq!(score("", "main.rs"), None);
    assert_eq!(score("xyz", "main.rs"), None);
    // Smart case: an uppercase query character requires the same case.
    assert_eq!(score("ML", "main.rs"), None);
    assert_eq!(score("Ml", "main.rs"), None);
    assert!(score("ml", "model.rs").is_some());
    assert!(score("Main", "Main.rs").is_some());
}

#[test]
fn score_prefers_prefixes_boundaries_and_consecutive_matches() {
    let prefix = score("ma", "main.rs");
    let scattered = score("mi", "main.rs");
    assert!(prefix.is_some_and(|prefix| prefix > 0));
    assert!(
        prefix
            .zip(scattered)
            .is_some_and(|(prefix, scattered)| prefix > scattered)
    );

    let whole_word = score("main", "main.rs");
    assert!(
        whole_word
            .zip(scattered)
            .is_some_and(|(whole, scattered)| whole > scattered)
    );

    let boundary = score("r", "main.rs");
    let middle = score("a", "main.rs");
    assert!(boundary.is_some_and(|boundary| boundary > 0));
    assert!(
        boundary
            .zip(middle)
            .is_some_and(|(boundary, middle)| boundary > middle)
    );
}

#[test]
fn rank_orders_name_matches_above_path_matches_and_breaks_ties_by_name() {
    let files = vec![
        file("docs.md", ""),       // name match
        file("readme.md", "docs"), // path-only match, later name
        file("alpha.md", "docs"),  // path-only match, earlier name
    ];
    let ranked = rank("docs", &files);
    assert_eq!(ranked.first(), Some(&0));
    let path_matches: Vec<usize> = ranked.iter().copied().filter(|index| *index != 0).collect();
    assert_eq!(path_matches, vec![2, 1]);
}

#[test]
fn rank_returns_empty_for_empty_queries_and_unmatched_files() {
    let files = vec![file("main.rs", "src")];
    assert!(rank("", &files).is_empty());
    assert!(rank("zzz", &files).is_empty());
}

#[test]
fn rank_limits_results_and_keeps_order_stable() {
    let files: Vec<_> = (0..(MAX_RESULTS + 10))
        .map(|index| file(&format!("module{index:02}.rs"), "src"))
        .collect();
    let ranked = rank("module", &files);
    assert_eq!(ranked.len(), MAX_RESULTS);
    assert_eq!(ranked, rank("module", &files));
    // Name sorting places module00 first among equal scores.
    assert_eq!(ranked.first(), Some(&0));
}

#[test]
fn rank_matches_names_and_directories() {
    let files = vec![file("app.rs", "src")];
    // The name alone matches.
    assert_eq!(rank("app", &files), vec![0]);
    // The directory must come before the name in the query.
    assert_eq!(rank("srcapp", &files), vec![0]);
    assert_eq!(rank("srapp", &files), vec![0]);
    // A directory after the name cannot match.
    assert!(rank("appsrc", &files).is_empty());
}
