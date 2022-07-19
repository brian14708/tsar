use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};

fn criterion_benchmark(c: &mut Criterion) {
    const N: usize = 1024 * 1024;
    let mut group = c.benchmark_group("diff");
    group.throughput(Throughput::Bytes(N as u64));
    group.bench_function("byte", |b| {
        let src = vec![0; N];
        let targ = (0..N).map(|i| i as u8).collect::<Vec<_>>();
        b.iter(|| tsar::DataType::Byte.relative_error(black_box(&src), black_box(&targ)))
    });
    group.bench_function("i32", |b| {
        let src = vec![0; N];
        let targ = (0..N).map(|i| i as u8).collect::<Vec<_>>();
        b.iter(|| tsar::DataType::Int32.relative_error(black_box(&src), black_box(&targ)))
    });
    group.bench_function("u32", |b| {
        let src = vec![0; N];
        let targ = (0..N).map(|i| i as u8).collect::<Vec<_>>();
        b.iter(|| tsar::DataType::Uint32.relative_error(black_box(&src), black_box(&targ)))
    });
    group.bench_function("f32", |b| {
        let src = vec![0; N];
        let targ = vec![0; N];
        b.iter(|| tsar::DataType::Float32.relative_error(black_box(&src), black_box(&targ)))
    });
    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
