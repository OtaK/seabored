//! Public profile entry points and the compatibility policy for the legacy APIs.

use seabored::{
    Profile, Value, de,
    error::{SeaboredDeError, SeaboredSerError},
    mt::MajorType,
    ser,
    ser::CborSerialize as _,
    types::CborContainer,
};

#[test]
fn native_encode_helpers_and_aliases() {
    let value = Value::Integer(7u8.into());

    assert_eq!(
        ser::to_vec_with_profile(&value, Profile::PreferredPlus).unwrap(),
        [0x07]
    );
    assert_eq!(ser::to_vec_preferred_plus(&value).unwrap(), [0x07]);
    assert_eq!(ser::to_vec_deterministic(&value).unwrap(), [0x07]);

    let mut buf = Vec::new();
    ser::to_writer_with_profile(&mut buf, &value, Profile::Deterministic).unwrap();
    assert_eq!(buf, [0x07]);
}

#[test]
fn encode_aliases_enforce_strict_profiles() {
    let mut sequence = CborContainer::new_indefinite(MajorType::Array);
    sequence.extend([1u8, 2u8]);

    assert_eq!(
        sequence.cbor_serialize().unwrap(),
        hex_literal::hex!("9f0102ff")
    );
    assert!(matches!(
        ser::to_vec_preferred_plus(&sequence),
        Err(SeaboredSerError::IndefiniteForbidden { .. })
    ));
    assert!(matches!(
        ser::to_vec_deterministic(&sequence),
        Err(SeaboredSerError::IndefiniteForbidden { .. })
    ));
}

#[test]
fn native_decode_aliases_check_their_profile() {
    assert_eq!(de::from_bytes_preferred_plus::<u8>(&[0x07]).unwrap(), 7);
    assert_eq!(
        de::from_bytes_deterministic_checked::<u8>(&[0x07]).unwrap(),
        7
    );

    // Non-shortest integer header.
    assert!(matches!(
        de::from_bytes_preferred_plus::<u8>(&[0x18, 0x01]),
        Err(SeaboredDeError::NonPreferredEncoding { .. })
    ));

    // Preferred-plus does not require map ordering; deterministic checking does.
    let unsorted = hex_literal::hex!("a2016178006179");
    assert!(de::from_bytes_preferred_plus::<Value>(&unsorted).is_ok());
    assert!(matches!(
        de::from_bytes_deterministic_checked::<Value>(&unsorted),
        Err(SeaboredDeError::NonPreferredEncoding { .. })
    ));
}

#[test]
fn legacy_entry_points_stay_general_and_permissive() {
    assert_eq!(de::from_bytes::<u8>(&[0x18, 0x01]).unwrap(), 1);

    let indefinite: Value<'_> = de::from_bytes(&hex_literal::hex!("9f0102ff")).unwrap();
    assert_eq!(
        indefinite.cbor_serialize().unwrap(),
        hex_literal::hex!("9f0102ff")
    );
}

#[cfg(feature = "serde")]
mod serde_api {
    use super::*;

    #[test]
    fn serde_profile_entry_points() {
        let non_shortest = [0x18, 0x01];

        assert!(
            seabored::serde::from_slice_with_profile::<u8>(&non_shortest, Profile::PreferredPlus)
                .is_err()
        );
        assert_eq!(
            seabored::serde::from_slice_with_profile::<u8>(&non_shortest, Profile::General)
                .unwrap(),
            1
        );

        assert!(matches!(
            seabored::serde::from_reader_with_profile::<u8, _>(
                &non_shortest[..],
                Profile::PreferredPlus
            ),
            Err(SeaboredDeError::ProfileUnsupported { .. })
        ));
        assert_eq!(
            seabored::serde::from_reader_with_profile::<u8, _>(&non_shortest[..], Profile::General)
                .unwrap(),
            1
        );
    }
}
