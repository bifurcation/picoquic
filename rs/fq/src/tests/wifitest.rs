//! Test cases for `picoquictest/wifitest*.c`.
//!
//! Each test sets up a wi-fi-like link with one or more "suspension" events
//! (periods where the link is unavailable for scanning) and verifies that
//! the chosen congestion-control algorithm recovers within `target_time`.

#![allow(non_snake_case)]

use crate::tests::util::{WifiTestSpec, WifiTestSuspension, wifi_test_one};

// Wi-fi test ID values match the C `wifi_test_enum` discriminants used to
// seed the initial connection ID inside `wifi_test_one`.
const WIFI_TEST_RENO: u32 = 0;
const WIFI_TEST_CUBIC: u32 = 1;
const WIFI_TEST_BBR: u32 = 2;
const WIFI_TEST_RENO_HARD: u32 = 3;
const WIFI_TEST_CUBIC_HARD: u32 = 4;
const WIFI_TEST_BBR_HARD: u32 = 5;
const WIFI_TEST_RENO_LONG: u32 = 6;
const WIFI_TEST_CUBIC_LONG: u32 = 7;
const WIFI_TEST_BBR_LONG: u32 = 8;
const WIFI_TEST_BBR_SHADOW: u32 = 9;
const WIFI_TEST_BBR_MANY: u32 = 10;
const WIFI_TEST_BBR1: u32 = 11;
const WIFI_TEST_BBR1_HARD: u32 = 12;
const WIFI_TEST_BBR1_LONG: u32 = 13;

static SUSPENSION_BASIC: &[WifiTestSuspension] = &[WifiTestSuspension {
    suspend_time: 1_000_000,
    suspend_interval: 250_000,
}];

static SUSPENSION_HARD: &[WifiTestSuspension] = &[
    WifiTestSuspension {
        suspend_time: 1_000_000,
        suspend_interval: 250_000,
    },
    WifiTestSuspension {
        suspend_time: 1_255_000,
        suspend_interval: 250_000,
    },
    WifiTestSuspension {
        suspend_time: 1_510_000,
        suspend_interval: 250_000,
    },
    WifiTestSuspension {
        suspend_time: 1_765_000,
        suspend_interval: 250_000,
    },
    WifiTestSuspension {
        suspend_time: 2_020_000,
        suspend_interval: 250_000,
    },
    WifiTestSuspension {
        suspend_time: 2_275_000,
        suspend_interval: 250_000,
    },
];

static SUSPENSION_MANY: &[WifiTestSuspension] = &[
    WifiTestSuspension {
        suspend_time: 1_000_000,
        suspend_interval: 250_000,
    },
    WifiTestSuspension {
        suspend_time: 1_500_000,
        suspend_interval: 250_000,
    },
    WifiTestSuspension {
        suspend_time: 2_000_000,
        suspend_interval: 250_000,
    },
    WifiTestSuspension {
        suspend_time: 2_500_000,
        suspend_interval: 250_000,
    },
    WifiTestSuspension {
        suspend_time: 3_000_000,
        suspend_interval: 250_000,
    },
    WifiTestSuspension {
        suspend_time: 3_500_000,
        suspend_interval: 250_000,
    },
];

fn default_spec(
    ccalgo_id: &'static str,
    suspension: &'static [WifiTestSuspension],
    target_time: u64,
) -> WifiTestSpec {
    WifiTestSpec {
        latency: 3_000,
        suspension,
        ccalgo_id,
        cc_algo_option: None,
        target_time,
        simulate_receive_block: false,
        queue_max_delay: 260_000,
    }
}

/// C: `wifi_bbr_test` in `picoquictest/wifitest.c`.
#[test]
fn wifi_bbr() {
    let spec = default_spec("bbr", SUSPENSION_BASIC, 2_800_000);
    wifi_test_one(WIFI_TEST_BBR, &spec).expect("wifi_bbr");
}

/// C: `wifi_bbr1_test` in `picoquictest/wifitest.c`.
#[test]
fn wifi_bbr1() {
    let spec = default_spec("bbr1", SUSPENSION_BASIC, 2_800_000);
    wifi_test_one(WIFI_TEST_BBR1, &spec).expect("wifi_bbr1");
}

/// C: `wifi_bbr1_hard_test` in `picoquictest/wifitest.c`.
#[test]
fn wifi_bbr1_hard() {
    let spec = WifiTestSpec {
        latency: 3_000,
        suspension: SUSPENSION_HARD,
        ccalgo_id: "bbr1",
        cc_algo_option: None,
        target_time: 4_060_000,
        simulate_receive_block: false,
        queue_max_delay: 0,
    };
    wifi_test_one(WIFI_TEST_BBR1_HARD, &spec).expect("wifi_bbr1_hard");
}

/// C: `wifi_bbr1_long_test` in `picoquictest/wifitest.c`.
#[test]
fn wifi_bbr1_long() {
    let spec = WifiTestSpec {
        latency: 50_000,
        suspension: SUSPENSION_BASIC,
        ccalgo_id: "bbr1",
        cc_algo_option: None,
        target_time: 3_400_000,
        simulate_receive_block: true,
        queue_max_delay: 0,
    };
    wifi_test_one(WIFI_TEST_BBR1_LONG, &spec).expect("wifi_bbr1_long");
}

/// C: `wifi_bbr_hard_test` in `picoquictest/wifitest.c`.
#[test]
fn wifi_bbr_hard() {
    let spec = WifiTestSpec {
        latency: 3_000,
        suspension: SUSPENSION_HARD,
        ccalgo_id: "bbr",
        cc_algo_option: None,
        target_time: 4_060_000,
        simulate_receive_block: false,
        queue_max_delay: 0,
    };
    wifi_test_one(WIFI_TEST_BBR_HARD, &spec).expect("wifi_bbr_hard");
}

/// C: `wifi_bbr_long_test` in `picoquictest/wifitest.c`.
#[test]
fn wifi_bbr_long() {
    let spec = WifiTestSpec {
        latency: 50_000,
        suspension: SUSPENSION_BASIC,
        ccalgo_id: "bbr",
        cc_algo_option: None,
        target_time: 3_400_000,
        simulate_receive_block: true,
        queue_max_delay: 0,
    };
    wifi_test_one(WIFI_TEST_BBR_LONG, &spec).expect("wifi_bbr_long");
}

/// C: `wifi_bbr_many_test` in `picoquictest/wifitest.c`.
#[test]
fn wifi_bbr_many() {
    let spec = WifiTestSpec {
        latency: 3_000,
        suspension: SUSPENSION_MANY,
        ccalgo_id: "bbr",
        cc_algo_option: None,
        target_time: 4_070_000,
        simulate_receive_block: false,
        queue_max_delay: 0,
    };
    wifi_test_one(WIFI_TEST_BBR_MANY, &spec).expect("wifi_bbr_many");
}

/// C: `wifi_bbr_shadow_test` in `picoquictest/wifitest.c`.
///
/// Uses the "shadow RTT" CC option (`T250000`) to speed up recovery.
#[test]
fn wifi_bbr_shadow() {
    let spec = WifiTestSpec {
        latency: 3_000,
        suspension: SUSPENSION_BASIC,
        ccalgo_id: "bbr",
        cc_algo_option: Some("T250000"),
        target_time: 2_750_000,
        simulate_receive_block: true,
        queue_max_delay: 600_000,
    };
    wifi_test_one(WIFI_TEST_BBR_SHADOW, &spec).expect("wifi_bbr_shadow");
}

/// C: `wifi_cubic_test` in `picoquictest/wifitest.c`.
#[test]
fn wifi_cubic() {
    let spec = default_spec("cubic", SUSPENSION_BASIC, 2_870_000);
    wifi_test_one(WIFI_TEST_CUBIC, &spec).expect("wifi_cubic");
}

/// C: `wifi_cubic_hard_test` in `picoquictest/wifitest.c`.
#[test]
fn wifi_cubic_hard() {
    let spec = WifiTestSpec {
        latency: 3_000,
        suspension: SUSPENSION_HARD,
        ccalgo_id: "cubic",
        cc_algo_option: None,
        target_time: 4_700_000,
        simulate_receive_block: false,
        queue_max_delay: 0,
    };
    wifi_test_one(WIFI_TEST_CUBIC_HARD, &spec).expect("wifi_cubic_hard");
}

/// C: `wifi_cubic_long_test` in `picoquictest/wifitest.c`.
#[test]
fn wifi_cubic_long() {
    let spec = WifiTestSpec {
        latency: 50_000,
        suspension: SUSPENSION_BASIC,
        ccalgo_id: "cubic",
        cc_algo_option: None,
        target_time: 3_100_000,
        simulate_receive_block: true,
        queue_max_delay: 260_000,
    };
    wifi_test_one(WIFI_TEST_CUBIC_LONG, &spec).expect("wifi_cubic_long");
}

/// C: `wifi_reno_test` in `picoquictest/wifitest.c`.
#[test]
fn wifi_reno() {
    let spec = default_spec("newreno", SUSPENSION_BASIC, 2_800_000);
    wifi_test_one(WIFI_TEST_RENO, &spec).expect("wifi_reno");
}

/// C: `wifi_reno_hard_test` in `picoquictest/wifitest.c`.
#[test]
fn wifi_reno_hard() {
    let spec = WifiTestSpec {
        latency: 3_000,
        suspension: SUSPENSION_HARD,
        ccalgo_id: "newreno",
        cc_algo_option: None,
        target_time: 4_250_000,
        simulate_receive_block: false,
        queue_max_delay: 0,
    };
    wifi_test_one(WIFI_TEST_RENO_HARD, &spec).expect("wifi_reno_hard");
}

/// C: `wifi_reno_long_test` in `picoquictest/wifitest.c`.
#[test]
fn wifi_reno_long() {
    let spec = WifiTestSpec {
        latency: 50_000,
        suspension: SUSPENSION_BASIC,
        ccalgo_id: "newreno",
        cc_algo_option: None,
        target_time: 3_000_000,
        simulate_receive_block: true,
        queue_max_delay: 260_000,
    };
    wifi_test_one(WIFI_TEST_RENO_LONG, &spec).expect("wifi_reno_long");
}
