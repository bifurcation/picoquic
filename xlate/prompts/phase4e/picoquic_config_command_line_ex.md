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

## `picoquic/config.c:picoquic_config_command_line_ex`
* Phase 4C status: `suspect`
* Phase 4C rationale: C prints an unknown-option message and returns ret defaulting to 0; Rust maps parse failure to Error::InvalidArgument and has no visible stderr behavior.
* Phase 4D analysis: C command_line_ex reports an unknown option to stderr but returns 0; Rust currently turns an unrecognized option string into Error::InvalidArgument, changing observable success/failure behavior.
* Phase 4D fix note: Adjust command_line_ex so unrecognized option strings do not return an error, preserving C's return semantics; optionally add std-gated stderr reporting if desired.
* C source: `picoquic/config.c:742-757`
* C signature: `int picoquic_config_command_line_ex(const char *, int *, int, const char **, const char *, picoquic_quic_config_t *)`
* Rust source: `rs/fq/src/config.rs:1297-1307`
* Rust item: `command_line_ex`

### C body
```c
{
    int ret = 0;
    int option_index = -1;

    option_index = picoquic_config_get_command_line_option_index(opt_string);

    if (option_index == -1) {
        fprintf(stderr, "Unknown option: %s\n", opt_string);
    }
    else {
        ret = picoquic_get_command_line_option_value(option_index, opt_string, p_optind,
            argv, argc, optarg, config);
    }
    return ret;
}
```

### Rust body
```rust
    /// Like [`Self::command_line`] but accepts both single-character
    /// (`-x`) and long-form (`--name`) option strings; the leading
    /// dashes are part of `opt_string`, matching the C contract.
    /// C: `picoquic_config_command_line_ex`.
    pub fn command_line_ex(
```
