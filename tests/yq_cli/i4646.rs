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
