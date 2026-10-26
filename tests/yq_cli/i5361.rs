//! #5361: the empty string repeated a huge number of times is the empty string in yq mode too: the
//! cap there compares the byte total, which is 0 for it, so the repeat loop ran once per
//! repetition. yq v4.53.3 answers an empty line at once.

use anyhow::Result;

use crate::run_yq_stdin;

#[test]
fn test_empty_string_repeated_a_huge_number_of_times_is_empty_5361() -> Result<()> {
    for filter in [r#""" * 9223372036854775807"#, r#""" * 3"#] {
        let (stdout, code) = run_yq_stdin(filter, "", &["-n"])?;
        assert_eq!((stdout.as_str(), code), ("\n", 0), "{filter}");
    }
    Ok(())
}
