# Phase 4D deep translation classification

You are classifying Phase 4C non-OK C/Rust function-pair
audit entries.  Phase 4C was intentionally body-only and
shallow; Phase 4D classification is allowed to inspect
broader context.

For each entry:

1. Read the C function and any directly relevant C context:
   types, constants/macros, helper callees, and callers when
   needed to understand observable behavior.
2. Read the Rust function in context, including local types,
   helpers, tests, and nearby translated functions.
3. Decide whether the Phase 4C concern is a false positive.

Do not edit files in this classification pass.  Report:

* `ok` when the Rust behavior is acceptable after deeper
  inspection.
* `needs_fix` when the Rust translation is actually wrong and
  should be repaired in a later 4D repair pass.
* `blocked` only when the analysis cannot be completed without
  a concrete external decision or missing dependency.

Return final JSON with this shape:

```json
{"results":[{"c_id":"...","outcome":"ok|needs_fix|blocked","analysis":"short deeper-review conclusion","fix_summary":"empty unless outcome is needs_fix","files_changed":[],"verification":[]}]}
```

Entries:

## `picoquic/quicctx.c:picoquic_insert_output_stream`
* Phase 4C status: `suspect`
* Phase 4C rationale: C visibly maintains first/last/previous/next output-stream ordering by priority; Rust marks the stream and enqueues a token without visible equivalent linked-list insertion logic.
* C source: `picoquic/quicctx.c:3502-3562`
* C signature: `void picoquic_insert_output_stream(picoquic_cnx_t *, picoquic_stream_head_t *)`
* Rust source: `rs/fq/src/internal.rs:9567-9595`
* Rust item: `insert_output_stream`

### C body
```c
{
    if (stream->is_output_stream == 0)  
    {
        if (IS_CLIENT_STREAM_ID(stream->stream_id) == cnx->client_mode) {
            if (stream->stream_id > ((IS_BIDIR_STREAM_ID(stream->stream_id)) ? cnx->max_stream_id_bidir_remote : cnx->max_stream_id_unidir_remote)) {
                return;
            }
        }

        if (cnx->last_output_stream == NULL) {
            /* insert first stream */
            cnx->last_output_stream = stream;
            cnx->first_output_stream = stream;
        }
        else if (picoquic_compare_stream_priority(stream, cnx->last_output_stream) >= 0) {
            /* insert after last stream. Common case for most applications. */
            stream->previous_output_stream = cnx->last_output_stream;
            cnx->last_output_stream->next_output_stream = stream;
            cnx->last_output_stream = stream;
        }
        else {
            picoquic_stream_head_t* current = cnx->first_output_stream;

            while (current != NULL) {
                int cmp = picoquic_compare_stream_priority(stream, current);

                if (cmp < 0) {
                    /* insert before the current stream, then break */
                    stream->previous_output_stream = current->previous_output_stream;
                    if (stream->previous_output_stream == NULL) {
                        cnx->first_output_stream = stream;
                    }
                    else {
                        stream->previous_output_stream->next_output_stream = stream;
                    }
                    current->previous_output_stream = stream;
                    stream->next_output_stream = current;
                    break;
                }
                else if (cmp == 0) {
                    /* Stream is already there. This is unexpected */
                    break;
                }
                else {
                    current = current->next_output_stream;
                }
            }
            if (current == NULL) {
                /* insert after last stream */
                stream->previous_output_stream = cnx->last_output_stream;
                cnx->last_output_stream->next_output_stream = stream;
                cnx->last_output_stream = stream;
            }
        }

        stream->is_output_stream = 1;
    }
}
```

### Rust body
```rust
    pub fn insert_output_stream(&mut self, stream: &mut StreamHead) {
        if !stream.is_output_stream {
            // Check remote flow-control limit.
            use crate::stream::{Role, StreamId};
            let sid = StreamId(stream.stream_id);
            let local_role = if self.client_mode {
                Role::Client
            } else {
                Role::Server
            };
            if sid.is_local(local_role) {
                let max = if sid.is_bidir() {
                    self.max_stream_id_bidir_remote
                } else {
                    self.max_stream_id_unidir_remote
                };
                if stream.stream_id > max {
                    return;
                }
            }
            stream.is_output_stream = true;
            // Find the token for this stream via its tree membership.
            if let Some(splay_tok) = stream.stream_tree_membership
                && let Some(tok) = self.stream_tree.get(splay_tok).copied()
            {
                self.enqueue_output_stream_token(tok);
            }
        }
    }
```
