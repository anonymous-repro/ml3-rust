/* Include our library */
use linked_lives::*;

/* include the time library, needed for throughput data */
use std::time::Instant;

/* the main function -> entry point of the programm */
fn main() {
    for _ in 0..100 {
        #[derive(AgentEnum, Debug)]
        enum Agent {
            Person { id: usize },
            Oher {},
        }

        add_uni_edge_multi!(Person network);
        add_single_multi_edge!(Person aside Oher bside);

        let mut model = Agent::new_model();

        model.add_transition_for_Person(
            /* guard */ |ego| true,
            /* rate */ |_| 1e-4,
            /* effect */
            |ego| {
                let old_id: usize = ego.get_id();
                ego.set_id(old_id + 1);
                //ego.create_new_agent(Agent::Person { id: 100 });
            },
        );

        model.create_new_agent(Agent::Person { id: 12 });

        // do exacly as many steps as are present in the system.
        let num_steps = 100;
        for _ in 0..num_steps {
            model.step();
        }
    }

    // run with IO until time 60
    //model.run_and_write_csv(60., "output.csv", 100);

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

fn test_adding() {
    use crate::*;
    #[derive(AgentEnum, Debug, Hash, Clone)]
    enum Agent {
        Generator { count: usize },
        Generated { has_reacted: bool },
    }
    let mut model = Agent::new_model();

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
}
