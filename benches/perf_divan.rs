#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static ALLOC: divan::AllocProfiler<tikv_jemallocator::Jemalloc> =
    divan::AllocProfiler::new(tikv_jemallocator::Jemalloc);

mod datasets;

fn find_sample(sample_id: &str) -> &'static [u8] {
    datasets::SAMPLES
        .iter()
        .find_map(|(id, sample)| (*id == sample_id).then_some(*sample))
        .expect("A sample must be found")
}

#[divan::bench_group]
mod value {
    use divan::{black_box, counter::BytesCount};
    use seabored::{de::from_bytes, ser::CborSerialize as _};

    use crate::find_sample;

    #[divan::bench(args = super::datasets::sample_ids())]
    fn value_ser(bencher: divan::Bencher, sample_id: &str) {
        let sample = find_sample(sample_id);
        let value = from_bytes::<seabored::Value>(sample).unwrap();
        let capacity = value.cbor_serialize().unwrap().len();

        bencher
            .counter(BytesCount::new(capacity))
            .with_inputs(|| Vec::with_capacity(capacity))
            .bench_local_refs(|buf| {
                black_box(value.cbor_serialize_to(black_box(buf)).unwrap());
            });
    }

    #[divan::bench(args = super::datasets::sample_ids())]
    fn value_de(bencher: divan::Bencher, sample_id: &str) {
        let sample = find_sample(sample_id);
        bencher
            .counter(BytesCount::of_slice(sample))
            .bench_local(|| {
                black_box(from_bytes::<seabored::Value>(black_box(sample)).unwrap());
            });
    }
}

#[divan::bench_group]
mod serde {
    use crate::datasets::{self, HasSample};
    use divan::{black_box, counter::BytesCount};

    #[divan::bench(
        types = [
            datasets::mimi_content_multipart_3::MimiContent<seabored::Value>,
            datasets::log::BorrowLogs,
            datasets::mesh::Mesh,
            datasets::minecraft_savedata::BorrowPlayers,
            datasets::mk48::Updates,
        ],
    )]
    fn serde_ser<'de, T: serde::Serialize + serde::Deserialize<'de> + HasSample>(
        bencher: divan::Bencher,
    ) {
        let sample = T::sample();
        let value = seabored::serde::from_slice::<T>(sample).unwrap();
        let capacity = seabored::serde::to_vec(&value).unwrap().len();

        bencher
            .counter(BytesCount::new(capacity))
            .with_inputs(|| Vec::with_capacity(capacity))
            .bench_local_refs(|buf| {
                black_box(seabored::serde::to_writer(black_box(buf), black_box(&value)).unwrap());
            });
    }

    #[divan::bench(
        types = [
            datasets::mimi_content_multipart_3::MimiContent<seabored::Value>,
            datasets::log::BorrowLogs,
            datasets::mesh::Mesh,
            datasets::minecraft_savedata::BorrowPlayers,
            datasets::mk48::Updates,
        ],
    )]
    fn serde_de<'de, T: serde::Serialize + serde::Deserialize<'de> + HasSample>(
        bencher: divan::Bencher,
    ) {
        let sample = T::sample();
        bencher
            .counter(BytesCount::of_slice(sample))
            .bench_local(|| {
                black_box(seabored::serde::from_slice::<T>(black_box(sample)).unwrap());
            });
    }
}

fn main() {
    divan::main();
}
