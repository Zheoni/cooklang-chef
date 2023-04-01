use criterion::{black_box, criterion_group, criterion_main, Criterion};

use cooklang::CooklangParser;

const TEST_RECIPE: &str = include_str!("./test_recipe.cook");

fn complete_recipe(c: &mut Criterion) {
    let mut group = c.benchmark_group("complete recipe");
    let parser = CooklangParser::default();
    let input = black_box(TEST_RECIPE);
    group.bench_function("cooklang-rs", |b| {
        b.iter(|| parser.parse(input, "benchmark"))
    });
}

fn just_metadata(c: &mut Criterion) {
    let mut group = c.benchmark_group("just metadata");
    let parser = CooklangParser::default();
    let input = black_box(TEST_RECIPE);
    group.bench_function("cooklang-rs", |b| b.iter(|| parser.parse_metadata(input)));
}

criterion_group!(benches, complete_recipe, just_metadata);
criterion_main!(benches);
