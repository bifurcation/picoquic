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

## `picoquic/bytestream.c:bytestream_ptr`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C returns data plus current pointer; Rust returns total capacity length instead of a tail pointer/slice.
* C source: `picoquic/bytestream.c:78-81`
* C signature: `const uint8_t * bytestream_ptr(bytestream *)`
* Rust source: `rs/fq/src/bytestream.rs:185-193`
* Rust item: `tail`

### C body
```c
{
    return s->data + s->ptr;
}
```

### Rust body
```rust
    pub fn capacity(&self) -> usize {
        self.data_ref().len()
    }
```

## `picoquic/c4.c:c4_notify`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: C handles many notification cases and updates congestion state; Rust body only returns.
* C source: `picoquic/c4.c:1019-1109`
* C signature: `void c4_notify(picoquic_cnx_t *, picoquic_path_t *, picoquic_congestion_notification_t, picoquic_per_ack_state_t *, uint64_t)`
* Rust source: `rs/fq/src/c4.rs:880-894`
* Rust item: `notify`

### C body
```c
{
    c4_state_t* c4_state = (c4_state_t*)path_x->congestion_alg_state;
    path_x->is_cc_data_updated = 1;

    if (ack_state != NULL && ack_state->pc != picoquic_packet_context_application) {
        return;
    }

    if (c4_state != NULL) {
        switch (notification) {
        case picoquic_congestion_notification_acknowledgement:
            c4_handle_ack(path_x, c4_state, ack_state);
            c4_apply_rate_and_cwin(path_x, c4_state);
            break;
        case picoquic_congestion_notification_ecn_ec:
            /* TODO: ECN is special? Implement the prague logic */
            c4_state->ecn_threshold = c4_ecn_threshold(c4_state);
            c4_update_ecn_alpha(path_x, c4_state);
            if (c4_state->ecn_alpha > c4_state->ecn_threshold) {
                if (c4_state->alg_state == c4_initial) {
                    if (c4_state->recent_delay_excess > 0
                        && c4_state->nb_eras_no_increase > 1
                        && c4_state->push_rate_old >= c4_state->nominal_rate) {

                        c4_exit_initial(path_x, c4_state);
                    }
                }
                else {
                    c4_notify_congestion(path_x, c4_state, c4_congestion_ecn);
                }
            }
            break;
        case picoquic_congestion_notification_repeat:
            if (c4_state->alg_state == c4_recovery && ack_state->lost_packet_number < c4_state->era_sequence) {
                /* Do not worry about loss of packets sent before entering recovery */
                break;
            }
            c4_update_loss_rate(c4_state, ack_state->lost_packet_number);

            if (c4_state->smoothed_drop_rate > c4_loss_threshold(c4_state)) {
                if (c4_state->alg_state == c4_initial) {
                    c4_initial_handle_loss(path_x, c4_state);
                }
                else {
                    c4_notify_congestion(path_x, c4_state, c4_congestion_loss);
                }
            }
            break;
        case picoquic_congestion_notification_timeout:
            /* Treat timeout as PTO: no impact on congestion control */
            break;
        case picoquic_congestion_notification_spurious_repeat:
            /* Remove handling of spurious repeat, as it was tied to timeout */
            break;
        case picoquic_congestion_notification_rtt_measurement:
            c4_update_rtt(c4_state, ack_state->rtt_measurement);
            if (c4_state->alg_state == c4_initial) {
                c4_initial_handle_rtt_excess(path_x, c4_state);
                c4_apply_rate_and_cwin(path_x, c4_state);
            }
            else {
                c4_handle_rtt_excess(path_x, c4_state);
            }
            break;
        case picoquic_congestion_notification_lost_feedback:
            break;
        case picoquic_congestion_notification_cwin_blocked:
            break;
        case picoquic_congestion_notification_reset:
            c4_reset(c4_state, path_x, c4_state->option_string);
            break;
        case picoquic_congestion_notification_seed_cwin:
            c4_seed_cwin(c4_state, ack_state->nb_bytes_acknowledged);
            break;
        default:
            /* ignore */
            break;
        }
    }
}
```

### Rust body
```rust
        {
            return;
        }
```

## `picoquic/config.c:config_set_port`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body is a test function with cases and assertions, not the implementation body that parses and writes config fields.
* C source: `picoquic/config.c:226-272`
* C signature: `int config_set_port(picoquic_quic_config_t *, const char *)`
* Rust source: `rs/fq/src/tests/config.rs:704-756`
* Rust item: `config_set_port`

### C body
```c
{
    int ret = 0;
    char opval_buffer[256];
    char const* p = port_string;
    int is_port_shared = 0;
    int p1 = 0;
    int p2 = 0;
    int nb_threads = 0;

    if (*p == 'S') {
        is_port_shared = 1;
        p++;
    }
    while (*p >= '0' && *p <= '9') {
        p1 *= 10;
        p1 += (*p - '0');
        p++;
    }
    if (*p == ':') {
        p++;
        while (*p >= '0' && *p <= '9') {
            p2 *= 10;
            p2 += (*p - '0');
            p++;
        }
    }
    if (*p == '*') {
        p++;
        while (*p >= '0' && *p <= '9') {
            nb_threads *= 10;
            nb_threads += (*p - '0');
            p++;
        }
    }
    if (*p != 0 || p1 < 0 || p1 > 65535 || p2 < 0 || p2 > 65535) {
        fprintf(stderr, "Invalid port: %s\n", config_optval_string(opval_buffer, 256, port_string, strlen(port_string)));
        ret = -1;
    }
    else {
        config->server_port = (uint16_t)p1;
        config->local_port = (uint16_t)p2;
        config->is_port_shared = is_port_shared;
        config->nb_threads = nb_threads;
    }
    return ret;
}
```

### Rust body
```rust
fn config_set_port() {
    // (port_string, is_valid, server_port, local_port, is_port_shared)
    let cases: &[(&str, bool, u16, u16, bool)] = &[
        ("4433", true, 4433, 0, false),
        ("443:4434", true, 443, 4434, false),
        ("S4433", true, 4433, 0, true),
        ("S443:4434", true, 443, 4434, true),
        ("S443:4434*", true, 443, 4434, true),
        ("S443:4434*1", true, 443, 4434, true),
        ("S443:4434*256", true, 443, 4434, true),
        ("4433*7", true, 4433, 0, false),
        ("", true, 0, 0, false),
        ("0", true, 0, 0, false),
        ("*5", true, 0, 0, false),
        ("0*3", true, 0, 0, false),
        ("65535", true, 65535, 0, false),
        ("S65535", true, 65535, 0, true),
        ("S65534:65535", true, 65534, 65535, true),
        ("65536", false, 0, 0, false),
        ("-1", false, 0, 0, false),
        ("abc", false, 0, 0, false),
        ("4433:abc", false, 0, 0, false),
        ("abc:4433", false, 0, 0, false),
        ("S4433:abc", false, 0, 0, false),
        ("Sabc:4433", false, 0, 0, false),
    ];

    for (i, &(port_string, is_valid, exp_server_port, exp_local_port, exp_is_port_shared)) in
        cases.iter().enumerate()
    {
        let mut config = Config::default();
        let result = config.set_port(port_string);
        let got_valid = result.is_ok();
        assert_eq!(
            got_valid, is_valid,
            "Case {i} ({port_string:?}): validity mismatch"
        );
        if got_valid {
            assert_eq!(
                config.server_port, exp_server_port,
                "Case {i} ({port_string:?}): server_port"
            );
            assert_eq!(
                config.local_port, exp_local_port,
                "Case {i} ({port_string:?}): local_port"
            );
            assert_eq!(
                config.is_port_shared, exp_is_port_shared,
                "Case {i} ({port_string:?}): is_port_shared"
            );
        }
    }
}
```

## `picoquic/dualq_aqm.c:dualq_params_init`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body only shows the limit calculation fragment and omits the many parameter initializations visible in C.
* C source: `picoquic/dualq_aqm.c:396-452`
* C signature: `void dualq_params_init(dualq_state_t *, uint64_t, picoquictest_sim_link_t *)`
* Rust source: `rs/fq/src/tests/dualq.rs:396-403`
* Rust item: `params_init`

### C body
```c
{
    /* Set input parameter defaults */
    /* DualQ Coupled framework parameters */
    if (link->queue_delay_max <= link->microsec_latency || link->picosec_per_byte == 0) {
        dualq->limit = PICOQUIC_BYTES_FROM_RATE(250ull, DUALQ_MAX_LINK_RATE); /* Dual buffer size */
    }
    else {
        uint64_t queue_delay = link->queue_delay_max - link->microsec_latency;
        dualq->limit = (queue_delay * 1000000) / link->picosec_per_byte;
    }
    dualq->k = 2.0; /* Coupling factor */
    /* NOT SHOWN % scheduler - dependent weight or equival't parameter */
    /* PI2 Classic AQM parameters */
    dualq->target = 15000; /* Queue delay target for Classic queue, microseconds */
    uint64_t RTT_max = 100000;  /* Worst case RTT expected, microseconds */
    /* PI2 constants derived from above PI2 parameters */
    dualq->p_Cmax = 1.0 / (dualq->k * dualq->k);
    if (dualq->p_Cmax > 1.0) {
        dualq->p_Cmax = 1;
    }
    /* PI sampling interval */
    dualq->Tupdate = RTT_max / 3;
    if (dualq->Tupdate > dualq->target) {
        dualq->Tupdate = dualq->target;
    }
    /* PI coefficients */
    /* The spec says Hz, we measure in 1/us because times are in microseconds */
    dualq->pi2_alpha = (0.1 * (double)dualq->Tupdate) / ((double)RTT_max * (double)RTT_max);
    dualq->pi2_beta = (0.3) / RTT_max; /* PI proportional gain in Hz */
    /* L4S ramp AQM parameters */
    dualq->minTh = 800; /* L4S min marking threshold in micros seconds */
    if (l4s_max == 0) {
        dualq->maxTh = 1200;
        dualq->minTh = 800;
    }
    else {
        dualq->maxTh = l4s_max;
        if (l4s_max > 1200) {
            dualq->minTh = 800;
        }
        else {
            dualq->minTh = l4s_max / 3;
        }
    }
    dualq->range = dualq->maxTh - dualq->minTh; /* Range of L4S ramp in time units */

    /* 19 : Th_len = 1 pkt % Min L4S marking threshold in packets */
    /* L4S constants */
    dualq->p_Lmax = 1.0; /* Max L4S marking prob */
}
```

### Rust body
```rust
        } else {
            let queue_delay = link.queue_delay_max - link.microsec_latency;
            (queue_delay * 1_000_000) / link.picosec_per_byte
        };
```

## `picoquic/ech.c:picoquic_ech_get_kem_from_curve`
* Phase 4C status: `definitely_not_ok`
* Phase 4C rationale: Rust body is a PublicKeyAsn1 struct definition, not code that searches KEMs by curve/group id and returns success or failure.
* C source: `picoquic/ech.c:655-670`
* C signature: `int picoquic_ech_get_kem_from_curve(ptls_hpke_kem_t **, uint16_t)`
* Rust source: `rs/fq/src/ech.rs:424-462`
* Rust item: `ech_get_kem_from_curve`

### C body
```c
{
    int ret = -1;

    for (int i = 0; i < PICOQUIC_HPKE_KEM_NB_MAX; i++) {
        if (picoquic_hpke_kems[i] == NULL) {
            break;
        }
        else if (picoquic_hpke_kems[i]->keyex->id == group_id) {
            *kem = picoquic_hpke_kems[i];
            ret = 0;
            break;
        }
    }
    return ret;
}
```

### Rust body
```rust
pub struct PublicKeyAsn1<'a> {
    /// Algorithm OID value bytes (tag + length stripped).
    ///
    /// C: `public_key_algo.{base,len}`
    pub algo: &'a [u8],
    /// Parameters field: raw bytes from after the algorithm OID to the
    /// end of the inner AlgorithmIdentifier SEQUENCE.  Includes the
    /// tag + length of any nested TLV; empty when there are no parameters
    /// (e.g. X25519 keys).
    ///
    /// C: `public_key_param.{base,len}`
    pub param: &'a [u8],
    /// BIT STRING value bytes.  The first byte is the padding-count
    /// (always `0x00` for the curves picoquic supports); the key bytes
    /// follow immediately.
    ///
    /// C: `public_key_bit_string.{base,len}`
    pub bit_string: &'a [u8],
    /// Total number of bytes consumed from the input.
    pub consumed: usize,
}
```
