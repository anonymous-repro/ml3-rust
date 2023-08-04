/* Include our library */
use linked_lives::*;

/* the main function -> entry point of the programm */
fn main() {
    #[derive(Debug, Eq, PartialEq, Copy, Clone)]
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

    let grid_length = 64 * 64;

    #[derive(AgentEnum, Debug)]
    enum Agent {
        Person { status: HealthState },
    }

    add_uni_edge_multi!(Person network);

    /* We create the model object that holds the state of the system and propagates it */
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

    let agent_ids: Vec<_> = std::iter::once(model.create_new_agent(Agent::Person {
        status: HealthState::Infected,
    }))
    .chain(
        std::iter::repeat_with(|| {
            model.create_new_agent(Agent::Person {
                status: HealthState::Susceptible,
            })
        })
        .take(grid_length * grid_length - 1),
    )
    .collect();

    assert_eq!(agent_ids.len(), grid_length * grid_length);

    /* ADD NETWORK */
    for x in 0..(grid_length - 1) {
        for y in 0..(grid_length) {
            model.add_edge_network(
                agent_ids[x + y * grid_length],
                agent_ids[x + 1 + y * grid_length],
            );
        }
    }
    for x in 0..(grid_length) {
        for y in 0..(grid_length - 1) {
            model.add_edge_network(
                agent_ids[x + y * grid_length],
                agent_ids[x + (y + 1) * grid_length],
            );
        }
    }

    loop {
        model.step();
    }
}
