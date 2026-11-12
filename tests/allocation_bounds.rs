//! Peak heap allocated by a decode, versus input bytes (RFC 8949 §10).
//!
//! Baseline heap is snapshotted per measurement, so harness allocations stay out. A `Value` is
//! 40 bytes and an item needs >= 1 wire byte: ~40x input is the floor; extra capacity is the bug.
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

/// Peak bytes held at once, per decode path
fn allocated(bytes: &[u8]) -> (usize, usize) {
    let (native_result, native) = measure(|| seabored::de::from_bytes::<seabored::Value>(bytes));
    drop(native_result);
    let (serde_result, serde) = measure(|| seabored::serde::from_slice::<seabored::Value>(bytes));
    drop(serde_result);
    (native, serde)
}

const SLACK: usize = 1024;
const PER_BYTE: usize = 48;

#[test]
fn peak_heap_stays_proportional_to_input() {
    let mut nested = vec![0x9fu8; 100];
    nested.resize(200, 0xff);
    let huge_array = [0x9b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff];
    let huge_map = [0xbb, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff];
    let one_pair = [0xbf, 0x00, 0x00, 0xff];

    let cases: [(&str, &[u8]); 7] = [
        ("empty indefinite bstr  5f ff", &[0x5f, 0xff]),
        ("empty indefinite map   bf ff", &[0xbf, 0xff]),
        ("empty indefinite array 9f ff", &[0x9f, 0xff]),
        ("indefinite map, one pair bf 00 00 ff", &one_pair),
        ("huge claimed array     9b ff..", &huge_array),
        ("huge claimed map       bb ff..", &huge_map),
        ("100 nested indefinites 9f x100 ff x100", &nested),
    ];

    println!(
        "{:<40}{:>9}{:>10}{:>10}",
        "input", "bytes in", "native", "serde"
    );
    for (name, bytes) in cases {
        let (native, serde) = allocated(bytes);
        let budget = SLACK + PER_BYTE * bytes.len();
        println!("{:<40}{:>9}{:>10}{:>10}", name, bytes.len(), native, serde);
        if bytes.len() <= 9 {
            assert!(
                native <= budget,
                "{name}: native allocated {native} for {} bytes",
                bytes.len()
            );
            assert!(
                serde <= budget,
                "{name}: serde allocated {serde} for {} bytes",
                bytes.len()
            );
        }
    }

    // A real payload, measured in the same test for the same reason: ~40 bytes per element is
    // the item cost, with growth rounding up to the next power of two
    let mut arr = vec![0x99, 0x03, 0xe8];
    arr.extend(std::iter::repeat_n(0x00u8, 1000));

    let (value, native) = measure(|| seabored::de::from_bytes::<seabored::Value>(&arr));
    let (res, serde) = measure(|| seabored::serde::from_slice::<seabored::Value>(&arr));
    println!(
        "{:<40}{:>9}{:>10}{:>10}",
        "1000-element array",
        arr.len(),
        native,
        serde
    );
    // ~40 bytes per element is the item cost; growth may round up to the next power of two
    assert_eq!(value.unwrap().mt(), seabored::mt::MajorType::Array);
    assert!(res.is_ok());
    assert!(native <= 48 * arr.len(), "native: {native} bytes");
    assert!(serde <= 48 * arr.len(), "serde: {serde} bytes");

    // A wire length must not drive preallocation for the native Vec impl either.
    let (res, peak) = measure(|| seabored::de::from_bytes::<Vec<u8>>(&huge_array));
    assert!(res.is_err());
    assert!(
        peak <= SLACK + PER_BYTE * huge_array.len(),
        "Vec preallocated {peak} bytes for {} input bytes",
        huge_array.len()
    );
}
