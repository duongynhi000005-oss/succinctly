//! #4646: `--eval-all` evaluates each document through a reindexed copy, so `line` and `column`
//! of a node read from YAML answered the position in that regenerated text (`[1,6]`, `[1,14]`)
//! where yq answers the position in the file. The rows are live yq v4.53.3 captures of
//! `yq ea -o=json -I=0 <filter> <files>`.

use anyhow::Result;
use tempfile::TempDir;

use crate::run_yq_files;

const ARGV: &[&str] = &["--eval-all", "-o", "json", "-I", "0"];

/// A file's name and its text.
type Source = (&'static str, &'static str);

fn run(filter: &str, files: &[Source]) -> Result<String> {
    let dir = TempDir::new()?;
    let mut paths = Vec::new();
    for (name, text) in files {
        let path = dir.path().join(name);
        std::fs::write(&path, text)?;
        paths.push(path);
    }
    let refs: Vec<&std::path::Path> = paths.iter().map(std::path::PathBuf::as_path).collect();
    let (stdout, stderr, code) = run_yq_files(filter, &refs, ARGV)?;
    assert_eq!(code, 0, "{filter}: {stderr}");
    Ok(stdout)
}

const SEQ: &str = "a:\n  - 1\n  - b: 2\n";
const MERGED: &str = "x: &k\n  p: 1\n  q: [3, 4]\ny:\n  <<: *k\n  r: 5\n---\nz: 1\n";
const KEYS: &str = "k: 1\nl:\n  m: [1, 2]\n";
const MERGE_ONE: &str = "x: &k\n  p: 1\n  q: [3, 4]\ny:\n  <<: *k\n  r: 5\n";
const BARE: &str =
    "a:\n  - # c\n    k: v\n  -\n    z: 1\n  - plain\n  - [1, 2]\n  - - n\n    - m\n";
const ODD_KEYS: &str =
    "1: a\ntrue: b\nnull: c\n? [1, 2]\n: d\n2.5: e\n~: g\nx:\n  1: h\n  y: [1, {z: 1}]\n";
const DUPLICATE: &str = "a: 1\nb: 2\na: 3\nc:\n  - x\n  - y\n  - z\n";

/// (filter, files, expected stdout).
const ROWS: &[(&str, &[Source], &str)] = &[
    (".a | [line,column]", &[("t.yaml", SEQ)], "[2,3]\n"),
    (".a[1].b | [line,column]", &[("t.yaml", SEQ)], "[3,8]\n"),
    (".a[] | [line,column]", &[("t.yaml", SEQ)], "[2,5]\n[3,5]\n"),
    (
        "[.. | [line,column]]",
        &[("t.yaml", SEQ)],
        "[[1,1],[2,3],[2,5],[3,5],[3,8]]\n",
    ),
    ("[line,column]", &[("t.yaml", SEQ)], "[1,1]\n"),
    (".a[1].b | line", &[("t.yaml", SEQ)], "3\n"),
    (".a[1].b | column", &[("t.yaml", SEQ)], "8\n"),
    // A value read through a merge key reports the anchor's position, the other document of the
    // stream its own lines.
    (".y.p | [line,column]", &[("m.yaml", MERGED)], "[2,6]\n[0,0]\n"),
    (".y.r | [line,column]", &[("m.yaml", MERGED)], "[6,6]\n[0,0]\n"),
    (".z | [line,column]", &[("m.yaml", MERGED)], "[0,0]\n[8,4]\n"),
    (
        "[.[] | [line,column]]",
        &[("m.yaml", MERGED)],
        "[[1,4],[5,3],[8,4]]\n",
    ),
    // The file and document an entry came from are what pick the source.
    (
        "[.. | [file_index, document_index, line, column]]",
        &[("t.yaml", SEQ), ("f.yaml", KEYS)],
        "[[0,0,1,1],[0,0,2,3],[0,0,2,5],[0,0,3,5],[0,0,3,8],[1,0,1,1],[1,0,1,4],[1,0,3,3],[1,0,3,6],[1,0,3,7],[1,0,3,10]]\n",
    ),
    (
        ".l.m[] | [file_index, line, column]",
        &[("t.yaml", SEQ), ("f.yaml", KEYS)],
        "[1,3,7]\n[1,3,10]\n",
    ),
    (
        "[line,column]",
        &[("f.yaml", KEYS), ("t.yaml", SEQ)],
        "[1,1,1,1]\n",
    ),
    // The same mapping and sequence read again and out of order: the second lookup in a mapping
    // is indexed (the last of equal keys wins), one with a merge key keeps the walk, and a
    // sequence resumes after the element read last only when this is the next one.
    (
        "[(.y.p|line), (.y.r|line), (.y.p|line), (.y.q[1]|line), (.y.q[0]|line)]",
        &[("m.yaml", MERGE_ONE)],
        "[2,6,2,3,3]\n",
    ),
    (
        "[(.a|line), (.b|line), (.a|line), (.c[2]|line), (.c[1]|line), (.c[2]|line)]",
        &[("d.yaml", DUPLICATE)],
        "[3,2,3,7,6,7]\n",
    ),
    ("[.c[] | line]", &[("d.yaml", DUPLICATE)], "[5,6,7]\n"),
    // A sequence read out of order is listed once; a bare `-` item answers the position of its
    // value, and a key that is a number, a boolean, null or a collection is still found.
    (
        "[(.a[3]|line), (.a[0]|line), (.a[2]|line), (.a[4]|line), (.a[1]|line), (.a[5][1]|line)]",
        &[("b.yaml", BARE)],
        "[7,3,6,8,5,0]\n",
    ),
    (
        "[.a[] | [line,column]]",
        &[("b.yaml", BARE)],
        "[[3,5],[5,5],[6,5],[7,5],[8,5]]\n",
    ),
    (
        "[.. | [line,column]]",
        &[("o.yaml", ODD_KEYS)],
        "[[1,1],[1,4],[2,7],[3,7],[5,3],[6,6],[7,4],[9,3],[9,6],[10,6],[10,7],[10,10],[10,14]]\n",
    ),
    // A mapping key has a position of its own.
    (".l | key | [line,column]", &[("f.yaml", KEYS)], "[2,1]\n"),
    (".l | keys | .[] | [line,column]", &[("f.yaml", KEYS)], "[3,3]\n"),
    (
        "[.. | key | [line,column]]",
        &[("f.yaml", KEYS)],
        "[[1,1],[2,1],[3,3],[0,0],[0,0]]\n",
    ),
];

#[test]
fn test_eval_all_line_and_column_of_yaml_input_are_the_source_position_4646() -> Result<()> {
    for (filter, files, expected) in ROWS {
        assert_eq!(run(filter, files)?, *expected, "{filter}");
    }
    Ok(())
}

/// `--doc` keeps only one document of a stream, and the position is still the one in the file.
#[test]
fn test_eval_all_doc_flag_keeps_the_source_position_4646() -> Result<()> {
    let dir = TempDir::new()?;
    let path = dir.path().join("m.yaml");
    std::fs::write(&path, MERGED)?;
    let (stdout, stderr, code) = run_yq_files(
        ".z | [line,column]",
        &[path.as_path()],
        &["--eval-all", "--doc", "1", "-o", "json", "-I", "0"],
    )?;
    assert_eq!((stdout.as_str(), code), ("[8,4]\n", 0), "{stderr}");
    Ok(())
}

/// A filter that reads no position never installs the resolver, and a filter that reads a
/// position of a document read from JSON still answers `0` (#4544).
#[test]
fn test_eval_all_position_of_json_input_stays_zero_4646() -> Result<()> {
    let out = run(
        ".a | [line,column]",
        &[("t.json", "{\"a\":[1,{\"b\":2}]}\n")],
    )?;
    assert_eq!(out, "[0,0]\n");
    Ok(())
}

/// Reading the position of every element of a wide sequence and every member of a wide mapping is
/// one step per node, not a walk of the container per node. At 4000 entries the walk took 8 s on a
/// debug build, the steps 0.06 s; the bound leaves two orders of magnitude of slack.
#[test]
fn test_eval_all_position_of_every_node_of_a_wide_container_is_not_quadratic_4646() -> Result<()> {
    let n: usize = 4000;
    let dir = TempDir::new()?;
    let sequence: String = (1..=n).map(|i| format!("- k{i}: {i}\n")).collect();
    let mapping: String = (1..=n).map(|i| format!("k{i}: {i}\n")).collect();
    for (name, text) in [("s.yaml", sequence), ("m.yaml", mapping)] {
        let path = dir.path().join(name);
        std::fs::write(&path, text)?;
        let started = std::time::Instant::now();
        let (stdout, stderr, code) = run_yq_files("[.[] | line] | .[-1]", &[path.as_path()], ARGV)?;
        assert_eq!(
            (stdout.trim(), code),
            (n.to_string().as_str(), 0),
            "{name}: {stderr}"
        );
        assert!(
            started.elapsed() < std::time::Duration::from_secs(4),
            "{name} took {:?}",
            started.elapsed()
        );
    }
    Ok(())
}
