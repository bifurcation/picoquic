//! Congestion control algorithm registry.
//!
//! Translated from picoquic/register_all_cc_algorithms.c.
//!
//! This module provides a registry of available congestion control algorithms
//! that can be used with picoquic. Each algorithm is identified by an ID
//! and provides callbacks for initialization, notification, cleanup, and
//! observation.

// =============================================================================
// CC Algorithm Identifiers
// =============================================================================

/// Congestion control algorithm numbers.
///
/// These match the PICOQUIC_CC_ALGO_NUMBER_* constants in the C code.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CcAlgoNumber {
    /// NewReno algorithm.
    NewReno = 0,
    /// CUBIC algorithm.
    Cubic = 1,
    /// Delay-based CUBIC algorithm.
    DCubic = 2,
    /// FastCC algorithm.
    FastCc = 3,
    /// BBRv3 algorithm.
    Bbr = 4,
    /// Prague (L4S) algorithm.
    Prague = 5,
    /// BBRv1 algorithm.
    Bbr1 = 6,
    /// C4 experimental algorithm.
    C4 = 7,
}

impl CcAlgoNumber {
    /// Get the algorithm name string.
    pub fn name(&self) -> &'static str {
        match self {
            CcAlgoNumber::NewReno => "newreno",
            CcAlgoNumber::Cubic => "cubic",
            CcAlgoNumber::DCubic => "dcubic",
            CcAlgoNumber::FastCc => "fast",
            CcAlgoNumber::Bbr => "bbr",
            CcAlgoNumber::Prague => "prague",
            CcAlgoNumber::Bbr1 => "bbr1",
            CcAlgoNumber::C4 => "c4",
        }
    }

    /// Get algorithm from number.
    pub fn from_number(n: u32) -> Option<Self> {
        match n {
            0 => Some(CcAlgoNumber::NewReno),
            1 => Some(CcAlgoNumber::Cubic),
            2 => Some(CcAlgoNumber::DCubic),
            3 => Some(CcAlgoNumber::FastCc),
            4 => Some(CcAlgoNumber::Bbr),
            5 => Some(CcAlgoNumber::Prague),
            6 => Some(CcAlgoNumber::Bbr1),
            7 => Some(CcAlgoNumber::C4),
            _ => None,
        }
    }

    /// Get algorithm from name.
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "newreno" | "reno" => Some(CcAlgoNumber::NewReno),
            "cubic" => Some(CcAlgoNumber::Cubic),
            "dcubic" => Some(CcAlgoNumber::DCubic),
            "fast" | "fastcc" => Some(CcAlgoNumber::FastCc),
            "bbr" | "bbr3" | "bbrv3" => Some(CcAlgoNumber::Bbr),
            "prague" | "l4s" => Some(CcAlgoNumber::Prague),
            "bbr1" | "bbrv1" => Some(CcAlgoNumber::Bbr1),
            "c4" => Some(CcAlgoNumber::C4),
            _ => None,
        }
    }

    /// Get default ECN mode for algorithm.
    pub fn default_ecn_mode(&self) -> EcnMode {
        match self {
            CcAlgoNumber::Prague => EcnMode::Ect1,
            _ => EcnMode::Ect0,
        }
    }

    /// Get all available algorithms.
    pub fn all() -> &'static [CcAlgoNumber] {
        &[
            CcAlgoNumber::NewReno,
            CcAlgoNumber::Cubic,
            CcAlgoNumber::DCubic,
            CcAlgoNumber::FastCc,
            CcAlgoNumber::Bbr,
            CcAlgoNumber::Prague,
            CcAlgoNumber::Bbr1,
            CcAlgoNumber::C4,
        ]
    }
}

// =============================================================================
// ECN Mode
// =============================================================================

/// ECN marking mode used by an algorithm.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EcnMode {
    /// Not ECN-Capable.
    NotEct = 0,
    /// ECN-Capable Transport codepoint 1 (used by L4S).
    Ect1 = 1,
    /// ECN-Capable Transport codepoint 0 (default).
    Ect0 = 2,
}

// =============================================================================
// Algorithm Registry
// =============================================================================

/// Maximum number of registered algorithms.
pub const MAX_REGISTERED_ALGORITHMS: usize = 8;

/// Registry entry for a congestion control algorithm.
#[derive(Debug, Clone, Copy)]
pub struct CcAlgorithmEntry {
    /// Algorithm identifier.
    pub id: CcAlgoNumber,
    /// Algorithm name string.
    pub name: &'static str,
    /// Default ECN mode.
    pub ecn_mode: EcnMode,
}

/// Registry of congestion control algorithms.
#[derive(Debug, Clone)]
pub struct CcRegistry {
    /// Registered algorithms.
    algorithms: [Option<CcAlgorithmEntry>; MAX_REGISTERED_ALGORITHMS],
    /// Number of registered algorithms.
    count: usize,
}

impl Default for CcRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl CcRegistry {
    /// Create a new empty registry.
    pub const fn new() -> Self {
        Self {
            algorithms: [None; MAX_REGISTERED_ALGORITHMS],
            count: 0,
        }
    }

    /// Create a registry with all standard algorithms registered.
    pub fn with_all_algorithms() -> Self {
        let mut registry = Self::new();
        registry.register_all();
        registry
    }

    /// Register all standard congestion control algorithms.
    pub fn register_all(&mut self) {
        self.register(CcAlgoNumber::NewReno);
        self.register(CcAlgoNumber::Cubic);
        self.register(CcAlgoNumber::DCubic);
        self.register(CcAlgoNumber::FastCc);
        self.register(CcAlgoNumber::Bbr);
        self.register(CcAlgoNumber::Prague);
        self.register(CcAlgoNumber::Bbr1);
        self.register(CcAlgoNumber::C4);
    }

    /// Register a single algorithm.
    pub fn register(&mut self, algo: CcAlgoNumber) -> bool {
        if self.count >= MAX_REGISTERED_ALGORITHMS {
            return false;
        }

        // Check if already registered
        for entry in self.algorithms.iter().flatten() {
            if entry.id == algo {
                return true; // Already registered
            }
        }

        self.algorithms[self.count] = Some(CcAlgorithmEntry {
            id: algo,
            name: algo.name(),
            ecn_mode: algo.default_ecn_mode(),
        });
        self.count += 1;
        true
    }

    /// Get algorithm entry by number.
    pub fn get_by_number(&self, number: u32) -> Option<&CcAlgorithmEntry> {
        let algo = CcAlgoNumber::from_number(number)?;
        self.get(algo)
    }

    /// Get algorithm entry by name.
    pub fn get_by_name(&self, name: &str) -> Option<&CcAlgorithmEntry> {
        let algo = CcAlgoNumber::from_name(name)?;
        self.get(algo)
    }

    /// Get algorithm entry.
    pub fn get(&self, algo: CcAlgoNumber) -> Option<&CcAlgorithmEntry> {
        self.algorithms
            .iter()
            .flatten()
            .find(|entry| entry.id == algo)
    }

    /// Get number of registered algorithms.
    pub fn len(&self) -> usize {
        self.count
    }

    /// Check if registry is empty.
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Iterate over registered algorithms.
    pub fn iter(&self) -> impl Iterator<Item = &CcAlgorithmEntry> {
        self.algorithms.iter().take(self.count).flatten()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cc_algo_number_values() {
        assert_eq!(CcAlgoNumber::NewReno as u32, 0);
        assert_eq!(CcAlgoNumber::Cubic as u32, 1);
        assert_eq!(CcAlgoNumber::DCubic as u32, 2);
        assert_eq!(CcAlgoNumber::FastCc as u32, 3);
        assert_eq!(CcAlgoNumber::Bbr as u32, 4);
        assert_eq!(CcAlgoNumber::Prague as u32, 5);
        assert_eq!(CcAlgoNumber::Bbr1 as u32, 6);
        assert_eq!(CcAlgoNumber::C4 as u32, 7);
    }

    #[test]
    fn test_cc_algo_names() {
        assert_eq!(CcAlgoNumber::NewReno.name(), "newreno");
        assert_eq!(CcAlgoNumber::Cubic.name(), "cubic");
        assert_eq!(CcAlgoNumber::Bbr.name(), "bbr");
        assert_eq!(CcAlgoNumber::Prague.name(), "prague");
    }

    #[test]
    fn test_cc_algo_from_number() {
        assert_eq!(CcAlgoNumber::from_number(0), Some(CcAlgoNumber::NewReno));
        assert_eq!(CcAlgoNumber::from_number(4), Some(CcAlgoNumber::Bbr));
        assert_eq!(CcAlgoNumber::from_number(99), None);
    }

    #[test]
    fn test_cc_algo_from_name() {
        assert_eq!(
            CcAlgoNumber::from_name("newreno"),
            Some(CcAlgoNumber::NewReno)
        );
        assert_eq!(CcAlgoNumber::from_name("CUBIC"), Some(CcAlgoNumber::Cubic));
        assert_eq!(CcAlgoNumber::from_name("bbr3"), Some(CcAlgoNumber::Bbr));
        assert_eq!(CcAlgoNumber::from_name("l4s"), Some(CcAlgoNumber::Prague));
        assert_eq!(CcAlgoNumber::from_name("unknown"), None);
    }

    #[test]
    fn test_cc_algo_ecn_mode() {
        assert_eq!(CcAlgoNumber::NewReno.default_ecn_mode(), EcnMode::Ect0);
        assert_eq!(CcAlgoNumber::Prague.default_ecn_mode(), EcnMode::Ect1);
    }

    #[test]
    fn test_cc_algo_all() {
        let all = CcAlgoNumber::all();
        assert_eq!(all.len(), 8);
        assert!(all.contains(&CcAlgoNumber::NewReno));
        assert!(all.contains(&CcAlgoNumber::C4));
    }

    #[test]
    fn test_registry_new() {
        let registry = CcRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_registry_register() {
        let mut registry = CcRegistry::new();

        assert!(registry.register(CcAlgoNumber::NewReno));
        assert_eq!(registry.len(), 1);

        // Registering again should succeed (already registered)
        assert!(registry.register(CcAlgoNumber::NewReno));
        assert_eq!(registry.len(), 1);

        assert!(registry.register(CcAlgoNumber::Cubic));
        assert_eq!(registry.len(), 2);
    }

    #[test]
    fn test_registry_with_all() {
        let registry = CcRegistry::with_all_algorithms();
        assert_eq!(registry.len(), 8);
    }

    #[test]
    fn test_registry_get_by_number() {
        let registry = CcRegistry::with_all_algorithms();

        let entry = registry.get_by_number(0).unwrap();
        assert_eq!(entry.id, CcAlgoNumber::NewReno);
        assert_eq!(entry.name, "newreno");

        let entry = registry.get_by_number(5).unwrap();
        assert_eq!(entry.id, CcAlgoNumber::Prague);
        assert_eq!(entry.ecn_mode, EcnMode::Ect1);

        assert!(registry.get_by_number(99).is_none());
    }

    #[test]
    fn test_registry_get_by_name() {
        let registry = CcRegistry::with_all_algorithms();

        let entry = registry.get_by_name("cubic").unwrap();
        assert_eq!(entry.id, CcAlgoNumber::Cubic);

        let entry = registry.get_by_name("BBR").unwrap();
        assert_eq!(entry.id, CcAlgoNumber::Bbr);

        assert!(registry.get_by_name("unknown").is_none());
    }

    #[test]
    fn test_registry_iter() {
        let registry = CcRegistry::with_all_algorithms();

        let names: Vec<_> = registry.iter().map(|e| e.name).collect();
        assert_eq!(names.len(), 8);
        assert!(names.contains(&"newreno"));
        assert!(names.contains(&"c4"));
    }

    #[test]
    fn test_ecn_mode_values() {
        assert_eq!(EcnMode::NotEct as u8, 0);
        assert_eq!(EcnMode::Ect1 as u8, 1);
        assert_eq!(EcnMode::Ect0 as u8, 2);
    }
}
