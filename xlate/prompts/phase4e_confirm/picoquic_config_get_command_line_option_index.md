# Phase 4E repair confirmation

This is a read-only re-triage after a Phase 4E repair or
repair-level `ok` claim.  Do not edit files.

For each entry, inspect directly relevant C and Rust context
and decide whether the current Rust translation is now
acceptable.

Report:

* `ok` when the current Rust behavior is acceptable.
* `needs_fix` when a real mismatch remains.
* `blocked` only when a concrete external decision or missing
  dependency prevents classification.

Return final JSON with this shape:

```json
{"results":[{"c_id":"...","outcome":"ok|needs_fix|blocked","analysis":"short confirmation conclusion","fix_summary":"remaining mismatch if any, or empty","files_changed":[],"verification":["read-only context inspected"]}]}
```

Entries:

## `picoquic/config.c:picoquic_config_get_command_line_option_index`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C body parses a dash option string and returns an index; Rust body applies an already selected option index and parameters to config.
* Prior Phase 4D analysis: The shown Phase 4C Rust body is the adjacent value-helper due a bad body span, but deeper review still found a real mismatch: Rust long-option parsing uses exact option_entry_by_name, while C calls picoquic_config_get_option_name_index and accepts prefixes via strncmp(..., l).
* Phase 4E claimed outcome: `fixed`
* Phase 4E repair analysis: Rust command-line parsing now matches C long-option prefix lookup semantics: abbreviated names such as --long resolve through the same table-order prefix helper used by command_line_ex, while overlong names remain unknown.
* Phase 4E fix summary: Added focused unit coverage for C-style abbreviated long-option lookup through picoquic_config_get_command_line_option_index and Config::command_line_ex.
* C source: `picoquic/config.c:667-681`
* C signature: `int picoquic_config_get_command_line_option_index(const char *)`
* Current Rust source: `rs/fq/src/config.rs:718-755`
* Current Rust item: `picoquic_config_get_command_line_option_index`

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

### Current Rust body
```rust
) -> Result<(), Error> {
    let option_index = usize::try_from(option_index).map_err(|_| Error::InvalidArgument)?;
    let entry = OPTION_TABLE
        .get(option_index)
        .ok_or(Error::InvalidArgument)?;
    let argc = argc.min(argv.len());
    let mut params = Vec::new();

    if entry.nb_params > 0 {
        params.push(optarg.ok_or(Error::InvalidArgument)?);
        while params.len() < entry.nb_params {
            if *p_optind >= argc {
                return Err(Error::InvalidArgument);
            }
            params.push(argv[*p_optind]);
            *p_optind += 1;
        }
    }

    apply_option(config, entry, &params)
}
```
