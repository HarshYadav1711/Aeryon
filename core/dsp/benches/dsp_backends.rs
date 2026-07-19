//! Criterion benchmarks for Rust and optional C++ DSP kernel backends.
//!
//! Run with:
//! `cargo bench -p aeryon-dsp --bench dsp_backends`
//! or with native kernels:
//! `cargo bench -p aeryon-dsp --features cpp-dsp --bench dsp_backends`
//!
//! Results are machine-specific; record them under `benchmarks/results/` with
//! environment metadata. Do not treat a single run as a global ranking.

use std::hint::black_box;
use std::sync::Arc;

use aeryon_dsp::{
    DspBackendKind, DspKernelBackend, MotionEnergyInput, RustKernelBackend, create_backend,
};
use criterion::{Criterion, Throughput, criterion_group, criterion_main};

fn fill_csi(frames: usize, sc: usize) -> (Vec<f32>, Vec<f32>) {
    let n = frames * sc;
    let mut real = Vec::with_capacity(n);
    let mut imag = Vec::with_capacity(n);
    for t in 0..frames {
        for k in 0..sc {
            let phase = (t as f32) * 0.21 + (k as f32) * 0.07;
            real.push(phase.cos() * (1.0 + 0.01 * k as f32));
            imag.push(phase.sin() * (0.5 + 0.01 * t as f32));
        }
    }
    (real, imag)
}

fn fill_signal(len: usize) -> Vec<f64> {
    (0..len)
        .map(|n| {
            let x = n as f64;
            0.25 + (0.35 * x).sin() + 0.05 * (0.11 * x).cos()
        })
        .collect()
}

fn preprocess_excluding_fft(
    backend: &dyn DspKernelBackend,
    real: &[f32],
    imag: &[f32],
    frames: usize,
    sc: usize,
) {
    let motion = backend
        .motion_energy(MotionEnergyInput {
            real_samples: real,
            imag_samples: imag,
            frame_count: frames,
            subcarrier_count: sc,
        })
        .expect("motion");
    let _windowed = backend.center_and_apply_hann(&motion).expect("center_hann");
}

fn rust_backend() -> RustKernelBackend {
    RustKernelBackend
}

fn optional_cpp() -> Option<Arc<dyn DspKernelBackend>> {
    if !DspBackendKind::Cpp.is_compiled() {
        return None;
    }
    create_backend(DspBackendKind::Cpp).ok()
}

fn bench_motion_energy(c: &mut Criterion) {
    let cases = [
        ("small_16x16", 16usize, 16usize),
        ("medium_64x64", 64, 64),
        ("large_256x128", 256, 128),
    ];
    let rust = rust_backend();
    let cpp = optional_cpp();

    for (name, frames, sc) in cases {
        let (real, imag) = fill_csi(frames, sc);
        let mut group = c.benchmark_group(format!("motion_energy/{name}"));
        group.throughput(Throughput::Elements(((frames - 1) * sc) as u64));
        group.warm_up_time(std::time::Duration::from_millis(500));

        group.bench_function("rust", |b| {
            b.iter(|| {
                let out = rust
                    .motion_energy(MotionEnergyInput {
                        real_samples: black_box(&real),
                        imag_samples: black_box(&imag),
                        frame_count: frames,
                        subcarrier_count: sc,
                    })
                    .expect("rust motion");
                black_box(out);
            });
        });

        if let Some(cpp) = cpp.as_ref() {
            group.bench_function("cpp", |b| {
                b.iter(|| {
                    let out = cpp
                        .motion_energy(MotionEnergyInput {
                            real_samples: black_box(&real),
                            imag_samples: black_box(&imag),
                            frame_count: frames,
                            subcarrier_count: sc,
                        })
                        .expect("cpp motion");
                    black_box(out);
                });
            });
        }

        group.finish();
    }
}

fn bench_center_hann(c: &mut Criterion) {
    let lengths = [16usize, 64, 256, 1024];
    let rust = rust_backend();
    let cpp = optional_cpp();

    for len in lengths {
        let signal = fill_signal(len);
        let mut group = c.benchmark_group(format!("center_hann/len_{len}"));
        group.throughput(Throughput::Elements(len as u64));
        group.warm_up_time(std::time::Duration::from_millis(500));

        group.bench_function("rust", |b| {
            b.iter(|| {
                let out = rust
                    .center_and_apply_hann(black_box(&signal))
                    .expect("rust center");
                black_box(out);
            });
        });

        if let Some(cpp) = cpp.as_ref() {
            group.bench_function("cpp", |b| {
                b.iter(|| {
                    let out = cpp
                        .center_and_apply_hann(black_box(&signal))
                        .expect("cpp center");
                    black_box(out);
                });
            });
        }

        group.finish();
    }
}

fn bench_preprocess_excluding_fft(c: &mut Criterion) {
    let cases = [
        ("small_16x16", 16usize, 16usize),
        ("medium_64x64", 64, 64),
        ("large_256x128", 256, 128),
    ];
    let rust = rust_backend();
    let cpp = optional_cpp();

    for (name, frames, sc) in cases {
        let (real, imag) = fill_csi(frames, sc);
        let mut group = c.benchmark_group(format!("preprocess_no_fft/{name}"));
        group.warm_up_time(std::time::Duration::from_millis(500));

        group.bench_function("rust", |b| {
            b.iter(|| {
                preprocess_excluding_fft(&rust, black_box(&real), black_box(&imag), frames, sc);
            });
        });

        if let Some(cpp) = cpp.as_ref() {
            group.bench_function("cpp", |b| {
                b.iter(|| {
                    preprocess_excluding_fft(
                        cpp.as_ref(),
                        black_box(&real),
                        black_box(&imag),
                        frames,
                        sc,
                    );
                });
            });
        }

        group.finish();
    }
}

fn bench_ffi_overhead_small(c: &mut Criterion) {
    // Tiny inputs emphasize call/marshaling cost relative to arithmetic.
    let frames = 2usize;
    let sc = 1usize;
    let real = [1.0_f32, 0.0];
    let imag = [0.0_f32, 1.0];
    let signal = [1.0_f64, 2.0, 3.0, 4.0];
    let rust = rust_backend();
    let cpp = optional_cpp();

    let mut group = c.benchmark_group("ffi_overhead_small");
    group.warm_up_time(std::time::Duration::from_millis(750));

    group.bench_function("motion_energy/rust", |b| {
        b.iter(|| {
            let out = rust
                .motion_energy(MotionEnergyInput {
                    real_samples: black_box(&real),
                    imag_samples: black_box(&imag),
                    frame_count: frames,
                    subcarrier_count: sc,
                })
                .expect("rust");
            black_box(out);
        });
    });

    group.bench_function("center_hann/rust", |b| {
        b.iter(|| {
            let out = rust
                .center_and_apply_hann(black_box(&signal))
                .expect("rust");
            black_box(out);
        });
    });

    if let Some(cpp) = cpp.as_ref() {
        group.bench_function("motion_energy/cpp", |b| {
            b.iter(|| {
                let out = cpp
                    .motion_energy(MotionEnergyInput {
                        real_samples: black_box(&real),
                        imag_samples: black_box(&imag),
                        frame_count: frames,
                        subcarrier_count: sc,
                    })
                    .expect("cpp");
                black_box(out);
            });
        });

        group.bench_function("center_hann/cpp", |b| {
            b.iter(|| {
                let out = cpp.center_and_apply_hann(black_box(&signal)).expect("cpp");
                black_box(out);
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_motion_energy,
    bench_center_hann,
    bench_preprocess_excluding_fft,
    bench_ffi_overhead_small,
);
criterion_main!(benches);
