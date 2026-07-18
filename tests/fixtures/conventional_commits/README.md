# Conventional-commit classification fixtures

Each file is a raw commit message. `breaking/` messages must classify as a
breaking change (major bump); `not_breaking/` messages must not. The test
`fixtures_classify_by_directory` in `src/conventional_commits.rs` walks both
directories and asserts `is_breaking` matches the directory.

Add a variant by dropping a `.txt` file in the right directory — no test code
change needed.
