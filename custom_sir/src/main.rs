use custom_sir::*;
use std::fs::File;
use std::io::prelude::*;



fn main() {
    let mut s = Simulation::new(500, 500,false);
    let mut file = File::create("output.txt").unwrap();

    let t_max = 5000.;
    let num_out = 1000;
    for k in 0..num_out {
        while s.time < t_max / num_out as f64 * k as f64 {
            s.step();
        }
        writeln!(file, "{},{},{}", s.time, s.state_counts.num_of_infected,s.state_counts.num_of_susceptible);
    }
}
