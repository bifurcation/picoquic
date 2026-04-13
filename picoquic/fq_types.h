/*
 * fq_types.h - C-compatible type definitions for Rust FFI
 *
 * These types mirror the #[repr(C)] structs in the fq Rust crate.
 * They provide a minimal view of picoquic structs needed by Rust functions.
 */

#ifndef FQ_TYPES_H
#define FQ_TYPES_H

#include <stdint.h>

/*
 * CConnectionView - Minimal view of picoquic_cnx_t fields
 *
 * This struct is used to pass connection state to/from Rust functions.
 * Only fields needed by the Rust implementation are included.
 */
typedef struct fq_connection_view_t {
    int is_multipath_enabled;
    int cwin_blocked;
    uint64_t pkt_ctx_app_send_sequence;
    uint64_t pkt_ctx_app_highest_acknowledged;
    uint64_t pkt_ctx_app_latest_time_acknowledged;
    uint64_t path0_rtt_min;
    uint64_t maxdata_remote;
    int sent_blocked_frame;
} fq_connection_view_t;

/* Helper macro to initialize fq_connection_view_t from picoquic_cnx_t */
#define FQ_CNX_VIEW_INIT(cnx) { \
    .is_multipath_enabled = (cnx)->is_multipath_enabled, \
    .cwin_blocked = (cnx)->cwin_blocked, \
    .pkt_ctx_app_send_sequence = (cnx)->pkt_ctx[picoquic_packet_context_application].send_sequence, \
    .pkt_ctx_app_highest_acknowledged = (cnx)->pkt_ctx[picoquic_packet_context_application].highest_acknowledged, \
    .pkt_ctx_app_latest_time_acknowledged = (cnx)->pkt_ctx[picoquic_packet_context_application].latest_time_acknowledged, \
    .path0_rtt_min = (cnx)->path[0] ? (cnx)->path[0]->rtt_min : UINT64_MAX, \
    .maxdata_remote = (cnx)->maxdata_remote, \
    .sent_blocked_frame = (cnx)->sent_blocked_frame \
}

/* Helper macro to copy changed fields back to picoquic_cnx_t */
#define FQ_CNX_VIEW_WRITEBACK(view, cnx) do { \
    (cnx)->maxdata_remote = (view).maxdata_remote; \
    (cnx)->sent_blocked_frame = (view).sent_blocked_frame; \
} while(0)

#endif /* FQ_TYPES_H */
