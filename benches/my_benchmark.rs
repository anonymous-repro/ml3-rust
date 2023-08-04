use criterion::*;
use linked_lives::*;

#[derive(Debug, Eq, PartialEq, Clone)]
enum HealthState {
    Susceptible,
    Infected,
    Recovered,
}

impl Default for HealthState {
    fn default() -> Self {
        HealthState::Susceptible
    }
}

#[derive(AgentEnum, Debug, Clone)]
enum Agent {
    Person { status: HealthState },
}
add_uni_edge_multi!(Person network);

fn generate_infinite_sir(grid_x: usize, grid_y: usize) -> Model<Agent, 1> {
    let number_of_agents = grid_x * grid_y;
    if number_of_agents > 10000 * 10000 {
        panic!("Are you sure? That needs a lot of memory.");
    }
    let mut model = Agent::new_model();
    model.add_transition_for_Person(
        /* guard */ |ego| ego.get_status() == HealthState::Susceptible,
        /* rate */
        |ego| {
            ego.network()
                .filter(|alter| alter.get_status() == HealthState::Infected)
                .count() as f64
                + 1e-6 /* for random infection */
        },
        /* effect */ |ego| ego.set_status(HealthState::Infected),
    );
    model.add_transition_for_Person(
        /* guard */ |ego| ego.get_status() == HealthState::Infected,
        /* rate */ |_| 1e-2,
        /* effect */ |ego| ego.set_status(HealthState::Recovered),
    );
    model.add_transition_for_Person(
        /* guard */ |ego| ego.get_status() == HealthState::Recovered,
        /* rate */ |_| 1e-4,
        /* effect */ |ego| ego.set_status(HealthState::Susceptible),
    );
    /*model.add_transition_for_Person(
        /* guard */ |ego| *ego.get_status() == HealthState::Susceptible,
        /* rate */ |_| 1e-6,
        /* effect */ |ego| ego.set_status(HealthState::Infected),
    );*/

    /*in the next part we ADD AGENTS. This is rather boring an technical. We also setup the grid network. */
    let agent_ids: Vec<_> = std::iter::once(model.create_new_agent(Agent::Person {
        status: HealthState::Infected,
    }))
    .chain(
        std::iter::repeat_with(|| {
            model.create_new_agent(Agent::Person {
                status: HealthState::Susceptible,
            })
        })
        .take(number_of_agents - 1),
    )
    .collect();

    assert_eq!(agent_ids.len(), number_of_agents);

    /* ADD NETWORK */
    for x in 0..(grid_x - 1) {
        for y in 0..(grid_y) {
            model.add_edge_network(agent_ids[x + y * grid_x], agent_ids[x + 1 + y * grid_x]);
        }
    }
    for x in 0..(grid_x) {
        for y in 0..(grid_y - 1) {
            model.add_edge_network(agent_ids[x + y * grid_x], agent_ids[x + (y + 1) * grid_x]);
        }
    }
    model
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("Infinite Sir");
    group.throughput(Throughput::Elements(1));

    let multiplicator = 8.;
    let mut grid_x = 1;
    let mut grid_y = 1;

    while grid_x * grid_y < 5000 * 5000 {
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

        println!("Measure {}x{}", grid_x, grid_y);
        let mut m = generate_infinite_sir(grid_x, grid_y);
        group.bench_function(format!("infinite sir N={:08}", grid_x * grid_y), |b| {
            b.iter(|| m.step())
        });
    }
    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
