//! CBOR serialization profiles from
//! [`draft-ietf-cbor-serialization-09`](https://datatracker.ietf.org/doc/draft-ietf-cbor-serialization/).
//!
//! [`Profile`] selects the serialization an encoder targets. [`DecodeOptions`] additionally says
//! whether a decoder must *check* that input conforms to that serialization, rather than merely
//! accepting the profile's required floor.

/// A CBOR serialization profile.
///
/// The profiles are cumulative: [`Profile::Deterministic`] is preferred-plus plus ordered maps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Profile {
    /// General serialization: every well-formed CBOR form, with the wire form preserved.
    General,
    /// Preferred-plus serialization: shortest arguments, definite lengths, canonical NaN.
    PreferredPlus,
    /// Deterministic serialization: preferred-plus plus bytewise-sorted map keys.
    Deterministic,
}

impl Profile {
    /// Whether this profile imposes serialization requirements beyond general serialization.
    #[inline(always)]
    pub const fn is_strict(self) -> bool {
        !matches!(self, Self::General)
    }
}

/// Options for decoding a single CBOR item.
///
/// `checking` is independent of [`Profile`]: deterministic decoding without checking accepts
/// unordered maps, while checking decoding rejects every encoding a conforming encoder must not
/// produce. [`Profile::General`] ignores `checking`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodeOptions {
    /// The serialization profile to decode for.
    pub profile: Profile,
    /// Whether to reject well-formed input that does not conform to `profile`.
    pub checking: bool,
}

impl DecodeOptions {
    /// General, non-checking decoding: the behavior of the existing permissive entry points.
    pub const GENERAL: Self = Self {
        profile: Profile::General,
        checking: false,
    };

    /// Preferred-plus decoding floor: accepts non-preferred input.
    pub const PREFERRED_PLUS: Self = Self {
        profile: Profile::PreferredPlus,
        checking: false,
    };

    /// Preferred-plus checking: rejects every non-preferred encoding.
    pub const PREFERRED_PLUS_CHECKED: Self = Self {
        profile: Profile::PreferredPlus,
        checking: true,
    };

    /// Deterministic decoding floor: accepts unordered and otherwise non-preferred input.
    pub const DETERMINISTIC: Self = Self {
        profile: Profile::Deterministic,
        checking: false,
    };

    /// Deterministic checking: rejects unordered and otherwise non-preferred input.
    pub const DETERMINISTIC_CHECKED: Self = Self {
        profile: Profile::Deterministic,
        checking: true,
    };

    /// Whether this decoder must reject non-conforming input.
    ///
    /// [`Profile::General`] never checks, regardless of the `checking` field.
    #[inline(always)]
    pub const fn is_checking(self) -> bool {
        self.checking && self.profile.is_strict()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn general_ignores_checking() {
        let options = DecodeOptions {
            profile: Profile::General,
            checking: true,
        };
        assert!(!options.is_checking());
        assert!(!Profile::General.is_strict());
    }

    #[test]
    fn strict_profiles_only_check_when_requested() {
        assert!(!DecodeOptions::PREFERRED_PLUS.is_checking());
        assert!(DecodeOptions::PREFERRED_PLUS_CHECKED.is_checking());
        assert!(!DecodeOptions::DETERMINISTIC.is_checking());
        assert!(DecodeOptions::DETERMINISTIC_CHECKED.is_checking());
        assert!(Profile::PreferredPlus.is_strict());
        assert!(Profile::Deterministic.is_strict());
    }
}
