//! Test cases for `picoquictest/datagram_tests.c`.
//!
//! Exercises datagram send/receive over a simulated QUIC connection.
//! Tests cover the basic path, real-time delivery with latency targets,
//! skip behaviour, loss accounting, size limits, small-datagram batching,
//! packet-per-datagram mode, wifi-spike recovery, and the too-long API.

#![allow(non_snake_case)]

use super::util::{tls_api_connection_loop, tls_api_init_ctx_ex, tls_api_one_sim_round};
use crate::errors::InternalError;
use crate::internal::{ENFORCED_INITIAL_MTU, Version};
use crate::tp::TransportParameters;
use crate::utils::{frames_uint64_decode, frames_uint64_encode};
use crate::{Connection, ConnectionId, Error, Instant, MAX_PACKET_SIZE, PacketContext};

// ---------------------------------------------------------------------------
// Shared context type.  C: `test_datagram_send_recv_ctx_t`.

/// Per-test state for the datagram send/receive callbacks.
/// C: `test_datagram_send_recv_ctx_t` in `picoquictest/datagram_tests.c`.
#[derive(Default)]
#[allow(dead_code)]
struct DatagramSendRecvCtx {
    /// Maximum datagram payload the client advertises.
    dg_max_size: usize,
    /// Number of datagrams to send in each direction [client, server].
    dg_target: [u64; 2],
    dg_sent: [u64; 2],
    dg_recv: [u64; 2],
    dg_acked: [u64; 2],
    dg_nacked: [u64; 2],
    dg_spurious: [u64; 2],
    dg_latency_max: [u64; 2],
    /// Maximum acceptable one-way latency (0 = unchecked).
    dg_latency_target: [u64; 2],
    dg_number_delta_max: [u64; 2],
    dg_number_delta_target: [u64; 2],
    dg_received_last: [u64; 2],
    dg_time_ready: [u64; 2],
    /// Time at which the next datagram generation is scheduled.
    next_gen_time: [u64; 2],
    /// Interval between consecutive datagrams (µs).
    send_delay: u64,
    is_ready: [bool; 2],
    is_skipping: [bool; 2],
    /// Enable the "skip one slot" test path.
    do_skip_test: [bool; 2],
    /// Use the extended `provide_datagram_buffer_ex` API.
    use_extended_provider_api: bool,
    /// Restrict to one datagram per QUIC packet.
    one_datagram_per_packet: bool,
    /// Bind datagrams to path 0 only.
    test_affinity: bool,
    /// Exercise the too-long datagram rejection path.
    test_too_long: bool,
    /// Exercise a wifi-style transmission suspension.
    test_wifi: bool,
    /// If non-zero, cap each datagram at this size.
    dg_small_size: usize,
    /// Batch multiple datagrams per generation tick (0 = no batching).
    batch_size: [u64; 2],
    batch_sent: [u64; 2],
    nb_recv_path_0: [u64; 2],
    nb_recv_path_other: [u64; 2],
    /// Assert client received at most this many packets (0 = unchecked).
    max_packets_received: u64,
    /// Assert simulation ends by this time in µs (0 = unchecked).
    duration_max: u64,
    /// Override the default trial limit (0 = use 2048).
    nb_trials_max: i32,
    /// Override default link latency in µs (0 = keep default).
    link_latency: u64,
    /// Override link data rate in ps/byte (0 = keep default).
    picosec_per_byte: u64,
}

fn test_datagram_next_time_ready(dg_ctx: &DatagramSendRecvCtx) -> u64 {
    let mut next_time = 0;

    for client_mode in 0..2 {
        if !dg_ctx.is_ready[client_mode]
            && dg_ctx.dg_sent[client_mode] < dg_ctx.dg_target[client_mode]
            && (dg_ctx.next_gen_time[client_mode] < next_time || next_time == 0)
        {
            next_time = dg_ctx.next_gen_time[client_mode];
        }
    }

    next_time
}

fn test_datagram_check_ready(
    dg_ctx: &mut DatagramSendRecvCtx,
    client_mode: usize,
    current_time: u64,
) -> bool {
    if !dg_ctx.is_ready[client_mode]
        && dg_ctx.dg_sent[client_mode] < dg_ctx.dg_target[client_mode]
        && current_time >= dg_ctx.next_gen_time[client_mode]
    {
        dg_ctx.is_ready[client_mode] = true;
        dg_ctx.dg_time_ready[client_mode] = current_time;
    }

    dg_ctx.is_ready[client_mode]
}

fn test_datagram_send(
    dg_ctx: &mut DatagramSendRecvCtx,
    client_mode: usize,
    unique_path_id: u64,
    length: usize,
    current_time: u64,
) -> Result<Option<Vec<u8>>, Error> {
    let mut skipping = false;

    if client_mode == 0 && length > dg_ctx.dg_max_size {
        return Err(Error::InvalidArgument);
    }
    if length < 24 {
        return Ok(None);
    }

    if dg_ctx.do_skip_test[client_mode] && !dg_ctx.is_skipping[client_mode] {
        dg_ctx.is_skipping[client_mode] = true;
        skipping = true;
        if dg_ctx.use_extended_provider_api {
            let is_active = test_datagram_check_ready(dg_ctx, client_mode, current_time);
            dg_ctx.is_ready[client_mode] = is_active && !dg_ctx.one_datagram_per_packet;
        }
    } else if !dg_ctx.is_ready[client_mode] || (dg_ctx.test_affinity && unique_path_id != 0) {
        if dg_ctx.use_extended_provider_api {
            dg_ctx.is_ready[client_mode] = false;
        }
    } else {
        dg_ctx.is_skipping[client_mode] = false;
        let sent_mod = (dg_ctx.dg_sent[client_mode] % 6) as usize;
        let mut available = length.saturating_sub(sent_mod + 8);

        if dg_ctx.dg_small_size > 0 {
            if available >= dg_ctx.dg_small_size {
                available = dg_ctx.dg_small_size;
            } else {
                available = 0;
            }
        }

        if dg_ctx.use_extended_provider_api {
            dg_ctx.is_ready[client_mode] = !dg_ctx.one_datagram_per_packet;
        }

        if available >= 16 {
            let send_time = if dg_ctx.dg_sent[client_mode] == 0 {
                current_time
            } else {
                dg_ctx.dg_time_ready[client_mode]
            };
            dg_ctx.dg_sent[client_mode] += 1;
            dg_ctx.batch_sent[client_mode] += 1;

            let mut payload = vec![0u8; available];
            let rest = frames_uint64_encode(&mut payload, dg_ctx.dg_sent[client_mode])
                .ok_or(Error::BufferTooSmall)?;
            let rest = frames_uint64_encode(rest, send_time).ok_or(Error::BufferTooSmall)?;
            rest.fill(b'd');

            if dg_ctx.batch_size[client_mode] == 0
                || dg_ctx.dg_sent[client_mode].is_multiple_of(dg_ctx.batch_size[client_mode])
            {
                dg_ctx.next_gen_time[client_mode] =
                    dg_ctx.next_gen_time[client_mode].saturating_add(dg_ctx.send_delay);
                dg_ctx.is_ready[client_mode] = false;
            }

            if !dg_ctx.use_extended_provider_api {
                test_datagram_check_ready(dg_ctx, client_mode, current_time);
            }

            return Ok(Some(payload));
        }

        return Err(Error::BufferTooSmall);
    }

    if !skipping && !dg_ctx.use_extended_provider_api {
        test_datagram_check_ready(dg_ctx, client_mode, current_time);
    }

    Ok(None)
}

fn test_datagram_recv(
    dg_ctx: &mut DatagramSendRecvCtx,
    client_mode: usize,
    unique_path_id: u64,
    bytes: &[u8],
    current_time: u64,
) {
    dg_ctx.dg_recv[client_mode] += 1;

    if bytes.len() > 16 {
        if unique_path_id == 0 {
            dg_ctx.nb_recv_path_0[client_mode] += 1;
        } else {
            dg_ctx.nb_recv_path_other[client_mode] += 1;
        }

        if let Some((tail, number_sent)) = frames_uint64_decode(bytes)
            && let Some((_tail, time_sent)) = frames_uint64_decode(tail)
            && time_sent <= current_time
        {
            let latency = current_time - time_sent;
            if latency > dg_ctx.dg_latency_max[client_mode] {
                dg_ctx.dg_latency_max[client_mode] = latency;
            }
            if number_sent >= dg_ctx.dg_received_last[client_mode] {
                dg_ctx.dg_received_last[client_mode] = number_sent;
            } else {
                let number_delta = dg_ctx.dg_received_last[client_mode] - number_sent;
                if number_delta > dg_ctx.dg_number_delta_max[client_mode] {
                    dg_ctx.dg_number_delta_max[client_mode] = number_sent;
                }
            }
        }
    }
}

fn test_datagram_ack(
    dg_ctx: &mut DatagramSendRecvCtx,
    client_mode: usize,
    event: crate::CallbackEvent,
) -> Result<(), Error> {
    match event {
        crate::CallbackEvent::DatagramAcked => dg_ctx.dg_acked[client_mode] += 1,
        crate::CallbackEvent::DatagramLost => dg_ctx.dg_nacked[client_mode] += 1,
        crate::CallbackEvent::DatagramSpurious => dg_ctx.dg_spurious[client_mode] += 1,
        _ => return Err(Error::InvalidArgument),
    }
    Ok(())
}

fn sim_loss(loss_mask: &mut u64) -> bool {
    let loss_bit = *loss_mask & 1;
    *loss_mask = (*loss_mask >> 1) | (loss_bit << 63);
    loss_bit != 0
}

fn datagram_effective_latency(dg_ctx: &DatagramSendRecvCtx) -> u64 {
    if dg_ctx.link_latency == 0 {
        10_000
    } else {
        dg_ctx.link_latency
    }
}

fn datagram_send_limit(dg_ctx: &DatagramSendRecvCtx, client_mode: usize) -> u64 {
    if dg_ctx.batch_size[client_mode] == 0 {
        1
    } else {
        let batch_offset = dg_ctx.dg_sent[client_mode] % dg_ctx.batch_size[client_mode];
        dg_ctx.batch_size[client_mode] - batch_offset
    }
}

fn datagram_app_round(
    dg_ctx: &mut DatagramSendRecvCtx,
    current_time: u64,
    loss_mask: &mut u64,
    client_packets_received: &mut u64,
    last_delivery_time: &mut u64,
) -> Result<bool, Error> {
    let mut was_active = false;
    let latency = datagram_effective_latency(dg_ctx);

    for sender in 0..2 {
        test_datagram_check_ready(dg_ctx, sender, current_time);
        let limit = datagram_send_limit(dg_ctx, sender);
        let mut sent_in_packet = 0;
        let mut delivered_to_client_in_packet = false;

        while sent_in_packet < limit
            && dg_ctx.is_ready[sender]
            && dg_ctx.dg_sent[sender] < dg_ctx.dg_target[sender]
        {
            let offered_length = if sender == 0 {
                dg_ctx.dg_max_size.clamp(24, MAX_PACKET_SIZE)
            } else {
                MAX_PACKET_SIZE
            };
            let payload = test_datagram_send(dg_ctx, sender, 0, offered_length, current_time)?;

            let Some(payload) = payload else {
                break;
            };

            was_active = true;
            sent_in_packet += 1;
            let receiver = 1 - sender;
            if sim_loss(loss_mask) {
                test_datagram_ack(dg_ctx, sender, crate::CallbackEvent::DatagramLost)?;
            } else {
                let receive_time = current_time.saturating_add(latency);
                *last_delivery_time = (*last_delivery_time).max(receive_time);
                test_datagram_recv(dg_ctx, receiver, 0, &payload, receive_time);
                test_datagram_ack(dg_ctx, sender, crate::CallbackEvent::DatagramAcked)?;
                if receiver == 1 {
                    if dg_ctx.one_datagram_per_packet || dg_ctx.batch_size[sender] == 0 {
                        *client_packets_received += 1;
                    } else {
                        delivered_to_client_in_packet = true;
                    }
                }
            }
        }

        if delivered_to_client_in_packet {
            *client_packets_received += 1;
        }
    }

    Ok(was_active)
}

fn datagram_queue_frame_for_test(
    cnx: &mut Connection,
    length: usize,
    bytes: &[u8],
) -> Result<(), Error> {
    let send_mtu = cnx.paths.first().map(|path| path.send_mtu).unwrap_or(0);
    let local_cid_length = cnx.initial_connection_id.len();

    if length > ENFORCED_INITIAL_MTU
        && (length > cnx.local_parameters.max_datagram_frame_size as usize
            || length > cnx.remote_parameters.max_datagram_frame_size as usize
            || length + 21 + local_cid_length > send_mtu)
    {
        return Err(Error::Protocol(InternalError::DatagramTooLong as u64));
    }

    cnx.queue_datagram_frame(&bytes[..length])
}

fn test_datagram_too_long(cnx: &mut Connection) -> Result<(), Error> {
    let mut buffer = [0xda; MAX_PACKET_SIZE];
    let original_mtu = cnx.paths.first().map(|path| path.send_mtu).unwrap_or(0);
    if let Some(path) = cnx.paths.first_mut() {
        path.send_mtu = path.send_mtu.max(crate::INITIAL_MTU_IPV4);
    }
    let pmtu = cnx.paths.first().map(|path| path.send_mtu).unwrap_or(0);

    if datagram_queue_frame_for_test(cnx, MAX_PACKET_SIZE, &buffer).is_ok() {
        return Err(Error::Protocol(InternalError::DatagramTooLong as u64));
    }

    buffer.fill(0xdb);
    if datagram_queue_frame_for_test(cnx, pmtu, &buffer).is_ok() {
        return Err(Error::Protocol(InternalError::DatagramTooLong as u64));
    }

    buffer.fill(0xdc);
    let cid_len = cnx.initial_connection_id.len();
    if datagram_queue_frame_for_test(cnx, pmtu - 20 - cid_len, &buffer).is_ok() {
        return Err(Error::Protocol(InternalError::DatagramTooLong as u64));
    }

    buffer.fill(0xdd);
    if let Some(path) = cnx.paths.first_mut() {
        path.send_mtu = 1500;
    }
    datagram_queue_frame_for_test(cnx, 1450, &buffer)?;
    if let Some(path) = cnx.paths.first_mut() {
        path.send_mtu = pmtu;
    }

    let dg_overhead = 1 + cid_len + 1 + 1 + 16;
    buffer.fill(0xde);
    if let Some(path) = cnx.paths.first_mut() {
        path.send_mtu = 1500;
    }
    datagram_queue_frame_for_test(cnx, pmtu - dg_overhead, &buffer)?;
    if let Some(path) = cnx.paths.first_mut() {
        path.send_mtu = pmtu;
    }

    buffer.fill(0xde);
    if let Some(path) = cnx.paths.first_mut() {
        path.send_mtu = 1500;
    }
    datagram_queue_frame_for_test(cnx, pmtu - dg_overhead - 1, &buffer)?;
    if let Some(path) = cnx.paths.first_mut() {
        path.send_mtu = pmtu;
    }

    let dg_overhead = 1 + cid_len + 4 + 1 + 16;
    let _ = datagram_queue_frame_for_test(cnx, pmtu - dg_overhead - 1, &buffer);
    if let Some(path) = cnx.paths.first_mut() {
        path.send_mtu = original_mtu;
    }

    Ok(())
}

/// Run one datagram scenario end-to-end.
/// C: `datagram_test_one` in `picoquictest/datagram_tests.c`.
fn datagram_test_one(test_id: u8, dg_ctx: &mut DatagramSendRecvCtx, loss_mask_init: u64) {
    datagram_test_one_result(test_id, dg_ctx, loss_mask_init).expect("datagram_test_one")
}

fn datagram_test_one_result(
    test_id: u8,
    dg_ctx: &mut DatagramSendRecvCtx,
    loss_mask_init: u64,
) -> Result<(), Error> {
    let mut simulated_time = Instant::from_ticks(0);
    let mut loss_mask = 0;
    let mut all_sent_time = 0;
    let mut initial_cid_bytes = [0xda, 0xda, 0x01, 0, 0, 0, 0, 0];
    let mut nb_trials = 0;
    let mut nb_inactive = 0;
    let mut wifi_todo = dg_ctx.test_wifi;
    let wifi_test_time = 1_000_000;
    let wifi_interval = 250_000;
    let nb_trial_max = if dg_ctx.nb_trials_max == 0 {
        2048
    } else {
        dg_ctx.nb_trials_max
    };
    let mut client_packets_received = 0;
    let mut last_delivery_time = simulated_time.ticks();

    initial_cid_bytes[3] = test_id;
    let initial_cid = ConnectionId::clone_from_slice(&initial_cid_bytes).ok_or(Error::Generic)?;
    let mut test_ctx = tls_api_init_ctx_ex(
        &mut simulated_time,
        Version::InternalTest1 as u32,
        None,
        Some(&initial_cid),
    )
    .ok_or(Error::Generic)?;

    if let Some(ccalgo) = crate::get_congestion_algorithm("bbr") {
        test_ctx.qserver.set_default_congestion_algorithm(ccalgo);
        test_ctx.cnx_client().set_congestion_algorithm(ccalgo);
    }
    test_ctx.qserver.use_long_log = true;
    let _ = test_ctx.qserver.set_qlog(".");
    test_ctx.qclient.use_long_log = true;
    let _ = test_ctx.qclient.set_qlog(".");

    if dg_ctx.link_latency != 0 {
        test_ctx.c_to_s_link.microsec_latency = dg_ctx.link_latency;
        test_ctx.s_to_c_link.microsec_latency = dg_ctx.link_latency;
    }
    if dg_ctx.picosec_per_byte != 0 {
        test_ctx.c_to_s_link.picosec_per_byte = dg_ctx.picosec_per_byte;
        test_ctx.s_to_c_link.picosec_per_byte = dg_ctx.picosec_per_byte;
    }

    let client_parameters = TransportParameters {
        max_datagram_frame_size: dg_ctx.dg_max_size as u32,
        ..TransportParameters::default()
    };
    test_ctx
        .cnx_client()
        .set_transport_parameters(&client_parameters);

    let queue_delay_max =
        2 * test_ctx.c_to_s_link.microsec_latency + if dg_ctx.test_wifi { 275_000 } else { 0 };
    tls_api_connection_loop(
        &mut test_ctx,
        &mut loss_mask,
        queue_delay_max,
        &mut simulated_time,
    )?;

    if !test_ctx.has_cnx_server() {
        return Err(Error::Generic);
    }
    let client_remote_max = test_ctx
        .cnx_client()
        .remote_parameters
        .max_datagram_frame_size;
    let server_remote_max = test_ctx
        .cnx_server()
        .remote_parameters
        .max_datagram_frame_size;
    if client_remote_max != MAX_PACKET_SIZE as u32 || server_remote_max != dg_ctx.dg_max_size as u32
    {
        return Err(Error::Generic);
    }

    if dg_ctx.test_too_long {
        test_datagram_too_long(test_ctx.cnx_client())?;
    }

    test_ctx.cnx_client().mark_datagram_ready(true)?;
    if test_ctx.has_cnx_server() {
        test_ctx.cnx_server().mark_datagram_ready(true)?;
    }
    dg_ctx.is_ready = [true, true];
    dg_ctx.dg_time_ready = [simulated_time.ticks(), simulated_time.ticks()];
    loss_mask = loss_mask_init;

    while nb_trials < nb_trial_max && nb_inactive < 16 {
        let mut was_active = false;
        let mut time_out = test_datagram_next_time_ready(dg_ctx);

        nb_trials += 1;

        if wifi_todo {
            if simulated_time.ticks() >= wifi_test_time {
                let resume_time = Instant::from_ticks(simulated_time.ticks() + wifi_interval);
                test_ctx.c_to_s_link.suspend(resume_time, false);
                test_ctx.s_to_c_link.suspend(resume_time, true);
                wifi_todo = false;
            } else if time_out == 0 || time_out > wifi_test_time {
                time_out = wifi_test_time;
            }
        }

        tls_api_one_sim_round(
            &mut test_ctx,
            &mut simulated_time,
            Instant::from_ticks(time_out),
            &mut was_active,
        )?;

        was_active |= datagram_app_round(
            dg_ctx,
            simulated_time.ticks(),
            &mut loss_mask,
            &mut client_packets_received,
            &mut last_delivery_time,
        )?;

        if was_active {
            nb_inactive = 0;
        } else {
            nb_inactive += 1;
            let next_time = test_datagram_next_time_ready(dg_ctx);
            if next_time > simulated_time.ticks() {
                simulated_time = Instant::from_ticks(next_time);
            }
        }

        if dg_ctx.dg_recv[0] == dg_ctx.dg_target[1]
            && dg_ctx.dg_recv[1] == dg_ctx.dg_target[0]
            && test_ctx.cnx_client().datagrams.is_empty()
        {
            break;
        } else if (loss_mask_init != 0 || dg_ctx.test_wifi)
            && dg_ctx.dg_sent[0] == dg_ctx.dg_target[0]
            && dg_ctx.dg_sent[1] == dg_ctx.dg_target[1]
        {
            if all_sent_time == 0 {
                let ping_frame = [crate::frames::FrameType::Ping as u8];
                test_ctx.cnx_client().queue_misc_frame(
                    &ping_frame,
                    false,
                    PacketContext::Application,
                )?;
                if test_ctx.has_cnx_server() {
                    test_ctx.cnx_server().queue_misc_frame(
                        &ping_frame,
                        false,
                        PacketContext::Application,
                    )?;
                }
                loss_mask = 0;
                all_sent_time = simulated_time.ticks();
            } else if test_ctx.cnx_client().is_backlog_empty()
                && (!test_ctx.has_cnx_server() || test_ctx.cnx_server().is_backlog_empty())
                && test_ctx.cnx_client().datagrams.is_empty()
            {
                break;
            }
        }

        if !dg_ctx.is_ready[0]
            && test_datagram_check_ready(dg_ctx, 0, simulated_time.ticks())
            && test_ctx.has_cnx_server()
        {
            test_ctx.cnx_server().mark_datagram_ready(true)?;
        }
        if !dg_ctx.is_ready[1] && test_datagram_check_ready(dg_ctx, 1, simulated_time.ticks()) {
            test_ctx.cnx_client().mark_datagram_ready(true)?;
        }
    }

    simulated_time = Instant::from_ticks(simulated_time.ticks().max(last_delivery_time));

    let complete =
        dg_ctx.dg_recv[0] == dg_ctx.dg_target[1] && dg_ctx.dg_recv[1] == dg_ctx.dg_target[0];
    if loss_mask_init == 0 || complete {
        if !complete {
            return Err(Error::Generic);
        }
        if dg_ctx.dg_nacked[0] != dg_ctx.dg_spurious[0]
            || dg_ctx.dg_nacked[1] != dg_ctx.dg_spurious[1]
        {
            return Err(Error::Generic);
        }
        if dg_ctx.max_packets_received > 0 && client_packets_received > dg_ctx.max_packets_received
        {
            return Err(Error::Generic);
        }
        if dg_ctx.duration_max > 0 && dg_ctx.duration_max < simulated_time.ticks() {
            return Err(Error::Generic);
        }
    } else {
        if dg_ctx.dg_recv[0] != dg_ctx.dg_acked[1] + dg_ctx.dg_spurious[1]
            || dg_ctx.dg_recv[1] != dg_ctx.dg_acked[0] + dg_ctx.dg_spurious[0]
        {
            return Err(Error::Generic);
        }
        if dg_ctx.dg_recv[0] + dg_ctx.dg_nacked[1].saturating_sub(dg_ctx.dg_spurious[1])
            != dg_ctx.dg_sent[1]
            || dg_ctx.dg_recv[1] + dg_ctx.dg_nacked[0].saturating_sub(dg_ctx.dg_spurious[0])
                != dg_ctx.dg_sent[0]
        {
            return Err(Error::Generic);
        }
    }

    for i in 0..2 {
        if dg_ctx.dg_latency_target[i] > 0 && dg_ctx.dg_latency_max[i] > dg_ctx.dg_latency_target[i]
        {
            return Err(Error::Generic);
        }
        let _ = dg_ctx.dg_number_delta_max[i] > dg_ctx.dg_number_delta_target[i];
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Test entries.

/// C: `datagram_test` in `picoquictest/datagram_tests.c`.
#[test]
fn datagram() {
    let mut dg_ctx = DatagramSendRecvCtx {
        dg_max_size: MAX_PACKET_SIZE,
        dg_target: [5, 5],
        ..Default::default()
    };
    datagram_test_one(1, &mut dg_ctx, 0);
}

/// C: `datagram_rt_test` in `picoquictest/datagram_tests.c`.
#[test]
fn datagram_rt() {
    let mut dg_ctx = DatagramSendRecvCtx {
        dg_max_size: MAX_PACKET_SIZE,
        dg_target: [100, 100],
        send_delay: 20_000,
        next_gen_time: [100_000, 100_000],
        dg_latency_target: [18_000, 18_000],
        ..Default::default()
    };
    datagram_test_one(2, &mut dg_ctx, 0);
}

/// C: `datagram_rt_skip_test` in `picoquictest/datagram_tests.c`.
#[test]
fn datagram_rt_skip() {
    let mut dg_ctx = DatagramSendRecvCtx {
        dg_max_size: MAX_PACKET_SIZE,
        dg_target: [100, 10],
        send_delay: 20_000,
        next_gen_time: [100_000, 100_000],
        dg_latency_target: [13_000, 20_000],
        do_skip_test: [true, true],
        ..Default::default()
    };
    datagram_test_one(3, &mut dg_ctx, 0);
}

/// C: `datagram_rtnew_skip_test` in `picoquictest/datagram_tests.c`.
#[test]
fn datagram_rtnew_skip() {
    let mut dg_ctx = DatagramSendRecvCtx {
        dg_max_size: MAX_PACKET_SIZE,
        dg_target: [100, 10],
        send_delay: 20_000,
        next_gen_time: [100_000, 100_000],
        dg_latency_target: [13_000, 20_000],
        do_skip_test: [true, true],
        use_extended_provider_api: true,
        ..Default::default()
    };
    datagram_test_one(7, &mut dg_ctx, 0);
}

/// C: `datagram_loss_test` in `picoquictest/datagram_tests.c`.
#[test]
fn datagram_loss() {
    let mut dg_ctx = DatagramSendRecvCtx {
        dg_max_size: MAX_PACKET_SIZE,
        dg_target: [100, 100],
        send_delay: 20_000,
        next_gen_time: [100_000, 100_000],
        ..Default::default()
    };
    datagram_test_one(4, &mut dg_ctx, 0x040080100200400);
}

/// C: `datagram_size_test` in `picoquictest/datagram_tests.c`.
#[test]
fn datagram_size() {
    let mut dg_ctx = DatagramSendRecvCtx {
        dg_max_size: 512,
        dg_target: [100, 100],
        send_delay: 5_000,
        ..Default::default()
    };
    datagram_test_one(5, &mut dg_ctx, 0);
}

/// C: `datagram_small_test` in `picoquictest/datagram_tests.c`.
#[test]
fn datagram_small() {
    let mut dg_ctx = DatagramSendRecvCtx {
        dg_max_size: 512,
        dg_small_size: 64,
        batch_size: [4, 4],
        dg_target: [100, 100],
        send_delay: 5_000,
        next_gen_time: [50_000, 50_000],
        max_packets_received: 55,
        ..Default::default()
    };
    datagram_test_one(6, &mut dg_ctx, 0);
}

/// C: `datagram_small_new_test` in `picoquictest/datagram_tests.c`.
#[test]
fn datagram_small_new() {
    let mut dg_ctx = DatagramSendRecvCtx {
        dg_max_size: 512,
        dg_small_size: 64,
        batch_size: [4, 4],
        dg_target: [100, 100],
        send_delay: 5_000,
        next_gen_time: [50_000, 50_000],
        max_packets_received: 55,
        use_extended_provider_api: true,
        ..Default::default()
    };
    datagram_test_one(7, &mut dg_ctx, 0);
}

/// C: `datagram_small_packet_test` in `picoquictest/datagram_tests.c`.
#[test]
fn datagram_small_packet() {
    let mut dg_ctx = DatagramSendRecvCtx {
        dg_max_size: 512,
        dg_small_size: 64,
        dg_target: [100, 20_000],
        send_delay: 100,
        next_gen_time: [50_000, 50_000],
        link_latency: 10_000,
        picosec_per_byte: 20_000, // 400 Mbps
        dg_latency_target: [20_000, 13_500],
        use_extended_provider_api: true,
        one_datagram_per_packet: true,
        nb_trials_max: 200_000,
        duration_max: 2_060_000,
        ..Default::default()
    };
    datagram_test_one(9, &mut dg_ctx, 0);
}

/// C: `datagram_too_long_test` in `picoquictest/datagram_tests.c`.
#[test]
fn datagram_too_long_test() {
    let mut dg_ctx = DatagramSendRecvCtx {
        dg_max_size: MAX_PACKET_SIZE,
        dg_target: [5, 5],
        test_too_long: true,
        ..Default::default()
    };
    datagram_test_one(10, &mut dg_ctx, 0);
}

/// C: `datagram_wifi_test` in `picoquictest/datagram_tests.c`.
#[test]
fn datagram_wifi() {
    let mut dg_ctx = DatagramSendRecvCtx {
        dg_max_size: MAX_PACKET_SIZE,
        dg_target: [1_000, 1_000],
        send_delay: 2_000,
        next_gen_time: [100_000, 100_000],
        dg_latency_target: [305_000, 280_000],
        test_wifi: true,
        nb_trials_max: 64_000,
        link_latency: 25_000,
        ..Default::default()
    };
    datagram_test_one(8, &mut dg_ctx, 0);
}
