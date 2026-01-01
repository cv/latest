use criterion::{Criterion, black_box, criterion_group, criterion_main};
use latest::{is_newer, parse_package_arg, sources};

fn bench_is_newer(c: &mut Criterion) {
    let mut group = c.benchmark_group("version_comparison");

    group.bench_function("is_newer_simple", |b| {
        b.iter(|| is_newer(black_box("1.0.0"), black_box("1.0.1")))
    });

    group.bench_function("is_newer_major", |b| {
        b.iter(|| is_newer(black_box("1.9.9"), black_box("2.0.0")))
    });

    group.bench_function("is_newer_equal", |b| {
        b.iter(|| is_newer(black_box("1.0.0"), black_box("1.0.0")))
    });

    group.bench_function("is_newer_complex", |b| {
        b.iter(|| is_newer(black_box("1.2.3-alpha.1"), black_box("1.2.3-alpha.2")))
    });

    group.bench_function("is_newer_long", |b| {
        b.iter(|| is_newer(black_box("10.20.30.40.50"), black_box("10.20.30.40.51")))
    });

    group.finish();
}

fn bench_parse_package_arg(c: &mut Criterion) {
    let mut group = c.benchmark_group("package_arg_parsing");

    group.bench_function("simple_name", |b| {
        b.iter(|| parse_package_arg(black_box("express")))
    });

    group.bench_function("with_prefix", |b| {
        b.iter(|| parse_package_arg(black_box("npm:express")))
    });

    group.bench_function("unknown_prefix", |b| {
        b.iter(|| parse_package_arg(black_box("unknown:express")))
    });

    group.bench_function("scoped_package", |b| {
        b.iter(|| parse_package_arg(black_box("@babel/core")))
    });

    group.bench_function("go_module", |b| {
        b.iter(|| parse_package_arg(black_box("go:github.com/spf13/cobra")))
    });

    group.finish();
}

fn bench_extract_version(c: &mut Criterion) {
    let mut group = c.benchmark_group("version_extraction");

    group.bench_function("simple", |b| {
        b.iter(|| sources::extract_version(black_box("1.2.3")))
    });

    group.bench_function("with_prefix", |b| {
        b.iter(|| sources::extract_version(black_box("v1.2.3")))
    });

    group.bench_function("in_text", |b| {
        b.iter(|| sources::extract_version(black_box("express version 4.18.2")))
    });

    group.bench_function("semver_prerelease", |b| {
        b.iter(|| sources::extract_version(black_box("v1.2.3-beta.1+build.123")))
    });

    group.finish();
}

fn bench_source_by_name(c: &mut Criterion) {
    let mut group = c.benchmark_group("source_lookup");

    group.bench_function("first_source", |b| {
        b.iter(|| sources::source_by_name(black_box("path")))
    });

    group.bench_function("middle_source", |b| {
        b.iter(|| sources::source_by_name(black_box("cargo")))
    });

    group.bench_function("last_source", |b| {
        b.iter(|| sources::source_by_name(black_box("pub")))
    });

    group.bench_function("unknown_source", |b| {
        b.iter(|| sources::source_by_name(black_box("invalid")))
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_is_newer,
    bench_parse_package_arg,
    bench_extract_version,
    bench_source_by_name
);
criterion_main!(benches);
