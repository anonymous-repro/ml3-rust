use crate::simulation::StepRunResult;
use std::time::Instant;

#[test]
fn test_derive() {
    use crate::*;

    /*enum dummy {
        t { x2: usize },
    }*/

    #[derive(AgentEnum, Debug, Hash, Clone)]
    enum Agent {
        Person { x: usize, y: String },
        House(),
        Cat { t: i8 },
    }
    add_uni_edge_multi!(Person network);

    let mut model = Agent::new_model();

    model.add_transition_for_Person(
        /* guard */ |_ego| true,
        /* rate */
        |_ego| /*12.4 * *ego.get_x() as f64*/ 1.2,
        /* effect */
        |ego| {
            println!("effect");
            let old: usize = ego.get_x().clone();
            ego.set_x(old + 1);
            //let mut b = &mut ego.get_x_mut();
            //*b = 3;
        },
    );
    model.create_new_agent(Agent::Cat { t: 2 });
    let p1 = model.create_new_agent(Agent::Person {
        x: 2,
        y: "Test".to_string(),
    });
    let p2 = model.create_new_agent(Agent::Person {
        x: 100,
        y: "Test2".to_string(),
    });

    model.add_edge_network(p1, p2);

    println!("{:?}", model.state.agents.borrow());
    model.step();
    println!("{:?}", model.state.agents.borrow());
    model.step();
    println!("{:?}", model.state.agents.borrow());

    //let y = Cat {};
    /*match Agent::House() {
        Agent::Person { .. } => {}
        Agent::House() => {}
        Agent::Cat { .. } => {}
    }*/
    //y.get_t();

    //k.dummy();
}

#[test]
fn sir() {
    let now_with_initial = Instant::now();
    let grid_length = 3;
    use crate::*;

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

    #[derive(AgentEnum, Debug)]
    enum Agent {
        Person { status: HealthState },
    }
    add_uni_edge_multi!(Person network);

    let mut model = Agent::new_model();

    /* Recover */
    model.add_transition_for_Person(
        /* guard */ |ego| ego.get_status() == HealthState::Infected,
        /* rate */ |_| 1e-5,
        /* effect */ |ego| ego.set_status(HealthState::Recovered),
    );

    /* Infect */
    model.add_transition_for_Person(
        /* guard */ |ego| ego.get_status() == HealthState::Susceptible,
        /* rate */
        |ego| {
            ego.network()
                .filter(|alter| alter.get_status() == HealthState::Infected)
                .count() as f64
        },
        /* effect */ |ego| ego.set_status(HealthState::Infected),
    );

    /* ADD AGENTS */
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

    /* RUN SIMULATION */
    //println!("CONNECTION: {:?}", model.state.edges.borrow());
    let now = Instant::now();
    //println!("INITAL: {:?}", model.agent_counts());
    let num_steps = (agent_ids.len() - 1) * 2 + 1;
    for _ in 0..num_steps {
        assert!(model.step().success());
    }
    let fintime = now.elapsed().as_secs_f64();
    let fintime_total = now_with_initial.elapsed().as_secs_f64();

    println!("FINAL: {:?}", model.agent_counts());

    println!("Duration: {}s (with init: {}s)", fintime, fintime_total);
    println!(
        "Throughput: {}/s (with init: {}/s)",
        model.stepnumber as f64 / fintime,
        model.stepnumber as f64 / fintime_total
    );
    println!("Total steps: {}", model.stepnumber);
}

#[test]
fn test_timing() {
    use crate::*;

    /* This is a class we will later need to define the current health state of a person.
    we need this to provide equality functionality (Eq) as well as Debug formatability.
    These functions are added via the derive attributes */
    #[derive(Debug, Eq, PartialEq, Copy, Clone, Serialize)]
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

    // Timer to measure runtime
    let now_with_initial = Instant::now();

    // the simulation runs on a square grid of 32x32 agents. The size is set here
    let grid_length = 10;

    /* this is where we specify the types of Agentsn in the model, as well as their attributes.
    There are no contstraints on this specification.
    The agents contain complex data structures etc.
    For our Model we would only need the Person and its Health state, but he others Are given as examples of what could be possible*/
    #[derive(Debug, AgentEnum, Serialize)]
    enum Agent {
        Person {
            status: HealthState,
        },
        // Below not needed just examples (underscore _ to avoid warings)
        _House {
            _x_coordinate: u64,
            _y_coordinate: u64,
            _name: String,
        },
        _Cat {
            _decision_proccess_neural_network: Vec<f64>,
        },
    }

    /* Next we add the edges the system.
    In this case we connect persons among one another.
    The syntax here is is still preliminary due to time constains, but a more readable systax is planned */
    add_uni_edge_multi!(Person network);

    /* We create the model object that holds the state of the system and propagates it */
    let mut model = Agent::new_model();

    /* In the next section we specify the TRANSITIONS */

    /* Recover from illnes. Specified as the known guard-rate-effect triple of ml3.
    We use lambad functions, where the input is between the vertical bars | input| followed by the function.
    Whe can see how the derive macro earlier has now created the get_status() function for us.
    This is used for the simulator to track the access to the attribute and update the internal dependency tree.
    If the status of the Person where to change, this transiton would be updated.
    */
    model.add_transition_for_Person(
        /* guard */ |ego| ego.get_status() == HealthState::Infected,
        /* rate */ |_| 1e-4,
        /* effect */ |ego| ego.set_status(HealthState::Recovered),
    );

    /* Infect through a neighbor.
    Here we see another advantage of the empedded language: the included functionality!
    We return the network as an iterator which we then filter for infected and count to get the rate.
    */
    model.add_transition_for_Person(
        /* guard */ |ego| ego.get_status() == HealthState::Susceptible,
        /* rate */
        |ego| {
            ego.network()
                .filter(|alter| alter.get_status() == HealthState::Infected)
                .count() as f64
        },
        /* effect */ |ego| ego.set_status(HealthState::Infected),
    );

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

    /* RUN SIMULATION with timing and no IO*/
    let now = Instant::now();
    // do exacly as many steps as are present in the system.
    let num_steps = (agent_ids.len() - 1) * 2 + 1;
    for _ in 0..num_steps / 2 {
        model.step();
    }
    assert!(model.time > 1.);
    assert!(model.time < 1000.);
    for _ in 0..num_steps / 2 + 1 {
        model.step();
    }
    assert!(model.time > 1000.1);
    assert!(model.time < 100000.);
    assert!(!model.step().success());
}

#[test]
fn test_adding() {
    use crate::*;
    #[derive(AgentEnum, Debug, Hash, Clone)]
    enum Agent {
        Generator { count: usize },
        Generated { has_reacted: bool },
    }
    let mut model = Agent::new_model();

    add_multi_multi_edge!(Generator main2 Generated dependent2);
    add_single_multi_edge!(Generator main Generated dependent);

    model.add_transition_for_Generator(
        /* guard */ |ego| true,
        /* rate */ |_| 1e-3,
        /* effect */
        |ego| {
            let old_id: usize = ego.get_count();
            ego.set_count(old_id + 1);
            let mut new_agent = ego.create_new_Generated(Agent::Generated { has_reacted: false });
            assert_eq!(new_agent.get_has_reacted(), false);
            if ego.get_count() <= 1 {
                new_agent.set_has_reacted(true);
            }
            new_agent.set_main(&ego);
            assert_eq!(ego.dependent().count(), ego.get_count());
        },
    );

    model.add_transition_for_Generated(
        |ego| !ego.get_has_reacted(),
        |_| 1e5,
        |ego| ego.set_has_reacted(true),
    );
    model.create_new_agent(Agent::Generator { count: 0 });

    for _ in 0..12 {
        model.step();
    }
    // {"Generated { has_reacted: true }": 5, "Generated { has_reacted: false }": 1, "Generator { count: 6 }": 1}
    println!("FINAL: {:?}", model.agent_counts());
    let counts = model.agent_counts();
    assert_eq!(*counts.get("Generated { has_reacted: true }").unwrap(), 6);
    assert_eq!(*counts.get("Generator { count: 7 }").unwrap(), 1);

    //model.print_full_graph();
}
