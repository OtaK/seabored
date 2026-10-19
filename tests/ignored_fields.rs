//! Ignored field values are skipped, not materialised: the allocation counter is per test binary,
//! so this lives in its own file (a parallel test in the same binary would share the counters).

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

static LIVE: AtomicUsize = AtomicUsize::new(0);
static BASE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

fn bump(d: isize) {
    let live = (LIVE.fetch_add(d as usize, Ordering::Relaxed) as isize + d).max(0) as usize;
    let base = BASE.load(Ordering::Relaxed);
    if live > base {
        PEAK.fetch_max(live - base, Ordering::Relaxed);
    }
}

struct Counting;
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        bump(l.size() as isize);
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        bump(-(l.size() as isize));
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, new: usize) -> *mut u8 {
        bump(new as isize - l.size() as isize);
        unsafe { System.realloc(p, l, new) }
    }
}
#[global_allocator]
static A: Counting = Counting;

fn measure<T>(f: impl FnOnce() -> T) -> (T, usize) {
    BASE.store(LIVE.load(Ordering::Relaxed), Ordering::Relaxed);
    PEAK.store(0, Ordering::Relaxed);
    let out = f();
    let peak = PEAK.load(Ordering::Relaxed);
    BASE.store(0, Ordering::Relaxed);
    (out, peak)
}

#[test]
fn ignored_fields_are_skipped_and_consumed() {
    ignored_field_values_are_not_materialised_body();
    ignored_values_are_fully_consumed_body();
}

fn ignored_field_values_are_not_materialised_body() {
    #[derive(serde::Deserialize)]
    struct Known {
        known: u8,
    }

    // {"known": 1, "unknown": [1000 zeros]} — the unknown value used to be decoded into a full
    // `Value` and dropped
    let mut bytes = vec![
        0xa2, 0x65, b'k', b'n', b'o', b'w', b'n', 0x01, 0x67, b'u', b'n', b'k', b'n', b'o', b'w',
        b'n', 0x99, 0x03, 0xe8,
    ];
    bytes.extend(std::iter::repeat_n(0x00u8, 1000));

    let (result, peak) = measure(|| seabored::serde::from_slice::<Known>(&bytes));
    assert_eq!(result.unwrap().known, 1);
    assert!(
        peak <= 1024,
        "ignored field materialised {peak} bytes for a {} byte input",
        bytes.len()
    );
}

/// `{"unknown": <value>, "known": 1}` — the unknown field comes first, so an ignored value that
/// is not fully consumed desynchronises the field that follows.
fn unknown_before_known(value: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0xa2, 0x67, b'u', b'n', b'k', b'n', b'o', b'w', b'n'];
    bytes.extend_from_slice(value);
    bytes.extend_from_slice(&[0x65, b'k', b'n', b'o', b'w', b'n', 0x01]);
    bytes
}

fn ignored_values_are_fully_consumed_body() {
    #[derive(serde::Deserialize)]
    struct Known {
        known: u8,
    }

    let mut big_array = vec![0x99, 0x03, 0xe8];
    big_array.extend(std::iter::repeat_n(0x00u8, 1000));

    let shapes: [(&str, &[u8]); 15] = [
        ("array, definite", &big_array),
        ("array, indefinite", &[0x9f, 0x01, 0x02, 0xff]),
        ("map, definite", &[0xa1, 0x61, b'a', 0x01]),
        ("map, indefinite", &[0xbf, 0x61, b'a', 0x01, 0xff]),
        ("text, definite", &[0x63, b'a', b'b', b'c']),
        (
            "text, indefinite chunks",
            &[0x7f, 0x62, b'a', b'b', 0x61, b'c', 0xff],
        ),
        ("bytes, definite", &[0x43, 0x01, 0x02, 0x03]),
        (
            "bytes, indefinite chunks",
            &[0x5f, 0x41, 0x01, 0x41, 0x02, 0xff],
        ),
        ("tag on a scalar", &[0xc1, 0x01]),
        ("tag on an array", &[0xc1, 0x9f, 0x01, 0xff]),
        ("bignum tag", &[0xc2, 0x41, 0x01]),
        ("float", &[0xfa, 0x3f, 0x80, 0x00, 0x00]),
        ("simple value (two bytes)", &[0xf8, 0x20]),
        ("null", &[0xf6]),
        ("undefined", &[0xf7]),
    ];

    for (name, value) in shapes {
        let bytes = unknown_before_known(value);
        // `from_slice_complete` also proves nothing is left over after the struct
        let decoded = seabored::serde::from_slice_complete::<Known>(&bytes)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(decoded.known, 1, "{name}");
    }
}
