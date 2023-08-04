# On the Modeling Language for Linked Lives - Rust Edition

This readme is intended as a starting point to understand the basics and use the rust implementation of ML3.
Knowledge of the host language ([rust](https://www.rust-lang.org/learn)) is a requirement for all but the most simple models. 


## Technical requierments

In this version, ML3 is implemented as an embedded Domain Specific Language in the rust programming language.
Rust is available on most platforms. To install the rust toolchain (including the dependency manager *cargo*), visit https://rustup.rs/.
The project is intended to follow the common design concepts of a regular rust project.
Additional information on rust can be found at https://www.rust-lang.org/.

### Setup
To run the program use `cargo run --release --example sir_on_grid`

## A basic sir program

An implementation of the SIR infection model can be found in the main (`src/examples/sir_on_grid.rs`) of the repository.
Let's walk through this implementation to understand how this simulator works.
First, we need to understand the concept of the automatic *derive* in rust.
A struct in rust is similar to a class in other languages.
By itself, such a struct does not have many features.
We can, however, ask the program to generate certain functionality for us.
We can see this for the first time in the HealthState, where for example, the functionality needed for Equality is generated.

Later we see the `derive(AgentEnum)` clause. This is where most of the implementation lies.
This macro takes the specified agents and adds a layer on top of them that allows the automatic tracking of all attribute accesses and changes.
```rust
    /* This is a class we will later need to define the current health state of a person.
    We need this to provide equality functionality (Eq) as well as Debug format ability.
    These functions are added via the derive attributes */
    #[derive(Debug, Eq, PartialEq)]
    enum HealthState {
        Susceptible,
        Infected,
        Recovered,
    }

    /* this is where we specify the types of Agents in the model, as well as their attributes.
    There are no constraints on this specification.
    The agents contain complex data structures etc.
    For our model, we would only need the Person and its Health state, but  others Are given as examples of what could be possible*/
    #[derive(AgentEnum, Debug)]
    enum Agent {
        Person {
            status: HealthState,
        },
        // Below not needed just examples
        House {
            x_coordinate: u64,
            y_coordinate: u64,
            name: String,
        },
        Cat {
            decision_proccess_neural_network: Vec<f64>,
        },
    }

    /* Next, we add the edges to the system.
    In this case, we connect persons with one another.
    The syntax here is still preliminary due to time constraints, but a more readable syntax is planned */
    add_uni_edge_multi!(Person network);

    /* We create the model object that holds the state of the system and propagates it */
    let mut model = Agent::new_model();


    /* In the next section, we specify the TRANSITIONS */

    /* Recover from illness. They are specified as the known guard-rate-effect triple of ml3.
    We use lambda functions, where the input is between the vertical bars | input| followed by the function.
    We can see how the derive macro earlier has now created the get_status() function for us.
    This is used for the simulator to track the access to the attribute and update the internal dependency tree.
    If the status of the Person were to change, this transition would be updated.
    */
    model.add_transition_for_Person(
        /* guard */ |ego| *ego.get_status() == HealthState::Infected,
        /* rate */ |_| 1e-3,
        /* effect */ |ego| ego.set_status(HealthState::Recovered),
    );

    /* Infect through a neighbor.
    Here we see another advantage of the embedded language: the included functionality!
    We return the network as an iterator which we then filter for infected and count to get the rate.
    */
    model.add_transition_for_Person(
        /* guard */ |ego| *ego.get_status() == HealthState::Susceptible,
        /* rate */
        |ego| {
            ego.network()
                .filter(|alter| *alter.get_status() == HealthState::Infected)
                .count() as f64
        },
        /* effect */ |ego| ego.set_status(HealthState::Infected),
    );
```

## Migration Model
The migration model and related experiments can be found in [examples/routes.rs](examples/routes.rs).

## Reproducing results from the paper
The scripts as used to generate the figures from the paper can be found in `perf_plot.py` and `reproduce.py`