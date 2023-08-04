use custom_sir::*;
use criterion::*;


fn criterion_benchmark(c: &mut Criterion) {
    for seq in [false,true] {
        let name_xtra = match seq {
            true => "seq",
            false => "tree",
        };
        let mut group = c.benchmark_group(name_xtra);
        group.throughput(Throughput::Elements(1));

        let multiplicator = 2.;
        let mut grid_x = 1;
        let mut grid_y = 1;

        while grid_x * grid_y < 10000 * 10000 {
            let last_number_of_agents = grid_x * grid_y;
            let grid = (last_number_of_agents as f64 * multiplicator)
                .sqrt()
                .floor() as usize;
            grid_x = grid;
            grid_y = grid;
            if grid * grid <= last_number_of_agents {
                grid_x += 1;
            }
            if grid_x * grid_y <= last_number_of_agents {
                grid_y += 1;
            }

            if grid_x < 2 || grid_y < 2 {
                println!("skip, size to small {} x {}",grid_x,grid_y);
                continue
            }

            println!("Measure {}x{}", grid_x, grid_y);
            let mut m = custom_sir::Simulation::new(grid_x, grid_y,seq);

            group.bench_function(format!("{} N={:010}", name_xtra, grid_x * grid_y), |b| {
                b.iter(|| m.step())
            });
        }
        group.finish();
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
