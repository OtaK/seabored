use cbor4ii::core::{dec::Decode, enc::Encode};
use criterion::{BatchSize, Criterion, Throughput, criterion_group, criterion_main};
use seabored::{
    de::{CborDeserialize, parse_value},
    ser::CborSerialize,
};
use std::hint::black_box;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

mod datasets;
use datasets::*;

fn serde_bench_with_group_and_sample_cbor4ii<'de, T: serde::Serialize + serde::Deserialize<'de>>(
    sample: &'de [u8],
    c: &mut Criterion,
    group_name: &str,
) {
    let mut group = c.benchmark_group(format!("{group_name}/cbor4ii"));
    group.throughput(Throughput::Bytes(sample.len() as u64));

    group.bench_function("serde-de", |b| {
        b.iter(|| black_box(cbor4ii::serde::from_slice::<T>(black_box(&sample[..])).unwrap()));
    });

    group.bench_function("serde-ser", |b| {
        let value = cbor4ii::serde::from_slice::<T>(&sample[..]).unwrap();
        let capacity = cbor4ii::serde::to_vec(vec![], &value).unwrap().len();

        b.iter_batched_ref(
            || black_box(Vec::with_capacity(capacity)),
            |buf| black_box(cbor4ii::serde::to_writer(black_box(buf), black_box(&value)).unwrap()),
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

fn serde_bench_with_group_and_sample_ciborium<
    'de,
    T: serde::Serialize + serde::de::DeserializeOwned,
>(
    sample: &'de [u8],
    c: &mut Criterion,
    group_name: &str,
) {
    let mut group = c.benchmark_group(format!("{group_name}/ciborium"));
    group.throughput(Throughput::Bytes(sample.len() as u64));

    group.bench_function("serde-de", |b| {
        b.iter(|| black_box(ciborium::from_reader::<T, &[u8]>(black_box(&sample[..])).unwrap()));
    });

    group.bench_function("serde-ser", |b| {
        let value = ciborium::from_reader::<T, &[u8]>(&sample[..]).unwrap();
        let mut buf = vec![];
        ciborium::into_writer(&value, &mut buf).unwrap();
        let capacity = buf.len();

        b.iter_batched_ref(
            || black_box(Vec::with_capacity(capacity)),
            |buf| black_box(ciborium::into_writer(black_box(&value), black_box(buf)).unwrap()),
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

fn serde_bench_with_group_and_sample_seabored<
    'de,
    T: serde::Serialize + serde::Deserialize<'de>,
>(
    sample: &'de [u8],
    c: &mut Criterion,
    group_name: &str,
) {
    let mut group = c.benchmark_group(format!("{group_name}/seabored"));
    group.throughput(Throughput::Bytes(sample.len() as u64));

    group.bench_function("serde-de", |b| {
        b.iter(|| black_box(seabored::serde::from_slice::<T>(black_box(sample)).unwrap()));
    });

    group.bench_function("serde-ser", |b| {
        let value = seabored::adapters::serde::from_slice::<T>(sample).unwrap();
        let capacity = seabored::adapters::serde::to_vec(&value).unwrap().len();

        b.iter_batched_ref(
            || black_box(Vec::with_capacity(capacity)),
            |buf| {
                black_box(
                    seabored::adapters::serde::to_writer(black_box(buf), black_box(&value))
                        .unwrap(),
                )
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

fn value_bench_with_group_and_sample_cbor4ii(sample: &[u8], c: &mut Criterion, group_name: &str) {
    let mut group = c.benchmark_group(format!("{group_name}/cbor4ii"));
    group.throughput(Throughput::Bytes(sample.len() as u64));

    group.bench_function("value-de", |b| {
        b.iter(|| {
            black_box(
                cbor4ii::core::Value::decode(black_box(
                    &mut cbor4ii::core::utils::SliceReader::new(&sample[..]),
                ))
                .unwrap(),
            )
        });
    });

    group.bench_function("value-ser", |b| {
        let value =
            cbor4ii::core::Value::decode(&mut cbor4ii::core::utils::SliceReader::new(&sample[..]))
                .unwrap();
        let capacity = {
            let mut writer = cbor4ii::core::utils::BufWriter::new(vec![]);
            value.encode(&mut writer).unwrap();
            writer.into_inner().len()
        };

        b.iter_batched(
            || black_box(Vec::with_capacity(capacity)),
            |buf| {
                black_box(
                    value
                        .encode(black_box(&mut cbor4ii::core::utils::BufWriter::new(buf)))
                        .unwrap(),
                )
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

fn value_bench_with_group_and_sample_ciborium(sample: &[u8], c: &mut Criterion, group_name: &str) {
    let mut group = c.benchmark_group(format!("{group_name}/ciborium"));
    group.throughput(Throughput::Bytes(sample.len() as u64));

    group.bench_function("value-de", |b| {
        b.iter(|| {
            black_box(
                ciborium::from_reader::<ciborium::Value, &[u8]>(black_box(&sample[..])).unwrap(),
            )
        });
    });

    group.bench_function("value-ser", |b| {
        let value = ciborium::from_reader::<ciborium::Value, &[u8]>(&sample[..]).unwrap();
        let mut buf = vec![];
        ciborium::into_writer(&value, &mut buf).unwrap();
        let capacity = buf.len();

        b.iter_batched_ref(
            || black_box(Vec::with_capacity(capacity)),
            |buf| black_box(ciborium::into_writer(black_box(&value), black_box(buf)).unwrap()),
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

fn value_bench_with_group_and_sample_seabored(sample: &[u8], c: &mut Criterion, group_name: &str) {
    let mut group = c.benchmark_group(format!("{group_name}/seabored"));
    group.throughput(Throughput::Bytes(sample.len() as u64));

    group.bench_function("value-de", |b| {
        b.iter(|| black_box(parse_value(black_box(&mut &sample[..])).unwrap()));
    });

    group.bench_function("value-ser", |b| {
        let value = parse_value(&mut &sample[..]).unwrap();
        let capacity = value.cbor_serialize().unwrap().len();

        b.iter_batched_ref(
            || black_box(Vec::with_capacity(capacity)),
            |buf| black_box(value.cbor_serialize_to(black_box(buf)).unwrap()),
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

fn perf_value_winnow_homebrew(c: &mut Criterion) {
    let sample = mimi_content_multipart_3::BYTES;
    let mut group = c.benchmark_group("seabored/parser_impl");
    group.throughput(Throughput::Bytes(sample.len() as u64));

    group.bench_function("cbor_deserialize_from", |b| {
        b.iter(|| {
            black_box(seabored::Value::cbor_deserialize_from(black_box(&mut &sample[..])).unwrap())
        });
    });

    group.bench_function("parse_value", |b| {
        b.iter(|| black_box(parse_value(black_box(&mut &sample[..])).unwrap()));
    });

    group.finish();
}

fn perf_value(c: &mut Criterion) {
    value_bench_with_group_and_sample_seabored(
        &mimi_content_multipart_3::BYTES[..],
        c,
        "mimi_content_multipart_3",
    );
    value_bench_with_group_and_sample_cbor4ii(
        &mimi_content_multipart_3::BYTES[..],
        c,
        "mimi_content_multipart_3",
    );
    value_bench_with_group_and_sample_ciborium(
        &mimi_content_multipart_3::BYTES[..],
        c,
        "mimi_content_multipart_3",
    );
    value_bench_with_group_and_sample_seabored(&log::BYTES[..], c, "log");
    value_bench_with_group_and_sample_cbor4ii(&log::BYTES[..], c, "log");
    value_bench_with_group_and_sample_ciborium(&log::BYTES[..], c, "log");
    value_bench_with_group_and_sample_seabored(&mesh::BYTES[..], c, "mesh");
    value_bench_with_group_and_sample_cbor4ii(&mesh::BYTES[..], c, "mesh");
    value_bench_with_group_and_sample_ciborium(&mesh::BYTES[..], c, "mesh");
    value_bench_with_group_and_sample_seabored(
        &minecraft_savedata::BYTES[..],
        c,
        "minecraft_savedata",
    );
    value_bench_with_group_and_sample_cbor4ii(
        &minecraft_savedata::BYTES[..],
        c,
        "minecraft_savedata",
    );
    value_bench_with_group_and_sample_ciborium(
        &minecraft_savedata::BYTES[..],
        c,
        "minecraft_savedata",
    );
    value_bench_with_group_and_sample_seabored(&mk48::BYTES[..], c, "mk48");
    value_bench_with_group_and_sample_cbor4ii(&mk48::BYTES[..], c, "mk48");
    value_bench_with_group_and_sample_ciborium(&mk48::BYTES[..], c, "mk48");
}

fn perf_serde(c: &mut Criterion) {
    serde_bench_with_group_and_sample_seabored::<
        datasets::mimi_content_multipart_3::MimiContent<seabored::Value>,
    >(
        &datasets::mimi_content_multipart_3::BYTES[..],
        c,
        "mimi_content_multipart_3",
    );
    serde_bench_with_group_and_sample_cbor4ii::<
        datasets::mimi_content_multipart_3::MimiContent<cbor4ii::core::Value>,
    >(
        &datasets::mimi_content_multipart_3::BYTES[..],
        c,
        "mimi_content_multipart_3",
    );
    serde_bench_with_group_and_sample_ciborium::<
        datasets::mimi_content_multipart_3::MimiContentOwned<ciborium::Value>,
    >(
        &datasets::mimi_content_multipart_3::BYTES[..],
        c,
        "mimi_content_multipart_3",
    );
    serde_bench_with_group_and_sample_seabored::<log::BorrowLogs>(
        &datasets::log::BYTES[..],
        c,
        "log",
    );
    serde_bench_with_group_and_sample_cbor4ii::<log::BorrowLogs>(
        &datasets::log::BYTES[..],
        c,
        "log",
    );
    serde_bench_with_group_and_sample_ciborium::<log::Logs>(&datasets::log::BYTES[..], c, "log");
    serde_bench_with_group_and_sample_seabored::<mesh::Mesh>(&datasets::mesh::BYTES[..], c, "mesh");
    serde_bench_with_group_and_sample_cbor4ii::<mesh::Mesh>(&datasets::mesh::BYTES[..], c, "mesh");
    serde_bench_with_group_and_sample_ciborium::<mesh::Mesh>(&datasets::mesh::BYTES[..], c, "mesh");
    serde_bench_with_group_and_sample_seabored::<minecraft_savedata::BorrowPlayers>(
        &datasets::minecraft_savedata::BYTES[..],
        c,
        "minecraft_savedata",
    );
    serde_bench_with_group_and_sample_cbor4ii::<minecraft_savedata::BorrowPlayers>(
        &datasets::minecraft_savedata::BYTES[..],
        c,
        "minecraft_savedata",
    );
    serde_bench_with_group_and_sample_ciborium::<minecraft_savedata::Players>(
        &datasets::minecraft_savedata::BYTES[..],
        c,
        "minecraft_savedata",
    );
    serde_bench_with_group_and_sample_seabored::<mk48::Updates>(
        &datasets::mk48::BYTES[..],
        c,
        "mk48",
    );
    serde_bench_with_group_and_sample_cbor4ii::<mk48::Updates>(
        &datasets::mk48::BYTES[..],
        c,
        "mk48",
    );
    serde_bench_with_group_and_sample_ciborium::<mk48::Updates>(
        &datasets::mk48::BYTES[..],
        c,
        "mk48",
    );
}

criterion_group!(
    name = benches;
    config = Criterion::default();
    targets = perf_value_winnow_homebrew, perf_value, perf_serde
);
criterion_main!(benches);
