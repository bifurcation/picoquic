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

## `picoquic/ech.c:ech_init_opener_callback`
* Phase 4C status: `suspect`
* Phase 4C rationale: C allocates and initializes a callback object with callback/config/kem/keyex fields and cleanup on error; Rust returns an EchOpenerState with config, kem_id, and private_key, with no visible callback setup.
* C source: `picoquic/ech.c:280-331`
* C signature: `int ech_init_opener_callback(ech_opener_callback_t **, const char *, const char *)`
* Rust source: `rs/fq/src/ech.rs:880-923`
* Rust item: `ech_init_opener`

### C body
```c
{
    int ret = 0;
    /* Allocate an opener callback */
    ech_opener_callback_t* ech_cb = (ech_opener_callback_t*)malloc(sizeof(ech_opener_callback_t));
    if (ech_cb == NULL) {
        DBG_PRINTF("Cannot allocate callback memory (%zu bytes)", sizeof(ech_opener_callback_t));
        ret = PICOQUIC_ERROR_MEMORY;
    }
    else {
        memset(ech_cb, 0, sizeof(ech_opener_callback_t));
        /* set the callback */
        ech_cb->super.cb = ech_opener_callback;
        ptls_buffer_init(&ech_cb->config, "", 0);
        /* Read the config bytes into the ech_cb->config buffer */
        ret = picoquic_ech_read_config(&ech_cb->config, config_file_name);
        if (ret != 0) {
            DBG_PRINTF("Cannot read ech configuration from %s", config_file_name);
        } else {
            uint16_t kem_id;
            /* Get kem-id from config, then get kem from kem_id */
            kem_id = (((uint16_t)ech_cb->config.base[7]) << 8) + ech_cb->config.base[8];
            for (int i = 0; i < 4 && picoquic_hpke_kems[i] !=  NULL; i++) {
                if (picoquic_hpke_kems[i]->id == kem_id) {
                    ech_cb->kem = picoquic_hpke_kems[i];
                    break;
                }
            }
            if (ech_cb->kem == NULL){
                DBG_PRINTF("Cannot find hpke kwm for code 0x%04x", kem_id);
                ret = PICOQUIC_ERROR_UNEXPECTED_ERROR;
            }
            else if (picoquic_keyex_from_key_file_fn == NULL) {
                DBG_PRINTF("%s", "Cannot find picoquic_keyex_from_key_file_fn");
                ret = PICOQUIC_ERROR_UNEXPECTED_ERROR;
            }
            else {
                ret = picoquic_keyex_from_key_file_fn(&ech_cb->keyex, private_key_file);
                if (ret != 0) {
                    DBG_PRINTF("picoquic_keyex_from_key_file_fn fails, ret= %d(0x%x)", ret, ret);
                }
            }
        }
        if (ret != 0) {
            ech_dispose_opener_callback(ech_cb);
            ech_cb = NULL;
        }
    }
    *p_ech_cb = ech_cb;
    return ret;
}
```

### Rust body
```rust
) -> Result<EchOpenerState, Error> {
    // Read and decode the ECHConfigList from the base64-encoded config file.
    let config = ech_read_config_file(config_file_name)?;

    // Extract kem_id from ECHConfigList bytes[7:9]:
    //   bytes[0:2] = ECHConfigList outer length
    //   bytes[2:4] = ECHConfig version (0xFE0D)
    //   bytes[4:6] = ECHConfig contents length
    //   bytes[6]   = config_id
    //   bytes[7:9] = kem_id
    if config.len() < 9 {
        return Err(Error::Generic);
    }
    let kem_id = (config[7] as u16) << 8 | config[8] as u16;

    // Validate the kem_id against the supported set.
    if !matches!(
        kem_id,
        kem_id::P256_SHA256 | kem_id::P384_SHA384 | kem_id::X25519_SHA256
    ) {
        return Err(Error::Generic);
    }

    // Read and parse the PEM private key file.
    let pem_text = std::fs::read_to_string(private_key_file).map_err(|_| Error::Generic)?;
    // Determine the key format from the PEM label.
    let label_start = pem_text.find("-----BEGIN ").ok_or(Error::Generic)? + "-----BEGIN ".len();
    let label_end = pem_text[label_start..]
        .find("-----")
        .ok_or(Error::Generic)?;
    let label = &pem_text[label_start..label_start + label_end];

    let der = pem_base64_decode(&pem_text)?;
    let private_key = extract_private_key_der(&der, label)?;

    Ok(EchOpenerState {
        kem_id,
        private_key,
        config,
    })
}
```

## `picoquic/frames.c:picoquic_dequeue_data_repeat_packet`
* Phase 4C status: `suspect`
* Phase 4C rationale: Rust body shown is truncated after an else-if, so only a partial removal path is visible.
* C source: `picoquic/frames.c:2197-2201`
* C signature: `void picoquic_dequeue_data_repeat_packet(picoquic_cnx_t *, picoquic_packet_t *)`
* Rust source: `rs/fq/src/internal.rs:10463-10466`
* Rust item: `dequeue_data_repeat_packet`

### C body
```c
{
    picosplay_delete_hint(&cnx->queue_data_repeat_tree, &packet->queue_data_repeat_node);
}
```

### Rust body
```rust
        if let Some(st) = packet.queue_data_repeat_membership.take() {
            self.queue_data_repeat_tree.remove(st);
        } else if let Some(st) = self
```

## `picoquic/frames.c:picoquic_format_datagram_frame`
* Phase 4C status: `suspect`
* Phase 4C rationale: On src shorter than length, Rust returns None via src.get(..length)? without setting more_data or returning the original buffer; C has no visible src length check and only uses output-space failure to set more_data and reset bytes.
* C source: `picoquic/frames.c:5288-5305`
* C signature: `uint8_t * picoquic_format_datagram_frame(uint8_t *, uint8_t *, int *, int *, size_t, const uint8_t *)`
* Rust source: `rs/fq/src/internal.rs:13890-13910`
* Rust item: `format_datagram_frame`

### C body
```c
{
    uint8_t* bytes0 = bytes;

    if ((bytes = picoquic_frames_uint8_encode(bytes, bytes_max, picoquic_frame_type_datagram_l)) != NULL &&
        (bytes = picoquic_frames_varint_encode(bytes, bytes_max, length)) != NULL &&
        bytes + length <= bytes_max) {
        memcpy(bytes, src, length);
        bytes += length;
        *is_pure_ack = 0;
    }
    else {
        *more_data = 1;
        bytes = bytes0;
    }

    return bytes;
}
```

### Rust body
```rust
) -> Option<&'a mut [u8]> {
    let payload = src.get(..length)?;
    let mut off = 0;
    if !encode_varint_at(bytes, &mut off, crate::frames::FrameType::DatagramL as u64)
        || !encode_varint_at(bytes, &mut off, length as u64)
        || bytes.len() < off + length
    {
        *more_data = 1;
        return Some(bytes);
    }
    bytes[off..off + length].copy_from_slice(payload);
    off += length;
    *is_pure_ack = 0;
    Some(&mut bytes[off..])
}
```

## `picoquic/frames.c:picoquic_queue_data_repeat_init`
* Phase 4C status: `suspect`
* Phase 4C rationale: C initializes the repeat tree with comparator/create/delete/value callbacks, while Rust only clears an existing tree.
* C source: `picoquic/frames.c:2188-2191`
* C signature: `void picoquic_queue_data_repeat_init(picoquic_cnx_t *)`
* Rust source: `rs/fq/src/internal.rs:10427-10430`
* Rust item: `queue_data_repeat_init`

### C body
```c
void picoquic_queue_data_repeat_init(picoquic_cnx_t* cnx) {
    picosplay_init_tree(&cnx->queue_data_repeat_tree, picoquic_queue_data_repeat_compare,
        picoquic_queue_data_repeat_node_create, picoquic_queue_data_repeat_delete, picoquic_queue_data_repeat_node_value);
} 
```

### Rust body
```rust
    pub fn queue_data_repeat_init(&mut self) {
        // Reset the data-repeat splay tree (already empty on new connection).
        self.queue_data_repeat_tree.clear();
    }
```

## `picoquic/logger.c:picoquic_set_textlog`
* Phase 4C status: `suspect`
* Phase 4C rationale: The C body assigns quic->text_log_fns = &textlog_functions after opening stdout or a file; the Rust body opens/sets f_log and should_close_log but has no visible assignment of text_log_fns.
* C source: `picoquic/logger.c:2414-2442`
* C signature: `int picoquic_set_textlog(picoquic_quic_t *, const char *)`
* Rust source: `rs/fq/src/textlog.rs:46-76`
* Rust item: `set_textlog`

### C body
```c
{
    int ret = 0;
    FILE* F_log;

    picoquic_textlog_close(quic);

    if (textlog_file != NULL) {
        if (strcmp(textlog_file, "-") == 0) {
            quic->F_log = stdout;
            quic->should_close_log = 0;
        }
        else {
            F_log = picoquic_file_open(textlog_file, "w");
            if (F_log == NULL) {
                DBG_PRINTF("Cannot create log file <%s>\n", textlog_file);
                ret = -1;
            }
            else {
                quic->F_log = F_log;
                quic->should_close_log = 1;
            }
        }

        quic->text_log_fns = &textlog_functions;
    }

    return ret;
}
```

### Rust body
```rust
    ) -> Result<(), Error> {
        self.textlog_close();

        let Some(path) = textlog_file else {
            return Ok(());
        };
        let path = path.as_ref();

        if path == Path::new("-") {
            self.f_log = Some(Box::new(std::io::stdout()));
            self.should_close_log = false;
        } else {
            match OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(path)
            {
                Ok(f) => {
                    self.f_log = Some(Box::new(f));
                    self.should_close_log = true;
                }
                Err(_) => return Err(Error::NoSuchFile),
            }
        }

        Ok(())
    }
```
