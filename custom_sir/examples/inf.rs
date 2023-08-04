use custom_sir::*;
fn main() {
    let mut s = Simulation::new(20000, 20000, false);
    for _ in 0..100000000 {
        //println!("{:?}",s.propensity);
        //println!("{}", s);
        s.step();
    }
}
