mod simulation;

//const MAX_NUMBER_OF_EDGETYPES: usize = 2;
const CLEAR_AFTER_USE: bool = true;

#[cfg(test)]
mod tests;

use array_macro::array;
use indicatif::{ProgressBar, ProgressStyle};
use itertools::Itertools;
pub use linked_lives_macros::*;
use rand::prelude::*;
use rand_distr::Distribution;
pub use serde::{Deserialize, Serialize};
use serde_json::json;
pub use simulation::*;
pub use std::cell::{Cell, Ref, RefCell, RefMut};
use std::collections::{BTreeMap, BTreeSet, HashMap};
pub use std::fmt::Debug;
pub use std::hash::{Hash, Hasher};
use std::io::Write;
pub use std::rc::Rc;

use std::fs::File;
use std::mem::Discriminant;

pub trait SetInsertable {
    fn binary_insert(&mut self, val: usize) -> bool;
}
impl SetInsertable for Vec<usize> {
    fn binary_insert(&mut self, val: usize) -> bool {
        //println!("-- {} {}--", self.len(), self.capacity());
        match self.binary_search(&val) {
            Ok(_) => true,
            Err(pos) => {
                //println!("Insert {} at {} in {:?}", val, pos, self);
                self.insert(pos, val);
                false
            }
        }
    }
}

pub trait Only {
    type Item;
    fn only(self) -> Self::Item;
}

impl<T> Only for T
where
    T: Iterator,
{
    type Item = T::Item;

    fn only(mut self) -> T::Item {
        match self.next() {
            None => {
                panic!("Iterator is empty. `.only()` as failed.")
            }
            Some(x) => match self.next() {
                None => x,
                Some(_) => {
                    panic!("Iterator contains more than one element. `.only()` as failed.")
                }
            },
        }
    }
}

#[derive(Hash, Eq, PartialEq, Clone, Debug, Ord, PartialOrd)]
pub struct AttributeIdentifier {
    //pub name: String,
    pub name_hash: u64,
    //pub idx_of_agent: usize,
}

pub struct TransitionRule<AG, const MAX_NUM_ATTRIBUTES: usize> {
    pub rate: Box<dyn Fn(/*&AG,*/ usize, Rc<ReactionState<AG, MAX_NUM_ATTRIBUTES>>) -> f64>,
    pub effect: Box<dyn Fn(usize, Rc<ReactionState<AG, MAX_NUM_ATTRIBUTES>>)>,
    pub matches_type: Box<dyn Fn(&AG) -> bool>,
}

pub struct ReactionState<AG, const MAX_NUM_ATTRIBUTES: usize> {
    pub idx_of_current_potential_transition: Cell<Option<usize>>,
    //pub track_reads: Cell<bool>,
    // mapping Attributes/Edges to reactions
    pub dependency_graph_attributes: RefCell<Vec<[Vec<usize>; MAX_NUM_ATTRIBUTES]>>,
    pub dependency_graph_edges: RefCell<Vec<Vec<Vec<usize>>>>,
    pub idices_to_update: RefCell<BTreeSet<usize>>,
    pub agents: RefCell<Vec<AG>>,
    pub agentidx_by_type: RefCell<HashMap<Discriminant<AG>, Vec<usize>>>,
    pub idices_of_new_agents: RefCell<Vec<usize>>,
    pub edges: RefCell<Vec<Vec<BTreeSet<usize>>>>,
    pub edge_names: RefCell<Vec<Option<String>>>,
}

impl<AG, const MAX_NUM_ATTRIBUTES: usize> ReactionState<AG, MAX_NUM_ATTRIBUTES> {
    pub fn add_agent_to_state(&self, ag: AG) -> usize {
        let new_idx = self.agents.borrow().len();
        let discrim = std::mem::discriminant(&ag);
        if match self.agentidx_by_type.borrow_mut().get_mut(&discrim) {
            None => true,
            Some(y) => {
                y.push(new_idx);
                false
            }
        } {
            self.agentidx_by_type
                .borrow_mut()
                .insert(discrim, vec![new_idx]);
        }
        self.agents.borrow_mut().push(ag);
        self.idices_of_new_agents.borrow_mut().push(new_idx);
        self.dependency_graph_attributes.borrow_mut().push(
            array![ Vec::new();MAX_NUM_ATTRIBUTES], // BTreeSet::new(),
                                                    //BTreeSet::new(),
                                                    /* BTreeSet::new(),
                                                    BTreeSet::new(),
                                                    BTreeSet::new(),*/
                                                    // ],
        );
        self.dependency_graph_edges
            .borrow_mut()
            // TODO This sets the maximum number of EDGES (EDGE TYPES) an agent can have. This could relativly be determined at compile time...
            .push(vec![Vec::new(); 0]);
        self.edges.borrow_mut().push(vec![Default::default(); 10]);
        new_idx
    }

    pub fn remove_edge_internal(
        &self,
        edges: &mut RefMut<Vec<Vec<BTreeSet<usize>>>>,
        from: usize,
        to: usize,
        edge_number: usize,
        edge_name: &String,
    ) {
        assert!(from != to);

        //let mut edges = self.edges.borrow_mut();
        //let my_identifier = AttributeIdentifier{name_hash : #edge_name_hash};///*name: stringify!(#edge_name).to_string()*/,idx_of_agent: from};
        assert!(edges[from].get_mut(edge_number).unwrap().remove(&to));

        self.idices_to_update
            .borrow_mut()
            .extend(self.dependency_graph_edges.borrow_mut()[from][edge_number].iter());
        self.dependency_graph_edges.borrow_mut()[from][edge_number].retain(|v| *v != to);

        assert!(edges[to].get_mut(edge_number).unwrap().remove(&from));

        self.idices_to_update
            .borrow_mut()
            .extend(self.dependency_graph_edges.borrow_mut()[to][edge_number].iter());

        self.dependency_graph_edges.borrow_mut()[to][edge_number].retain(|v| *v != from);
    }

    pub fn add_edge_internal(
        &self,
        from: usize,
        to: usize,
        edge_number: usize,
        edge_name: String,
        _multi_from: bool,
        _multi_to: bool,
        force_overwrite: bool,
    ) -> bool {
        if self.edge_names.borrow().len() <= edge_number
            || self.edge_names.borrow()[edge_number].is_none()
        {
            let mut edge_names_ = self.edge_names.borrow_mut();
            while (*edge_names_).len() <= edge_number {
                edge_names_.push(None);
            }
            let variant = match _multi_to {
                true => "[n]",
                false => "[1]",
            };
            edge_names_[edge_number] = Some(format!("{} {}", edge_name, variant).to_string());
        }
        // println!("Add edge {} from {} to {}", edge_name, from, to);
        assert!(from != to);
        // Todo Explicitly check cardinality!!!

        let first_insertion_done;
        let insertion_completed;

        let mut edges = self.edges.borrow_mut();
        //let my_identifier = AttributeIdentifier{name_hash : #edge_name_hash};///*name: stringify!(#edge_name).to_string()*/,idx_of_agent: from};
        if edges[from]
            .get_mut(edge_number)
            .expect("Number of allowed edges is set to small")
            .insert(to)
        {
            first_insertion_done = true;
            if !_multi_to {
                if edges[from].get(edge_number).unwrap().len() > 1 {
                    if force_overwrite {
                        let other_to = *edges[from]
                            .get(edge_number)
                            .unwrap()
                            .iter()
                            .filter(|k| **k != to)
                            .only();
                        self.remove_edge_internal(
                            &mut edges,
                            from,
                            other_to,
                            edge_number,
                            &edge_name,
                        );
                    } else {
                        panic!(
                            "On this edge, there should be only one '{}' but you added a second",
                            edge_name
                        );
                    }
                }
            }
        } else {
            first_insertion_done = false;
            println!("Insert_failed")
        }

        while self.dependency_graph_edges.borrow_mut()[from].len() <= edge_number {
            self.dependency_graph_edges.borrow_mut()[from].push(vec![]);
        }

        self.idices_to_update
            .borrow_mut()
            .extend(self.dependency_graph_edges.borrow_mut()[from][edge_number].iter());
        if CLEAR_AFTER_USE {
            self.dependency_graph_edges.borrow_mut()[from][edge_number].clear();
        }

        while self.dependency_graph_edges.borrow_mut()[to].len() <= edge_number {
            self.dependency_graph_edges.borrow_mut()[to].push(vec![]);
        }

        if edges[to]
            .get_mut(edge_number)
            .expect("Number of allowed edges is set to small")
            .insert(from)
        {
            insertion_completed = true;

            if !_multi_from {
                if edges[to].get(edge_number).unwrap().len() > 1 {
                    if force_overwrite {
                        self.remove_edge_internal(&mut edges, to, from, edge_number, &edge_name);
                    } else {
                        panic!(
                            "On this edge, there should be only one '{}' but you added a second",
                            edge_name
                        );
                    }
                }
            }
        } else {
            if !force_overwrite && !first_insertion_done {
                panic!(
                    "Edge {} from id {} to id {} has already been added!",
                    stringify!(#edge_name),
                    to,
                    from
                );
            }
            insertion_completed = first_insertion_done;
        }

        self.idices_to_update
            .borrow_mut()
            .extend(self.dependency_graph_edges.borrow_mut()[to][edge_number].iter());
        if CLEAR_AFTER_USE {
            self.dependency_graph_edges.borrow_mut()[to][edge_number].clear();
        }
        return insertion_completed;
    }
}

struct PotentialReaction {
    idx_of_prototype: usize,
    idx_of_main_agent: usize,
}

pub struct Model<AG: Debug, const MAX_NUM_ATTRIBUTES: usize> {
    pub state: Rc<ReactionState<AG, MAX_NUM_ATTRIBUTES>>,
    pub transition_rule_prototypes: Vec<TransitionRule<AG, MAX_NUM_ATTRIBUTES>>,
    potential_reactions: Vec<PotentialReaction>,
    propensities: Vec<f64>,
    propensitiy_tree: Vec<f64>,
    //propensity_sum: f64,
    pub stepnumber: usize,
    pub time: f64,
    rng: rand::rngs::ThreadRng,
}

impl<AG: Debug, const MAX_NUM_ATTRIBUTES: usize> Model<AG, MAX_NUM_ATTRIBUTES> {
    pub fn new() -> Model<AG, MAX_NUM_ATTRIBUTES> {
        Model {
            state: Rc::new(ReactionState {
                idx_of_current_potential_transition: Cell::new(None),
                //track_reads: Cell::new(true),
                dependency_graph_attributes: RefCell::new(Default::default()),
                dependency_graph_edges: RefCell::new(vec![]),
                idices_to_update: RefCell::new(Default::default()),

                agents: RefCell::new(vec![]),
                agentidx_by_type: RefCell::new(Default::default()),
                idices_of_new_agents: RefCell::new(vec![]),
                edges: RefCell::new(Default::default()),
                edge_names: RefCell::new(vec![]),
            }),
            transition_rule_prototypes: vec![],
            potential_reactions: vec![],
            propensities: vec![],
            propensitiy_tree: vec![],
            //propensity_sum: 0.0,
            stepnumber: 0,
            time: 0.0,
            rng: Default::default(),
        }
    }

    fn handle_added_agents(&mut self) {
        let new_idices = RefCell::new(vec![]);
        self.state.idices_of_new_agents.swap(&new_idices);
        //println!("New state {:?}", self.state.agents.borrow());
        for agent_idx in new_idices.take() {
            let old_len = self.propensities.len();
            for (idx_of_prototype, rule) in self.transition_rule_prototypes.iter().enumerate() {
                if (rule.matches_type)(&self.state.agents.borrow()[agent_idx]) {
                    self.potential_reactions.push(PotentialReaction {
                        idx_of_prototype,
                        idx_of_main_agent: agent_idx,
                    });
                    self.propensities.push(0.);
                    self.propensitiy_tree.push(0.);
                }
            }

            for idx in old_len..self.propensities.len() {
                self.update_rate_calculation(idx);
            }
        }
    }

    pub fn create_new_agent(&mut self, agent: AG) -> usize {
        //println!("OLD state {:?}", self.state.agents.borrow());
        self.state.add_agent_to_state(agent);
        self.handle_added_agents();
        self.state.agents.borrow().len() - 1
    }
    pub fn print_full_graph(&self) {
        for (ag_id, ag) in self.state.agents.borrow().iter().enumerate() {
            println!("{} {:?}", ag_id, ag);
            for (edge_num, edges) in self.state.edges.borrow()[ag_id].iter().enumerate() {
                if !edges.is_empty() {
                    println!(
                        "   EDEGE_{} ({}) = {:?}",
                        edge_num,
                        self.state.edge_names.borrow()[edge_num].as_ref().unwrap(),
                        edges
                    );
                }
            }
        }
    }
}
impl<AG: Debug + Serialize, const MAX_NUM_ATTRIBUTES: usize> Model<AG, MAX_NUM_ATTRIBUTES> {
    pub fn generate_json(&self) -> serde_json::Value {
        let mut result: HashMap<usize, serde_json::Value> = HashMap::new();
        for (ag_id, ag) in self.state.agents.borrow().iter().enumerate() {
            //let agent = serde_json::to_value(ag).unwrap();
            let mut edges_map = HashMap::new();
            //println!("{} {:?}", ag_id, ag);
            for (edge_num, edges) in self.state.edges.borrow()[ag_id].iter().enumerate() {
                if !edges.is_empty() {
                    edges_map.insert(
                        self.state.edge_names.borrow()[edge_num]
                            .as_ref()
                            .unwrap()
                            .clone(),
                        edges.clone(),
                    );
                }
            }
            result.insert(ag_id, json!({"agent":ag, "edges":edges_map}));
        }
        serde_json::to_value(result).unwrap()
    }
}
