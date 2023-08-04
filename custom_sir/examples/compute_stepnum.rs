use custom_sir::*;
use rayon::prelude::*;
use std::fs::File;
use std::io::prelude::*;

fn main() {
    let t_max = 4000.;
    let num_out = 1000;
    let num_reps = 20;//000;
    let mut res: Vec<f64> = vec![];
    let mut xvals = vec![];
    for pow in 1..12 {
        let xmax = 2_usize.pow(pow);
        let stepsum : f64 = (0..num_reps)
            .into_par_iter()
            .map(|_| {
                let mut s = Simulation::new(xmax,xmax, false);
                let mut stepcount = 0;
                for k in 0..num_out {
                    let mut last_count = s.state_counts.num_of_infected;
                    while s.time < t_max / num_out as f64 * k as f64 {
                        s.step();
                        stepcount += 1;
                    }
                    //
                }
                stepcount
            }).sum::<usize>() as f64 / num_reps as f64;
        xvals.push(xmax);
        res.push(stepsum);
        println!("{:?} x {:?}",res,xvals);
        println!("{} {stepsum}",xmax);
    }


}
