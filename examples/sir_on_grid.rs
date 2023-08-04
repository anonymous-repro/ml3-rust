/* Include our library */
use linked_lives::*;

/* include the time library, needed for throughput data */
use std::time::Instant;

/* the main function -> entry point of the programm */
fn main() {
    for _ in 0..100 {
        /* This is a class we will later need to define the current health state of a person.
        we need this to provide equality functionality (Eq) as well as Debug formatability.
        These functions are added via the derive attributes */
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

        // Timer to measure runtime
        let now_with_initial = Instant::now();

        // the simulation runs on a square grid of 32x32 agents. The size is set here
        let grid_length = 32;

        /* this is where we specify the types of Agentsn in the model, as well as their attributes.
        There are no contstraints on this specification.
        The agents contain complex data structures etc.
        For our Model we would only need the Person and its Health state, but he others Are given as examples of what could be possible*/
        #[derive(AgentEnum, Debug)]
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
        for _ in 0..num_steps {
            model.step();
        }
        //run with IO until time 60
        //model.run_and_write_csv(60., "output.csv", 100);
    }

    /*let fintime = now.elapsed().as_secs_f64();
    let fintime_total = now_with_initial.elapsed().as_secs_f64();

    println!("FINAL: {:?}", model.agent_counts());

    println!("Duration: {}s (with init: {}s)", fintime, fintime_total);
    println!(
        "Throughput: {}/s (with init: {}/s)",
        model.stepnumber as f64 / fintime,
        model.stepnumber as f64 / fintime_total
    );
    println!("Total steps: {}", model.stepnumber);*/
}
