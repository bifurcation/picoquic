# Phase 4C body-only translation audit

Compare each C/Rust pair using only the function bodies shown
below. Do not infer from dependencies, type definitions, callers,
module context, tests, or external knowledge. This is a cheap
superficial check for obvious inconsistencies.

Return only JSON with this shape:

```json
{"reviews":[{"c_id":"...","status":"ok|suspect|definitely_not_ok","rationale":"body-visible reason"}]}
```

Status meanings:
* `ok`: no obvious body-level concern.
* `suspect`: possible mismatch visible from the bodies.
* `definitely_not_ok`: clear mismatch or placeholder-like code.

## Pair `picoquic/quicctx.c:picoquic_find_path_by_unique_id`
C: `picoquic/quicctx.c:2203-2215 picoquic_find_path_by_unique_id`
Rust: `rs/fq/src/internal.rs:4833-4838 find_path_by_unique_id`

### C body
```c
{
    int path_index = -1;
    
    for (int i = 0; i < cnx->nb_paths; i++) {
        if (cnx->path[i]->unique_path_id == unique_path_id) {
            path_index = i;
            break;
        }
    }

    return path_index;
}
```

### Rust body
```rust
        for (i, p) in self.paths.iter().enumerate() {
            if p.unique_path_id == unique_path_id {
                return i as i32;
            }
        }
```

## Pair `picoquic/quicctx.c:picoquic_issue_path_quality_update`
C: `picoquic/quicctx.c:2660-2676 picoquic_issue_path_quality_update`
Rust: `rs/fq/src/internal.rs:6249-6252 issue_path_quality_update`

### C body
```c
{
    int ret = 0;

    if ((path_x->rtt_update_delta > 0 && (
        path_x->smoothed_rtt < path_x->rtt_threshold_low || 
        path_x->smoothed_rtt > path_x->rtt_threshold_high)) ||
        (path_x->pacing_rate_update_delta > 0 && (
            path_x->pacing.rate < path_x->pacing_rate_threshold_low ||
            path_x->pacing.rate > path_x->pacing_rate_threshold_high ||
            path_x->receive_rate_estimate < path_x->receive_rate_threshold_low ||
            path_x->receive_rate_estimate > path_x->receive_rate_threshold_high))) {
        picoquic_refresh_path_quality_thresholds(path_x);
        ret = cnx->callback_fn(cnx, path_x->unique_path_id, NULL, 0, picoquic_callback_path_quality_changed, cnx->callback_ctx, NULL);
    }
    return ret;
}
```

### Rust body
```rust
        let rtt = if path_x.smoothed_rtt.ticks() > 0 {
            path_x.smoothed_rtt
        } else {
```
