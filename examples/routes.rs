/* Include our library */
use linked_lives::*;

/* include the time library, needed for throughput data */
use indicatif::{
    HumanDuration, MultiProgress, ParallelProgressIterator, ProgressBar, ProgressStyle,
};
use rand::distributions::WeightedIndex;
use rand::prelude::*;
use rayon::prelude::*;
use stats::mean;
use stats::*;
use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs::File;
use std::io::prelude::*;
use std::time::{Duration, Instant};

const MEASURE_REACTION_TIMES : bool = true;
const NUM_OUTPUTS :u64 = 1000;
//use test::stats::Stats;

fn count_reaction(name: &str) {
    //print!("{{ reaction: {},",name);
}

const rate_dep: f64 = 10.; // 20 // number of departures per time step

const dist_scale_slow: f64 = 10.0; // scale >= 1.0 required, otherwise path finding breaks
const dist_scale_fast: f64 = 1.0;
const frict_range: f64 = 0.0; // stochastic range of friction

const n_ini_contacts: usize = 10;
const ini_capital: f64 = 2000.0;
const p_know_target: f64 = 0.0;

const res_exp: f64 = 0.5;
const qual_exp: f64 = 0.5;
const frict_exp_fast: f64 = 1.25;
const frict_exp_slow: f64 = 12.5;
const p_find_links: f64 = 0.5;
const p_find_dests: f64 = 0.3;
const trust_travelled: f64 = 0.8;
const speed_expl_stay: f64 = 0.5;
const speed_expl_move: f64 = 0.5;

const costs_stay: f64 = 1.0;
const ben_resources: f64 = 5.0;
const costs_move: f64 = 2.0;

const qual_weight_x: f64 = 0.5;
const qual_weight_res: f64 = 0.1;
const qual_weight_frict: f64 = 0.1;

const p_keep_contact: f64 = 0.3;
const p_info_contacts: f64 = 0.3e-3;
const n_contacts_max: usize = 50;
const convince: f64 = 0.5; // change doubt into belief
const convert: f64 = 0.1; // change belief into other belief
const confuse: f64 = 0.3; // change belief into doubt
const error: f64 = 0.5; // stochastic error when transmitting information
const weight_arr: f64 = 1.0;

const expl_rate: f64 = 1.;
const cost_rate: f64 = 1.;
const transit_rate: f64 = 1.;

// IO FORMAT
#[derive(Serialize, Deserialize)]
struct TopologyNode {
    x: f64,
    y: f64,
    loc_type: LocationType,
    quality: f64,
    resources: f64,
}
#[derive(Serialize, Deserialize)]
struct TopologyLink {
    connects: (usize, usize),
    link_type: LinkType,
}
#[derive(Serialize, Deserialize)]
struct Topology {
    nodes: Vec<TopologyNode>,
    links: Vec<TopologyLink>,
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, Serialize, Deserialize)]
enum LocationType {
    Standard,
    Entry,
    Exit,
}

impl Default for LocationType {
    fn default() -> Self {
        LocationType::Entry
    }
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, Serialize, Deserialize)]
enum LinkType {
    Slow,
    Fast,
}

impl Default for LinkType {
    fn default() -> Self {
        LinkType::Slow
    }
}

#[derive(AgentEnum, Debug, Serialize, Copy, Clone)]
enum Agent {
    Migrant {
        capital: f64,
        in_transit: bool,
        steps: usize,
        activ_migration: bool,
    },
    Location {
        x: f64,
        y: f64,
        loc_type: LocationType,
        quality: f64,
        resources: f64,
        visits: usize,
        communication: usize,
        moves: usize,
    },
    Information {
        quality: f64,
        resources: f64,
        quality_trust: f64,
        resources_trust: f64,
    },
    Link {
        link_type: LinkType,
        friction: f64,
        distance: f64,
        visits: usize,
    },
    InfoLink {
        friction: f64,
        friction_trust: f64,
    },
    World {
        contacts_made: usize,
    },
}

//location:Location[1]<->[n]Migrant:migrants // migrants to locations
add_single_multi_edge!(Location location Migrant migrants);

//destination:Location[1]<->[n]Migrant:incoming // migrants on the move linked to their distination
add_single_multi_edge!(Location destination Migrant incoming);

//links:Link[n]<->[n]Location:endpoints // location to the link object to another location
add_multi_multi_edge!(Location endpoints Link links);

//knowledge:Information[n]<->[1]Migrant:owner // migrants to their knowledge
add_multi_multi_edge!(Information knowledge Migrant ownerI);

//link_knowledge:InfoLink[n]<->[1]Migrant:owner // migrants to their knowledge
add_multi_multi_edge!(InfoLink link_knowledge Migrant ownerL);

//subject:Location[1]<->[n]Information:information // pieces of information to the subject location
add_single_multi_edge!(Location subject1 Information information1);

//subject:Link[1]<->[n]InfoLink:information // pieces of information to the subject link
add_single_multi_edge!(Link subject2 InfoLink information2);

//contacts:Migrant[n]<->[n]Migrant:contacters // contact network
add_multi_multi_edge!(Migrant contacts Migrant contacters);

impl Information {
    fn accuracy(&self) -> f64 {
        ((self.get_quality() - self.subject1().unwrap().get_quality()).powi(2)
            + (self.get_resources() - self.subject1().unwrap().get_resources()).powi(2))
        .sqrt()
    }
}

impl InfoLink {
    fn accuracy(&self) -> f64 {
        (self.get_friction() - self.subject2().unwrap().get_friction()).abs()
    }
}

impl World {
    fn average_accuracy(&self) -> f64 {
        self.iterate_all_Information()
            .map(|info| info.accuracy())
            .sum::<f64>()
            / self.iterate_all_Information().count() as f64
    }
    fn average_accuracy_link(&self) -> f64 {
        self.iterate_all_InfoLink()
            .map(|link| link.accuracy())
            .sum::<f64>()
            / self.iterate_all_InfoLink().count() as f64
    }
}

impl Location {
    fn accuracy_about(&self) -> f64 {
        //ML3: if (ego.information.size() > 0) then ego.information.sum(alter.accuracy()) / ego.information.size() else -1
        let number_of_information = self.information1().count();
        if number_of_information > 0 {
            self.information1()
                .map(|alter| alter.accuracy() / number_of_information as f64)
                .sum()
        } else {
            f64::NAN
        }
    }
}

impl Link {
    fn accuracy_about(&self) -> f64 {
        let number_of_information = self.information2().count();
        if number_of_information > 0 {
            self.information2()
                .map(|alter| alter.accuracy() / number_of_information as f64)
                .sum()
        } else {
            f64::NAN
        }
    }

    fn other_side(&self, loc: &Location) -> Location {
        //assert_eq!(self.endpoints().filter(|alter| *loc != *alter).count(), 1);
        self.endpoints().filter(|alter| *loc != *alter).only()
    }
}

impl Migrant {
    fn knows(&self, loc: &Location) -> bool {
        self.knowledge()
            .filter(|alter| alter.subject1().unwrap() == *loc)
            .next()
            .is_some()
    }
    fn knowledge_about(&self, loc: &Location) -> Information {
        self.knowledge()
            .filter(|alter| alter.subject1().unwrap() == *loc)
            .only()
    }
    /*
     * ACTIVITIES WHILE STAYING
     */
    fn exchange_info(&mut self, other: &mut Migrant) {
        *self.location().unwrap().get_communication_mut() += 1;
        let ego_exclusiv: Vec<_> = self
            .knowledge()
            .filter(|alter| !other.knows(&alter.subject1().unwrap()))
            .collect();
        let other_exclusiv: Vec<_> = other
            .knowledge()
            .filter(|alter| !self.knows(&alter.subject1().unwrap()))
            .collect();
        let other_arrived = other.location().unwrap().get_loc_type() == LocationType::Exit;

        // ego discover's the places other knows
        for info in other_exclusiv {
            self.discover_location(&info.subject1().unwrap());
        }

        // other discovers the places ego knows
        if !other_arrived {
            for info in ego_exclusiv {
                other.discover_location(&info.subject1().unwrap());
            }
        }

        // exchange knowledge about the places both know now
        for info in self
            .knowledge()
            .filter(|loc| other.knows(&loc.subject1().unwrap()))
        {
            let other_info = other
                .knowledge()
                .filter(|alter| info.subject1() == alter.subject1())
                .only();
            self.exchange_believes(info, other_info, other_arrived);
        }

        // link knowledge
        let ego_exclusiv_link: Vec<_> = self
            .link_knowledge()
            .filter(|alter| !other.knows_link(&alter.subject2().unwrap()))
            .collect();
        let other_exclusiv_link: Vec<_> = other
            .link_knowledge()
            .filter(|alter| !self.knows_link(&alter.subject2().unwrap()))
            .collect();

        for info in other_exclusiv_link {
            if info
                .subject2()
                .unwrap()
                .endpoints()
                .filter(|alter| self.knows(alter))
                .count()
                == 2
            {
                self.discover_link(&info.subject2().unwrap());
            }
        }
        // other discovers the links ego knows
        if !other_arrived {
            for info in ego_exclusiv_link {
                if info
                    .subject2()
                    .unwrap()
                    .endpoints()
                    .filter(|alter| other.knows(alter))
                    .count()
                    == 2
                {
                    other.discover_link(&info.subject2().unwrap());
                }
            }
        }
        // exchange knowledge about the links both know now
        for info in self
            .link_knowledge()
            .filter(|alter| other.knows_link(&alter.subject2().unwrap()))
        {
            let other_info = other
                .link_knowledge()
                .filter(|alter| info.subject2().unwrap() == alter.subject2().unwrap())
                .only();
            self.exchange_believes_link(info, other_info, other_arrived);
        }
    }

    fn exchange_believes(&mut self, i1: Information, i2: Information, arrived: bool) {
        // agent 1 evaluated the information differently, if agent 2 has already arrived (agent 1 definitely did not arrive yet)

        let convince1 = if arrived {
            convince.powf(1. / weight_arr)
        } else {
            convince
        };
        let convert1 = if arrived {
            convert.powf(1. / weight_arr)
        } else {
            convert
        };
        let confuse1 = confuse;
        let v2_q_pcv = self.v_pcv(i2.get_quality());
        let v2_r_pcv = self.v_pcv(i2.get_resources());
        let t2_q_pcv = self.t_pcv(i2.get_quality_trust());
        let t2_r_pcv = self.t_pcv(i2.get_resources_trust());
        let v1_q = Migrant::receive_value(
            i1.get_quality(),
            i1.get_quality_trust(),
            v2_q_pcv,
            t2_q_pcv,
            convince1,
            convert1,
            confuse1,
        );
        let v1_r = Migrant::receive_value(
            i1.get_resources(),
            i1.get_resources_trust(),
            v2_r_pcv,
            t2_r_pcv,
            convince1,
            convert1,
            confuse1,
        );
        let d1_q = Migrant::receive_doubt(
            i1.get_quality(),
            i1.get_quality_trust(),
            v2_q_pcv,
            t2_q_pcv,
            convince1,
            convert1,
            confuse1,
        );
        let d1_r = Migrant::receive_doubt(
            i1.get_resources(),
            i1.get_resources_trust(),
            v2_r_pcv,
            t2_r_pcv,
            convince1,
            convert1,
            confuse1,
        );
        // agent 2 does not update their knowledge, if they have arrived
        if !arrived {
            let v1_q_pcv = self.v_pcv(i1.get_quality());
            let v1_r_pcv = self.v_pcv(i1.get_resources());
            let t1_q_pcv = self.t_pcv(i1.get_quality_trust());
            let t1_r_pcv = self.t_pcv(i1.get_resources_trust());
            let v2_q = Migrant::receive_value(
                i2.get_quality(),
                i2.get_quality_trust(),
                v1_q_pcv,
                t1_q_pcv,
                convince,
                convert,
                confuse,
            );
            let v2_r = Migrant::receive_value(
                i2.get_resources(),
                i2.get_resources_trust(),
                v1_r_pcv,
                t1_r_pcv,
                convince,
                convert,
                confuse,
            );
            let d2_q = Migrant::receive_doubt(
                i2.get_quality(),
                i2.get_quality_trust(),
                v1_q_pcv,
                t1_q_pcv,
                convince,
                convert,
                confuse,
            );
            let d2_r = Migrant::receive_doubt(
                i2.get_resources(),
                i2.get_resources_trust(),
                v1_r_pcv,
                t1_r_pcv,
                convince,
                convert,
                confuse,
            );
            // set new values
            *i2.get_quality_mut() = v2_q / (1. - d2_q);
            *i2.get_quality_trust_mut() = 1. - d2_q;
            *i2.get_resources_mut() = v2_r / (1. - d2_r);
            *i2.get_resources_trust_mut() = 1. - d2_r;
        }
        // set new values for i1 only after the calculations were made for i2
        *i1.get_quality_mut() = v1_q / (1. - d1_q);
        *i1.get_quality_trust_mut() = 1. - d1_q;
        *i1.get_resources_mut() = v1_r / (1. - d1_r);
        *i1.get_resources_trust_mut() = 1. - d1_r;
    }

    fn exchange_believes_link(&mut self, i1: InfoLink, i2: InfoLink, arrived: bool) {
        // agent 1 evaluated the information differently, if agent 2 has already arrived (agent 1 definitely did not arrive yet)
        let convince1 = if arrived {
            convince.powf(1. / weight_arr)
        } else {
            convince
        };
        let convert1 = if arrived {
            convert.powf(1. / weight_arr)
        } else {
            convert
        };
        let confuse1 = confuse;
        let v2_pcv = self.v_pcv(i2.get_friction());
        let t2_pcv = self.t_pcv(i2.get_friction_trust());
        let v1 = Migrant::receive_value(
            i1.get_friction(),
            i1.get_friction_trust(),
            v2_pcv,
            t2_pcv,
            convince1,
            convert1,
            confuse1,
        );
        let d1 = Migrant::receive_doubt(
            i1.get_friction(),
            i1.get_friction_trust(),
            v2_pcv,
            t2_pcv,
            convince1,
            convert1,
            confuse1,
        );
        // agent 2 does not update their knowledge, if they have arrived
        if !arrived {
            let v1_pcv = self.v_pcv(i1.get_friction());
            let t1_pcv = self.t_pcv(i1.get_friction_trust());
            let v2 = Migrant::receive_value(
                i2.get_friction(),
                i2.get_friction_trust(),
                v1_pcv,
                t1_pcv,
                convince,
                convert,
                confuse,
            );
            let d2 = Migrant::receive_doubt(
                i2.get_friction(),
                i2.get_friction_trust(),
                v1_pcv,
                t1_pcv,
                convince,
                convert,
                confuse,
            );
            // set new values
            *i2.get_friction_mut() = v2 / (1. - d2);
            *i2.get_friction_trust_mut() = 1. - d2;
        }

        // set new values for i1 only after the calculations were made for i2
        *i1.get_friction_mut() = v1 / (1. - d1);
        *i1.get_friction_trust_mut() = 1. - d1;
    }

    fn receive_value(v1: f64, t1: f64, v2_pcv: f64, t2_pcv: f64, ci: f64, ce: f64, cu: f64) -> f64 {
        let t = t1;
        let d = 1. - t;
        let v = v1;
        let t_pcv = t2_pcv;
        let d_pcv = 1.0 - t_pcv;
        let v_pcv = v2_pcv;
        let dist_pcv = (v - v_pcv).abs() / (v + v_pcv + 0.00001);
        t * d_pcv * v
            + d * t_pcv * ci * v_pcv
            + t * t_pcv * (1.0 - cu * dist_pcv) * ((1.0 - ce) * v + ce * v_pcv)
    }

    fn receive_doubt(v1: f64, t1: f64, v2_pcv: f64, t2_pcv: f64, ci: f64, ce: f64, cu: f64) -> f64 {
        let t = t1;
        let d = 1. - t;
        let v = v1;
        let t_pcv = t2_pcv;
        let d_pcv = 1.0 - t_pcv;
        let v_pcv = v2_pcv;
        let dist_pcv = (v - v_pcv).abs() / (v + v_pcv + 0.00001);
        d * d_pcv + d * t_pcv * (1.0 - ci) + t * t_pcv * cu * dist_pcv
    }

    /* mutable because random number generation may only be used in effect */
    fn t_pcv(&mut self, t: f64) -> f64 {
        Migrant::limit(0.000001, t + self.unf_delta(error), 0.99999)
    }
    fn v_pcv(&mut self, v: f64) -> f64 {
        0.0_f64.max(v + self.unf_delta(error))
    }
    fn unf_delta(&mut self, x: f64) -> f64 {
        rand::thread_rng().gen_range(0., 2.) * x - x
    }
    fn limit(mi: f64, v: f64, ma: f64) -> f64 {
        ma.min(v.max(mi))
    }

    fn discover_location(&mut self, loc: &Location) {
        let mut k = self.create_new_Information(Agent::Information {
            quality: qual_exp,
            quality_trust: 0.00001,
            resources: res_exp,
            resources_trust: 0.00001,
        });

        k.add_to_ownerI(self);
        k.set_subject1(loc);
    }

    fn discover_link(&mut self, link: &Link) {
        let mut k = self.create_new_InfoLink(Agent::InfoLink {
            friction: match link.get_link_type() {
                LinkType::Slow => link.get_distance() * frict_exp_slow,
                LinkType::Fast => link.get_distance() * frict_exp_fast,
            },
            friction_trust: 0.00001,
        });
        k.add_to_ownerL(self);
        k.set_subject2(link);
    }

    // explore at a location, possibly explore adjacent links
    fn explore_at(&mut self, loc: &Location, speed: f64, allow_indirect: bool) {
        if !self.knows(loc) {
            self.discover_location(loc)
        }
        let mut k = self.knowledge_about(loc);
        //let quality: f64 = k.get_quality();
        *k.get_quality_mut() = Migrant::weighted_average(k.get_quality(), loc.get_quality(), speed);
        *k.get_resources_mut() =
            Migrant::weighted_average(k.get_resources(), loc.get_resources(), speed);
        *k.get_quality_trust_mut() =
            Migrant::weighted_average(k.get_quality_trust(), 0.999999, speed);
        *k.get_resources_trust_mut() =
            Migrant::weighted_average(k.get_resources_trust(), 0.999999, speed);

        if allow_indirect {
            for link in loc.links() {
                if rand::thread_rng().gen_range(0., 1.) < p_find_links {
                    self.explore_link(&link, speed);
                    let other = link.endpoints().filter(|alter| alter != loc).only();
                    self.explore_at(&other, speed, false)
                }
            }
        }
    }
    fn knows_link(&self, link: &Link) -> bool {
        /*self.link_knowledge()
        .filter(|alter| alter.subject2().unwrap() == *link)
        .count()
        > 0*/
        //Optimized version
        self.link_knowledge()
            .filter(|alter| alter.subject2().unwrap() == *link)
            .next()
            .is_some()
    }

    fn knowledge_about_link(&self, link: &Link) -> InfoLink {
        self.link_knowledge()
            .filter(|alter| alter.subject2().unwrap() == *link)
            .only()
    }

    fn explore_link(&mut self, link: &Link, speed: f64) {
        if !self.knows_link(link) {
            self.discover_link(link);
        }
        let mut k = self.knowledge_about_link(link);
        *k.get_friction_mut() =
            Migrant::weighted_average(k.get_friction(), link.get_friction(), speed);
        *k.get_friction_trust_mut() =
            Migrant::weighted_average(k.get_friction_trust(), 0.999999, speed);
    }

    // while in transit
    fn explore_move(&mut self) {
        self.explore_at(&self.destination().unwrap(), speed_expl_move, false);

        self.explore_link(
            &self
                .location()
                .unwrap()
                .links()
                .filter(|alter| {
                    alter
                        .endpoints()
                        .find(|i| *i == self.destination().unwrap())
                        .is_some()
                })
                .only(),
            speed_expl_move,
        );
    }

    fn weighted_average(val: f64, target: f64, weight: f64) -> f64 {
        val * (1. - weight) + target * weight
    }

    /*
     * MOVING AROUND
     */
    fn move_rate(&self) -> f64 {
        let relevant_link_knowledge: Vec<_> = self
            .link_knowledge()
            .filter(|alter| {
                alter
                    .subject2()
                    .unwrap()
                    .endpoints()
                    .find(|k| *k == self.location().unwrap())
                    .is_some()
            })
            .collect();
        let quality_sum: f64 = relevant_link_knowledge
            .iter()
            .map(|alter| self.quality_link(alter))
            .sum();
        let expected_quality = relevant_link_knowledge
            .iter()
            .map(|alter| self.quality_link(alter).powi(2))
            .sum::<f64>()
            / quality_sum;
        let here_quality = self.quality_loc(&self.location().unwrap());
        if relevant_link_knowledge.len() == 0 || quality_sum == 0. {
            0.
        } else if here_quality > 0. {
            expected_quality / here_quality
        } else {
            1.
        }
    }

    // (== decide_move)
    fn decide_destination(&self) -> Location {
        let relevant_link_knowledge: Vec<_> = self
            .link_knowledge()
            .filter(|alter| {
                alter
                    .subject2()
                    .unwrap()
                    .endpoints()
                    .find(|k| self.location().unwrap() == *k)
                    .is_some()
            })
            .collect();
        let dist = WeightedIndex::new(
            &relevant_link_knowledge
                .iter()
                .map(|alter| self.quality_link(alter))
                .collect::<Vec<_>>(),
        )
        .unwrap();
        relevant_link_knowledge[dist.sample(&mut thread_rng())]
            .subject2()
            .unwrap()
            .endpoints()
            .filter(|alter| *alter != self.location().unwrap())
            .only()
    }

    fn quality_link(&self, info: &InfoLink) -> f64 {
        let other_side = info
            .subject2()
            .unwrap()
            .endpoints()
            .filter(|alter| *alter != self.location().unwrap())
            .only();
        self.quality_loc(&other_side) / (1. + info.get_friction() * qual_weight_frict)
    }

    // (== quality for locations)
    fn quality_loc(&self, loc: &Location) -> f64 {
        let closeness = loc.get_x() * qual_weight_x;
        let mut infos = self
            .knowledge()
            .filter(|alter| alter.subject1().unwrap() == *loc);
        match infos.next() {
            None => closeness,
            Some(info) => {
                assert!(infos.next().is_none()); // -> only
                let quality = info.get_quality() * info.get_quality_trust();
                let resources = info.get_resources() * info.get_resources_trust() * qual_weight_res;
                quality + closeness + resources
            }
        }
    }

    // (== costs_move)
    fn move_cost(&self, destination: &Location) -> f64 {
        costs_move
            * self
                .location()
                .unwrap()
                .links()
                .filter(|alter| alter.endpoints().find(|k| *k == *destination).is_some())
                .only()
                .get_friction()
    }
    // distance of the link the agent currently moves through
    fn move_distance(&self) -> f64 {
        let destination = self.destination().unwrap();
        self.location()
            .unwrap()
            .links()
            .filter(|alter| alter.endpoints().find(|k| *k == destination).is_some())
            .only()
            .get_distance()
    }
}

fn generate_model() -> linked_lives::Model<Agent, 8_usize> {
    let mut model = Agent::new_model();

    // (== costs_stay)
    model.add_transition_for_Migrant(
        |ego| !ego.get_in_transit() && ego.get_activ_migration(),
        |_| cost_rate,
        |ego| {
            count_reaction("cost_stay");
            *ego.get_capital_mut() =
                ben_resources + ego.location().unwrap().get_resources() - costs_stay
        },
    );
    // (==explore)
    model.add_transition_for_Migrant(
        |ego| !ego.get_in_transit() && ego.get_activ_migration(),
        |_| expl_rate,
        |ego| { count_reaction("explore"); ego.explore_at(&ego.location().unwrap(), speed_expl_stay, true) },
    );

    // (== mingle)
    // the making contact part of mingle
    model.add_transition_for_Migrant(
        |ego| {
            !ego.get_in_transit()
                && ego.contacts().count() < n_contacts_max
                && ego.get_activ_migration()
        },
        |ego| {
            // hashset needed to avoid counting duplicates
            let people: HashSet<_> = ego
                .location()
                .unwrap()
                .migrants()
                .filter(|k| k != ego)
                .collect();
            people.len() as f64 * p_keep_contact
        },
        |ego| {
            count_reaction("mingle");
            let people: HashSet<_> = ego
                .location()
                .unwrap()
                .migrants()
                .filter(|k| k != ego)
                .collect();
            let mut other = people.into_iter().choose(&mut thread_rng()).unwrap();
            if (ego.contacts().find(|k| *k == other).is_none()) {
                ego.add_to_contacts(&other);
            }
            ego.exchange_info(&mut other);
            if other.contacts().count() < n_contacts_max
                && other.contacts().find(|k| *k == *ego).is_none()
            {
                other.add_to_contacts(&other);
            }
        },
    );

    // communication
    model.add_transition_for_Migrant(
        |ego| !ego.get_in_transit() && ego.get_activ_migration(),
        |ego| ego.contacts().count() as f64 * p_info_contacts,
        |ego| {
            count_reaction("communication");
            ego.exchange_info(&mut ego.contacts().choose(&mut thread_rng()).unwrap()) },
    );

    // decide to move to a new location
    model.add_transition_for_Migrant(
        |ego| !ego.get_in_transit() && ego.get_activ_migration(),
        |ego| ego.move_rate(),
        |ego| {
            count_reaction("decide_new_location");
            *ego.get_in_transit_mut() = true;
            ego.set_destination(&ego.decide_destination());
            let link = ego
                .location()
                .unwrap()
                .links()
                .filter(|alter| {
                    (alter
                        .endpoints()
                        .find(|k| *k == ego.location().unwrap())
                        .is_some())
                        && (alter
                            .endpoints()
                            .find(|k| *k == ego.destination().unwrap())
                            .is_some())
                })
                .only();
            *link.get_visits_mut() += 1;
            *ego.get_capital_mut() -= ego.move_cost(&ego.destination().unwrap());
            *ego.get_steps_mut() += 1;
            *ego.location().unwrap().get_moves_mut() += 1
        },
    );

    // people in transit arrive at their destination
    model.add_transition_for_Migrant(
        |ego| ego.get_in_transit() && ego.get_activ_migration(),
        |_ego| transit_rate, // / ego.move_distance(),
        |ego| {
            count_reaction("arrive_destination");
            ego.set_location(&ego.destination().unwrap());
            *ego.location().unwrap().get_visits_mut() += 1;
            *ego.get_in_transit_mut() = false;
            if ego.location().unwrap().get_loc_type() == LocationType::Exit {
                //ego.die();
                ego.set_activ_migration(false);
            }
        },
    );

    // (== handle_departures)
    model.add_transition_for_World(
        |_ego| true,
        |_ego| rate_dep, // / ego.move_distance(),
        |ego| {
            count_reaction("handle_departures");
            let mut new_migrant = ego.create_new_Migrant(Agent::Migrant {
                capital: ini_capital,
                in_transit: false,
                steps: 0,
                activ_migration: true,
            });
            let all_other_migrants: Vec<_> = ego
                .iterate_all_Migrant()
                .filter(|m| m.get_activ_migration())
                .filter(|m| *m != new_migrant)
                .collect();
            all_other_migrants
                .choose_multiple(&mut thread_rng(), n_ini_contacts)
                .into_iter()
                .for_each(|om| new_migrant.add_to_contacts(om));
            //migrant.contacts = (Migrant.all - [migrant]).random(n_ini_contacts)
            for other in new_migrant.contacts() {
                if other.contacts().count() < n_contacts_max && other.contacts().find(|k|*k==new_migrant).is_none(){
                    other.add_to_contacts(&new_migrant);
                }
            }
            *ego.get_contacts_made_mut() += new_migrant.contacts().count();
            //println!("EDGES_A: {:?}",new_migrant.state.edges.borrow()[new_migrant.idx]);
            new_migrant.set_location(
                &ego.iterate_all_Location()
                    .filter(|alter| alter.get_loc_type() == LocationType::Entry)
                    .choose(&mut thread_rng())
                    .unwrap(),
            );
            /*println!("Migrant {}", new_migrant.idx);
            println!("EDGES_B: {:?}",new_migrant.state.edges.borrow()[new_migrant.idx]);
            println!("Retrive {:?}",new_migrant.state.edges.borrow()[new_migrant.idx][0]);
            println!("Retrive {:?}",new_migrant.state.edges.borrow()[new_migrant.idx][0].iter().next());*/
            *new_migrant.location().unwrap().get_visits_mut() += 1;
            new_migrant.explore_at(&new_migrant.location().unwrap(), speed_expl_stay, true);
            let exits = ego
                .iterate_all_Location()
                .filter(|alter| alter.get_loc_type() == LocationType::Exit);
            for exit in exits {
                if thread_rng().gen_range(0., 1.) < p_know_target {
                    new_migrant.explore_at(&exit, speed_expl_stay, true)
                }
            }
        },
    );
    model
}

fn build_three_city() -> linked_lives::Model<Agent, 8_usize> {
    let mut model = generate_model();
    let world = model.create_new_agent(Agent::World { contacts_made: 0 });
    let entry = model.create_new_agent(Agent::Location {
        x: 0.0,
        y: 0.5,
        loc_type: LocationType::Entry,
        quality: 0.0,
        resources: 0.0,
        visits: 0,
        communication: 0,
        moves: 0,
    });
    let exit1 = model.create_new_agent(Agent::Location {
        x: 0.99,
        y: 0.0,
        loc_type: LocationType::Exit,
        quality: 0.5,
        resources: 0.5,
        visits: 0,
        communication: 0,
        moves: 0,
    });
    let exit2 = model.create_new_agent(Agent::Location {
        x: 0.99,
        y: 1.,
        loc_type: LocationType::Exit,
        quality: 1.0,
        resources: 1.0,
        visits: 0,
        communication: 0,
        moves: 0,
    });

    let dummy_d = 1.109099;

    let d1 = dummy_d; //0.5;
    let l1 = model.create_new_agent(Agent::Link {
        link_type: LinkType::Fast,
        friction: dist_scale_fast * d1,
        distance: d1,
        visits: 0,
    });
    model.add_edge_endpoints(l1, entry);
    model.add_edge_endpoints(l1, exit1);

    let d2 = dummy_d; //2.0_f64.sqrt();
    let l2 = model.create_new_agent(Agent::Link {
        link_type: LinkType::Fast,
        friction: dist_scale_fast * d2,
        distance: d2,
        visits: 0,
    });
    model.add_edge_endpoints(l2, entry);
    model.add_edge_endpoints(l2, exit2);

    model
}

fn road_topology_file(name: String) -> linked_lives::Model<Agent, 8_usize> {
    let mut file = File::open(name.clone()).expect(&*format!("file {} not found", name));
    //let json = serde_json::from_reader(file).expect("file not json");
    let topo = Topology::deserialize(&mut serde_json::Deserializer::from_reader(file))
        .expect("Could not parse json");
    let mut model = generate_model();
    let world = model.create_new_agent(Agent::World { contacts_made: 0 });
    let node_ids: Vec<_> = topo
        .nodes
        .iter()
        .inspect(|k| assert!(k.x >= 0.))
        .map(|n| {
            model.create_new_agent(Agent::Location {
                x: n.x,
                y: n.y,
                loc_type: n.loc_type,
                quality: n.quality,
                resources: n.resources,
                visits: 0,
                communication: 0,
                moves: 0,
            })
        })
        .collect();
    topo.links.iter().for_each(|e| {
        let (n_id_1, n_id_2) = e.connects;
        let (x1, y1) = (topo.nodes[n_id_1].x, topo.nodes[n_id_1].y);
        let (x2, y2) = (topo.nodes[n_id_2].x, topo.nodes[n_id_2].y);
        let dist = ((x1 - x2).powi(2) + (y1 + y2).powi(2)).sqrt();

        let link_id = model.create_new_agent(Agent::Link {
            link_type: e.link_type,
            friction: match e.link_type {
                LinkType::Slow => dist_scale_slow,
                LinkType::Fast => dist_scale_fast,
            } * dist,
            distance: dist,
            visits: 0,
        });
        model.add_edge_endpoints(link_id, node_ids[n_id_1]);
        model.add_edge_endpoints(link_id, node_ids[n_id_2]);
    });

    model
}

fn build_four_city() -> linked_lives::Model<Agent, 8_usize> {
    let mut model = generate_model();
    let world = model.create_new_agent(Agent::World { contacts_made: 0 });
    let entry = model.create_new_agent(Agent::Location {
        x: 0.0,
        y: 0.5,
        loc_type: LocationType::Entry,
        quality: 0.0,
        resources: 0.0,
        visits: 0,
        communication: 0,
        moves: 0,
    });
    let mid = model.create_new_agent(Agent::Location {
        x: 0.5,
        y: 0.5,
        loc_type: LocationType::Standard,
        quality: 0.1,
        resources: 0.1,
        visits: 0,
        communication: 0,
        moves: 0,
    });
    let exit1 = model.create_new_agent(Agent::Location {
        x: 0.99,
        y: 0.0,
        loc_type: LocationType::Exit,
        quality: 0.5,
        resources: 0.5,
        visits: 0,
        communication: 0,
        moves: 0,
    });
    let exit2 = model.create_new_agent(Agent::Location {
        x: 0.99,
        y: 1.,
        loc_type: LocationType::Exit,
        quality: 1.0,
        resources: 1.0,
        visits: 0,
        communication: 0,
        moves: 0,
    });

    let dlmid = 0.5;
    let lmid = model.create_new_agent(Agent::Link {
        link_type: LinkType::Fast,
        friction: dist_scale_fast * dlmid,
        distance: dlmid,
        visits: 0,
    });
    model.add_edge_endpoints(lmid, entry);
    model.add_edge_endpoints(lmid, mid);

    let d1 = ((0.99f64 - 0.5).powi(2) + 0.5f64.powi(2)).sqrt();
    let l1 = model.create_new_agent(Agent::Link {
        link_type: LinkType::Fast,
        friction: dist_scale_fast * d1,
        distance: d1,
        visits: 0,
    });
    model.add_edge_endpoints(l1, mid);
    model.add_edge_endpoints(l1, exit1);

    let d2 = ((0.99f64 - 0.5).powi(2) + 0.5f64.powi(2)).sqrt();
    let l2 = model.create_new_agent(Agent::Link {
        link_type: LinkType::Fast,
        friction: dist_scale_fast * d2,
        distance: d2,
        visits: 0,
    });
    model.add_edge_endpoints(l2, mid);
    model.add_edge_endpoints(l2, exit2);

    model
}

fn check_property_vector_multi(pb: ProgressBar,until : f64) -> HashMap<String, Vec<f64>> {
    let mut m = road_topology_file("topo_100_city.json".to_string());
    let timelimit = until; //100.;
    let mut stored_values = Vec::new();
    for iter in 0..NUM_OUTPUTS {
        let this_limit = timelimit / NUM_OUTPUTS as f64 * iter as f64;
        let old_step_count = m.stepnumber;
        let start = Instant::now();
        let mut prototype_time = vec![Duration::new(0, 0); m.transition_rule_prototypes.len()];
        let mut prototyp_counts = vec![0; m.transition_rule_prototypes.len()];
        while m.time < this_limit {
            let before;
            if MEASURE_REACTION_TIMES {
               before = Some(Instant::now());
            } else {
                before = None
            }
            match m.step_limited(Some(this_limit)) {
                StepRunResult::NoStep => {}
                StepRunResult::Step { reaction_num } => {
                    if MEASURE_REACTION_TIMES && before.is_some() {
                        prototype_time[reaction_num] += before.unwrap().elapsed();
                    }
                    prototyp_counts[reaction_num] += 1;
                    //println!(": {}}}",reaction_num);
                }
            }
        }

        let time_per_reac: Vec<f64> = prototyp_counts
            .iter()
            .zip(prototype_time.iter())
            .map(|(count, t)| {
                if *count > 0 {
                    t.as_secs_f64() / *count as f64
                } else {
                    0.
                }
            })
            .collect();
        assert_eq!(time_per_reac.len(), m.transition_rule_prototypes.len());
        /*println!(
            "{}: {}/s",
            m.time,
            (m.stepnumber - old_step_count) as f64 / start.elapsed().as_secs_f64()
        );*/
        pb.set_position(iter);
        /*let mig_travel = observe_all_Migrant(&m)
            .filter(|m| m.get_activ_migration())
            .count();
        let seto: hashbag::HashBag<usize> = observe_all_Migrant(&m)
            .filter(|m| m.get_activ_migration())
            .map(|m| m.location().unwrap().idx)
            .collect();
        //println!("{:?}", seto);
        for (loc, n) in seto.set_iter() {
            print!("{}:{} ", loc, n);
        }
        println!("Trav: {}/{}", mig_travel, observe_all_Migrant(&m).count());*/
        let mut obs = get_validation_observables(&m);
        assert_eq!(prototyp_counts.len(), time_per_reac.len());
        obs.insert("time".to_string(), m.time);
        for (n, (c, tpc)) in prototyp_counts
            .into_iter()
            .zip(time_per_reac.into_iter())
            .enumerate()
        {
            obs.insert(format!("reac_counts_{}", n), c as f64);
            obs.insert(format!("reac_time_spend_exec_{}", n), tpc);
        }
        stored_values.push(obs);
    }
    pb.finish();
    // merge stores
    stored_values
        .iter()
        .next()
        .unwrap()
        .keys()
        .map(|key| {
            (
                key.clone(),
                stored_values
                    .iter()
                    .map(|store| *store.get(key).expect(&*format!("Could not find {}", key)))
                    .collect(),
            )
        })
        .collect()

    /*let r = m.state.agents.borrow_mut().clone();
    r*/
    //m
}

fn check_property_vector() -> Model<Agent, 8> {
    let mut m = build_four_city();
    let timelimit = 1000.;
    while m.time < timelimit {
        m.step_limited(Some(timelimit));
    }
    /*let r = m.state.agents.borrow_mut().clone();
    r*/
    m
}

fn check_city_counts() {
    let mut m = build_three_city();
    while m.time < 10. {
        m.step();
    }
    for ag in m.state.agents.borrow().iter() {
        match ag {
            Agent::Location { .. } => println!("{:?}", ag),
            _ => {}
        }
    }
}

fn get_validation_observables(model: &Model<Agent, 8>) -> HashMap<String, f64> {
    let dat = model.state.agents.borrow_mut().clone();
    let mut result = HashMap::new();
    result.insert(
        "num_cities/locations".to_string(),
        dat.iter()
            .filter(|k| match k {
                Agent::Location { .. } => true,
                _ => false,
            })
            .count() as f64,
    );
    let cities: Vec<_> = observe_all_Location(&model).collect();
    for (n, loc) in cities.iter().enumerate() {
        result.insert(
            format!("city_{}_Migrants", n),
            loc.migrants().count() as f64,
        );
        result.insert(format!("city_{}_x", n), loc.get_x());
        result.insert(format!("city_{}_accuracy", n), loc.accuracy_about());
        result.insert(format!("city_{}_quality", n), loc.get_quality());
        result.insert(format!("city_{}_visits", n), loc.get_visits() as f64);
        result.insert(
            format!("city_{}_N_Info", n),
            loc.information1().count() as f64,
        );
    }

    let links: Vec<_> = observe_all_Link(&model).collect();
    for (n, link) in links.iter().enumerate() {
        result.insert(format!("link_{}_accuracy", n), link.accuracy_about());
        result.insert(format!("link_{}_visits", n), link.get_visits() as f64);
        result.insert(format!("link_{}_distance", n), link.get_distance() as f64);
        result.insert(format!("link_{}_friction", n), link.get_friction());
        result.insert(
            format!("link_{}_N_Info", n),
            link.information2().count() as f64,
        );
    }

    let informations: Vec<_> = observe_all_Information(&model).collect();
    result.insert(
        format!("information_accuracy"),
        mean(informations.iter().map(|loc| loc.accuracy())),
    );
   /* result.insert(
        format!("information_quality"),
        mean(informations.iter().map(|loc| loc.get_quality())),
    );*/
    /*result.insert(
        format!("information_rec"),
        mean(informations.iter().map(|loc| loc.get_resources())),
    );*/

    /*result.insert(
        format!("#Migrants("),
        observe_all_Migrant(&model).count() as f64,
    );
    result.insert(
        format!("#Information("),
        observe_all_Information(&model).count() as f64,
    );
    result.insert(
        format!("#InfoLink("),
        observe_all_InfoLink(&model).count() as f64,
    );
    result.insert(format!("#Links("), observe_all_Link(&model).count() as f64);
    result.insert(
        format!("active migrants"),
        observe_all_Migrant(&model)
            .filter(|m| m.get_activ_migration())
            .count() as f64,
    );*/

    result
}

fn main_() {
    let now = Instant::now();
    let mut steps_taken = 0;
    for _ in 0..10 {
        let m = check_property_vector();
        steps_taken += m.stepnumber;
    }
    let fintime = now.elapsed().as_secs_f64();
    println!(
        "{} steps in {}s -> {} steps/s",
        steps_taken,
        fintime,
        steps_taken as f64 / fintime
    );
}
fn write_100_cities() {
    let len = 100;
    let num_link = 4;
    let num_endpoint = 4;
    let mut nodes: Vec<_> = (0..len)
        .map(|_| TopologyNode {
            x: thread_rng().gen_range(0., 1.0),
            y: thread_rng().gen_range(0., 1.0),
            loc_type: LocationType::Standard,
            quality: thread_rng().gen_range(0., 1.0),
            resources: thread_rng().gen_range(0., 1.0),
        })
        .collect();
    nodes.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap());
    for idx in (0..num_endpoint) {
        nodes[idx].loc_type = LocationType::Entry;
        nodes[len - 1 - idx].loc_type = LocationType::Exit;
    }
    let mut links = BTreeSet::new();
    for (idx1, n1) in nodes.iter().enumerate() {
        let mut distances: Vec<(usize, f64)> = nodes
            .iter()
            .map(|o| (o.x - n1.x).powi(2) + (o.y - n1.y).powi(2))
            .enumerate()
            .collect();
        distances.sort_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap());
        println!("{:?}", distances);
        for idx in (1..=num_link) {
            let oidx = distances[idx].0;
            if oidx < idx1 {
                links.insert((oidx, idx1));
            } else {
                links.insert((idx1, oidx));
            }
        }
    }
    let t = Topology {
        nodes: nodes,
        links: links
            .into_iter()
            .map(|(a, b)| TopologyLink {
                connects: (a, b),
                link_type: match thread_rng().gen_bool(0.80) {
                    true => LinkType::Fast,
                    false => LinkType::Slow,
                },
            })
            .collect(),
    };
    let mut file = File::create("topo_100_city.json").unwrap();
    serde_json::to_writer_pretty(file, &t).unwrap();
}
fn write_four_cities() {
    let t = Topology {
        nodes: vec![
            TopologyNode {
                // 0: entry
                x: 0.0,
                y: 0.5,
                loc_type: LocationType::Entry,
                quality: 0.0,
                resources: 0.0,
            },
            TopologyNode {
                //1: mid
                x: 0.5,
                y: 0.5,
                loc_type: LocationType::Standard,
                quality: 0.1,
                resources: 0.1,
            },
            TopologyNode {
                //2: Exit1
                x: 0.999,
                y: 0.0,
                loc_type: LocationType::Exit,
                quality: 0.5,
                resources: 0.5,
            },
            TopologyNode {
                //3: Exit1
                x: 0.999,
                y: 1.,
                loc_type: LocationType::Exit,
                quality: 1.0,
                resources: 1.0,
            },
        ],
        links: vec![
            TopologyLink {
                connects: (0, 1),
                link_type: LinkType::Fast,
            },
            TopologyLink {
                connects: (1, 2),
                link_type: LinkType::Fast,
            },
            TopologyLink {
                connects: (1, 3),
                link_type: LinkType::Fast,
            },
        ],
    };
    let mut file = File::create("topo_four_city.json").unwrap();
    serde_json::to_writer_pretty(file, &t).unwrap();
}

///////////////////
use std::cell::Ref;
use std::iter::Map;
use std::slice::Iter;

/*struct MigrantIterator<'a> {
    internal_iterator: Ref<'a, std::collections::btree_set::Iter<'a, usize>>,
}

impl<'a> MigrantIterator<'a> {
    fn new(inp: Ref<'a, BTreeSet<usize>>) -> MigrantIterator<'a> {
        MigrantIterator {
            internal_iterator: Ref::map(inp, |inp| &inp.iter()),
        }
    }
}

impl<'a> Iterator for MigrantIterator<'a> {
    type Item = Migrant;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}

impl<'a> Migrant {
    pub fn EdgeTest(&'a self) -> MigrantIterator<'a> {
        let r = Ref::map(self.state.edges.borrow(), |edgs| &edgs[2][2]);
        MigrantIterator {
            internal_iterator: r,
        }
        //result
        /*
        MIter {
            inner: Some(Ref::map(r, |v| &v[..])),
        }*/
        //impl Iterator<Item = Migrant> {
        /*let r =
        Ref::map(r, |r| &r.iter())*/
        //unimplemented!()
        /*.iter().*/
        //}
    }
}
*/
//////////
fn main() {
    //#write_100_cities();
    //return;
    //write_four_cities();
    /*let spinner_style = ProgressStyle::with_template("{prefix:.bold.dim} {spinner} {wide_msg}")
    .unwrap()
    .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ");*/

    let num_reps : u64 = std::env::args().nth(1).expect("no num reps").parse().unwrap(); //4 * 8;
    let time_limit : f64 =std::env::args().nth(2).expect("no endtime").parse().unwrap();

    let m = MultiProgress::new();
    let result: HashMap<String, Vec<Vec<f64>>> = (0..num_reps)
        .into_par_iter()
        .map(|k| {
            let mut pb = m.add(ProgressBar::new(NUM_OUTPUTS));
            ////pb.set_style(spinner_style.clone());
            /*pb.set_style(
                ProgressStyle::default_bar()
                    .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
                    .progress_chars("##-"),
            );*/
            pb.set_prefix(format!("[{}/{}]", k + 1, num_reps));

            pb
        })
        .progress_count(num_reps)
        .map(|bar| check_property_vector_multi(bar,time_limit))
        .map(|k| k.into_iter().map(|(key, val)| (key, vec![val])).collect())
        .reduce(
            || HashMap::new(),
            |mut old, add| {
                assert!(old.len() <= add.len());
                for (key, mut val) in add.into_iter() {
                    match old.insert(key.clone(), val.clone()) {
                        Some(mut x) => {
                            val.append(&mut x);
                            old.insert(key, val);
                        }
                        None => {}
                    }
                }
                old
            },
        );

    //println!("{:?}", result);

    let results_analysed: HashMap<String, Vec<f64>> = result
        .par_iter()
        .map(|(k, v)| {
            (
                k.to_owned() + "_median",
                (0..v[0].len())
                    .map(|n| stats::median(v.iter().map(|k| k[n])).unwrap())
                    .collect(),
            )
        })
        .chain(result.par_iter().map(|(k, v)| {
            (
                k.to_owned() + "_mean",
                (0..v[0].len())
                    .map(|n| stats::mean(v.iter().map(|k| k[n])))
                    .collect(),
            )
        }))
        .chain(result.par_iter().map(|(k, v)| {
            (
                k.to_owned() + "_stddev",
                (0..v[0].len())
                    .map(|n| stats::stddev(v.iter().map(|k| k[n])))
                    .collect(),
            )
        }))
        .collect();
    println!("writing file");
    let mut file = File::create("routes_output.json").unwrap();
    serde_json::to_writer(file, &results_analysed).unwrap();
    //let mut keys: Vec<_> = result.keys().collect();
    //keys.sort();
    /*for (key) in keys {
        let val = &result[key];
        println!(
            "{} : {} (+-{})",
            key,
            mean(val.clone().into_iter()),
            stddev(val.clone().into_iter())
        ); //, val.mean());
    }*/

    //check_city_counts();
    //let mut m = build_three_city();
    // println!("generated");

    /*for n in 0..100 {
        //println!("\nState at step {}", n);
        //m.print_full_graph();
        //println!("{}",m.generate_json().to_string());
        m.step();
    }*/
}
