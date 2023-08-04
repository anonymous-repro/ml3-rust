use custom_sir::*;
use rayon::prelude::*;
use std::fs::File;
use std::io::prelude::*;
use indicatif::ParallelProgressIterator;
use rayon::iter::{ParallelIterator, IntoParallelRefIterator};

fn main() {
    let t_max = 4000.;
    let num_out = 1000;
    let num_reps : u64 = match std::env::args().nth(1){
        Some(x) => x.parse().unwrap(),
        _ => 10000,
    };
    let results : Vec<(Vec<usize>,usize)> = (0..num_reps)
        .into_par_iter().progress_count(num_reps)
        .map(|_| {
            let mut s = Simulation::new(601, 601, false);
            let mut output = vec![];
            let mut stepcount = 0;
            for k in 0..num_out {
                let mut last_count = s.state_counts.num_of_infected;
                while s.time < t_max / num_out as f64 * k as f64 {
                    last_count = s.state_counts.num_of_infected;
                    s.step();
                    stepcount+=1;
                }
                output.push(last_count);
                //
            }
            (output,stepcount)
        }).collect();

    let mut file = File::create("reference.csv").unwrap();
    write!(file,"time,infcted");
    /*for k in 0..num_reps {
        write!(file,",{k}");
    }*/
    writeln!(file);
    let mut count_sum : usize = results.iter().map(|(_,k)|k).sum();
    for k in 0..num_out {
        write!(file, "{}", t_max / num_out as f64 * k as f64);
        /*for (v,_) in results.iter(){
            write!(file,",{}",v[k]);

        }*/
        let mean : f64= results.iter().map(|(x,_)|x[k] as f64).sum::<f64>()  / num_reps as f64 ;
        writeln!(file,",{}",mean);
    }

    println!("Average number of steps: {}",count_sum as f64 / results.len() as f64)


}
