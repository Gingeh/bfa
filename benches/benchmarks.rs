use std::{num::NonZeroUsize, time::Duration};

use bfa::{Program, Table};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

const PROGRAMS: &[(&str, NonZeroUsize)] = &[
    ("+[>,,.<]", NonZeroUsize::new(2).unwrap()),
    (",>,[-<->]<[>.,<]", NonZeroUsize::new(2).unwrap()),
    ("+[>,]+[[.,]+]", NonZeroUsize::new(3).unwrap()),
    (">+[>.,[>]<<]", NonZeroUsize::new(3).unwrap()),
    ("+[>.,[<->[-]]<[,]+]", NonZeroUsize::new(2).unwrap()),
    (
        ",>>+[.[,<<[->+>-<<]>[-<+>]>]+]",
        NonZeroUsize::new(3).unwrap(),
    ),
    (",[-[-]]]", NonZeroUsize::new(1).unwrap()),
];

pub fn build_min_dot(c: &mut Criterion) {
    let mut group = c.benchmark_group("build_min_dot");
    group.measurement_time(Duration::from_secs(10));
    for program in PROGRAMS {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{program:?}")),
            program,
            |b, &program| {
                b.iter(|| {
                    let program = Program::new(program.0, program.1);
                    let mut table = Table::build(&program);
                    table.minimize();
                    black_box(table.dot());
                });
            },
        );
    }
    group.bench_function(BenchmarkId::from_parameter("all".to_string()), |b| {
        b.iter(|| {
            for (code, cells) in PROGRAMS.iter().map(black_box) {
                let program = Program::new(code, *cells);
                let mut table = Table::build(&program);
                table.minimize();
                black_box(table.dot());
            }
        });
    });
    group.finish();
}

criterion_group!(benches, build_min_dot);
criterion_main!(benches);
