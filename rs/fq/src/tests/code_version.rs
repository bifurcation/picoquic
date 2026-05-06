//! Test cases for `picoquictest/code_version_test.c`.

/// C: `code_version_test` in `picoquictest/code_version_test.c`.
///
/// Cross-checks [`crate::VERSION`] against the version declared in
/// `CMakeLists.txt`'s `project(picoquic VERSION …)` block, mirroring
/// what the C test does against `PICOQUIC_VERSION`.
#[test]
fn code_version() {
    use std::io::BufRead;

    let cmake_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../CMakeLists.txt");
    let file = std::fs::File::open(cmake_path)
        .unwrap_or_else(|e| panic!("cannot open CMakeLists.txt: {e}"));

    let mut lines = std::io::BufReader::new(file).lines();
    let mut found = false;

    while let Some(Ok(line)) = lines.next() {
        if line.trim_start().starts_with("project(picoquic") {
            // Next line: <whitespace>VERSION<whitespace>x.y.z.t
            if let Some(Ok(next)) = lines.next() {
                let trimmed = next.trim_start();
                if let Some(after_version_kw) = trimmed.strip_prefix("VERSION") {
                    let cmake_ver = after_version_kw.trim_start();
                    let rest = cmake_ver.strip_prefix(crate::VERSION).unwrap_or_else(|| {
                        panic!(
                            "CMakeLists.txt VERSION {cmake_ver:?} does not match crate::VERSION {:?}",
                            crate::VERSION
                        )
                    });
                    assert!(
                        rest.is_empty() || rest.starts_with(char::is_whitespace),
                        "CMakeLists.txt VERSION {cmake_ver:?} has unexpected suffix after {:?}",
                        crate::VERSION
                    );
                    found = true;
                }
            }
            break;
        }
    }

    assert!(
        found,
        "could not find `project(picoquic` VERSION line in CMakeLists.txt"
    );
}
