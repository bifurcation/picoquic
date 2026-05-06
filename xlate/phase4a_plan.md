# Phase 4A Function Map and Implementation Plan

Status: draft

This plan is generated from `xlate/function_translation_map.json`.
Review the `required_missing`, `expected_omission`, and `blocked`
entries before approving Phase 4B.

## Summary

* `implemented`: 752
* `required_missing`: 571
* `expected_omission`: 284
* `blocked`: 0

Total in-scope C functions: 1607

## Required Missing Implementations

| C function | C span | Rust span | Action | Proposed destination | Reason |
| --- | --- | --- | --- | --- | --- |
| `update_windowed_max_filter` | `picoquic/bbr.c:381-392` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `start_windowed_max_filter_period` | `picoquic/bbr.c:394-397` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `update_windowed_min_filter` | `picoquic/bbr.c:399-408` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRInitRandom` | `picoquic/bbr.c:411-432` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRInitFullPipe` | `picoquic/bbr.c:434-439` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRSetOptions` | `picoquic/bbr.c:441-556` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBROnInit` | `picoquic/bbr.c:558-596` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_bbr_reset` | `picoquic/bbr.c:598-601` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_bbr_init` | `picoquic/bbr.c:603-612` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRModulateCwndForRecovery` | `picoquic/bbr.c:627-644` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRBoundCwndForModel` | `picoquic/bbr.c:647-671` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRProbeRTTCwnd` | `picoquic/bbr.c:673-680` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRBoundCwndForProbeRTT` | `picoquic/bbr.c:682-690` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRSetCwnd` | `picoquic/bbr.c:692-717` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRSaveCwnd` | `picoquic/bbr.c:720-734` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRRestoreCwnd` | `picoquic/bbr.c:736-744` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBROnEnterFastRecovery` | `picoquic/bbr.c:746-763` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBREnterLostFeedback` | `picoquic/bbr.c:765-780` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRExitLostFeedback` | `picoquic/bbr.c:782-788` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBROnEnterRTO` | `picoquic/bbr.c:790-806` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBROnExitRecovery` | `picoquic/bbr.c:808-838` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBROnSpuriousLoss` | `picoquic/bbr.c:840-845` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `InLossRecovery` | `picoquic/bbr.c:847-850` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRCheckRecovery` | `picoquic/bbr.c:852-866` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRBDPMultipleWithBw` | `picoquic/bbr.c:868-876` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRBDPMultiple` | `picoquic/bbr.c:878-881` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRUpdateOffloadBudget` | `picoquic/bbr.c:883-886` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRQuantizationBudget` | `picoquic/bbr.c:888-901` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRInflightWithBw` | `picoquic/bbr.c:903-907` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRInflight` | `picoquic/bbr.c:909-912` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRUpdateMaxInflight` | `picoquic/bbr.c:914-930` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRInitPacingRate` | `picoquic/bbr.c:932-942` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRSetPacingRateWithGain` | `picoquic/bbr.c:944-960` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRSetPacingRate` | `picoquic/bbr.c:962-965` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRSetSendQuantum` | `picoquic/bbr.c:967-982` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRUpdateLatestDeliverySignals` | `picoquic/bbr.c:985-1004` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRAdvanceLatestDeliverySignals` | `picoquic/bbr.c:1006-1012` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRResetCongestionSignals` | `picoquic/bbr.c:1014-1022` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRInitLowerBounds` | `picoquic/bbr.c:1024-1033` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRLossLowerBounds` | `picoquic/bbr.c:1035-1048` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRAdaptLowerBoundsFromCongestion` | `picoquic/bbr.c:1051-1051` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRUpdateCongestionSignals` | `picoquic/bbr.c:1066-1086` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRResetLowerBounds` | `picoquic/bbr.c:1088-1092` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRBoundBWForModel` | `picoquic/bbr.c:1094-1104` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRUpdateMaxBw` | `picoquic/bbr.c:1107-1116` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRAdvanceMaxBwFilter` | `picoquic/bbr.c:1118-1127` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRUpdateACKAggregation` | `picoquic/bbr.c:1129-1147` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `IsInflightTooHigh` | `picoquic/bbr.c:1149-1172` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRHandleInflightTooHigh` | `picoquic/bbr.c:1174-1186` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `CheckInflightTooHigh` | `picoquic/bbr.c:1188-1201` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRInitRoundCounting` | `picoquic/bbr.c:1203-1210` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRStartRound` | `picoquic/bbr.c:1212-1217` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRUpdateRound` | `picoquic/bbr.c:1219-1231` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRAdaptMinRttMargin` | `picoquic/bbr.c:1289-1297` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRUpdateRTTJitterBuffer` | `picoquic/bbr.c:1299-1320` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRResetRTTJitterBuffer` | `picoquic/bbr.c:1322-1330` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRUpdateMinRTT` | `picoquic/bbr.c:1332-1384` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `IsRTTTooHigh` | `picoquic/bbr.c:1386-1389` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRExitProbeRTT` | `picoquic/bbr.c:1431-1442` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRCheckProbeRTTDone` | `picoquic/bbr.c:1444-1454` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRHandleProbeRTT` | `picoquic/bbr.c:1456-1485` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBREnterProbeRTT` | `picoquic/bbr.c:1487-1493` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRCheckProbeRTT` | `picoquic/bbr.c:1495-1513` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `IsInAProbeBWState` | `picoquic/bbr.c:1522-1530` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRIsProbingBW` | `picoquic/bbr.c:1532-1540` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRInflightWithHeadroom` | `picoquic/bbr.c:1542-1559` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRRaiseInflightHiSlope` | `picoquic/bbr.c:1561-1568` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRProbeInflightHiUpward` | `picoquic/bbr.c:1570-1587` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRAdaptUpperBounds` | `picoquic/bbr.c:1589-1624` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRCheckTimeToCruise` | `picoquic/bbr.c:1626-1636` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRRandomIntBetween` | `picoquic/bbr.c:1639-1647` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRPickProbeWait` | `picoquic/bbr.c:1658-1673` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRPickProbeWaitEarly` | `picoquic/bbr.c:1675-1690` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRTargetInflight` | `picoquic/bbr.c:1691-1696` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRCheckPathSaturated` | `picoquic/bbr.c:1698-1720` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRCheckAppLimitedEnded` | `picoquic/bbr.c:1724-1766` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRIsRenoCoexistenceProbeTime` | `picoquic/bbr.c:1768-1773` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRHasElapsedInPhase` | `picoquic/bbr.c:1775-1778` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRCheckTimeToProbeBW` | `picoquic/bbr.c:1780-1791` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRStartProbeBW_DOWN` | `picoquic/bbr.c:1793-1814` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRStartProbeBW_CRUISE` | `picoquic/bbr.c:1816-1821` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRStartProbeBW_REFILL` | `picoquic/bbr.c:1823-1835` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRStartProbeBW_UP` | `picoquic/bbr.c:1837-1848` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRUpdateProbeBWCyclePhase` | `picoquic/bbr.c:1850-1923` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBREnterProbeBW` | `picoquic/bbr.c:1925-1929` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBREnterDrain` | `picoquic/bbr.c:1932-1941` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRCheckDrain` | `picoquic/bbr.c:1943-1948` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRCheckStartupFullBandwidthGeneric` | `picoquic/bbr.c:1951-1970` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBREnterStartupResume` | `picoquic/bbr.c:1972-1980` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRCheckStartupResume` | `picoquic/bbr.c:1982-2000` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRCheckStartupHighLoss` | `picoquic/bbr.c:2002-2018` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRCheckStartupFullBandwidth` | `picoquic/bbr.c:2020-2040` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRCheckStartupDone` | `picoquic/bbr.c:2042-2059` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBREnterStartup` | `picoquic/bbr.c:2061-2067` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRReEnterStartup` | `picoquic/bbr.c:2069-2076` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBREnterStartupLongRTT` | `picoquic/bbr.c:2080-2101` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRExitStartupLongRtt` | `picoquic/bbr.c:2103-2126` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRCheckStartupLongRtt` | `picoquic/bbr.c:2128-2153` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRUpdateStartupLongRtt` | `picoquic/bbr.c:2155-2171` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRSetBdpSeed` | `picoquic/bbr.c:2173-2180` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRUpdateRecoveryOnLoss` | `picoquic/bbr.c:2201-2216` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRAccessEcnPacketContext` | `picoquic/bbr.c:2244-2260` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRComputeEcnFrac` | `picoquic/bbr.c:2262-2286` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRAdvanceEcnFrac` | `picoquic/bbr.c:2288-2305` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRUpdateModelAndState` | `picoquic/bbr.c:2307-2326` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRUpdateControlParameters` | `picoquic/bbr.c:2328-2333` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRUpdateOnACK` | `picoquic/bbr.c:2335-2344` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBRSetRsFromAckState` | `picoquic/bbr.c:2346-2386` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_bbr_notify_ack` | `picoquic/bbr.c:2388-2398` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_bbr_notify` | `picoquic/bbr.c:2400-2464` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_bbr_observe` | `picoquic/bbr.c:2468-2473` | - | `required_missing` | `rs/fq/src/bbr.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBR1GetBtlBW` | `picoquic/bbr1.c:304-307` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBR1EnterStartupLongRTT` | `picoquic/bbr1.c:309-325` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBR1EnterStartup` | `picoquic/bbr1.c:327-332` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBR1SetSendQuantum` | `picoquic/bbr1.c:334-348` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBR1Inflight` | `picoquic/bbr1.c:350-364` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBR1UpdateTargetCwnd` | `picoquic/bbr1.c:367-370` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_bbr1_set_options` | `picoquic/bbr1.c:373-447` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_bbr1_reset` | `picoquic/bbr1.c:449-467` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_bbr1_init` | `picoquic/bbr1.c:469-479` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBR1ltbwResetInterval` | `picoquic/bbr1.c:494-501` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBR1ltbwResetSampling` | `picoquic/bbr1.c:503-509` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBR1ltbwIntervalDone` | `picoquic/bbr1.c:511-528` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBR1ltbwSampling` | `picoquic/bbr1.c:530-608` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBR1UpdateBtlBw` | `picoquic/bbr1.c:616-677` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBR1UpdateRTprop` | `picoquic/bbr1.c:679-696` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBR1IsNextCyclePhase` | `picoquic/bbr1.c:698-713` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBR1SetMinimalGain` | `picoquic/bbr1.c:715-729` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBR1AdvanceCyclePhase` | `picoquic/bbr1.c:731-754` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBR1CheckCyclePhase` | `picoquic/bbr1.c:756-762` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBR1ResetProbeBwMode` | `picoquic/bbr1.c:764-769` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBR1CheckFullPipe` | `picoquic/bbr1.c:771-785` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBR1EnterProbeBW` | `picoquic/bbr1.c:787-812` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBR1EnterDrain` | `picoquic/bbr1.c:814-822` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBR1CheckDrain` | `picoquic/bbr1.c:824-833` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBR1ExitStartupLongRtt` | `picoquic/bbr1.c:835-858` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBR1ExitStartupSeedBDP` | `picoquic/bbr1.c:860-883` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBR1EnterProbeRTT` | `picoquic/bbr1.c:885-890` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBR1ExitProbeRTT` | `picoquic/bbr1.c:892-900` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `InLossRecovery1` | `picoquic/bbr1.c:902-905` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBR1SaveCwnd` | `picoquic/bbr1.c:907-916` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBR1RestoreCwnd` | `picoquic/bbr1.c:918-923` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBR1HandleProbeRTT` | `picoquic/bbr1.c:926-953` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBR1CheckProbeRTT` | `picoquic/bbr1.c:955-969` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBR1UpdateModelAndState` | `picoquic/bbr1.c:971-980` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBR1SetPacingRateWithGain` | `picoquic/bbr1.c:982-989` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBR1SetPacingRate` | `picoquic/bbr1.c:991-994` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBR1ModulateCwndForRecovery` | `picoquic/bbr1.c:996-1013` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBR1ModulateCwndForProbeRTT` | `picoquic/bbr1.c:1015-1023` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBR1SetCwnd` | `picoquic/bbr1.c:1025-1047` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBR1UpdateControlParameters` | `picoquic/bbr1.c:1050-1055` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBR1HandleRestartFromIdle` | `picoquic/bbr1.c:1057-1066` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBR1UpdateOnACK` | `picoquic/bbr1.c:1074-1081` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBR1OnTransmit` | `picoquic/bbr1.c:1083-1086` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBR1OnAllPacketsLost` | `picoquic/bbr1.c:1091-1095` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBR1OnEnterFastRecovery` | `picoquic/bbr1.c:1097-1105` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBR1AfterOneRoundtripInFastRecovery` | `picoquic/bbr1.c:1107-1110` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `BBR1ExitFastRecovery` | `picoquic/bbr1.c:1112-1116` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_bbr1_notify_congestion` | `picoquic/bbr1.c:1118-1157` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_bbr1_suspension_almost_over` | `picoquic/bbr1.c:1159-1172` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_bbr1_suspension_exit` | `picoquic/bbr1.c:1174-1186` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_bbr1_notify` | `picoquic/bbr1.c:1191-1319` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_bbr1_observe` | `picoquic/bbr1.c:1323-1328` | - | `required_missing` | `rs/fq/src/bbr1.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `bytestream_vint_len` | `picoquic/bytestream.c:163-166` | - | `required_missing` | `rs/fq/src/bytestream.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `bytestream_error` | `picoquic/bytestream.c:433-437` | - | `required_missing` | `rs/fq/src/bytestream.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `c4_logger` | `picoquic/c4.c:199-216` | - | `required_missing` | `rs/fq/src/c4.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `c4_sensitivity_1024` | `picoquic/c4.c:244-260` | - | `required_missing` | `rs/fq/src/c4.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `c4_delay_threshold` | `picoquic/c4.c:266-275` | - | `required_missing` | `rs/fq/src/c4.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `c4_ecn_threshold` | `picoquic/c4.c:277-288` | - | `required_missing` | `rs/fq/src/c4.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `c4_loss_threshold` | `picoquic/c4.c:290-299` | - | `required_missing` | `rs/fq/src/c4.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `c4_update_loss_rate` | `picoquic/c4.c:301-319` | - | `required_missing` | `rs/fq/src/c4.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `c4_update_ecn_alpha` | `picoquic/c4.c:321-350` | - | `required_missing` | `rs/fq/src/c4.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `c4_apply_rate_and_cwin` | `picoquic/c4.c:359-424` | - | `required_missing` | `rs/fq/src/c4.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `c4_growth_evaluate` | `picoquic/c4.c:426-447` | - | `required_missing` | `rs/fq/src/c4.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `c4_growth_reset` | `picoquic/c4.c:449-456` | - | `required_missing` | `rs/fq/src/c4.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `c4_era_check` | `picoquic/c4.c:459-473` | - | `required_missing` | `rs/fq/src/c4.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `c4_era_reset` | `picoquic/c4.c:475-484` | - | `required_missing` | `rs/fq/src/c4.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `c4_enter_initial` | `picoquic/c4.c:486-497` | - | `required_missing` | `rs/fq/src/c4.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `c4_set_options` | `picoquic/c4.c:499-515` | - | `required_missing` | `rs/fq/src/c4.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `c4_reset` | `picoquic/c4.c:517-525` | - | `required_missing` | `rs/fq/src/c4.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `c4_seed_cwin` | `picoquic/c4.c:527-533` | - | `required_missing` | `rs/fq/src/c4.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `c4_exit_initial` | `picoquic/c4.c:535-549` | - | `required_missing` | `rs/fq/src/c4.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `c4_initial_handle_rtt_excess` | `picoquic/c4.c:551-566` | - | `required_missing` | `rs/fq/src/c4.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `c4_initial_handle_loss` | `picoquic/c4.c:568-574` | - | `required_missing` | `rs/fq/src/c4.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `c4_initial_handle_ack` | `picoquic/c4.c:576-623` | - | `required_missing` | `rs/fq/src/c4.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `c4_init` | `picoquic/c4.c:625-641` | - | `required_missing` | `rs/fq/src/c4.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `c4_enter_recovery` | `picoquic/c4.c:643-666` | - | `required_missing` | `rs/fq/src/c4.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `c4_exit_recovery` | `picoquic/c4.c:674-709` | - | `required_missing` | `rs/fq/src/c4.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `c4_enter_cruise` | `picoquic/c4.c:711-742` | - | `required_missing` | `rs/fq/src/c4.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `c4_enter_push` | `picoquic/c4.c:744-754` | - | `required_missing` | `rs/fq/src/c4.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `c4_update_min_max_rtt` | `picoquic/c4.c:756-800` | - | `required_missing` | `rs/fq/src/c4.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `c4_handle_ack` | `picoquic/c4.c:802-883` | - | `required_missing` | `rs/fq/src/c4.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `c4_notify_congestion` | `picoquic/c4.c:885-968` | - | `required_missing` | `rs/fq/src/c4.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `c4_update_rtt` | `picoquic/c4.c:976-1006` | - | `required_missing` | `rs/fq/src/c4.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `c4_handle_rtt_excess` | `picoquic/c4.c:1008-1017` | - | `required_missing` | `rs/fq/src/c4.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `c4_notify` | `picoquic/c4.c:1019-1109` | - | `required_missing` | `rs/fq/src/c4.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `c4_observe` | `picoquic/c4.c:1120-1126` | - | `required_missing` | `rs/fq/src/c4.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `config_parse_target_version` | `picoquic/config.c:116-146` | - | `required_missing` | `rs/fq/src/config.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `config_set_string_param` | `picoquic/config.c:148-179` | - | `required_missing` | `rs/fq/src/config.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `config_optval_string` | `picoquic/config.c:181-189` | - | `required_missing` | `rs/fq/src/config.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `config_optval_param_string` | `picoquic/config.c:191-200` | - | `required_missing` | `rs/fq/src/config.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `config_atoi` | `picoquic/config.c:202-224` | - | `required_missing` | `rs/fq/src/config.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `config_set_option` | `picoquic/config.c:274-553` | - | `required_missing` | `rs/fq/src/config.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_config_get_option_char_index` | `picoquic/config.c:641-652` | - | `required_missing` | `rs/fq/src/config.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_config_get_option_name_index` | `picoquic/config.c:654-665` | - | `required_missing` | `rs/fq/src/config.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_config_get_command_line_option_index` | `picoquic/config.c:667-681` | - | `required_missing` | `rs/fq/src/config.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_get_command_line_option_value` | `picoquic/config.c:683-720` | - | `required_missing` | `rs/fq/src/config.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_config_clear` | `picoquic/config.c:1046-1121` | - | `required_missing` | `rs/fq/src/config.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `cubic_reset` | `picoquic/cubic.c:53-68` | - | `required_missing` | `rs/fq/src/cubic.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `cubic_init` | `picoquic/cubic.c:70-81` | - | `required_missing` | `rs/fq/src/cubic.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `cubic_root` | `picoquic/cubic.c:83-119` | - | `required_missing` | `rs/fq/src/cubic.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `cubic_W_cubic` | `picoquic/cubic.c:121-130` | - | `required_missing` | `rs/fq/src/cubic.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `cubic_enter_avoidance` | `picoquic/cubic.c:132-142` | - | `required_missing` | `rs/fq/src/cubic.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `cubic_enter_recovery` | `picoquic/cubic.c:144-204` | - | `required_missing` | `rs/fq/src/cubic.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `cubic_correct_spurious` | `picoquic/cubic.c:206-235` | - | `required_missing` | `rs/fq/src/cubic.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `cubic_notify` | `picoquic/cubic.c:237-423` | - | `required_missing` | `rs/fq/src/cubic.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `dcubic_exit_slow_start` | `picoquic/cubic.c:424-456` | - | `required_missing` | `rs/fq/src/cubic.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `dcubic_notify` | `picoquic/cubic.c:458-548` | - | `required_missing` | `rs/fq/src/cubic.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `cubic_observe` | `picoquic/cubic.c:562-567` | - | `required_missing` | `rs/fq/src/cubic.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `dualq_dequeue_queue` | `picoquic/dualq_aqm.c:114-139` | - | `required_missing` | `rs/fq/src/dualq_aqm.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `dualq_recur` | `picoquic/dualq_aqm.c:182-191` | - | `required_missing` | `rs/fq/src/dualq_aqm.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `dualq_scheduler` | `picoquic/dualq_aqm.c:193-208` | - | `required_missing` | `rs/fq/src/dualq_aqm.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `dualq_laqm` | `picoquic/dualq_aqm.c:210-227` | - | `required_missing` | `rs/fq/src/dualq_aqm.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_base64_encode` | `picoquic/ech.c:76-93` | - | `required_missing` | `rs/fq/src/ech.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_ech_read_config` | `picoquic/ech.c:95-124` | rs/fq/src/lib.rs:3492-3502 `ech_read_config` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_ech_save_config` | `picoquic/ech.c:127-169` | rs/fq/src/lib.rs:3499-3502 `ech_save_config` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `ech_opener_callback` | `picoquic/ech.c:230-267` | - | `required_missing` | `rs/fq/src/ech.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `ech_init_opener_callback` | `picoquic/ech.c:280-331` | - | `required_missing` | `rs/fq/src/ech.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_ech_configure_quic_ctx` | `picoquic/ech.c:333-366` | - | `required_missing` | `rs/fq/src/ech.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_ech_configure_client` | `picoquic/ech.c:391-413` | rs/fq/src/lib.rs:3460-3463 `ech_configure_client` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_is_ech_handshake` | `picoquic/ech.c:415-422` | rs/fq/src/lib.rs:3466-3469 `is_ech_handshake` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_parse_public_key_asn1` | `picoquic/ech.c:502-593` | - | `required_missing` | `rs/fq/src/ech.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_ech_parse_public_key` | `picoquic/ech.c:601-653` | - | `required_missing` | `rs/fq/src/ech.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_ech_get_kem_from_curve` | `picoquic/ech.c:655-670` | - | `required_missing` | `rs/fq/src/ech.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_ech_get_ciphers_from_kem` | `picoquic/ech.c:672-736` | - | `required_missing` | `rs/fq/src/ech.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `ech_config_id_from_config` | `picoquic/ech.c:738-759` | - | `required_missing` | `rs/fq/src/ech.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_ech_create_config_list_from_config` | `picoquic/ech.c:761-800` | - | `required_missing` | `rs/fq/src/ech.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_ech_create_config_from_pk` | `picoquic/ech.c:802-825` | - | `required_missing` | `rs/fq/src/ech.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_ech_create_rr_from_binary` | `picoquic/ech.c:827-842` | - | `required_missing` | `rs/fq/src/ech.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_ech_create_config_from_binary` | `picoquic/ech.c:844-855` | - | `required_missing` | `rs/fq/src/ech.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_ech_create_config_from_public_key` | `picoquic/ech.c:857-876` | rs/fq/src/lib.rs:3506-3522 `ech_create_config_from_public_key` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_ech_create_config_from_private_key` | `picoquic/ech.c:879-925` | rs/fq/src/lib.rs:3516-3528 `ech_create_config_from_private_key` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_ech_create_config_file` | `picoquic/ech.c:927-940` | rs/fq/src/lib.rs:3481-3488 `ech_create_config_file` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_ech_get_retry_config` | `picoquic/ech.c:1067-1075` | - | `required_missing` | `rs/fq/src/ech.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_fastcc_delay_threshold` | `picoquic/fastcc.c:65-72` | - | `required_missing` | `rs/fq/src/fastcc.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_fastcc_reset` | `picoquic/fastcc.c:74-83` | - | `required_missing` | `rs/fq/src/fastcc.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_fastcc_seed_cwin` | `picoquic/fastcc.c:85-92` | - | `required_missing` | `rs/fq/src/fastcc.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_fastcc_init` | `picoquic/fastcc.c:94-117` | - | `required_missing` | `rs/fq/src/fastcc.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `fastcc_notify_congestion` | `picoquic/fastcc.c:119-160` | - | `required_missing` | `rs/fq/src/fastcc.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_fastcc_notify` | `picoquic/fastcc.c:162-306` | - | `required_missing` | `rs/fq/src/fastcc.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_fastcc_observe` | `picoquic/fastcc.c:320-325` | - | `required_missing` | `rs/fq/src/fastcc.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_enforce_reset_stream_frame` | `picoquic/frames.c:252-282` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_format_reset_stream_frame` | `picoquic/frames.c:284-303` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_signal_stream_reset` | `picoquic/frames.c:305-328` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_apply_reset_stream_frame` | `picoquic/frames.c:330-371` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_format_reset_stream_at_frame` | `picoquic/frames.c:485-505` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_format_retire_connection_id_frame` | `picoquic/frames.c:818-835` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_format_new_token_frame` | `picoquic/frames.c:1013-1027` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_format_stop_sending_frame` | `picoquic/frames.c:1087-1112` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_format_data_blocked_frame` | `picoquic/frames.c:1674-1690` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_format_stream_data_blocked_frame` | `picoquic/frames.c:1692-1710` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_format_stream_blocked_frame` | `picoquic/frames.c:1712-1748` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_encode_length_of_stream_frame` | `picoquic/frames.c:1810-1838` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_queue_data_repeat_node_create` | `picoquic/frames.c:2129-2136` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_queue_data_repeat_node_value` | `picoquic/frames.c:2138-2141` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_queue_data_repeat_compare` | `picoquic/frames.c:2144-2172` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_queue_data_repeat_adjust` | `picoquic/frames.c:2203-2252` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_copy_single_stream_frame_for_retransmit` | `picoquic/frames.c:2399-2460` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_estimate_path_bandwidth` | `picoquic/frames.c:2865-2922` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_estimate_max_path_bandwidth` | `picoquic/frames.c:2924-2961` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_compute_packets_in_window` | `picoquic/frames.c:2974-2995` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_compute_ack_gap` | `picoquic/frames.c:2997-3045` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_compute_ack_delay_max` | `picoquic/frames.c:3047-3064` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_format_ack_frame_in_context` | `picoquic/frames.c:4074-4215` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_ack_gap_override_if_needed` | `picoquic/frames.c:4290-4307` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_is_ack_needed_in_ctx` | `picoquic/frames.c:4309-4353` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_format_datagram_frame` | `picoquic/frames.c:5288-5305` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_format_path_available_or_backup_frame` | `picoquic/frames.c:5863-5878` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_queue_max_path_id_frame` | `picoquic/frames.c:6012-6026` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_format_paths_blocked_frame` | `picoquic/frames.c:6114-6126` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_queue_paths_blocked_frame` | `picoquic/frames.c:6128-6142` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_format_path_cid_blocked_frame` | `picoquic/frames.c:6224-6237` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_path_cid_next_sequence_number` | `picoquic/frames.c:6239-6255` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_queue_path_cid_blocked_frame` | `picoquic/frames.c:6257-6276` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoformat_16` | `picoquic/intformat.c:27-31` | - | `required_missing` | `rs/fq/src/utils.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoformat_24` | `picoquic/intformat.c:33-38` | - | `required_missing` | `rs/fq/src/utils.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoformat_32` | `picoquic/intformat.c:40-46` | - | `required_missing` | `rs/fq/src/utils.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoformat_64` | `picoquic/intformat.c:48-58` | - | `required_missing` | `rs/fq/src/utils.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `create_binlog` | `picoquic/logwriter.c:1123-1145` | - | `required_missing` | `rs/fq/src/binlog.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_binlog_message_v` | `picoquic/logwriter.c:1237-1272` | - | `required_missing` | `rs/fq/src/binlog.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `binlog_app_message` | `picoquic/logwriter.c:1300-1307` | - | `required_missing` | `rs/fq/src/binlog.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `binlog_close` | `picoquic/logwriter.c:1309-1315` | - | `required_missing` | `rs/fq/src/binlog.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_retransmit_needed_loop` | `picoquic/loss_recovery.c:203-223` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_retransmit_needed_packet` | `picoquic/loss_recovery.c:297-533` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_is_packet_probably_lost` | `picoquic/loss_recovery.c:535-644` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_set_wake_up_from_packet_retransmit` | `picoquic/loss_recovery.c:646-662` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_process_lost_packet` | `picoquic/loss_recovery.c:784-871` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_check_path_mtu_on_losses` | `picoquic/loss_recovery.c:873-893` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_count_and_notify_loss` | `picoquic/loss_recovery.c:895-932` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_is_packet_ack_eliciting` | `picoquic/loss_recovery.c:934-967` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_retransmit_path_packet_queue` | `picoquic/loss_recovery.c:969-1008` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_newreno_sim_enter_recovery` | `picoquic/newreno.c:45-71` | - | `required_missing` | `rs/fq/src/newreno.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_newreno_sim_seed_cwin` | `picoquic/newreno.c:73-85` | - | `required_missing` | `rs/fq/src/newreno.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_newreno_reset` | `picoquic/newreno.c:182-187` | - | `required_missing` | `rs/fq/src/newreno.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_newreno_init` | `picoquic/newreno.c:189-205` | - | `required_missing` | `rs/fq/src/newreno.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_newreno_notify` | `picoquic/newreno.c:207-296` | - | `required_missing` | `rs/fq/src/newreno.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_newreno_observe` | `picoquic/newreno.c:309-314` | - | `required_missing` | `rs/fq/src/newreno.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_pacing_init` | `picoquic/pacing.c:27-35` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_report_pacing_update` | `picoquic/pacing.c:107-135` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_screen_initial_packet` | `picoquic/packet.c:85-207` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_remove_header_protection` | `picoquic/packet.c:617-631` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_remove_packet_protection` | `picoquic/packet.c:633-768` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_incoming_version_negotiation` | `picoquic/packet.c:904-986` | rs/fq/src/lib.rs:2622-2633 `incoming_packet_ex` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_prepare_version_negotiation` | `picoquic/packet.c:994-1075` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_process_unexpected_cnxid` | `picoquic/packet.c:1077-1138` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_queue_stateless_retry` | `picoquic/packet.c:1144-1208` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_queue_retry_packet` | `picoquic/packet.c:1210-1238` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_queue_busy_packet` | `picoquic/packet.c:1240-1315` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_queue_immediate_close` | `picoquic/packet.c:1317-1334` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_ignore_incoming_handshake` | `picoquic/packet.c:1345-1387` | rs/fq/src/lib.rs:2622-2633 `incoming_packet_ex` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_incoming_client_initial` | `picoquic/packet.c:1394-1505` | rs/fq/src/lib.rs:2622-2633 `incoming_packet_ex` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_incoming_retry` | `picoquic/packet.c:1521-1613` | rs/fq/src/lib.rs:2622-2633 `incoming_packet_ex` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_incoming_server_initial` | `picoquic/packet.c:1619-1701` | rs/fq/src/lib.rs:2622-2633 `incoming_packet_ex` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_incoming_server_handshake` | `picoquic/packet.c:1704-1752` | rs/fq/src/lib.rs:2622-2633 `incoming_packet_ex` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_incoming_client_handshake` | `picoquic/packet.c:1753-1811` | rs/fq/src/lib.rs:2622-2633 `incoming_packet_ex` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_incoming_stateless_reset` | `picoquic/packet.c:1813-1829` | rs/fq/src/lib.rs:2622-2633 `incoming_packet_ex` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_incoming_0rtt` | `picoquic/packet.c:1835-1877` | rs/fq/src/lib.rs:2622-2633 `incoming_packet_ex` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_ecn_accounting` | `picoquic/packet.c:1880-1908` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_incoming_1rtt` | `picoquic/packet.c:1910-2017` | rs/fq/src/lib.rs:2622-2633 `incoming_packet_ex` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_incoming_not_decrypted` | `picoquic/packet.c:2019-2067` | rs/fq/src/lib.rs:2622-2633 `incoming_packet_ex` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_incoming_segment` | `picoquic/packet.c:2073-2387` | rs/fq/src/lib.rs:2622-2633 `incoming_packet_ex` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_incoming_packet_ex` | `picoquic/packet.c:2389-2430` | rs/fq/src/lib.rs:2622-2633 `incoming_packet_ex` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_incoming_packet` | `picoquic/packet.c:2432-2447` | rs/fq/src/lib.rs:2606-2617 `incoming_packet` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_prepare_tuple_challenge_frames` | `picoquic/paths.c:33-138` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_tuple_challenge_time` | `picoquic/paths.c:235-257` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_check_path_control_needed` | `picoquic/paths.c:288-321` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_verify_path_available` | `picoquic/paths.c:323-373` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_sort_available_paths` | `picoquic/paths.c:379-486` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_lb_compat_cid_generate_first_byte` | `picoquic/picoquic_lb.c:42-52` | - | `required_missing` | `rs/fq/src/lb.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_lb_compat_cid_generate_clear` | `picoquic/picoquic_lb.c:54-59` | - | `required_missing` | `rs/fq/src/lb.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_lb_compat_cid_generate_stream_cipher` | `picoquic/picoquic_lb.c:84-101` | - | `required_missing` | `rs/fq/src/lb.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_lb_compat_cid_generate_block_cipher` | `picoquic/picoquic_lb.c:110-121` | - | `required_missing` | `rs/fq/src/lb.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_lb_compat_cid_verify_clear` | `picoquic/picoquic_lb.c:150-161` | - | `required_missing` | `rs/fq/src/lb.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_lb_compat_cid_verify_stream_cipher` | `picoquic/picoquic_lb.c:163-186` | - | `required_missing` | `rs/fq/src/lb.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_lb_compat_cid_verify_block_cipher` | `picoquic/picoquic_lb.c:188-205` | - | `required_missing` | `rs/fq/src/lb.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_mbedtls_load` | `picoquic/picoquic_mbedtls.c:27-42` | - | `required_missing` | `rs/fq/src/sys/mod.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_ptls_fusion_load` | `picoquic/picoquic_ptls_fusion.c:77-84` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `set_minicrypto_private_key_from_key_file` | `picoquic/picoquic_ptls_minicrypto.c:35-38` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_clear_minicrypto` | `picoquic/picoquic_ptls_minicrypto.c:43-46` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_init_minicrypto` | `picoquic/picoquic_ptls_minicrypto.c:48-51` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_ptls_minicrypto_load` | `picoquic/picoquic_ptls_minicrypto.c:54-83` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_init_openssl` | `picoquic/picoquic_ptls_openssl.c:71-88` | - | `required_missing` | `rs/fq/src/sys/openssl.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_clear_openssl` | `picoquic/picoquic_ptls_openssl.c:90-108` | - | `required_missing` | `rs/fq/src/sys/openssl.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `set_openssl_sign_certificate_from_key` | `picoquic/picoquic_ptls_openssl.c:110-136` | - | `required_missing` | `rs/fq/src/sys/openssl.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `set_openssl_private_key_from_key_file` | `picoquic/picoquic_ptls_openssl.c:138-158` | - | `required_missing` | `rs/fq/src/sys/openssl.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_openssl_get_public_key_from_key_file` | `picoquic/picoquic_ptls_openssl.c:160-199` | - | `required_missing` | `rs/fq/src/sys/openssl.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_openssl_get_certs_from_file` | `picoquic/picoquic_ptls_openssl.c:210-235` | - | `required_missing` | `rs/fq/src/sys/openssl.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_openssl_get_openssl_certificate_verifier` | `picoquic/picoquic_ptls_openssl.c:237-267` | - | `required_missing` | `rs/fq/src/sys/openssl.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_openssl_get_certificate_verifier` | `picoquic/picoquic_ptls_openssl.c:277-292` | - | `required_missing` | `rs/fq/src/sys/openssl.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_openssl_set_tls_root_certificates` | `picoquic/picoquic_ptls_openssl.c:298-319` | - | `required_missing` | `rs/fq/src/sys/openssl.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_open_ssl_explain_crypto_error` | `picoquic/picoquic_ptls_openssl.c:321-332` | - | `required_missing` | `rs/fq/src/sys/openssl.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_openssl_clear_crypto_errors` | `picoquic/picoquic_ptls_openssl.c:334-340` | - | `required_missing` | `rs/fq/src/sys/openssl.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `openssl_keyex_from_key_file` | `picoquic/picoquic_ptls_openssl.c:342-365` | - | `required_missing` | `rs/fq/src/sys/openssl.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_ptls_openssl_load` | `picoquic/picoquic_ptls_openssl.c:406-454` | - | `required_missing` | `rs/fq/src/sys/openssl.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_ptls_openssl_log_version` | `picoquic/picoquic_ptls_openssl.c:456-464` | - | `required_missing` | `rs/fq/src/sys/openssl.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_socket_set_pkt_info` | `picoquic/picosocks.c:58-91` | - | `required_missing` | `rs/fq/src/socks.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_socket_set_ecn_options_ex` | `picoquic/picosocks.c:93-223` | - | `required_missing` | `rs/fq/src/socks.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_socket_set_ecn_options` | `picoquic/picosocks.c:225-228` | - | `required_missing` | `rs/fq/src/socks.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_socket_set_pmtud_options` | `picoquic/picosocks.c:230-248` | - | `required_missing` | `rs/fq/src/socks.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `cmsg_format_header_return_data_ptr` | `picoquic/picosocks.c:519-543` | - | `required_missing` | `rs/fq/src/socks.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_send_through_socket` | `picoquic/picosocks.c:1262-1271` | - | `required_missing` | `rs/fq/src/socks.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `zig` | `picoquic/picosplay.c:59-62` | - | `required_missing` | `rs/fq/src/splay.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `zigzig` | `picoquic/picosplay.c:64-70` | - | `required_missing` | `rs/fq/src/splay.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `zigzag` | `picoquic/picosplay.c:72-78` | - | `required_missing` | `rs/fq/src/splay.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picosplay_init_tree` | `picoquic/picosplay.c:80-88` | - | `required_missing` | `rs/fq/src/splay.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picosplay_new_tree` | `picoquic/picosplay.c:90-97` | - | `required_missing` | `rs/fq/src/splay.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `mark_gp` | `picoquic/picosplay.c:307-318` | - | `required_missing` | `rs/fq/src/splay.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_disable_port_blocking` | `picoquic/port_blocking.c:162-166` | - | `required_missing` | `rs/fq/src/port_blocking.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_prague_init_reno` | `picoquic/prague.c:118-124` | - | `required_missing` | `rs/fq/src/prague.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_prague_init` | `picoquic/prague.c:126-142` | - | `required_missing` | `rs/fq/src/prague.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_prague_get_pkt_ctx` | `picoquic/prague.c:144-154` | - | `required_missing` | `rs/fq/src/prague.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_prague_reset_l3s` | `picoquic/prague.c:156-163` | - | `required_missing` | `rs/fq/src/prague.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_prague_reset` | `picoquic/prague.c:166-170` | - | `required_missing` | `rs/fq/src/prague.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_prague_initialize_era` | `picoquic/prague.c:172-184` | - | `required_missing` | `rs/fq/src/prague.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_prague_enter_recovery` | `picoquic/prague.c:186-208` | - | `required_missing` | `rs/fq/src/prague.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_prague_update_alpha` | `picoquic/prague.c:210-249` | - | `required_missing` | `rs/fq/src/prague.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_prague_process_ack` | `picoquic/prague.c:251-292` | - | `required_missing` | `rs/fq/src/prague.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_prague_process_start_ack` | `picoquic/prague.c:294-317` | - | `required_missing` | `rs/fq/src/prague.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_prague_notify` | `picoquic/prague.c:320-394` | - | `required_missing` | `rs/fq/src/prague.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_prague_observe` | `picoquic/prague.c:407-412` | - | `required_missing` | `rs/fq/src/prague.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_update_issued_ticket` | `picoquic/quicctx.c:444-459` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_registered_token_create` | `picoquic/quicctx.c:551-554` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_set_tp_value_by_type` | `picoquic/quicctx.c:819-891` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_unregister_net_id` | `picoquic/quicctx.c:1280-1290` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_register_net_id` | `picoquic/quicctx.c:1292-1310` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_unregister_net_icid` | `picoquic/quicctx.c:1356-1363` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_unregister_net_secret` | `picoquic/quicctx.c:1365-1372` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_get_quic_ctx` | `picoquic/quicctx.c:1419-1422` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_get_next_cnx` | `picoquic/quicctx.c:1430-1434` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_insert_cnx_in_list` | `picoquic/quicctx.c:1436-1448` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_remove_cnx_from_list` | `picoquic/quicctx.c:1450-1469` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_wake_list_init` | `picoquic/quicctx.c:1499-1503` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_remove_cnx_from_wake_list` | `picoquic/quicctx.c:1505-1508` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_insert_cnx_by_wake_time` | `picoquic/quicctx.c:1510-1513` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_create_random_cnx_id` | `picoquic/quicctx.c:1630-1639` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_unchain_tuple` | `picoquic/quicctx.c:1733-1751` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_clear_path_data` | `picoquic/quicctx.c:1885-1899` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_notify_destination_unreachable` | `picoquic/quicctx.c:2217-2244` | rs/fq/src/lib.rs:2720-2729 `notify_destination_unreachable` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_notify_destination_unreachable_by_cnxid` | `picoquic/quicctx.c:2246-2262` | rs/fq/src/lib.rs:2736-2746 `notify_destination_unreachable_by_connection_id` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_check_new_path_allowed` | `picoquic/quicctx.c:2300-2342` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_verify_proposed_tuple` | `picoquic/quicctx.c:2385-2435` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_get_path_quality_from_context` | `picoquic/quicctx.c:2678-2699` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_subscribe_to_quality_update_per_path_context` | `picoquic/quicctx.c:2721-2727` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_remove_stashed_cnxid` | `picoquic/quicctx.c:3093-3100` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_dereference_stashed_cnxid` | `picoquic/quicctx.c:3149-3152` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_stream_from_node` | `picoquic/quicctx.c:3459-3466` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_compare_stream_priority` | `picoquic/quicctx.c:3486-3500` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_mark_direct_receive_stream` | `picoquic/quicctx.c:3703-3759` | rs/fq/src/lib.rs:2815-2822 `mark_direct_receive_stream` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_create_local_cnxid` | `picoquic/quicctx.c:3795-3866` | rs/fq/src/lib.rs:1263-1452 `new` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_retire_local_cnxid` | `picoquic/quicctx.c:3965-3985` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_find_local_cnxid` | `picoquic/quicctx.c:4017-4034` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_create_cnx_internal` | `picoquic/quicctx.c:4039-4348` | rs/fq/src/internal.rs:2525-3251 `create_cnx_internal` | `required_missing` | `rs/fq/src/internal.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_get_local_cnxid` | `picoquic/quicctx.c:4459-4463` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_get_cnx_state` | `picoquic/quicctx.c:4501-4505` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_cnx_get_padding_policy` | `picoquic/quicctx.c:4526-4531` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_use_unique_log_names` | `picoquic/quicctx.c:4646-4650` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_enable_sslkeylog` | `picoquic/quicctx.c:4652-4657` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_set_alpn_select_fn_v2` | `picoquic/quicctx.c:4737-4748` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_get_default_callback_function` | `picoquic/quicctx.c:4773-4777` | rs/fq/src/lib.rs:2592-2634 `default_callback` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_get_default_callback_context` | `picoquic/quicctx.c:4779-4783` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_get_callback_function` | `picoquic/quicctx.c:4785-4789` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_get_callback_context` | `picoquic/quicctx.c:4791-4795` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_reset_cnx` | `picoquic/quicctx.c:4939-4989` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_get_close_reasons` | `picoquic/quicctx.c:5181-5189` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_cnx_by_id` | `picoquic/quicctx.c:5216-5240` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_cnx_by_net` | `picoquic/quicctx.c:5242-5256` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_cnx_by_icid` | `picoquic/quicctx.c:5258-5275` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_cnx_by_secret` | `picoquic/quicctx.c:5277-5292` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_get_rtt` | `picoquic/quicctx.c:5438-5442` | rs/fq/src/lib.rs:3429-3456 `rtt` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_set_verify_certificate_callback` | `picoquic/quicctx.c:5486-5492` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_get_application_error` | `picoquic/quicctx.c:5514-5518` | rs/fq/src/lib.rs:3266-3274 `application_error` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_get_remote_stream_error` | `picoquic/quicctx.c:5520-5529` | rs/fq/src/lib.rs:3271-3280 `remote_stream_error` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_uniform_random` | `picoquic/quicctx.c:5600-5604` | - | `required_missing` | `rs/fq/src/lib.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_sack_first_item` | `picoquic/sacks.c:68-72` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_sack_last_item` | `picoquic/sacks.c:74-77` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_sack_insert_item` | `picoquic/sacks.c:89-108` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_sack_list_is_empty` | `picoquic/sacks.c:121-126` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_sack_find_range_below_number` | `picoquic/sacks.c:156-168` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_update_sack_list` | `picoquic/sacks.c:197-256` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_sack_select_ack_ranges` | `picoquic/sacks.c:301-321` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_check_sack_list` | `picoquic/sacks.c:323-340` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_sack_list_first` | `picoquic/sacks.c:407-412` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_sack_list_last` | `picoquic/sacks.c:414-420` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_sack_list_first_range` | `picoquic/sacks.c:422-428` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_sack_list_reset` | `picoquic/sacks.c:439-447` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_sack_item_record_sent` | `picoquic/sacks.c:476-485` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_sack_item_record_reset` | `picoquic/sacks.c:487-496` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_sack_list_size` | `picoquic/sacks.c:498-501` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_set_stream_not_coalesced` | `picoquic/sender.c:180-192` | rs/fq/src/lib.rs:2886-2893 `set_stream_not_coalesced` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_set_stream_priority` | `picoquic/sender.c:213-226` | rs/fq/src/lib.rs:2896-2903 `set_stream_priority` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_mark_high_priority_stream` | `picoquic/sender.c:228-243` | rs/fq/src/lib.rs:2907-2914 `mark_high_priority_stream` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_reset_stream_at` | `picoquic/sender.c:379-407` | rs/fq/src/lib.rs:3081-3089 `reset_stream_at` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_reset_stream` | `picoquic/sender.c:408-412` | rs/fq/src/lib.rs:3074-3077 `reset_stream` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_stop_sending` | `picoquic/sender.c:430-458` | rs/fq/src/lib.rs:3152-3155 `stop_sending` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_discard_stream` | `picoquic/sender.c:460-490` | rs/fq/src/lib.rs:3159-3166 `discard_stream` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_create_long_packet_type` | `picoquic/sender.c:586-639` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_insert_hole_in_send_sequence_if_needed` | `picoquic/sender.c:1133-1169` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_is_pkt_ctx_backlog_empty` | `picoquic/sender.c:1274-1308` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_preemptive_retransmit_packet` | `picoquic/sender.c:1332-1419` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_preemptive_retransmit_in_context` | `picoquic/sender.c:1421-1492` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_preemptive_retransmit_as_needed` | `picoquic/sender.c:1494-1543` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_next_mtu_probe_length` | `picoquic/sender.c:1545-1585` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_is_mtu_probe_needed` | `picoquic/sender.c:1587-1628` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_prepare_mtu_probe` | `picoquic/sender.c:1630-1647` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_prepare_packet_0rtt` | `picoquic/sender.c:1649-1743` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_packet_type_from_epoch` | `picoquic/sender.c:1745-1769` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_prepare_packet_old_context` | `picoquic/sender.c:1771-1833` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_prepare_server_address_migration` | `picoquic/sender.c:1868-1930` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_prepare_packet_client_init` | `picoquic/sender.c:1932-2213` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_prepare_packet_server_init` | `picoquic/sender.c:2215-2349` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_prepare_packet_closing` | `picoquic/sender.c:2351-2594` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_format_new_local_id_as_needed` | `picoquic/sender.c:2596-2658` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_prepare_datagram_ready` | `picoquic/sender.c:2777-2801` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_prepare_stream_and_datagrams` | `picoquic/sender.c:2848-2962` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_prepare_packet_almost_ready` | `picoquic/sender.c:2964-3284` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_prepare_packet_ready` | `picoquic/sender.c:3286-3717` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_check_idle_timer` | `picoquic/sender.c:3719-3759` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_prepare_segment` | `picoquic/sender.c:3761-3829` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_set_path_addresses_from_tuple` | `picoquic/sender.c:3832-3846` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_check_cc_feedback_timer` | `picoquic/sender.c:3848-3879` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_handle_app_wake_time` | `picoquic/sender.c:3881-3892` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_program_app_wake_time` | `picoquic/sender.c:3894-3903` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_handle_send_timers` | `picoquic/sender.c:3905-3932` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_handle_send_paths` | `picoquic/sender.c:3934-3957` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_handle_send_train_statistics` | `picoquic/sender.c:3959-3979` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_prepare_packet_ex` | `picoquic/sender.c:3981-4181` | rs/fq/src/lib.rs:2698-2716 `prepare_packet_ex` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_prepare_packet` | `picoquic/sender.c:4183-4194` | rs/fq/src/lib.rs:2709-2729 `prepare_packet` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_close_ex` | `picoquic/sender.c:4201-4222` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_prepare_next_packet_ex` | `picoquic/sender.c:4244-4324` | rs/fq/src/lib.rs:2662-2680 `prepare_next_packet_ex` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquictest_sim_link_testloss` | `picoquic/sim_link.c:144-158` | - | `required_missing` | `rs/fq/src/tests/harness.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquictest_sim_link_simloss` | `picoquic/sim_link.c:160-184` | - | `required_missing` | `rs/fq/src/tests/harness.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquictest_sim_link_wifi_jitter` | `picoquic/sim_link.c:201-228` | - | `required_missing` | `rs/fq/src/tests/harness.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquictest_sim_link_jitter` | `picoquic/sim_link.c:230-247` | - | `required_missing` | `rs/fq/src/tests/harness.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_packet_loop_open_socket` | `picoquic/sockloop.c:363-462` | - | `required_missing` | `rs/fq/src/packet_loop.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_packet_loop_set_fds` | `picoquic/sockloop.c:884-905` | - | `required_missing` | `rs/fq/src/packet_loop.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_packet_loop_poll` | `picoquic/sockloop.c:907-992` | - | `required_missing` | `rs/fq/src/packet_loop.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_close_network_wake_up` | `picoquic/sockloop.c:1691-1703` | - | `required_missing` | `rs/fq/src/packet_loop.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_open_network_wake_up` | `picoquic/sockloop.c:1705-1725` | - | `required_missing` | `rs/fq/src/packet_loop.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_spinbit_basic_incoming` | `picoquic/spinbit.c:29-35` | - | `required_missing` | `rs/fq/src/spinbit.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_spinbit_basic_outgoing` | `picoquic/spinbit.c:37-42` | - | `required_missing` | `rs/fq/src/spinbit.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_spinbit_null_incoming` | `picoquic/spinbit.c:48-53` | - | `required_missing` | `rs/fq/src/spinbit.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_spinbit_null_outgoing` | `picoquic/spinbit.c:55-59` | - | `required_missing` | `rs/fq/src/spinbit.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_spinbit_random_incoming` | `picoquic/spinbit.c:65-70` | - | `required_missing` | `rs/fq/src/spinbit.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_spinbit_random_outgoing` | `picoquic/spinbit.c:72-76` | - | `required_missing` | `rs/fq/src/spinbit.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_format_ticket` | `picoquic/ticket_store.c:29-99` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_update_stored_ticket` | `picoquic/ticket_store.c:529-571` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_validate_bdp_seed` | `picoquic/timing.c:90-114` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_update_path_rtt_one_way` | `picoquic/timing.c:122-172` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_tls_api_init` | `picoquic/tls_api.c:229-236` | rs/fq/src/lib.rs:3526-3528 `tls_api_init` | `required_missing` | `rs/fq/src/lib.rs` | mapped Rust function still contains incomplete markers |

| `picoquic_get_cipher_suite_by_id` | `picoquic/tls_api.c:435-449` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_get_ecb_cipher_by_id` | `picoquic/tls_api.c:451-469` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_ecb_create_by_name` | `picoquic/tls_api.c:478-488` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_get_hash_algorithm_by_name` | `picoquic/tls_api.c:493-508` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_get_aes128gcm_sha256` | `picoquic/tls_api.c:709-713` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_get_aes128gcm_v` | `picoquic/tls_api.c:720-729` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_get_cipher_suite_by_id_v` | `picoquic/tls_api.c:731-734` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_hash_update` | `picoquic/tls_api.c:736-738` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_hash_finalize` | `picoquic/tls_api.c:740-742` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_crypto_random` | `picoquic/tls_api.c:771-776` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_crypto_uniform_random` | `picoquic/tls_api.c:778-788` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_public_random_seed` | `picoquic/tls_api.c:859-866` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_public_random` | `picoquic/tls_api.c:868-880` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_public_uniform_random` | `picoquic/tls_api.c:882-892` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_set_aead_from_secret` | `picoquic/tls_api.c:1296-1309` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_set_pn_enc_from_secret` | `picoquic/tls_api.c:1311-1330` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_set_key_from_secret` | `picoquic/tls_api.c:1342-1361` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_compute_initial_secrets` | `picoquic/tls_api.c:1478-1498` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_tls_get_negotiated_alpn` | `picoquic/tls_api.c:2135-2142` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_tls_get_sni` | `picoquic/tls_api.c:2144-2151` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_add_to_tls_stream` | `picoquic/tls_api.c:2165-2204` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_add_proposed_alpn` | `picoquic/tls_api.c:2206-2223` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_pn_iv_size` | `picoquic/tls_api.c:2369-2372` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_pn_encrypt` | `picoquic/tls_api.c:2374-2378` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_aead_get_checksum_length` | `picoquic/tls_api.c:2393-2401` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_server_setup_ticket_aead_contexts` | `picoquic/tls_api.c:2414-2442` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_aead_integrity_limit` | `picoquic/tls_api.c:2444-2448` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_aead_confidentiality_limit` | `picoquic/tls_api.c:2450-2454` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_aead_decrypt_generic` | `picoquic/tls_api.c:2456-2471` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_aead_encrypt_generic` | `picoquic/tls_api.c:2473-2483` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_setup_cleartext_aead_salt` | `picoquic/tls_api.c:2532-2541` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_server_encrypt_retry_token` | `picoquic/tls_api.c:2849-2886` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_format_retry_protection_pseudo_packet` | `picoquic/tls_api.c:3173-3186` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_format_token` | `picoquic/token_store.c:29-59` | - | `required_missing` | `rs/fq/src/tls_api.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_transport_param_varint_decode` | `picoquic/transport.c:31-41` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_transport_param_varint_encode` | `picoquic/transport.c:43-57` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_transport_param_type_varint_encode` | `picoquic/transport.c:59-66` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_transport_param_type_flag_encode` | `picoquic/transport.c:68-75` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_transport_param_cid_encode` | `picoquic/transport.c:77-85` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_transport_param_cid_decode` | `picoquic/transport.c:87-96` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_encode_transport_preferred_address_address` | `picoquic/transport.c:98-128` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_decode_transport_preferred_address_address` | `picoquic/transport.c:130-162` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_encode_transport_param_version_negotiation` | `picoquic/transport.c:173-224` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_negotiate_multipath_option` | `picoquic/transport.c:283-304` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_clear_transport_extensions` | `picoquic/transport.c:503-532` | - | `required_missing` | `rs/fq/src/internal.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_log_app_message_v` | `picoquic/unified_log.c:59-73` | - | `required_missing` | `rs/fq/src/textlog.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_string_create` | `picoquic/util.c:45-71` | - | `required_missing` | `rs/fq/src/utils.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `get_debug_out` | `picoquic/util.c:111-114` | - | `required_missing` | `rs/fq/src/utils.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `get_debug_suspended` | `picoquic/util.c:116-119` | - | `required_missing` | `rs/fq/src/utils.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_parse_hexa_digit` | `picoquic/util.c:270-284` | - | `required_missing` | `rs/fq/src/utils.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_compare_connection_id` | `picoquic/util.c:353-365` | - | `required_missing` | `rs/fq/src/utils.rs` | no Rust counterpart found by C doc reference or conservative name matching |

| `picoquic_set_abs_delay` | `picoquic/util.c:1115-1124` | - | `required_missing` | `rs/fq/src/utils.rs` | no Rust counterpart found by C doc reference or conservative name matching |


## Blocked Classifications

| C function | C span | Rust span | Action | Proposed destination | Reason |
| --- | --- | --- | --- | --- | --- |
| _none_ | | | | | |


## Expected Omissions

| C function | C span | Rust span | Action | Proposed destination | Reason |
| --- | --- | --- | --- | --- | --- |
| `picoquic_bbr_delete` | `picoquic/bbr.c:616-623` | - | `expected_omission` | `rs/fq/src/bbr.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_bbr1_delete` | `picoquic/bbr1.c:481-488` | - | `expected_omission` | `rs/fq/src/bbr1.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `c4_delete` | `picoquic/c4.c:1111-1118` | - | `expected_omission` | `rs/fq/src/c4.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `cubic_delete` | `picoquic/cubic.c:551-558` | - | `expected_omission` | `rs/fq/src/cubic.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `ech_dispose_opener_callback` | `picoquic/ech.c:269-278` | - | `expected_omission` | `rs/fq/src/ech.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_release_quic_ech_ctx` | `picoquic/ech.c:375-389` | - | `expected_omission` | `rs/fq/src/ech.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_fastcc_delete` | `picoquic/fastcc.c:308-315` | - | `expected_omission` | `rs/fq/src/fastcc.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_is_stream_acked` | `picoquic/frames.c:117-132` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame/stream dispatch helper likely folded into Rust connection or frame processing |

| `picoquic_flow_control_check_stream_offset` | `picoquic/frames.c:204-228` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame/stream dispatch helper likely folded into Rust connection or frame processing |

| `picoquic_skip_reset_stream_frame` | `picoquic/frames.c:236-250` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_reset_stream_frame` | `picoquic/frames.c:373-394` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_process_ack_of_reset_stream_frame` | `picoquic/frames.c:396-425` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_check_reset_stream_needs_repeat` | `picoquic/frames.c:427-449` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_reset_stream_at_frame` | `picoquic/frames.c:459-478` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_reset_stream_at_frame` | `picoquic/frames.c:507-526` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_process_ack_of_reset_stream_at_frame` | `picoquic/frames.c:528-558` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_check_reset_stream_at_needs_repeat` | `picoquic/frames.c:560-587` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_new_connection_id_frame` | `picoquic/frames.c:622-636` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_parse_new_connection_id_frame` | `picoquic/frames.c:638-659` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_new_connection_id_frame` | `picoquic/frames.c:661-727` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_process_ack_of_new_cid_frame` | `picoquic/frames.c:729-771` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_check_new_cid_needs_repeat` | `picoquic/frames.c:773-810` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_retire_connection_id_frame` | `picoquic/frames.c:864-877` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_parse_retire_connection_id_frame` | `picoquic/frames.c:879-898` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_retire_connection_id_frame` | `picoquic/frames.c:900-932` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_check_retire_connection_id_needs_repeat` | `picoquic/frames.c:938-970` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_process_ack_of_retire_connection_id_frame` | `picoquic/frames.c:972-1007` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_new_token_frame` | `picoquic/frames.c:1045-1048` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_new_token_frame` | `picoquic/frames.c:1050-1081` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_stop_sending_frame` | `picoquic/frames.c:1115-1155` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_stop_sending_frame` | `picoquic/frames.c:1157-1163` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_check_stop_sending_needs_repeat` | `picoquic/frames.c:1166-1192` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_stream_data_chunk_callback` | `picoquic/frames.c:1262-1289` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame/stream dispatch helper likely folded into Rust connection or frame processing |

| `picoquic_stream_data_callback` | `picoquic/frames.c:1291-1306` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame/stream dispatch helper likely folded into Rust connection or frame processing |

| `add_chunk_node` | `picoquic/frames.c:1308-1343` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame/stream dispatch helper likely folded into Rust connection or frame processing |

| `picoquic_stream_network_input` | `picoquic/frames.c:1407-1517` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame/stream dispatch helper likely folded into Rust connection or frame processing |

| `picoquic_is_last_stream_frame` | `picoquic/frames.c:1519-1525` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame/stream dispatch helper likely folded into Rust connection or frame processing |

| `picoquic_queue_data_repeat_delete` | `picoquic/frames.c:2174-2186` | - | `expected_omission` | `rs/fq/src/internal.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_parse_crypto_hs_frame` | `picoquic/frames.c:2526-2538` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_crypto_stream_from_ptype` | `picoquic/frames.c:2576-2598` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame/stream dispatch helper likely folded into Rust connection or frame processing |

| `picoquic_process_ack_of_crypto_frame` | `picoquic/frames.c:2600-2624` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_check_crypto_frame_needs_repeat` | `picoquic/frames.c:2626-2652` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_check_spurious_retransmission` | `picoquic/frames.c:2757-2840` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_dequeue_old_retransmitted_packets` | `picoquic/frames.c:2842-2863` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame/stream dispatch helper likely folded into Rust connection or frame processing |

| `process_decoded_packet_data` | `picoquic/frames.c:3174-3230` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame/stream dispatch helper likely folded into Rust connection or frame processing |

| `picoquic_find_acked_packet` | `picoquic/frames.c:3232-3251` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame/stream dispatch helper likely folded into Rust connection or frame processing |

| `picoquic_process_ack_of_ack_body` | `picoquic/frames.c:3253-3350` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_process_ack_of_path_ack_frame` | `picoquic/frames.c:3372-3410` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_process_ack_range` | `picoquic/frames.c:3852-3918` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame/stream dispatch helper likely folded into Rust connection or frame processing |

| `picoquic_decode_ack_frame` | `picoquic/frames.c:3920-4071` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_connection_close_frame` | `picoquic/frames.c:4395-4424` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_application_close_frame` | `picoquic/frames.c:4447-4473` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_max_data_frame` | `picoquic/frames.c:4503-4515` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_process_ack_of_max_data_frame` | `picoquic/frames.c:4517-4539` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_max_stream_data_frame` | `picoquic/frames.c:4568-4600` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_process_ack_of_max_stream_data_frame` | `picoquic/frames.c:4602-4629` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_max_streams_frame` | `picoquic/frames.c:4724-4761` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_process_ack_of_max_streams_frame` | `picoquic/frames.c:4763-4791` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_check_max_streams_frame_needs_repeat` | `picoquic/frames.c:4793-4821` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_path_challenge_frame` | `picoquic/frames.c:4902-4987` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_path_response_frame` | `picoquic/frames.c:5004-5063` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_blocked_frame` | `picoquic/frames.c:5106-5113` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_stream_blocked_frame` | `picoquic/frames.c:5116-5131` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_streams_blocked_frame` | `picoquic/frames.c:5134-5150` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_0len_frame` | `picoquic/frames.c:5153-5160` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_handshake_done_frame` | `picoquic/frames.c:5162-5189` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_datagram_frame` | `picoquic/frames.c:5204-5227` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_datagram_frame` | `picoquic/frames.c:5247-5286` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_ack_frequency_frame` | `picoquic/frames.c:5526-5537` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_ack_frequency_frame` | `picoquic/frames.c:5552-5592` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_immediate_ack_frame` | `picoquic/frames.c:5642-5648` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_immediate_ack_frame` | `picoquic/frames.c:5650-5667` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_time_stamp_frame` | `picoquic/frames.c:5680-5687` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_parse_time_stamp_frame` | `picoquic/frames.c:5689-5694` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_time_stamp_frame` | `picoquic/frames.c:5696-5717` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_parse_path_abandon_frame` | `picoquic/frames.c:5752-5759` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_path_abandon_frame` | `picoquic/frames.c:5761-5828` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_parse_path_available_or_backup_frame` | `picoquic/frames.c:5921-5928` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_path_available_or_backup_frame` | `picoquic/frames.c:5930-5970` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_path_available_or_backup_frame_need_repeat` | `picoquic/frames.c:5972-5997` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_max_path_id_frame` | `picoquic/frames.c:6028-6033` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_parse_max_path_id_frame` | `picoquic/frames.c:6035-6040` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_max_path_id_frame` | `picoquic/frames.c:6042-6066` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_max_path_id_frame_needs_repeat` | `picoquic/frames.c:6068-6088` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_process_ack_of_max_path_id_frame` | `picoquic/frames.c:6091-6112` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_paths_blocked_frame` | `picoquic/frames.c:6144-6149` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_parse_paths_blocked_frame` | `picoquic/frames.c:6151-6156` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_paths_blocked_frame` | `picoquic/frames.c:6158-6176` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_paths_blocked_frame_needs_repeat` | `picoquic/frames.c:6178-6198` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_process_ack_of_paths_blocked_frame` | `picoquic/frames.c:6201-6222` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_path_cid_blocked_frame` | `picoquic/frames.c:6278-6285` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_parse_path_cid_blocked_frame` | `picoquic/frames.c:6287-6294` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_path_cid_blocked_frame` | `picoquic/frames.c:6296-6315` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_path_cid_blocked_frame_needs_repeat` | `picoquic/frames.c:6317-6353` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_process_ack_of_path_cid_blocked_frame` | `picoquic/frames.c:6355-6380` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_observed_address_frame` | `picoquic/frames.c:6467-6477` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_observed_address_frame` | `picoquic/frames.c:6494-6535` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_process_ack_of_observed_address_frame` | `picoquic/frames.c:6537-6553` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_bdp_frame` | `picoquic/frames.c:6559-6568` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_parse_bdp_frame` | `picoquic/frames.c:6570-6587` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_decode_bdp_frame` | `picoquic/frames.c:6589-6635` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_stream_frame` | `picoquic/frames.c:6992-7008` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_crypto_hs_frame` | `picoquic/frames.c:7014-7020` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_connection_close_frame` | `picoquic/frames.c:7022-7033` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_application_close_frame` | `picoquic/frames.c:7035-7044` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_ack_frame_maybe_ecn` | `picoquic/frames.c:7047-7082` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_ack_frame` | `picoquic/frames.c:7084-7086` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_ack_ecn_frame` | `picoquic/frames.c:7088-7090` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_max_stream_data_frame` | `picoquic/frames.c:7095-7101` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `picoquic_skip_stream_blocked_frame` | `picoquic/frames.c:7103-7109` | - | `expected_omission` | `rs/fq/src/internal.rs` | C frame-specific helper likely folded into Rust frame dispatch; verify before adding a direct item |

| `textlog_time` | `picoquic/logger.c:41-50` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_address` | `picoquic/logger.c:59-88` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_packet_address` | `picoquic/logger.c:90-117` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_ptype_name` | `picoquic/logger.c:193-224` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_frame_names` | `picoquic/logger.c:226-363` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_connection_id` | `picoquic/logger.c:365-372` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_packet_header` | `picoquic/logger.c:374-435` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_negotiation_packet` | `picoquic/logger.c:437-453` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_retry_packet` | `picoquic/logger.c:455-491` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_stream_frame` | `picoquic/logger.c:493-521` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_ack_frame` | `picoquic/logger.c:523-663` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_reset_stream_frame` | `picoquic/logger.c:665-697` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_reset_stream_at_frame` | `picoquic/logger.c:699-738` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_stop_sending_frame` | `picoquic/logger.c:740-765` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_reason_text` | `picoquic/logger.c:767-786` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_generic_close_frame` | `picoquic/logger.c:788-848` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_max_data_frame` | `picoquic/logger.c:855-873` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_max_stream_data_frame` | `picoquic/logger.c:875-897` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_max_stream_id_frame` | `picoquic/logger.c:899-917` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_blocked_frame` | `picoquic/logger.c:919-940` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_stream_blocked_frame` | `picoquic/logger.c:942-962` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_streams_blocked_frame` | `picoquic/logger.c:964-981` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_new_connection_id_frame` | `picoquic/logger.c:983-1049` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_retire_connection_id_frame` | `picoquic/logger.c:1051-1085` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_new_token_frame` | `picoquic/logger.c:1087-1118` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_path_frame` | `picoquic/logger.c:1120-1143` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_crypto_hs_frame` | `picoquic/logger.c:1145-1181` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_datagram_frame` | `picoquic/logger.c:1183-1225` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_ack_frequency_frame` | `picoquic/logger.c:1227-1261` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_immediate_ack_frame` | `picoquic/logger.c:1263-1276` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_time_stamp_frame` | `picoquic/logger.c:1277-1307` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_path_abandon_frame` | `picoquic/logger.c:1309-1342` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_path_available_or_backup_frame` | `picoquic/logger.c:1344-1376` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_max_path_id_frame` | `picoquic/logger.c:1378-1406` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_paths_blocked_frame` | `picoquic/logger.c:1408-1437` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_path_cid_blocked_frame` | `picoquic/logger.c:1439-1470` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_bdp_frame` | `picoquic/logger.c:1473-1516` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_observed_address_frame` | `picoquic/logger.c:1518-1551` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `picoquic_textlog_frames` | `picoquic/logger.c:1553-1740` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_decrypted_segment` | `picoquic/logger.c:1742-1798` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_outgoing_segment` | `picoquic/logger.c:1800-1850` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `picoquic_textlog_transport_extension` | `picoquic/logger.c:1949-1956` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `picoquic_textlog_negotiated_alpn` | `picoquic/logger.c:1958-1985` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_congestion_state` | `picoquic/logger.c:1987-2000` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `picoquic_textlog_tls_ticket` | `picoquic/logger.c:2016-2110` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `picoquic_textlog_picotls_ticket` | `picoquic/logger.c:2124-2189` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `picoquic_txtlog_message_v` | `picoquic/logger.c:2195-2207` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `txtlog_context_free_app_message` | `picoquic/logger.c:2209-2214` | - | `expected_omission` | `rs/fq/src/logger.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `textlog_quic_pdu` | `picoquic/logger.c:2223-2234` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_pdu_ex` | `picoquic/logger.c:2236-2250` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_packet` | `picoquic/logger.c:2252-2259` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `textlog_tls_ticket` | `picoquic/logger.c:2355-2361` | - | `expected_omission` | `rs/fq/src/logger.rs` | C textlog detail helper folded into Rust logger/textlog traits and methods |

| `picoquic_log_reset_stream_frame` | `picoquic/logwriter.c:173-184` | - | `expected_omission` | `rs/fq/src/binlog.rs` | C binlog frame helper folded into Rust binlog_frames dispatch or generic frame helper |

| `picoquic_log_reset_stream_at_frame` | `picoquic/logwriter.c:186-198` | - | `expected_omission` | `rs/fq/src/binlog.rs` | C binlog frame helper folded into Rust binlog_frames dispatch or generic frame helper |

| `picoquic_log_stop_sending_frame` | `picoquic/logwriter.c:200-210` | - | `expected_omission` | `rs/fq/src/binlog.rs` | C binlog frame helper folded into Rust binlog_frames dispatch or generic frame helper |

| `picoquic_log_max_data_frame` | `picoquic/logwriter.c:241-250` | - | `expected_omission` | `rs/fq/src/binlog.rs` | C binlog frame helper folded into Rust binlog_frames dispatch or generic frame helper |

| `picoquic_log_max_stream_data_frame` | `picoquic/logwriter.c:252-262` | - | `expected_omission` | `rs/fq/src/binlog.rs` | C binlog frame helper folded into Rust binlog_frames dispatch or generic frame helper |

| `picoquic_log_max_stream_id_frame` | `picoquic/logwriter.c:264-273` | - | `expected_omission` | `rs/fq/src/binlog.rs` | C binlog frame helper folded into Rust binlog_frames dispatch or generic frame helper |

| `picoquic_log_blocked_frame` | `picoquic/logwriter.c:275-284` | - | `expected_omission` | `rs/fq/src/binlog.rs` | C binlog frame helper folded into Rust binlog_frames dispatch or generic frame helper |

| `picoquic_log_stream_blocked_frame` | `picoquic/logwriter.c:286-296` | - | `expected_omission` | `rs/fq/src/binlog.rs` | C binlog frame helper folded into Rust binlog_frames dispatch or generic frame helper |

| `picoquic_log_streams_blocked_frame` | `picoquic/logwriter.c:298-307` | - | `expected_omission` | `rs/fq/src/binlog.rs` | C binlog frame helper folded into Rust binlog_frames dispatch or generic frame helper |

| `picoquic_log_retire_connection_id_frame` | `picoquic/logwriter.c:344-353` | - | `expected_omission` | `rs/fq/src/binlog.rs` | C binlog frame helper folded into Rust binlog_frames dispatch or generic frame helper |

| `picoquic_log_path_retire_connection_id_frame` | `picoquic/logwriter.c:355-365` | - | `expected_omission` | `rs/fq/src/binlog.rs` | C binlog frame helper folded into Rust binlog_frames dispatch or generic frame helper |

| `picoquic_newreno_delete` | `picoquic/newreno.c:298-305` | - | `expected_omission` | `rs/fq/src/newreno.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_perflog_item_free` | `picoquic/performance_log.c:69-75` | - | `expected_omission` | `rs/fq/src/performance_log.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_perflog_free` | `picoquic/performance_log.c:212-223` | - | `expected_omission` | `rs/fq/src/performance_log.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picohash_delete_item` | `picoquic/picohash.c:111-141` | - | `expected_omission` | `rs/fq/src/hash.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picohash_delete_key` | `picoquic/picohash.c:143-153` | - | `expected_omission` | `rs/fq/src/hash.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_openssl_dispose_sign_certificate` | `picoquic/picoquic_ptls_openssl.c:202-207` | - | `expected_omission` | `rs/fq/src/sys/openssl.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_openssl_dispose_certificate_verifier` | `picoquic/picoquic_ptls_openssl.c:269-275` | - | `expected_omission` | `rs/fq/src/sys/openssl.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `openssl_keyex_dispose` | `picoquic/picoquic_ptls_openssl.c:367-371` | - | `expected_omission` | `rs/fq/src/sys/openssl.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_prague_delete` | `picoquic/prague.c:396-403` | - | `expected_omission` | `rs/fq/src/prague.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_local_cnxid_hash` | `picoquic/quicctx.c:268-273` | - | `expected_omission` | `rs/fq/src/lib.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_local_cnxid_compare` | `picoquic/quicctx.c:275-281` | - | `expected_omission` | `rs/fq/src/lib.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_local_cnxid_to_item` | `picoquic/quicctx.c:283-288` | - | `expected_omission` | `rs/fq/src/lib.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_net_id_hash` | `picoquic/quicctx.c:290-296` | - | `expected_omission` | `rs/fq/src/lib.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_local_netid_to_item` | `picoquic/quicctx.c:298-303` | - | `expected_omission` | `rs/fq/src/lib.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_net_id_compare` | `picoquic/quicctx.c:306-312` | - | `expected_omission` | `rs/fq/src/lib.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_net_icid_hash` | `picoquic/quicctx.c:314-325` | - | `expected_omission` | `rs/fq/src/lib.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_net_icid_compare` | `picoquic/quicctx.c:327-337` | - | `expected_omission` | `rs/fq/src/lib.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_net_icid_to_item` | `picoquic/quicctx.c:339-344` | - | `expected_omission` | `rs/fq/src/lib.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_net_secret_hash` | `picoquic/quicctx.c:346-357` | - | `expected_omission` | `rs/fq/src/lib.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_net_secret_compare` | `picoquic/quicctx.c:359-373` | - | `expected_omission` | `rs/fq/src/lib.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_net_secret_to_item` | `picoquic/quicctx.c:375-380` | - | `expected_omission` | `rs/fq/src/lib.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_issued_ticket_hash` | `picoquic/quicctx.c:403-408` | - | `expected_omission` | `rs/fq/src/lib.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_issued_ticket_compare` | `picoquic/quicctx.c:410-417` | - | `expected_omission` | `rs/fq/src/lib.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_issued_ticket_key_to_item` | `picoquic/quicctx.c:419-424` | - | `expected_omission` | `rs/fq/src/lib.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_delete_issued_ticket` | `picoquic/quicctx.c:461-483` | - | `expected_omission` | `rs/fq/src/lib.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_registered_token_compare` | `picoquic/quicctx.c:528-549` | - | `expected_omission` | `rs/fq/src/lib.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_registered_token_value` | `picoquic/quicctx.c:557-560` | - | `expected_omission` | `rs/fq/src/lib.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_registered_token_delete` | `picoquic/quicctx.c:562-566` | - | `expected_omission` | `rs/fq/src/lib.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_delete_stateless_packet` | `picoquic/quicctx.c:1226-1229` | - | `expected_omission` | `rs/fq/src/lib.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_wake_list_node_value` | `picoquic/quicctx.c:1473-1476` | - | `expected_omission` | `rs/fq/src/lib.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_wake_list_compare` | `picoquic/quicctx.c:1478-1484` | - | `expected_omission` | `rs/fq/src/lib.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_wake_list_create_node` | `picoquic/quicctx.c:1486-1489` | - | `expected_omission` | `rs/fq/src/lib.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_wake_list_delete_node` | `picoquic/quicctx.c:1491-1497` | - | `expected_omission` | `rs/fq/src/lib.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_delete_remote_cnxid_stashes` | `picoquic/quicctx.c:3271-3276` | - | `expected_omission` | `rs/fq/src/lib.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_stream_data_node_compare` | `picoquic/quicctx.c:3339-3344` | - | `expected_omission` | `rs/fq/src/lib.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_stream_data_node_create` | `picoquic/quicctx.c:3346-3349` | - | `expected_omission` | `rs/fq/src/lib.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_stream_data_node_value` | `picoquic/quicctx.c:3352-3355` | - | `expected_omission` | `rs/fq/src/lib.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_stream_data_node_delete` | `picoquic/quicctx.c:3370-3375` | - | `expected_omission` | `rs/fq/src/lib.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_stream_node_compare` | `picoquic/quicctx.c:3410-3414` | - | `expected_omission` | `rs/fq/src/lib.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_stream_node_create` | `picoquic/quicctx.c:3416-3419` | - | `expected_omission` | `rs/fq/src/lib.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_stream_node_value` | `picoquic/quicctx.c:3422-3425` | - | `expected_omission` | `rs/fq/src/lib.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_stream_node_delete` | `picoquic/quicctx.c:3448-3455` | - | `expected_omission` | `rs/fq/src/lib.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_delete_local_cnxid_listed` | `picoquic/quicctx.c:3868-3925` | - | `expected_omission` | `rs/fq/src/lib.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_sack_node_value` | `picoquic/sacks.c:35-40` | - | `expected_omission` | `rs/fq/src/internal.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_sack_item_value` | `picoquic/sacks.c:42-45` | - | `expected_omission` | `rs/fq/src/internal.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_sack_item_compare` | `picoquic/sacks.c:47-53` | - | `expected_omission` | `rs/fq/src/internal.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_sack_node_create` | `picoquic/sacks.c:55-58` | - | `expected_omission` | `rs/fq/src/internal.rs` | C collection/index callback helper; Rust collection ownership and keying should cover this without a direct function |

| `picoquic_sack_node_delete` | `picoquic/sacks.c:60-66` | - | `expected_omission` | `rs/fq/src/internal.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_sack_delete_item` | `picoquic/sacks.c:109-119` | - | `expected_omission` | `rs/fq/src/internal.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_sack_list_free` | `picoquic/sacks.c:449-457` | - | `expected_omission` | `rs/fq/src/internal.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquictest_sim_link_delete` | `picoquic/sim_link.c:64-78` | - | `expected_omission` | `rs/fq/src/tests/harness.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_delete_network_thread` | `picoquic/sockloop.c:1851-1875` | - | `expected_omission` | `rs/fq/src/packet_loop.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_free_tickets` | `picoquic/ticket_store.c:500-509` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_tls_api_init_providers` | `picoquic/tls_api.c:143-181` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_tls_api_zero` | `picoquic/tls_api.c:183-202` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_tls_api_log_versions` | `picoquic/tls_api.c:204-227` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_tls_api_unload` | `picoquic/tls_api.c:238-245` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_register_ciphersuite` | `picoquic/tls_api.c:258-274` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_register_key_exchange_algorithm` | `picoquic/tls_api.c:276-292` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_register_hpke_cipher_suite` | `picoquic/tls_api.c:294-306` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_register_hpke_kem` | `picoquic/tls_api.c:308-319` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_register_tls_key_provider_fn` | `picoquic/tls_api.c:321-337` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_register_verify_certificate_fn` | `picoquic/tls_api.c:339-346` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_register_explain_crypto_error_fn` | `picoquic/tls_api.c:348-353` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_register_crypto_random_provider_fn` | `picoquic/tls_api.c:355-358` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_register_keyex_from_key_file_fn` | `picoquic/tls_api.c:360-365` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_set_cipher_suite_list` | `picoquic/tls_api.c:367-389` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_set_cipher_suite_in_ctx` | `picoquic/tls_api.c:391-425` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_set_key_exchange_in_ctx` | `picoquic/tls_api.c:549-571` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_set_random_provider_in_ctx` | `picoquic/tls_api.c:594-598` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_dispose_sign_certificate` | `picoquic/tls_api.c:616-629` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_get_certificate_verifier` | `picoquic/tls_api.c:643-654` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_dispose_certificate_verifier` | `picoquic/tls_api.c:656-664` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_explain_crypto_error` | `picoquic/tls_api.c:679-689` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_clear_crypto_errors` | `picoquic/tls_api.c:691-699` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_log_crypto_errors` | `picoquic/tls_api.c:750-762` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_tls_get_quic_extension_id` | `picoquic/tls_api.c:894-917` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_tls_collect_extensions_cb` | `picoquic/tls_api.c:925-936` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_tls_set_extensions` | `picoquic/tls_api.c:938-962` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_tls_collected_extensions_cb` | `picoquic/tls_api.c:969-996` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_client_hello_call_back` | `picoquic/tls_api.c:1006-1069` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_server_encrypt_ticket_call_back` | `picoquic/tls_api.c:1085-1198` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_client_save_ticket_call_back` | `picoquic/tls_api.c:1205-1235` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_get_simulated_time_cb` | `picoquic/tls_api.c:1237-1244` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_dispose_verify_certificate_callback` | `picoquic/tls_api.c:1259-1277` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_tls_set_verify_certificate_callback` | `picoquic/tls_api.c:1279-1289` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_aes128_ecb_free` | `picoquic/tls_api.c:1332-1335` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_update_traffic_key_callback` | `picoquic/tls_api.c:1390-1419` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_set_update_traffic_key_callback` | `picoquic/tls_api.c:1421-1431` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `free_certificates_list` | `picoquic/tls_api.c:1863-1872` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_log_event_call_back` | `picoquic/tls_api.c:2002-2019` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | C TLS/provider callback or registry hook folded into Rust TLS provider traits and state |

| `picoquic_free_log_event` | `picoquic/tls_api.c:2021-2037` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_tlscontext_free` | `picoquic/tls_api.c:2086-2115` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_aead_free` | `picoquic/tls_api.c:2382-2385` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_cipher_free` | `picoquic/tls_api.c:2387-2390` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_delete_one_retry_protection_context` | `picoquic/tls_api.c:3154-3165` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_free_tokens` | `picoquic/token_store.c:350-359` | - | `expected_omission` | `rs/fq/src/tls_api.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_string_free` | `picoquic/util.c:86-93` | - | `expected_omission` | `rs/fq/src/utils.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_file_open_ex` | `picoquic/util.c:726-750` | - | `expected_omission` | `rs/fq/src/utils.rs` | C portability wrapper folded into Rust standard library types or Drop |

| `picoquic_file_open` | `picoquic/util.c:751-754` | - | `expected_omission` | `rs/fq/src/utils.rs` | C portability wrapper folded into Rust standard library types or Drop |

| `picoquic_file_close` | `picoquic/util.c:756-763` | - | `expected_omission` | `rs/fq/src/utils.rs` | C portability wrapper folded into Rust standard library types or Drop |

| `picoquic_create_thread` | `picoquic/util.c:1127-1139` | - | `expected_omission` | `rs/fq/src/utils.rs` | C portability wrapper folded into Rust standard library types or Drop |

| `picoquic_wait_thread` | `picoquic/util.c:1141-1152` | - | `expected_omission` | `rs/fq/src/utils.rs` | C portability wrapper folded into Rust standard library types or Drop |

| `picoquic_delete_thread` | `picoquic/util.c:1154-1174` | - | `expected_omission` | `rs/fq/src/utils.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_delete_mutex` | `picoquic/util.c:1190-1200` | - | `expected_omission` | `rs/fq/src/utils.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_lock_mutex` | `picoquic/util.c:1202-1214` | - | `expected_omission` | `rs/fq/src/utils.rs` | C portability wrapper folded into Rust standard library types or Drop |

| `picoquic_unlock_mutex` | `picoquic/util.c:1216-1227` | - | `expected_omission` | `rs/fq/src/utils.rs` | C portability wrapper folded into Rust standard library types or Drop |

| `picoquic_create_event` | `picoquic/util.c:1229-1247` | - | `expected_omission` | `rs/fq/src/utils.rs` | C portability wrapper folded into Rust standard library types or Drop |

| `picoquic_delete_event` | `picoquic/util.c:1249-1259` | - | `expected_omission` | `rs/fq/src/utils.rs` | name suggests C allocation/lifetime cleanup; verify Rust ownership/Drop covers it |

| `picoquic_signal_event` | `picoquic/util.c:1261-1275` | - | `expected_omission` | `rs/fq/src/utils.rs` | C portability wrapper folded into Rust standard library types or Drop |

| `picoquic_wait_for_event` | `picoquic/util.c:1277-1303` | - | `expected_omission` | `rs/fq/src/utils.rs` | C portability wrapper folded into Rust standard library types or Drop |

