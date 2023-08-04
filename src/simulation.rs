use crate::simulation::StepRunResult::Step;
use crate::*;
use assert_float_eq::*;

const USE_SLOW_FIRST_REACTION_METHOD: bool = false;

#[derive(Eq, PartialEq)]
pub enum StepRunResult {
    NoStep,
    Step { reaction_num: usize },
}
impl StepRunResult {
    pub fn success(&self) -> bool {
        match self {
            StepRunResult::NoStep => false,
            Step { .. } => true,
        }
    }
}

/* Propagation */
impl<AG: Debug, const MAX_NUM_ATTRIBUTES: usize> Model<AG, MAX_NUM_ATTRIBUTES> {
    fn select_from_threshold(&mut self, first_try: bool, rand_val: f64) -> Option<usize> {
        if self.propensitiy_tree[0] <= 0. {
            return None;
        }
        let mut thrshold = rand_val * self.propensitiy_tree[0];
        let mut idx = 0;
        loop {
            let y = match self.propensities.get(idx) {
                None => break,
                Some(&y) => y,
            };
            if y >= thrshold {
                return Some(idx);
            } else {
                thrshold -= y;
            }

            let left_child = idx * 2 + 1;
            let right_child = idx * 2 + 2;

            match self.propensitiy_tree.get(left_child) {
                None => break,
                Some(&y) => {
                    if y >= thrshold {
                        idx = left_child;
                    } else {
                        thrshold -= y;
                        idx = right_child;
                    }
                }
            }

            /*if idx >= self.propensitiy_tree.len() {
                if first_try {
                    self.rebuild_tree();
                    return self.select_from_threshold(false, rand_val);
                }
                println!("Internal reaction selection error at {}! Tree problem, right child {} does not exist in {}!\n Tree {:?}\n{:?} \n sum={}, rand_val={}",idx,right_child,self.propensitiy_tree.len(),self.propensitiy_tree,self.propensities,self.propensity_sum,rand_val);
                let rand_val = self.rng.gen_range(0.0, 1.);
                return self.select_from_threshold(false, rand_val);
            }*/
        }

        if first_try {
            self.rebuild_tree();
            return self.select_from_threshold(false, rand_val);
        }
        if self.propensitiy_tree.len() < 1000 {
            println!("Internal reaction selection error at {}! Tree problem {}!\n Tree {:?}\n{:?} \n sum={}, rand_val={}", idx, self.propensitiy_tree.len(), self.propensitiy_tree, self.propensities, self.propensitiy_tree[0], rand_val);
        }
        println!(
            "Internal reaction selection error at {}! Tree problem {}!\n \n sum={}, rand_val={}",
            idx,
            self.propensitiy_tree.len(),
            self.propensitiy_tree[0],
            rand_val
        );
        let rand_val = self.rng.gen_range(0.0, 1.);
        return self.select_from_threshold(false, rand_val);
    }
    fn get_next_reaction_idx(&mut self) -> Option<usize> {
        assert!(self.propensitiy_tree[0] >= 0.);
        /*if ((self.stepnumber % (1000 * self.propensitiy_tree.len())) == 0)
            || (self.propensitiy_tree[0] < 0.)
        {

            self.rebuild_tree();
        }*/
        let rand_val = self.rng.gen_range(0.0, 1.);
        self.select_from_threshold(false, rand_val)
    }

    fn get_next_reaction_idx_slow(&mut self) -> Option<usize> {
        let propsum: f64 = self.propensities.iter().sum();
        let propensity_threshold = self.rng.gen_range(0.0, propsum);
        match self
            .propensities
            .iter()
            .scan(0.0, |sum, p| {
                *sum += p;
                Some(*sum)
            })
            .enumerate()
            .find(|(_idx, sum)| *sum > propensity_threshold)
        {
            None => None,
            Some((idx, _)) => Some(idx),
        }
    }

    pub fn step(&mut self) -> StepRunResult {
        self.step_limited(None)
    }

    pub fn step_limited(&mut self, steplimit: Option<f64>) -> StepRunResult {
        self.stepnumber += 1;
        self.handle_added_agents();
        self.update_reactions_that_have_been_marked();
        //let propsum: f64 = self.propensities.iter().sum();

        if self.propensitiy_tree[0] < 0. {
            panic!("this should no longer be posseble!")
            //self.rebuild_tree();
        }
        if self.propensitiy_tree[0] <= 0. {
            println!("no reaction possible");
            return StepRunResult::NoStep;
        }
        let distr = rand_distr::Exp::new(self.propensitiy_tree[0]).unwrap();
        let timestep = distr.sample(&mut self.rng);
        self.time += timestep;
        match steplimit {
            None => {}
            Some(x) => {
                if self.time > x {
                    self.time = x;
                    return StepRunResult::NoStep;
                }
            }
        }

        let idx_of_reaction = match USE_SLOW_FIRST_REACTION_METHOD {
            false => self.get_next_reaction_idx(),
            true => self.get_next_reaction_idx_slow(),
        };
        if idx_of_reaction.is_none() {
            return StepRunResult::NoStep;
        }
        let idx_of_reaction = idx_of_reaction.unwrap();

        let reac: &PotentialReaction = &self.potential_reactions[idx_of_reaction];
        let prototyp_idx = reac.idx_of_prototype.clone();
        self.state.idx_of_current_potential_transition.set(None);
        (self.transition_rule_prototypes[reac.idx_of_prototype].effect)(
            reac.idx_of_main_agent,
            self.state.clone(),
        );
        /*println!(
             "Called effect! on {} for {}", reac.idx_of_prototype,
            idx_of_reaction
        );*/
        self.update_reactions_that_have_been_marked();
        return Step {
            reaction_num: prototyp_idx,
        };
    }

    fn update_tree(&mut self, mut idx: usize) {
        if USE_SLOW_FIRST_REACTION_METHOD {
            return;
        }
        loop {
            let left_child = idx * 2 + 1;
            let right_child = idx * 2 + 2;

            if right_child < self.propensitiy_tree.len() {
                self.propensitiy_tree[idx] =
                    self.propensitiy_tree[right_child] + self.propensitiy_tree[left_child];
            } else if left_child < self.propensitiy_tree.len() {
                self.propensitiy_tree[idx] = self.propensitiy_tree[left_child];
            } else {
                self.propensitiy_tree[idx] = 0.;
            }
            self.propensitiy_tree[idx] += self.propensities[idx];
            if idx == 0 {
                break;
            }
            idx = (idx - 1) / 2; // parent node
        }
    }

    fn rebuild_tree(&mut self) {
        if USE_SLOW_FIRST_REACTION_METHOD {
            return;
        }
        let mut sum = 0.;
        /*.propensitiy_tree = vec![0.; self.propensitiy_tree.len()];
        let copyied_ids: Vec<_> = self.propensities.iter().cloned().enumerate().collect();
        for (idx, propensity) in copyied_ids {
            self.update_tree(idx, propensity);
            sum += propensity;
        }
         */
        for (idx, p) in self.propensities.iter().enumerate().rev() {
            let left_child = idx * 2 + 1;
            let right_child = idx * 2 + 2;
            if right_child < self.propensitiy_tree.len() {
                self.propensitiy_tree[idx] =
                    self.propensitiy_tree[right_child] + self.propensitiy_tree[left_child];
            } else if left_child < self.propensitiy_tree.len() {
                self.propensitiy_tree[idx] = self.propensitiy_tree[left_child];
            } else {
                self.propensitiy_tree[idx] = 0.;
            }
            sum += p;
            self.propensitiy_tree[idx] += p;
        }
        assert_float_relative_eq!(sum, self.propensitiy_tree[0]);
        //assert_float_relative_eq!(sum, self.state_counts.get_propensity_sum());
        //assert_approx_eq!(sum, self.propensitiy_tree[0]);
        //assert_approx_eq!(sum, self.state_counts.get_propensity_sum());

        assert!(sum >= 0.);
        //self.propensity_sum = sum;
    }

    pub fn run_and_write_csv(
        &mut self,
        target_time: f64,
        filename: &str,
        number_of_outputs: usize,
    ) {
        assert!(number_of_outputs >= 3);
        assert!(target_time > 0.);
        let pb = ProgressBar::new(10000);
        pb.set_style(
            ProgressStyle::default_bar()
                .template(
                    "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {percent}% ({eta})",
                )
                .progress_chars("#>-"),
        );

        let mut next_io = 0.;
        let mut store_times = vec![];
        let mut stored_values: HashMap<String, Vec<usize>> = Default::default();

        while self.time < target_time {
            if next_io <= self.time {
                next_io += target_time / number_of_outputs as f64;
                self.agent_counts().into_iter().for_each(|(name, count)| {
                    match stored_values.get_mut(&name) {
                        None => {
                            let mut vals = vec![0; store_times.len()];
                            vals.push(count);
                            stored_values.insert(name, vals);
                        }
                        Some(y) => {
                            while y.len() < store_times.len() {
                                y.push(0);
                            }
                            y.push(count);
                        }
                    }
                });
                store_times.push(self.time);
            }
            if self.stepnumber % 10000 == 0 {
                let time: f64 = self.time;
                let target: f64 = self.time;
                pb.set_position((10000. * time / (10000. * target) * 10000.) as u64);
            }
            if self.step() == StepRunResult::NoStep {
                self.time = next_io;
            }
        }
        pb.finish_with_message("simulation complete");

        println!("writing file {}", filename);
        let mut file = File::create(filename).unwrap();
        let observed_types: Vec<_> = stored_values.keys().collect();
        let observed_numbers: Vec<&Vec<usize>> = observed_types
            .iter()
            .map(|n| stored_values.get(*n).unwrap())
            .collect();
        writeln!(file, "time,{}", stored_values.keys().join(",")).unwrap();
        for i in 0..(store_times.len() - 1) {
            writeln!(
                file,
                "{},{}",
                store_times[i],
                observed_numbers
                    .iter()
                    .map(|vals| format!(
                        "{}",
                        match vals.get(i) {
                            None => 0,
                            Some(k) => *k,
                        }
                    ))
                    .join(",")
            )
            .unwrap();
        }
    }

    pub fn update_rate_calculation(&mut self, idx: usize) {
        self.state
            .idx_of_current_potential_transition
            .set(Some(idx));
        //let old_prop_sum = self.propensity_sum;
        //self.propensity_sum -= self.propensities[idx];
        let old_propensity = self.propensities[idx];
        self.propensities[idx] =
            (self.transition_rule_prototypes[self.potential_reactions[idx].idx_of_prototype].rate)(
                self.potential_reactions[idx].idx_of_main_agent,
                self.state.clone(),
            );
        //self.propensity_sum += self.propensities[idx];

        //if (old_propensity != self.propensities[idx]) {
        self.update_tree(idx);
        //}
        /*if old_prop_sum > self.propensity_sum * 100. {
            self.rebuild_tree();
        }*/

        /*println!(
            "SUM {} vs ROOT {}",
            self.propensity_sum, self.propensitiy_tree[0]
        );*/
    }

    pub fn update_reactions_that_have_been_marked(&mut self) {
        for idx in self.state.idices_to_update.take() {
            self.update_rate_calculation(idx);
        }
    }

    pub fn agent_counts(&self) -> HashMap<String, usize> {
        let mut res = HashMap::new();
        for ag in self.state.agents.borrow().iter() {
            let stringname = format!("{:?}", ag).to_string();
            match res.get_mut(&stringname) {
                None => {
                    res.insert(stringname, 1);
                }
                Some(y) => {
                    *y += 1;
                }
            }
        }
        res
    }
}
