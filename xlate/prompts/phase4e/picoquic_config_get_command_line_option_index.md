# Phase 4E repair confirmed translation mismatches

You are repairing Phase 4D `needs_fix` entries.  Phase 4D
already performed deeper classification and concluded that
these Rust translations need repair.

Rules:

* Edit Rust only.  Do not edit C sources.
* Keep edits limited to the owned Rust file(s) for this batch
  unless a directly related helper in `rs/fq/` must change.
* Preserve safe, idiomatic Rust and existing public API shape
  unless the current shape cannot express the C behavior.
* Do not replace code with stubs, placeholders, fabricated
  defaults, or weaker behavior.
* If deeper repair inspection proves Phase 4D was mistaken,
  report outcome `ok` and do not edit source.
* The driver will run a separate read-only re-triage before
  recording any `fixed` or `ok` result as resolved.
* Report `blocked` only with a concrete human-actionable
  reason.

Owned Rust file(s): `rs/fq/src/config.rs`

Return final JSON with this shape:

```json
{"repairs":[{"c_id":"...","outcome":"fixed|ok|blocked","analysis":"short repair conclusion","fix_summary":"what changed, or empty","files_changed":["rs/fq/src/..."],"verification":["cargo ..."]}]}
```

Entries:

## `picoquic/config.c:picoquic_config_get_command_line_option_index`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C body parses a dash option string and returns an index; Rust body applies an already selected option index and parameters to config.
* Phase 4D analysis: The shown Phase 4C Rust body is the adjacent value-helper due a bad body span, but deeper review still found a real mismatch: Rust long-option parsing uses exact option_entry_by_name, while C calls picoquic_config_get_option_name_index and accepts prefixes via strncmp(..., l).
* Phase 4D fix note: Make command-line long-option parsing use the same prefix lookup semantics as picoquic_config_get_option_name_index, preferably by sharing the helper used by picoquic_config_get_command_line_option_index and Config::command_line_ex.
* C source: `picoquic/config.c:667-681`
* C signature: `int picoquic_config_get_command_line_option_index(const char *)`
* Rust source: `rs/fq/src/config.rs:705-742`
* Rust item: `picoquic_config_get_command_line_option_index`

### C body
```c
{
    int option_index = -1;

    if (opt_string[0] == '-' && opt_string[1] != 0) {
        if (opt_string[2] == 0) {
            option_index = picoquic_config_get_option_char_index(opt_string[1]);
        }
        else if (opt_string[1] == '-' && opt_string[2] != 0) {
            char const* opt_name = opt_string + 2;
            option_index = picoquic_config_get_option_name_index(opt_name, strlen(opt_name));
        }
    }
    return option_index;
}
```

### Rust body
```rust
/// C: `picoquic_get_command_line_option_value` (picoquic/config.c:683)
///
/// Collect the value(s) required by `OPTION_TABLE[option_index]`, advancing
/// `p_optind` for extra argv entries exactly like the C helper, then apply the
/// option to `config`.
pub fn picoquic_get_command_line_option_value(
    option_index: i32,
    _opt_string: &str,
    p_optind: &mut usize,
    argv: &[&str],
    argc: usize,
    optarg: Option<&str>,
    config: &mut Config,
) -> Result<(), Error> {
    let option_index = usize::try_from(option_index).map_err(|_| Error::InvalidArgument)?;
    let entry = OPTION_TABLE
        .get(option_index)
        .ok_or(Error::InvalidArgument)?;
    let argc = argc.min(argv.len());
    let mut params = Vec::new();

```
