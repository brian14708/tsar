use bytes::{BufMut, BytesMut};
use criterion::BenchmarkId;
use criterion::Throughput;
use criterion::{criterion_group, criterion_main, Criterion};
use rand::Rng;

pub fn criterion_benchmark(c: &mut Criterion) {
    static MB: usize = 1024 * 1024;
    let mut rng = rand::thread_rng();
    let mut group = c.benchmark_group("compress");

    {
        let size = &(16 * MB);
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let mut buf = BytesMut::with_capacity(size);
            for _ in 0..(size / 4) {
                buf.put_f32_le(rng.gen());
            }
            let buf: Vec<u8> = buf.freeze().iter().cloned().collect();
            let compressor = tsar::Compressor::new(
                [
                    tsar::Stage::DeltaEncode(tsar::DeltaEncodeMode::DiffFloat32),
                    tsar::Stage::DataConvert(tsar::DataConvertMode::Float32ToBfloat16),
                    tsar::Stage::ColumnarSplit(tsar::ColumnarSplitMode::Bfloat16),
                ],
                tsar::CompressionMode::Zstd,
            );
            b.iter(|| {
                use std::io::Cursor;
                let mut buff = Cursor::new(&buf);
                let mut i = 0;
                compressor
                    .compress(&mut buff, || {
                        i += 1;
                        Ok(Box::new(std::fs::File::create(format!("/tmp/tsar.{}", i))?))
                    })
                    .unwrap();
            });
        });
    }
    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
