use std::path::{Path, PathBuf};
#[cfg(feature = "rusqlite-c")]
use std::time::Instant;

#[cfg(all(feature = "turso-rust", feature = "rusqlite-c"))]
use std::collections::HashSet;
#[cfg(all(feature = "turso-rust", feature = "rusqlite-c"))]
use std::time::Duration;

#[cfg(all(feature = "turso-rust", feature = "rusqlite-c"))]
use cirup_core::Resource;
#[cfg(all(feature = "turso-rust", feature = "rusqlite-c"))]
use cirup_core::Triple;
#[cfg(feature = "rusqlite-c")]
use cirup_core::config::QueryBackendKind;
#[cfg(feature = "rusqlite-c")]
use cirup_core::query;

struct FixtureTriplet {
    #[cfg(feature = "rusqlite-c")]
    name: &'static str,
    en: &'static str,
    fr: &'static str,
    de: &'static str,
}

fn fixtures_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("test/benchmark/rdm_resx")
}

fn fixture_triplets() -> Vec<FixtureTriplet> {
    vec![
        FixtureTriplet {
            #[cfg(feature = "rusqlite-c")]
            name: "UIResources",
            en: "UIResources.resx",
            fr: "UIResources.fr.resx",
            de: "UIResources.de.resx",
        },
        FixtureTriplet {
            #[cfg(feature = "rusqlite-c")]
            name: "MsgResources",
            en: "MsgResources.resx",
            fr: "MsgResources.fr.resx",
            de: "MsgResources.de.resx",
        },
        FixtureTriplet {
            #[cfg(feature = "rusqlite-c")]
            name: "BusinessResources",
            en: "BusinessResources.resx",
            fr: "BusinessResources.fr.resx",
            de: "BusinessResources.de.resx",
        },
    ]
}

#[cfg(feature = "rusqlite-c")]
fn fixture_path(file_name: &str) -> String {
    fixtures_root().join(file_name).to_string_lossy().to_string()
}

#[cfg(all(feature = "turso-rust", feature = "rusqlite-c"))]
fn normalize(resources: &mut [Resource]) {
    resources.sort_by(|a, b| a.name.cmp(&b.name).then(a.value.cmp(&b.value)));
}

#[cfg(all(feature = "turso-rust", feature = "rusqlite-c"))]
fn normalize_triples(triples: &mut [Triple]) {
    triples.sort_by(|a, b| {
        a.name
            .cmp(&b.name)
            .then(a.value.cmp(&b.value))
            .then(a.base.cmp(&b.base))
    });
}

#[cfg(all(feature = "turso-rust", feature = "rusqlite-c"))]
fn triples_to_tuples(triples: &[Triple]) -> Vec<(String, String, String)> {
    triples
        .iter()
        .map(|triple| (triple.name.clone(), triple.value.clone(), triple.base.clone()))
        .collect()
}

#[cfg(all(feature = "turso-rust", feature = "rusqlite-c"))]
fn benchmark_resource_operation<F>(label: &str, ordered: bool, mut operation: F) -> (Duration, Duration, usize)
where
    F: FnMut(QueryBackendKind) -> Vec<Resource>,
{
    let started = Instant::now();
    let mut rusqlite_result = operation(QueryBackendKind::Rusqlite);
    let rusqlite_elapsed = started.elapsed();

    let started = Instant::now();
    let mut turso_result = operation(QueryBackendKind::TursoLocal);
    let turso_elapsed = started.elapsed();

    let rows = rusqlite_result.len();

    if ordered {
        assert_eq!(rusqlite_result, turso_result, "ordered result mismatch for {label}");
    } else {
        normalize(&mut rusqlite_result);
        normalize(&mut turso_result);
        assert_eq!(rusqlite_result, turso_result, "result mismatch for {label}");
    }

    (rusqlite_elapsed, turso_elapsed, rows)
}

#[cfg(all(feature = "turso-rust", feature = "rusqlite-c"))]
fn benchmark_triple_operation<F>(label: &str, mut operation: F) -> (Duration, Duration, usize)
where
    F: FnMut(QueryBackendKind) -> Vec<Triple>,
{
    let started = Instant::now();
    let mut rusqlite_result = operation(QueryBackendKind::Rusqlite);
    let rusqlite_elapsed = started.elapsed();

    let started = Instant::now();
    let mut turso_result = operation(QueryBackendKind::TursoLocal);
    let turso_elapsed = started.elapsed();

    let rows = rusqlite_result.len();

    normalize_triples(&mut rusqlite_result);
    normalize_triples(&mut turso_result);

    let rusqlite_tuples = triples_to_tuples(&rusqlite_result);
    let turso_tuples = triples_to_tuples(&turso_result);

    assert_eq!(rusqlite_tuples, turso_tuples, "triple result mismatch for {label}");

    (rusqlite_elapsed, turso_elapsed, rows)
}

#[test]
fn benchmark_fixture_set_is_present() {
    let root = fixtures_root();
    assert!(root.is_dir(), "fixture root directory missing: {}", root.display());

    for triplet in fixture_triplets() {
        for file in [triplet.en, triplet.fr, triplet.de] {
            let path = root.join(file);
            assert!(path.is_file(), "fixture file missing: {}", path.display());

            let metadata = std::fs::metadata(&path)
                .unwrap_or_else(|e| panic!("unable to read fixture metadata for {}: {}", path.display(), e));
            assert!(metadata.len() > 0, "fixture file is empty: {}", path.display());
        }
    }
}

#[test]
#[cfg(feature = "rusqlite-c")]
#[ignore = "benchmark: run manually with --ignored --nocapture"]
#[allow(clippy::print_stdout)]
fn benchmark_performance_rusqlite_large_resx() {
    for triplet in fixture_triplets() {
        let en = fixture_path(triplet.en);
        let fr = fixture_path(triplet.fr);
        let de = fixture_path(triplet.de);

        let started = Instant::now();
        let diff_fr = query::query_diff_with_backend(&en, &fr, QueryBackendKind::Rusqlite).run();
        let diff_fr_elapsed = started.elapsed();

        let started = Instant::now();
        let diff_de = query::query_diff_with_backend(&en, &de, QueryBackendKind::Rusqlite).run();
        let diff_de_elapsed = started.elapsed();

        let started = Instant::now();
        let merge_fr = query::query_merge_with_backend(&en, &fr, QueryBackendKind::Rusqlite).run();
        let merge_fr_elapsed = started.elapsed();

        println!(
            "{} / rusqlite: diff(fr)={} in {:?}, diff(de)={} in {:?}, merge(fr)={} in {:?}",
            triplet.name,
            diff_fr.len(),
            diff_fr_elapsed,
            diff_de.len(),
            diff_de_elapsed,
            merge_fr.len(),
            merge_fr_elapsed
        );
    }
}

#[test]
#[cfg(all(feature = "turso-rust", feature = "rusqlite-c"))]
#[ignore = "comparative benchmark: run manually with --ignored --nocapture"]
#[allow(clippy::print_stdout)]
fn benchmark_correctness_rusqlite_vs_turso_local() {
    let expected_covered_operations: HashSet<&'static str> = [
        "file-print",
        "file-convert",
        "file-sort",
        "file-diff",
        "file-diff-show-changes",
        "file-merge",
        "file-intersect",
        "file-subtract",
        "diff-with-base",
        "pull-query-left-join",
        "push-query-changed-values",
    ]
    .into_iter()
    .collect();

    let mut covered_operations: HashSet<&'static str> = HashSet::new();
    let mut total_rusqlite = Duration::default();
    let mut total_turso = Duration::default();

    let pull_left_join_query = r"
        SELECT
            A.key, A.val
        FROM A
        LEFT OUTER JOIN B on A.key = B.key";

    let push_changed_values_query = r"
        SELECT
            B.key, B.val
        FROM B
        INNER JOIN A on (A.key = B.key) AND (A.val <> B.val)";

    for triplet in fixture_triplets() {
        let en = fixture_path(triplet.en);
        let fr = fixture_path(triplet.fr);
        let de = fixture_path(triplet.de);

        macro_rules! bench_resource_op {
            ($key:expr, $label:expr, $ordered:expr, $run:expr) => {{
                covered_operations.insert($key);
                let (rusqlite_elapsed, turso_elapsed, rows) = benchmark_resource_operation($label, $ordered, $run);
                total_rusqlite += rusqlite_elapsed;
                total_turso += turso_elapsed;
                println!(
                    "{}: rows={} rusqlite={:?} turso={:?}",
                    $label, rows, rusqlite_elapsed, turso_elapsed
                );
            }};
        }

        macro_rules! bench_triple_op {
            ($key:expr, $label:expr, $run:expr) => {{
                covered_operations.insert($key);
                let (rusqlite_elapsed, turso_elapsed, rows) = benchmark_triple_operation($label, $run);
                total_rusqlite += rusqlite_elapsed;
                total_turso += turso_elapsed;
                println!(
                    "{}: rows={} rusqlite={:?} turso={:?}",
                    $label, rows, rusqlite_elapsed, turso_elapsed
                );
            }};
        }

        bench_resource_op!(
            "file-print",
            &format!("{} / file-print(en)", triplet.name),
            false,
            |backend| query::query_print_with_backend(&en, backend).run()
        );

        bench_resource_op!(
            "file-convert",
            &format!("{} / file-convert(en)", triplet.name),
            false,
            |backend| query::query_convert_with_backend(&en, backend).run()
        );

        bench_resource_op!(
            "file-sort",
            &format!("{} / file-sort(en)", triplet.name),
            true,
            |backend| query::query_sort_with_backend(&en, backend).run()
        );

        bench_resource_op!(
            "file-diff",
            &format!("{} / file-diff(en,fr)", triplet.name),
            false,
            |backend| query::query_diff_with_backend(&en, &fr, backend).run()
        );

        bench_resource_op!(
            "file-diff",
            &format!("{} / file-diff(en,de)", triplet.name),
            false,
            |backend| query::query_diff_with_backend(&en, &de, backend).run()
        );

        bench_resource_op!(
            "file-diff-show-changes",
            &format!("{} / file-diff --show-changes(en,fr)", triplet.name),
            false,
            |backend| query::query_change_with_backend(&en, &fr, backend).run()
        );

        bench_resource_op!(
            "file-merge",
            &format!("{} / file-merge(en,fr)", triplet.name),
            false,
            |backend| query::query_merge_with_backend(&en, &fr, backend).run()
        );

        bench_resource_op!(
            "file-intersect",
            &format!("{} / file-intersect(en,fr)", triplet.name),
            false,
            |backend| query::query_intersect_with_backend(&en, &fr, backend).run()
        );

        bench_resource_op!(
            "file-subtract",
            &format!("{} / file-subtract(en,fr)", triplet.name),
            false,
            |backend| query::query_subtract_with_backend(&en, &fr, backend).run()
        );

        bench_triple_op!(
            "diff-with-base",
            &format!("{} / diff-with-base(en,fr,de)", triplet.name),
            |backend| query::query_diff_with_base_with_backend(&en, &fr, &de, backend).run_triple()
        );

        bench_resource_op!(
            "pull-query-left-join",
            &format!("{} / pull-query-left-join(en,fr)", triplet.name),
            false,
            |backend| {
                query::CirupQuery::new_with_backend(pull_left_join_query, &en, Some(&fr), None, backend).run()
            }
        );

        bench_resource_op!(
            "push-query-changed-values",
            &format!("{} / push-query-changed-values(en,fr)", triplet.name),
            false,
            |backend| {
                query::CirupQuery::new_with_backend(push_changed_values_query, &en, Some(&fr), None, backend).run()
            }
        );
    }

    assert_eq!(
        covered_operations, expected_covered_operations,
        "benchmark operation coverage mismatch"
    );

    let ratio = total_turso.as_secs_f64() / total_rusqlite.as_secs_f64();

    println!(
        "TOTAL comparative benchmark: rusqlite={:?}, turso-local={:?}, ratio(turso/rusqlite)={:.3}",
        total_rusqlite, total_turso, ratio
    );

    println!("analyzed non-query CLI operations (not query-backend comparable): vcs-log, vcs-diff");
}
