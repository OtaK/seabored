//! Draft-09 Appendix K.2 serialization vectors.
//!
//! Covers K.1.1 encode, K.1.2 decode, K.1.3 checking, and K.1.4 non-checking
//! behavior, plus the `f16` NaN encoder rule and nested deterministic `Value`
//! sorting through a derived container.

use half::f16;
use seabored::{
    DecodeOptions, Profile, Value, de::from_bytes, de::from_bytes_with_options,
    error::SeaboredDeError, ser::CborSerialize as _,
};

struct Vector {
    name: &'static str,
    general: &'static [&'static [u8]],
    preferred_plus: &'static [&'static [u8]],
    deterministic: &'static [&'static [u8]],
    /// General forms that a preferred-plus checking decoder must reject.
    checking_rejects: &'static [&'static [u8]],
    /// General forms that a deterministic checking decoder must reject.
    /// This is a superset of `checking_rejects` and includes unsorted maps.
    deterministic_checking_rejects: &'static [&'static [u8]],
}

const VECTORS: &[Vector] = &[
    Vector {
        name: "zero",
        general: &[
            &hex_literal::hex!("00"),
            &hex_literal::hex!("1800"),
            &hex_literal::hex!("190000"),
            &hex_literal::hex!("1a00000000"),
            &hex_literal::hex!("1b0000000000000000"),
            &hex_literal::hex!("c2420000"),
            &hex_literal::hex!("c240"),
        ],
        preferred_plus: &[&hex_literal::hex!("00")],
        deterministic: &[&hex_literal::hex!("00")],
        checking_rejects: &[
            &hex_literal::hex!("1800"),
            &hex_literal::hex!("190000"),
            &hex_literal::hex!("1a00000000"),
            &hex_literal::hex!("1b0000000000000000"),
            &hex_literal::hex!("c2420000"),
            &hex_literal::hex!("c240"),
        ],
        deterministic_checking_rejects: &[
            &hex_literal::hex!("1800"),
            &hex_literal::hex!("190000"),
            &hex_literal::hex!("1a00000000"),
            &hex_literal::hex!("1b0000000000000000"),
            &hex_literal::hex!("c2420000"),
            &hex_literal::hex!("c240"),
        ],
    },
    Vector {
        name: "three",
        general: &[
            &hex_literal::hex!("03"),
            &hex_literal::hex!("1803"),
            &hex_literal::hex!("190003"),
            &hex_literal::hex!("1a00000003"),
            &hex_literal::hex!("1b0000000000000003"),
            &hex_literal::hex!("c2420003"),
        ],
        preferred_plus: &[&hex_literal::hex!("03")],
        deterministic: &[&hex_literal::hex!("03")],
        checking_rejects: &[
            &hex_literal::hex!("1803"),
            &hex_literal::hex!("190003"),
            &hex_literal::hex!("1a00000003"),
            &hex_literal::hex!("1b0000000000000003"),
            &hex_literal::hex!("c2420003"),
        ],
        deterministic_checking_rejects: &[
            &hex_literal::hex!("1803"),
            &hex_literal::hex!("190003"),
            &hex_literal::hex!("1a00000003"),
            &hex_literal::hex!("1b0000000000000003"),
            &hex_literal::hex!("c2420003"),
        ],
    },
    Vector {
        name: "minus_twenty_five",
        general: &[
            &hex_literal::hex!("3818"),
            &hex_literal::hex!("390018"),
            &hex_literal::hex!("3a00000018"),
            &hex_literal::hex!("3b0000000000000018"),
            &hex_literal::hex!("c3420018"),
        ],
        preferred_plus: &[&hex_literal::hex!("3818")],
        deterministic: &[&hex_literal::hex!("3818")],
        checking_rejects: &[
            &hex_literal::hex!("390018"),
            &hex_literal::hex!("3a00000018"),
            &hex_literal::hex!("3b0000000000000018"),
            &hex_literal::hex!("c3420018"),
        ],
        deterministic_checking_rejects: &[
            &hex_literal::hex!("390018"),
            &hex_literal::hex!("3a00000018"),
            &hex_literal::hex!("3b0000000000000018"),
            &hex_literal::hex!("c3420018"),
        ],
    },
    Vector {
        name: "65_bit_neg",
        general: &[
            &hex_literal::hex!("3bffffffffffffffff"),
            &hex_literal::hex!("c348ffffffffffffffff"),
        ],
        preferred_plus: &[&hex_literal::hex!("3bffffffffffffffff")],
        deterministic: &[&hex_literal::hex!("3bffffffffffffffff")],
        checking_rejects: &[&hex_literal::hex!("c348ffffffffffffffff")],
        deterministic_checking_rejects: &[&hex_literal::hex!("c348ffffffffffffffff")],
    },
    Vector {
        name: "byte_string",
        general: &[
            &hex_literal::hex!("43010203"),
            &hex_literal::hex!("5f4101420203ff"),
            &hex_literal::hex!("5f5801015a000000020203ff"),
        ],
        preferred_plus: &[&hex_literal::hex!("43010203")],
        deterministic: &[&hex_literal::hex!("43010203")],
        checking_rejects: &[
            &hex_literal::hex!("5f4101420203ff"),
            &hex_literal::hex!("5f5801015a000000020203ff"),
        ],
        deterministic_checking_rejects: &[
            &hex_literal::hex!("5f4101420203ff"),
            &hex_literal::hex!("5f5801015a000000020203ff"),
        ],
    },
    Vector {
        name: "text_string",
        general: &[
            &hex_literal::hex!("686869207468657265"),
            &hex_literal::hex!("7f686869207468657265ff"),
            &hex_literal::hex!("7f64686920746468657265ff"),
            &hex_literal::hex!("7f790004686920747b000000000000000468657265ff"),
        ],
        preferred_plus: &[&hex_literal::hex!("686869207468657265")],
        deterministic: &[&hex_literal::hex!("686869207468657265")],
        checking_rejects: &[
            &hex_literal::hex!("7f686869207468657265ff"),
            &hex_literal::hex!("7f64686920746468657265ff"),
            &hex_literal::hex!("7f790004686920747b000000000000000468657265ff"),
        ],
        deterministic_checking_rejects: &[
            &hex_literal::hex!("7f686869207468657265ff"),
            &hex_literal::hex!("7f64686920746468657265ff"),
            &hex_literal::hex!("7f790004686920747b000000000000000468657265ff"),
        ],
    },
    Vector {
        name: "array",
        general: &[
            &hex_literal::hex!("83010203"),
            &hex_literal::hex!("9f010203ff"),
        ],
        preferred_plus: &[&hex_literal::hex!("83010203")],
        deterministic: &[&hex_literal::hex!("83010203")],
        checking_rejects: &[&hex_literal::hex!("9f010203ff")],
        deterministic_checking_rejects: &[&hex_literal::hex!("9f010203ff")],
    },
    Vector {
        name: "map",
        general: &[
            &hex_literal::hex!("a301617802617903617a"),
            &hex_literal::hex!("a301617803617a026179"),
            &hex_literal::hex!("a302617903617a016178"),
            &hex_literal::hex!("a302617901617803617a"),
            &hex_literal::hex!("a303617a016178026179"),
            &hex_literal::hex!("a303617a026179016178"),
            &hex_literal::hex!("bf03617a026179016178ff"),
            &hex_literal::hex!("a31a00000003617a19000261791b00000000000000016178"),
        ],
        preferred_plus: &[
            &hex_literal::hex!("a301617802617903617a"),
            &hex_literal::hex!("a301617803617a026179"),
            &hex_literal::hex!("a302617903617a016178"),
            &hex_literal::hex!("a302617901617803617a"),
            &hex_literal::hex!("a303617a016178026179"),
            &hex_literal::hex!("a303617a026179016178"),
        ],
        deterministic: &[&hex_literal::hex!("a301617802617903617a")],
        checking_rejects: &[
            &hex_literal::hex!("bf03617a026179016178ff"),
            &hex_literal::hex!("a31a00000003617a19000261791b00000000000000016178"),
        ],
        deterministic_checking_rejects: &[
            &hex_literal::hex!("a301617803617a026179"),
            &hex_literal::hex!("a302617903617a016178"),
            &hex_literal::hex!("a302617901617803617a"),
            &hex_literal::hex!("a303617a016178026179"),
            &hex_literal::hex!("a303617a026179016178"),
            &hex_literal::hex!("bf03617a026179016178ff"),
            &hex_literal::hex!("a31a00000003617a19000261791b00000000000000016178"),
        ],
    },
    Vector {
        name: "map_strings",
        general: &[
            &hex_literal::hex!("a3636162630163646566026367686903"),
            &hex_literal::hex!("a3636162630163676869036364656602"),
            &hex_literal::hex!("a3636465660263616263016367686903"),
            &hex_literal::hex!("a3636465660263676869036361626301"),
            &hex_literal::hex!("a3636768690363616263016364656602"),
            &hex_literal::hex!("a3636768690363646566026361626301"),
            &hex_literal::hex!("bf636162630163646566026367686903ff"),
            &hex_literal::hex!("bf7f6161626263ff017f6264656166ff027f63676869ff03ff"),
        ],
        preferred_plus: &[
            &hex_literal::hex!("a3636162630163646566026367686903"),
            &hex_literal::hex!("a3636162630163676869036364656602"),
            &hex_literal::hex!("a3636465660263616263016367686903"),
            &hex_literal::hex!("a3636465660263676869036361626301"),
            &hex_literal::hex!("a3636768690363616263016364656602"),
            &hex_literal::hex!("a3636768690363646566026361626301"),
        ],
        deterministic: &[&hex_literal::hex!("a3636162630163646566026367686903")],
        checking_rejects: &[
            &hex_literal::hex!("bf636162630163646566026367686903ff"),
            &hex_literal::hex!("bf7f6161626263ff017f6264656166ff027f63676869ff03ff"),
        ],
        deterministic_checking_rejects: &[
            &hex_literal::hex!("a3636162630163676869036364656602"),
            &hex_literal::hex!("a3636465660263616263016367686903"),
            &hex_literal::hex!("a3636465660263676869036361626301"),
            &hex_literal::hex!("a3636768690363616263016364656602"),
            &hex_literal::hex!("a3636768690363646566026361626301"),
            &hex_literal::hex!("bf636162630163646566026367686903ff"),
            &hex_literal::hex!("bf7f6161626263ff017f6264656166ff027f63676869ff03ff"),
        ],
    },
    Vector {
        name: "positive_bignum",
        general: &[
            &hex_literal::hex!("c24cffffffffffffffffffffffff"),
            &hex_literal::hex!("c24e0000ffffffffffffffffffffffff"),
        ],
        preferred_plus: &[&hex_literal::hex!("c24cffffffffffffffffffffffff")],
        deterministic: &[&hex_literal::hex!("c24cffffffffffffffffffffffff")],
        checking_rejects: &[&hex_literal::hex!("c24e0000ffffffffffffffffffffffff")],
        deterministic_checking_rejects: &[&hex_literal::hex!("c24e0000ffffffffffffffffffffffff")],
    },
    Vector {
        name: "negative_bignum",
        general: &[
            &hex_literal::hex!("c349010000000000000000"),
            &hex_literal::hex!("c34c000000010000000000000000"),
            &hex_literal::hex!("c35f450000000001480000000000000000ff"),
        ],
        preferred_plus: &[&hex_literal::hex!("c349010000000000000000")],
        deterministic: &[&hex_literal::hex!("c349010000000000000000")],
        checking_rejects: &[
            &hex_literal::hex!("c34c000000010000000000000000"),
            &hex_literal::hex!("c35f450000000001480000000000000000ff"),
        ],
        deterministic_checking_rejects: &[
            &hex_literal::hex!("c34c000000010000000000000000"),
            &hex_literal::hex!("c35f450000000001480000000000000000ff"),
        ],
    },
    Vector {
        name: "date_epoch_tag",
        general: &[
            &hex_literal::hex!("c11a69e4fbd3"),
            &hex_literal::hex!("d8011a69e4fbd3"),
            &hex_literal::hex!("d900011a69e4fbd3"),
            &hex_literal::hex!("da000000011a69e4fbd3"),
            &hex_literal::hex!("db00000000000000011a69e4fbd3"),
            &hex_literal::hex!("c11b0000000069e4fbd3"),
        ],
        preferred_plus: &[&hex_literal::hex!("c11a69e4fbd3")],
        deterministic: &[&hex_literal::hex!("c11a69e4fbd3")],
        checking_rejects: &[
            &hex_literal::hex!("d8011a69e4fbd3"),
            &hex_literal::hex!("d900011a69e4fbd3"),
            &hex_literal::hex!("da000000011a69e4fbd3"),
            &hex_literal::hex!("db00000000000000011a69e4fbd3"),
            &hex_literal::hex!("c11b0000000069e4fbd3"),
        ],
        deterministic_checking_rejects: &[
            &hex_literal::hex!("d8011a69e4fbd3"),
            &hex_literal::hex!("d900011a69e4fbd3"),
            &hex_literal::hex!("da000000011a69e4fbd3"),
            &hex_literal::hex!("db00000000000000011a69e4fbd3"),
            &hex_literal::hex!("c11b0000000069e4fbd3"),
        ],
    },
    Vector {
        name: "date_string_tag",
        general: &[
            &hex_literal::hex!("c074323032362d30342d31395430333a35393a31355a"),
            &hex_literal::hex!("d80074323032362d30342d31395430333a35393a31355a"),
            &hex_literal::hex!("d9000074323032362d30342d31395430333a35393a31355a"),
            &hex_literal::hex!("da0000000074323032362d30342d31395430333a35393a31355a"),
            &hex_literal::hex!("db000000000000000074323032362d30342d31395430333a35393a31355a"),
            &hex_literal::hex!("c07f74323032362d30342d31395430333a35393a31355aff"),
            &hex_literal::hex!("c07f6232307232362d30342d31395430333a35393a31355aff"),
        ],
        preferred_plus: &[&hex_literal::hex!(
            "c074323032362d30342d31395430333a35393a31355a"
        )],
        deterministic: &[&hex_literal::hex!(
            "c074323032362d30342d31395430333a35393a31355a"
        )],
        checking_rejects: &[
            &hex_literal::hex!("d80074323032362d30342d31395430333a35393a31355a"),
            &hex_literal::hex!("d9000074323032362d30342d31395430333a35393a31355a"),
            &hex_literal::hex!("da0000000074323032362d30342d31395430333a35393a31355a"),
            &hex_literal::hex!("db000000000000000074323032362d30342d31395430333a35393a31355a"),
            &hex_literal::hex!("c07f74323032362d30342d31395430333a35393a31355aff"),
            &hex_literal::hex!("c07f6232307232362d30342d31395430333a35393a31355aff"),
        ],
        deterministic_checking_rejects: &[
            &hex_literal::hex!("d80074323032362d30342d31395430333a35393a31355a"),
            &hex_literal::hex!("d9000074323032362d30342d31395430333a35393a31355a"),
            &hex_literal::hex!("da0000000074323032362d30342d31395430333a35393a31355a"),
            &hex_literal::hex!("db000000000000000074323032362d30342d31395430333a35393a31355a"),
            &hex_literal::hex!("c07f74323032362d30342d31395430333a35393a31355aff"),
            &hex_literal::hex!("c07f6232307232362d30342d31395430333a35393a31355aff"),
        ],
    },
    Vector {
        name: "true",
        general: &[&hex_literal::hex!("f5")],
        preferred_plus: &[&hex_literal::hex!("f5")],
        deterministic: &[&hex_literal::hex!("f5")],
        checking_rejects: &[],
        deterministic_checking_rejects: &[],
    },
    Vector {
        name: "simple111",
        general: &[&hex_literal::hex!("f86f")],
        preferred_plus: &[&hex_literal::hex!("f86f")],
        deterministic: &[&hex_literal::hex!("f86f")],
        checking_rejects: &[],
        deterministic_checking_rejects: &[],
    },
    Vector {
        name: "float_zero",
        general: &[
            &hex_literal::hex!("f90000"),
            &hex_literal::hex!("fa00000000"),
            &hex_literal::hex!("fb0000000000000000"),
        ],
        preferred_plus: &[&hex_literal::hex!("f90000")],
        deterministic: &[&hex_literal::hex!("f90000")],
        checking_rejects: &[
            &hex_literal::hex!("fa00000000"),
            &hex_literal::hex!("fb0000000000000000"),
        ],
        deterministic_checking_rejects: &[
            &hex_literal::hex!("fa00000000"),
            &hex_literal::hex!("fb0000000000000000"),
        ],
    },
    Vector {
        name: "float_neg_zero",
        general: &[
            &hex_literal::hex!("f98000"),
            &hex_literal::hex!("fa80000000"),
            &hex_literal::hex!("fb8000000000000000"),
        ],
        preferred_plus: &[&hex_literal::hex!("f98000")],
        deterministic: &[&hex_literal::hex!("f98000")],
        checking_rejects: &[
            &hex_literal::hex!("fa80000000"),
            &hex_literal::hex!("fb8000000000000000"),
        ],
        deterministic_checking_rejects: &[
            &hex_literal::hex!("fa80000000"),
            &hex_literal::hex!("fb8000000000000000"),
        ],
    },
    Vector {
        name: "float_double",
        general: &[&hex_literal::hex!("fb7fefffffffffffff")],
        preferred_plus: &[&hex_literal::hex!("fb7fefffffffffffff")],
        deterministic: &[&hex_literal::hex!("fb7fefffffffffffff")],
        checking_rejects: &[],
        deterministic_checking_rejects: &[],
    },
    Vector {
        name: "float_double_subnormal",
        general: &[&hex_literal::hex!("fb8000000000000001")],
        preferred_plus: &[&hex_literal::hex!("fb8000000000000001")],
        deterministic: &[&hex_literal::hex!("fb8000000000000001")],
        checking_rejects: &[],
        deterministic_checking_rejects: &[],
    },
    Vector {
        name: "float_single",
        general: &[
            &hex_literal::hex!("fbc170000000000000"),
            &hex_literal::hex!("facb800000"),
        ],
        preferred_plus: &[&hex_literal::hex!("facb800000")],
        deterministic: &[&hex_literal::hex!("facb800000")],
        checking_rejects: &[&hex_literal::hex!("fbc170000000000000")],
        deterministic_checking_rejects: &[&hex_literal::hex!("fbc170000000000000")],
    },
    Vector {
        name: "float_single_subnormal",
        general: &[
            &hex_literal::hex!("fb3800000000000000"),
            &hex_literal::hex!("fa00400000"),
        ],
        preferred_plus: &[&hex_literal::hex!("fa00400000")],
        deterministic: &[&hex_literal::hex!("fa00400000")],
        checking_rejects: &[&hex_literal::hex!("fb3800000000000000")],
        deterministic_checking_rejects: &[&hex_literal::hex!("fb3800000000000000")],
    },
    Vector {
        name: "float_half",
        general: &[
            &hex_literal::hex!("fb40effc0000000000"),
            &hex_literal::hex!("fa477fe000"),
            &hex_literal::hex!("f97bff"),
        ],
        preferred_plus: &[&hex_literal::hex!("f97bff")],
        deterministic: &[&hex_literal::hex!("f97bff")],
        checking_rejects: &[
            &hex_literal::hex!("fb40effc0000000000"),
            &hex_literal::hex!("fa477fe000"),
        ],
        deterministic_checking_rejects: &[
            &hex_literal::hex!("fb40effc0000000000"),
            &hex_literal::hex!("fa477fe000"),
        ],
    },
    Vector {
        name: "float_half_subnormal",
        general: &[
            &hex_literal::hex!("fb3f00000000000000"),
            &hex_literal::hex!("fa38000000"),
            &hex_literal::hex!("f90200"),
        ],
        preferred_plus: &[&hex_literal::hex!("f90200")],
        deterministic: &[&hex_literal::hex!("f90200")],
        checking_rejects: &[
            &hex_literal::hex!("fb3f00000000000000"),
            &hex_literal::hex!("fa38000000"),
        ],
        deterministic_checking_rejects: &[
            &hex_literal::hex!("fb3f00000000000000"),
            &hex_literal::hex!("fa38000000"),
        ],
    },
    Vector {
        name: "float_neg_infinity",
        general: &[
            &hex_literal::hex!("f9fc00"),
            &hex_literal::hex!("faff800000"),
            &hex_literal::hex!("fbfff0000000000000"),
        ],
        preferred_plus: &[&hex_literal::hex!("f9fc00")],
        deterministic: &[&hex_literal::hex!("f9fc00")],
        checking_rejects: &[
            &hex_literal::hex!("faff800000"),
            &hex_literal::hex!("fbfff0000000000000"),
        ],
        deterministic_checking_rejects: &[
            &hex_literal::hex!("faff800000"),
            &hex_literal::hex!("fbfff0000000000000"),
        ],
    },
    Vector {
        name: "float_quiet_nan",
        general: &[
            &hex_literal::hex!("f97e00"),
            &hex_literal::hex!("fa7fc00000"),
            &hex_literal::hex!("fb7ff8000000000000"),
        ],
        preferred_plus: &[&hex_literal::hex!("f97e00")],
        deterministic: &[&hex_literal::hex!("f97e00")],
        checking_rejects: &[
            &hex_literal::hex!("fa7fc00000"),
            &hex_literal::hex!("fb7ff8000000000000"),
        ],
        deterministic_checking_rejects: &[
            &hex_literal::hex!("fa7fc00000"),
            &hex_literal::hex!("fb7ff8000000000000"),
        ],
    },
    Vector {
        name: "float_nan_payload",
        general: &[
            &hex_literal::hex!("f97dff"),
            &hex_literal::hex!("fa7fbfe000"),
            &hex_literal::hex!("fb7ff7fc0000000000"),
        ],
        preferred_plus: &[],
        deterministic: &[],
        checking_rejects: &[
            &hex_literal::hex!("f97dff"),
            &hex_literal::hex!("fa7fbfe000"),
            &hex_literal::hex!("fb7ff7fc0000000000"),
        ],
        deterministic_checking_rejects: &[
            &hex_literal::hex!("f97dff"),
            &hex_literal::hex!("fa7fbfe000"),
            &hex_literal::hex!("fb7ff7fc0000000000"),
        ],
    },
];

fn decode_value(bytes: &[u8]) -> Value<'_> {
    from_bytes::<Value>(bytes).unwrap_or_else(|error| panic!("{bytes:02x?}: {error}"))
}

fn encode_value(value: &Value<'_>, profile: Profile) -> Vec<u8> {
    value
        .cbor_serialize_with_profile(profile)
        .unwrap_or_else(|error| panic!("{value:?}: {error}"))
}

#[test]
fn general_decoding_accepts_every_general_form() {
    for vector in VECTORS {
        for bytes in vector.general {
            from_bytes::<Value>(bytes)
                .unwrap_or_else(|error| panic!("{} / {bytes:02x?}: {error}", vector.name));
        }
    }
}

#[test]
fn preferred_plus_decoding_accepts_the_floor_forms() {
    for vector in VECTORS {
        for bytes in vector.preferred_plus {
            from_bytes_with_options::<Value>(bytes, DecodeOptions::PREFERRED_PLUS)
                .unwrap_or_else(|error| panic!("{} / {bytes:02x?}: {error}", vector.name));
        }
    }
}

#[test]
fn deterministic_decoding_accepts_the_canonical_forms() {
    for vector in VECTORS {
        for bytes in vector.deterministic {
            from_bytes_with_options::<Value>(bytes, DecodeOptions::DETERMINISTIC)
                .unwrap_or_else(|error| panic!("{} / {bytes:02x?}: {error}", vector.name));
        }
    }
}

#[test]
fn preferred_plus_encoding_from_general_forms_matches_draft09() {
    let mut failures = Vec::new();
    for vector in VECTORS {
        // K.2 has no EDN representation for payload NaNs; the canonicalizing
        // encoder is covered by the dedicated f16 test instead.
        if vector.preferred_plus.is_empty() {
            continue;
        }
        for bytes in vector.general {
            let actual = encode_value(&decode_value(bytes), Profile::PreferredPlus);
            if !vector.preferred_plus.contains(&actual.as_slice()) {
                failures.push(format!(
                    "{}: {bytes:02x?} encoded as {actual:02x?}, expected one of {:02x?}",
                    vector.name, vector.preferred_plus
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "preferred-plus encode failures:\n{}",
        failures.join("\n")
    );
}

#[test]
fn deterministic_encoding_from_general_forms_matches_draft09() {
    let mut failures = Vec::new();
    for vector in VECTORS {
        if vector.deterministic.is_empty() {
            continue;
        }
        for bytes in vector.general {
            let actual = encode_value(&decode_value(bytes), Profile::Deterministic);
            if !vector.deterministic.contains(&actual.as_slice()) {
                failures.push(format!(
                    "{}: {bytes:02x?} encoded as {actual:02x?}, expected one of {:02x?}",
                    vector.name, vector.deterministic
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "deterministic encode failures:\n{}",
        failures.join("\n")
    );
}

fn check_rejected(
    vector: &Vector,
    bytes: &[u8],
    result: Result<Value<'_>, SeaboredDeError>,
) -> Option<String> {
    match result {
        Ok(_) => Some(format!("{}: {bytes:02x?} was accepted", vector.name)),
        Err(SeaboredDeError::NonPreferredEncoding { .. }) => None,
        Err(error) => Some(format!(
            "{}: {bytes:02x?} failed with the wrong class: {error}",
            vector.name
        )),
    }
}

#[test]
fn preferred_plus_checking_rejects_general_only_forms() {
    let mut failures = Vec::new();
    for vector in VECTORS {
        for bytes in vector.checking_rejects {
            if let Some(failure) = check_rejected(
                vector,
                bytes,
                from_bytes_with_options::<Value>(bytes, DecodeOptions::PREFERRED_PLUS_CHECKED),
            ) {
                failures.push(failure);
            }
        }
    }
    assert!(
        failures.is_empty(),
        "preferred-plus checking failures:\n{}",
        failures.join("\n")
    );
}

#[test]
fn deterministic_checking_rejects_unordered_and_nonpreferred_forms() {
    let mut failures = Vec::new();
    for vector in VECTORS {
        for bytes in vector.deterministic_checking_rejects {
            if let Some(failure) = check_rejected(
                vector,
                bytes,
                from_bytes_with_options::<Value>(bytes, DecodeOptions::DETERMINISTIC_CHECKED),
            ) {
                failures.push(failure);
            }
        }
    }
    assert!(
        failures.is_empty(),
        "deterministic checking failures:\n{}",
        failures.join("\n")
    );
}

/// K.1.4: a non-checking decoder accepts the general forms the checking decoder rejects.
#[test]
fn nonchecking_profiles_accept_every_general_form() {
    let mut failures = Vec::new();
    for vector in VECTORS {
        for bytes in vector.general {
            for options in [DecodeOptions::PREFERRED_PLUS, DecodeOptions::DETERMINISTIC] {
                if let Err(error) = from_bytes_with_options::<Value>(bytes, options) {
                    failures.push(format!(
                        "{} / {options:?} rejected {bytes:02x?}: {error}",
                        vector.name
                    ));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "non-checking decode failures:\n{}",
        failures.join("\n")
    );
}

#[test]
fn f16_nan_encodes_as_the_canonical_half_precision_nan() {
    for bits in [0x7e01_u16, 0x7dff, 0xfe00, 0x7c01, 0xfc01] {
        let encoded = f16::from_bits(bits).cbor_serialize().unwrap();
        assert_eq!(
            encoded,
            hex_literal::hex!("f97e00"),
            "f16 bits {bits:#06x} encoded as {encoded:02x?}"
        );
    }
}

#[cfg(feature = "derive")]
#[derive(Debug, PartialEq, seabored::CborSerialize, seabored::CborDeserialize)]
#[cbor(profile = "deterministic")]
struct DeterministicOuter<'a> {
    m: Value<'a>,
}

#[cfg(feature = "derive")]
#[test]
fn deterministic_profile_sorts_nested_value_maps() {
    let bytes = hex_literal::hex!("a1616da2016178006179");
    let value: DeterministicOuter<'_> =
        from_bytes_with_options(&bytes, DecodeOptions::GENERAL).unwrap();
    assert_eq!(
        value.cbor_serialize().unwrap(),
        hex_literal::hex!("a1616da2006179016178")
    );
}
