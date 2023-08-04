extern crate core;

use crate::State::{Infected, Susceptible};
use rand::prelude::*;
use rand::rngs::*;
use rand_distr::Distribution;
use rand_distr::*;
use std::fmt::{write, Display, Formatter};
use assert_float_eq::*;

const INFECTION_RATE: f64 = 5.0e-2;
const RECOVERY_RATE: f64 = 1e-2;
const RANDOM_INFECT: f64 = 2e-6;
const IMUNITY_DECAY: f64 = 1.5e-3;
const INITIALLY_INFECTED : f64 = 1e-3;
//pub const use_slow_first_reaction_method: bool = true;
#[derive(Clone,Eq, PartialEq)]
enum State {
    Susceptible,
    Infected,
    Recovered,
}

impl Display for State {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Susceptible => "S",
                State::Infected => "I",
                State::Recovered => "R",
            }
        )
    }
}

#[derive(Copy, Clone)]
enum InfectionCountChange {
    Add,
    Remove,
}

#[derive(Debug)]
pub struct PropensityBuildup {
    pub num_of_infected: usize,
    pub num_of_susceptible: usize,
    infected_near_susceptible: usize,
    pub num_of_recoverd: usize,
}
impl PropensityBuildup {
    fn get_propensity_sum(&self) -> f64 {
        self.num_of_infected as f64 * RECOVERY_RATE
            + self.num_of_susceptible as f64 * (RANDOM_INFECT)
            + IMUNITY_DECAY * self.num_of_recoverd as f64
            + self.infected_near_susceptible as f64 * INFECTION_RATE
    }
}

pub struct Simulation {
    size: (usize, usize),
    state: Vec<State>,
    infected_neighbors: Vec<usize>,
    propensity: Vec<f64>,
    propensitiy_tree: Vec<f64>,
    stepnumber: usize,

    use_slow_first_reaction_method: bool,

    pub time: f64,
    rng: rand::rngs::SmallRng,
    pub state_counts: PropensityBuildup,
}

impl Simulation {}

impl Display for Simulation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for x in 0..self.size.0 {
            for y in 0..self.size.1 {
                let idx = self.xy_2_idx((x, y));
                write!(f, "{}{} ", self.state[idx], self.propensity[idx])?
            }
            write!(f, "\n")?
        }
        write!(f, "")
    }
}

impl Simulation {
    pub fn new(x: usize, y: usize,sequential: bool) -> Simulation {
        if x <2 || y < 2{
            panic!("system size to small! {}x{}",x,y);
        }
        let mut s = Simulation {
            size: (x, y),
            state: vec![Susceptible; x * y],
            infected_neighbors: vec![0; x * y],
            propensity: vec![RANDOM_INFECT; x * y],
            propensitiy_tree: vec![0.; x * y],
            stepnumber: 0,
            use_slow_first_reaction_method: sequential,
            time: 0.0,
            rng: rand::rngs::SmallRng::from_entropy(),
            state_counts: PropensityBuildup {
                num_of_infected: 0,
                num_of_susceptible: x * y,
                infected_near_susceptible: 0,
                num_of_recoverd: 0,
            },
        };
        if !s.use_slow_first_reaction_method {
            s.rebuild_tree();
        }
        for _ in 0..(s.state.len()as f64*INITIALLY_INFECTED) as usize{
            let mut idx;
            loop {
                idx = s.rng.gen_range(0..s.state.len());
                if s.state[idx] == Susceptible{
                    break
                }
            }
            s.fire_at(idx);
        }
        //s.fire_at(s.xy_2_idx((x / 2, y / 2)));
        s
    }
    fn set_propensity(&mut self, idx: usize, value: f64) {
        match self.use_slow_first_reaction_method {
            true => {
                self.propensity[idx] = value;
            }
            false => {
                let old_val = self.propensity[idx];
                self.propensity[idx] = value;
                self.update_tree(idx);
            }
        }
    }
    fn compute_infection_rate(&mut self, idx: usize) {
        self.set_propensity(
            idx,
            self.infected_neighbors[idx] as f64 * INFECTION_RATE + RANDOM_INFECT,
        );
    }
    fn update_infected_count_at(&mut self, idx_at: usize, change_type: InfectionCountChange) {
        match change_type {
            InfectionCountChange::Add => {
                self.infected_neighbors[idx_at] += 1;
            }
            InfectionCountChange::Remove => {
                self.infected_neighbors[idx_at] -= 1;
            }
        }

        match self.state[idx_at] {
            State::Susceptible => {
                self.compute_infection_rate(idx_at);
                match change_type {
                    InfectionCountChange::Add => {
                        self.state_counts.infected_near_susceptible += 1;
                    }
                    InfectionCountChange::Remove => {
                        self.state_counts.infected_near_susceptible -= 1;
                    }
                }
            }
            _ => {}
        }
    }
    fn fire_at(&mut self, idx: usize) {
        match self.state[idx].clone() {
            State::Susceptible => {
                self.state_counts.num_of_susceptible -= 1;
                self.state_counts.num_of_infected += 1;
                self.state_counts.infected_near_susceptible -= self.infected_neighbors[idx];
                self.state[idx] = State::Infected;
                self.update_infected_count(idx, InfectionCountChange::Add);
                self.set_propensity(idx, RECOVERY_RATE);
            }
            State::Infected => {
                self.state_counts.num_of_infected -= 1;
                self.state_counts.num_of_recoverd += 1;
                self.state[idx] = State::Recovered;
                self.update_infected_count(idx, InfectionCountChange::Remove);
                self.set_propensity(idx, IMUNITY_DECAY);
            }
            State::Recovered => {
                self.state_counts.num_of_recoverd -= 1;
                self.state_counts.num_of_susceptible += 1;
                self.state_counts.infected_near_susceptible += self.infected_neighbors[idx];
                self.state[idx] = State::Susceptible;
                self.compute_infection_rate(idx);
            }
        }
    }
    const fn idx_2_xy(&self, idx: usize) -> (usize, usize) {
        (idx / self.size.0, idx % self.size.0)
    }
    const fn xy_2_idx(&self, xy: (usize, usize)) -> usize {
        (xy.0) * self.size.0 + xy.1
    }
    fn update_infected_count(&mut self, idx: usize, change_type: InfectionCountChange) {
        let (x, y) = self.idx_2_xy(idx);
        if x > 0 {
            self.update_infected_count_at(self.xy_2_idx((x - 1, y)), change_type);
        }
        if x + 1 < self.size.1 {
            self.update_infected_count_at(self.xy_2_idx((x + 1, y)), change_type);
        }
        if y > 0 {
            self.update_infected_count_at(self.xy_2_idx((x, y - 1)), change_type);
        }
        if y + 1 < self.size.0 {
            //println!("{} -> ({},{})",idx,x,y);
            //println!("{} +1 < {} -> {}",y,self.size.1,self.xy_2_idx((x , y+1)));
            self.update_infected_count_at(self.xy_2_idx((x, y + 1)), change_type);
        }
    }

    fn update_tree(&mut self, mut idx: usize) {
        if self.use_slow_first_reaction_method {
            panic!("No need to update tree with this")
        }
        loop {
            let left_child = idx * 2 + 1;
            let right_child = idx * 2 + 2;

            if right_child < self.propensitiy_tree.len() {
                self.propensitiy_tree[idx] = self.propensitiy_tree[right_child] + self.propensitiy_tree[left_child];
            } else if left_child < self.propensitiy_tree.len() {
                self.propensitiy_tree[idx] = self.propensitiy_tree[left_child];
            } else {
                self.propensitiy_tree[idx] = 0.;
            }
            self.propensitiy_tree[idx]+= self.propensity[idx];
            if idx == 0 {
                break;
            }
            idx = (idx - 1) / 2; // parent node
        }
    }
    fn get_from_tree(&self, mut threshold: f64) -> Option<usize> {
        let mut idx = 0;
        loop {
            let y = match self.propensity.get(idx) {
                None => break,
                Some(&y) => y,
            };
            if y >= threshold {
                return Some(idx);
            } else {
                threshold -= y;
            }

            let left_child = idx * 2 + 1;
            let right_child = idx * 2 + 2;

            match self.propensitiy_tree.get(left_child) {
                None => break,
                Some(&y) => {
                    if y >= threshold {
                        idx = left_child;
                    } else {
                        threshold -= y;
                        idx = right_child;
                    }
                }
            }
        }
        None
    }

    fn rebuild_tree(&mut self) {
        if self.use_slow_first_reaction_method {
            panic!("Ne need to do this!");
        }
        let mut sum = 0.;
        assert_eq!(self.propensitiy_tree.len(),self.propensity.len());

        for (idx, p) in self.propensity.iter().enumerate().rev(){
            let left_child = idx * 2 + 1;
            let right_child = idx * 2 + 2;
            if right_child < self.propensitiy_tree.len() {
                self.propensitiy_tree[idx] = self.propensitiy_tree[right_child] + self.propensitiy_tree[left_child];
            } else if left_child < self.propensitiy_tree.len() {
                self.propensitiy_tree[idx] = self.propensitiy_tree[left_child];
            } else {
                self.propensitiy_tree[idx] = 0.;
            }
            sum += p;
            self.propensitiy_tree[idx] += p;
        }
        assert_float_relative_eq!(sum,self.propensitiy_tree[0]);
        assert_float_relative_eq!(sum,self.state_counts.get_propensity_sum());

        //self.propensitiy_tree = vec![0.; self.propensitiy_tree.len()];
        /*let copyied_ids: Vec<_> = self.propensity.iter().cloned().enumerate().collect();
        for (idx, propensity) in copyied_ids {
            self.update_tree(idx, propensity);
            sum += propensity;
        }*/
        assert!(sum >= 0.);

        //self.propensity_sum = sum;
    }

    pub fn step_tree(&mut self) {
       /* if self.stepnumber % 100000 * self.propensity.len() == 0 {
            self.rebuild_tree();
        }*/
        let propensity_sum = self.state_counts.get_propensity_sum();
        let frac = self.propensitiy_tree[0] / propensity_sum;
        let allowed_delta = 1e-7;
        if frac > 1. + allowed_delta || frac < 1. - allowed_delta{
            panic!("Tree diversion should no longer be possible! {} vs {} -> {}",self.propensitiy_tree[0] , propensity_sum, frac);
            //self.rebuild_tree();
        }
        let mut threshold = self.rng.gen_range(0.0..propensity_sum);
        match self.get_from_tree(threshold) {
            None => {
                panic!("Rebuield at {}",self.stepnumber);
                self.step_slow();
            }
            Some(x) => {
                self.fire_at(x);
                let distr = rand_distr::Exp::new(propensity_sum).unwrap();
                self.time += distr.sample(&mut self.rng);
            }
        }

        // TODO update time
    }
    pub fn step(&mut self) {
        self.stepnumber += 1;
        match self.use_slow_first_reaction_method {
            true => self.step_slow(),
            false => self.step_tree(),
        }
    }
    pub fn step_slow(&mut self) {
        let propensity_sum = self.state_counts.get_propensity_sum();
        assert!(propensity_sum > 0.);
        //println!("SUM {}",propensity_sum);
        let threshold = self.rng.gen_range(0.0..propensity_sum);
        let idx = match self
            .propensity
            .iter()
            .scan(0.0, |sum, p| {
                *sum += p;
                Some(*sum)
            })
            .enumerate()
            .find(|(_idx, sum)| *sum > threshold)
        {
            None => panic!("{}\nreaction finding problem", self),
            Some((idx, _)) => idx,
        };
        // TODO update time
        self.fire_at(idx);
        let distr = rand_distr::Exp::new(propensity_sum).unwrap();
        self.time += distr.sample(&mut self.rng);
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idx_conversion() {
        for xmax in [10, 11, 12] {
            for ymax in [10, 11, 12, 13] {
                let mut s = Simulation::new(xmax, ymax,true);
                for idx in 0..(ymax * xmax) {
                    let xy = s.idx_2_xy(idx);
                    let idx2 = s.xy_2_idx(xy);
                    println!("{} -> {:?} -> {}", idx, xy, idx2);
                    assert_eq!(idx2, idx);
                }
            }
        }
    }
    #[test]
    fn test_propensity() {
        let mut s = Simulation::new(5, 7,true);
        for _ in 0..10000 {
            s.step();
            let propensity_sum: f64 = s.propensity.iter().sum();
            assert!(propensity_sum > s.state_counts.get_propensity_sum() * (1. - 1e-6));
            assert!(propensity_sum * (1. - 1e-6) < s.state_counts.get_propensity_sum());
        }

        let mut s = Simulation::new(5, 7,false);
        for _ in 0..10000 {
            s.step();
            let propensity_sum: f64 = s.propensity.iter().sum();
            assert!(propensity_sum > s.state_counts.get_propensity_sum() * (1. - 1e-6));
            assert!(propensity_sum * (1. - 1e-6) < s.state_counts.get_propensity_sum());
        }
    }
}
