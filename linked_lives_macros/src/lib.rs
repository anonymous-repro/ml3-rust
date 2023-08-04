extern crate proc_macro;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};

use quote::quote;
use syn::__private::TokenStream2;
use syn::parse::{Parse, ParseBuffer};
use syn::{parse_macro_input, Data, DeriveInput, Fields, Result, Variant};

use global_counter::primitive::exact::CounterUsize;

#[macro_use]
extern crate lazy_static;

use std::collections::HashMap;
use std::sync::Mutex;

static NUMBER_OF_EDGES: CounterUsize = CounterUsize::new(0);

const CLEAR_AFTER_USE: bool = true;

/*fn get_ident_hash(ident: &Ident) -> u64 {
    let mut hasher = DefaultHasher::new();
    ident.hash(&mut hasher);
    hasher.finish()
}*/

struct UniEdgeInput {
    agent_type: syn::Type,
    edge_name: syn::Ident,
}

impl Parse for UniEdgeInput {
    fn parse(input: &ParseBuffer) -> Result<Self> {
        Ok(UniEdgeInput {
            agent_type: input.parse()?,
            edge_name: input.parse()?,
        })
    }
}

struct MultiMultiEdgeInput {
    agent_type_a: syn::Type,
    agent_type_b: syn::Type,
    edge_name_a: syn::Ident,
    edge_name_b: syn::Ident,
}

impl Parse for MultiMultiEdgeInput {
    fn parse(input: &ParseBuffer) -> Result<Self> {
        Ok(MultiMultiEdgeInput {
            agent_type_a: input.parse()?,
            edge_name_a: input.parse()?,
            agent_type_b: input.parse()?,
            edge_name_b: input.parse()?,
        })
    }
}

#[proc_macro]
pub fn add_uni_edge_single(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as UniEdgeInput);
    let edge_name = input.edge_name;
    let agent_type = input.agent_type;

    let tokens = quote! {
        impl #agent_type{
            fn #edge_name() -> bool {
            return false;
            }
        }
    };

    tokens.into()
}

fn add_one_side_of_singleedge(
    edge_name: Ident,
    type_of_retured_agent: &syn::Type,
    type_of_this_agent: &syn::Type,
    edge_number: &syn::LitInt,
    allow_multi_on_this_side: bool,
) -> TokenStream {
    let model_trait_name = Ident::new(&format!("TraitToAdd{}", edge_name), Span::call_site());
    let model_trait_function = Ident::new(&format!("set_{}", edge_name), Span::call_site());

    let add_to_edge_name = Ident::new(&format!("set_{}", edge_name), Span::call_site());
    let remove_from_edge_name =
        Ident::new(&format!("remove_from_{}", edge_name), Span::call_site());

    let edge_name_mut = Ident::new(&format!("{}_mut", edge_name), Span::call_site());

    let tokens = quote! {
        impl #type_of_this_agent{
            fn #edge_name(&self) -> Option<#type_of_retured_agent> {
                //println!("Retriving {} as {}",stringify!(#edge_name),stringify!(#edge_number));
                assert!(self.state.edges.borrow()[self.idx][ #edge_number ].len() <= 1);
                let retval =
                    match self.state.edges.borrow()[self.idx][ #edge_number ].iter().next(){
                    None => None,
                    Some(idx) => Some(#type_of_retured_agent { idx: *idx,state:self.state.clone()}),
                };

                match self.state.idx_of_current_potential_transition.get(){
                    Some(x) => {
                                        let mut dep_graph = &mut self.state.dependency_graph_edges.borrow_mut()[self.idx];

                                        // when accessing for the first time, make shure there is enough space
                                        while dep_graph.len() <= #edge_number {
                                            dep_graph.push(vec![]);
                                        }
                                        dep_graph[ #edge_number ].binary_insert(x);
                    }
                    None => {}
                }


                retval
                /*match return_val{
                    Some(x) => Some(x),
                    None => None
                }*/
            }

            // TODO fix this function
            pub fn #edge_name_mut(&mut self) -> Option<#type_of_retured_agent>{
                unimplemented!()
                //match self.
            }
            pub fn #add_to_edge_name(&mut self,other_agent : &#type_of_retured_agent){
                self.state.add_edge_internal(self.idx,other_agent.idx,#edge_number,stringify!(#edge_name).to_string(),#allow_multi_on_this_side,false,true);
            }
            pub fn #remove_from_edge_name(&mut self,other_agent : &#type_of_retured_agent){
                unimplemented!("Removing from edge not yet implemented");
            }
        }

        trait #model_trait_name {
            fn #model_trait_function(&mut self,from : usize, to: usize);
        }
        impl <AG : Debug ,const MAX_NUM_ATTRIBUTES: usize> #model_trait_name for Model<AG,MAX_NUM_ATTRIBUTES> {
            fn #model_trait_function(&mut self,from : usize, to: usize){
                self.state.add_edge_internal(from,to,#edge_number,stringify!(#edge_name).to_string(),#allow_multi_on_this_side, false,false);
            }
        }
    };

    tokens.into()
}

fn add_one_side_of_multiedge(
    edge_name: Ident,
    type_of_retured_agent: &syn::Type,
    type_of_this_agent: &syn::Type,
    edge_number: &syn::LitInt,
    allow_multi_on_this_side: bool,
) -> TokenStream {
    /*let type_of_this_agent_mut = Ident::new(
        &format!("{}_mut", type_of_this_agent.into_token_stream()),
        Span::call_site(),
    );*/

    /*let type_of_retured_agent_mut = Ident::new(
        &format!("{}_mut", type_of_retured_agent.into_token_stream()),
        Span::call_site(),
    );*/

    let model_trait_name = Ident::new(&format!("TraitToAdd{}", edge_name), Span::call_site());
    let model_trait_function = Ident::new(&format!("add_edge_{}", edge_name), Span::call_site());

    let add_to_edge_name = Ident::new(&format!("add_to_{}", edge_name), Span::call_site());
    let remove_from_edge_name =
        Ident::new(&format!("remove_from_{}", edge_name), Span::call_site());

    let tokens = quote! {
        impl #type_of_this_agent{
            pub fn #edge_name(&self) -> impl Iterator<Item=#type_of_retured_agent> {
                let res : Vec<_> = self.state.edges.borrow()[self.idx][ #edge_number ]
                    .iter().map(|idx| #type_of_retured_agent { idx: *idx,state:self.state.clone()}).collect();

                match self.state.idx_of_current_potential_transition.get(){
                    Some(x) => {
                           let mut dep_graph = &mut self.state.dependency_graph_edges.borrow_mut()[self.idx];
                            // when accessing for the first time, make shure there is enough space
                            while dep_graph.len() <= #edge_number {
                                dep_graph.push(vec![]);
                            }
                            dep_graph[ #edge_number ].binary_insert(x);
                    },
                    None => {}
                }



                res.into_iter()
            }

            pub fn #add_to_edge_name(&self,other_agent : &#type_of_retured_agent){
                self.state.add_edge_internal(self.idx,other_agent.idx,#edge_number,stringify!(#edge_name).to_string(),#allow_multi_on_this_side,true,false);
            }
            pub fn #remove_from_edge_name(&self,other_agent : &#type_of_retured_agent){
                unimplemented!("Removing from edge not yet implemented");
            }
        }

        trait #model_trait_name {
            fn #model_trait_function(&mut self,from : usize, to: usize);
        }
        impl <AG : Debug ,const MAX_NUM_ATTRIBUTES: usize> #model_trait_name for Model<AG,MAX_NUM_ATTRIBUTES> {
            fn #model_trait_function(&mut self,from : usize, to: usize){
                self.state.add_edge_internal(from,to,#edge_number,stringify!(#edge_name).to_string(),#allow_multi_on_this_side,true,false);
            }
        }
    };

    tokens.into()
}

#[proc_macro]
pub fn add_uni_edge_multi(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as UniEdgeInput);
    let edge_name = input.edge_name;
    let agent_type = input.agent_type;

    let edge_number_v = NUMBER_OF_EDGES.get();
    let edge_number = syn::LitInt::new(&format!("{}", edge_number_v), Span::call_site());
    NUMBER_OF_EDGES.inc();

    add_one_side_of_multiedge(edge_name, &agent_type, &agent_type, &edge_number, true)
}

#[proc_macro]
pub fn add_multi_multi_edge(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as MultiMultiEdgeInput);
    let edge_name_a = input.edge_name_a;
    let edge_name_b = input.edge_name_b;
    let agent_type_a = input.agent_type_a;
    let agent_type_b = input.agent_type_b;

    let edge_number_v = NUMBER_OF_EDGES.get();
    let edge_number = syn::LitInt::new(&format!("{}", edge_number_v), Span::call_site());
    NUMBER_OF_EDGES.inc();

    let e1 = add_one_side_of_multiedge(
        edge_name_b,
        &agent_type_b,
        &agent_type_a,
        &edge_number,
        true,
    );
    let e2 = add_one_side_of_multiedge(
        edge_name_a,
        &agent_type_a,
        &agent_type_b,
        &edge_number,
        true,
    );

    e1.into_iter().chain(e2.into_iter()).collect()
}

#[proc_macro]
pub fn add_single_multi_edge(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as MultiMultiEdgeInput);
    let edge_name_a = input.edge_name_a;
    let edge_name_b = input.edge_name_b;
    let agent_type_a = input.agent_type_a;
    let agent_type_b = input.agent_type_b;

    let edge_number_v = NUMBER_OF_EDGES.get();
    let edge_number = syn::LitInt::new(&format!("{}", edge_number_v), Span::call_site());
    NUMBER_OF_EDGES.inc();

    let e1 = add_one_side_of_multiedge(
        edge_name_b,
        &agent_type_b,
        &agent_type_a,
        &edge_number,
        false,
    );
    let e2 = add_one_side_of_singleedge(
        edge_name_a,
        &agent_type_a,
        &agent_type_b,
        &edge_number,
        true,
    );
    e1.into_iter().chain(e2.into_iter()).collect()
}

fn generate_struct(
    variant: &Variant,
    enum_name: &Ident,
    match_pattern: &TokenStream2,
    max_num_fields: usize,
    all_variant_names: &Vec<&Ident>,
    all_variant_fields: &Vec<&Fields>,
) -> TokenStream2 {
    let name = &variant.ident;
    //let _name_mut = Ident::new(&format!("{}_mut", name), Span::call_site());
    let getters = variant.fields.iter().enumerate().map(|(field_counter,f)| {
        let name = f
            .ident
            .as_ref()
            .expect("Can not work with field without name!");
        let getter_name = Ident::new(&format!("get_{}", name), Span::call_site());
        let getter_name_ref = Ident::new(&format!("get_{}_ref", name), Span::call_site());
        let set_name = Ident::new(&format!("set_{}", name), Span::call_site());
         let getter_name_mut = Ident::new(&format!("get_{}_mut", name), Span::call_site());
        //let name_hash = get_ident_hash(name);

        let the_type = &f.ty;
        quote! { fn #getter_name_ref(&self) -> Ref<#the_type>{
            match self.state.idx_of_current_potential_transition.get(){
                Some(x) => { let mut dep_graph = &mut self.state.dependency_graph_attributes.borrow_mut()[self.idx];
            dep_graph[#field_counter].binary_insert(x);},
                None => {} // No need to track a read
            }


            Ref::map(self.state.agents.borrow(),|k|
                       match &k[self.idx] {
               #enum_name::#match_pattern => #name,
               _ => panic!("Calling {} on an {:?}!!",stringify!(#getter_name_ref),self.state.agents.borrow()[self.idx]),
                }
                )

            }
            fn #getter_name(&self) -> #the_type {
                (*self.#getter_name_ref()).clone()
            }

            fn #getter_name_mut(&self) -> RefMut<#the_type>{
                    //let current_potential_reaction_idx = self.state.idx_of_current_potential_transition.get();
                    let mut dep_graph = &mut self.state.dependency_graph_attributes.borrow_mut()[self.idx];
                    //let my_identifier = AttributeIdentifier{name_hash : #name_hash};///*name: stringify!(#name).to_string()*/,idx_of_agent: self.idx};
                    self.state.idices_to_update.borrow_mut().extend(dep_graph[#field_counter].iter());
                    if #CLEAR_AFTER_USE{
                    dep_graph[#field_counter].clear();
                    }

                    RefMut::map(self.state.agents.borrow_mut(),|k|
                               match &mut k[self.idx] {
                       #enum_name::#match_pattern => #name,
                       _ => panic!("Calling {} on an {:?}!!",stringify!(#getter_name_ref),self.state.agents.borrow()[self.idx]),
                   }
                    )
            }

            fn #set_name(&mut self, val : #the_type) {
                let mut v: &mut #the_type  = &mut self.#getter_name_mut();
                *v = val;
            }
        }
    });
    let getters = quote! { #(#getters) *};

    let agent_creator_functions =
        all_variant_names
            .iter()
            .zip(all_variant_fields.iter())
            .map(|(n, fields)| {
                let all_name = Ident::new(&format!("iterate_all_{}", n), Span::call_site());
                let function_name = Ident::new(&format!("create_new_{}", n), Span::call_site());

                let field_initial = fields
                    .iter()
                    .map(|k| k.ident.as_ref().unwrap())
                    .map(|k| quote!(#k : Default::default(),));
                let field_initials = quote! {#enum_name::#n{#(#field_initial)*}};
                //let return_type = Ident::new(&format!("{}_mut", n), Span::call_site());
                quote! {
                    pub fn #function_name(&self,agent : #enum_name) -> #n{
                        assert!(std::matches!(agent,#enum_name::#n{..}));
                        let idx = self.state.add_agent_to_state(agent);
                        #n { idx: idx,state:self.state.clone()}
                    }

                    pub fn #all_name(&self) -> impl Iterator<Item=#n> {
                            let tester = #field_initials ;
                            let disc = std::mem::discriminant(&tester);
                            let empty = vec![];
                            let result: Vec<_> = self
                                .state
                                .agentidx_by_type
                                .borrow()
                                .get(&disc)//std::mem::discriminant(&disc))
                                .unwrap_or(&empty)
                                /*.filter(|(n, k)| match k {
                                    #enum_name::#n { .. } => true,
                                    _ => false,
                                })*/
                                .iter()
                                .map(|n| #n {
                                    idx: *n,
                                    state: self.state.clone(),
                                })
                                .collect();
                            result.into_iter()
                    }
                }
            });
    let agent_creator_functions = quote! { #(#agent_creator_functions) *};
    let observables_name = Ident::new(&format!("observe_all_{}", name), Span::call_site());
    quote! {
        struct #name {
            idx : usize,
            state : Rc<ReactionState<#enum_name,#max_num_fields>>,
        }
        fn  #observables_name<'a>(m: &'a Model<#enum_name,#max_num_fields>) -> impl Iterator<Item=#name> + 'a{
                let result: Vec<_> = m
                    .state
                    .agents
                    .borrow()
                    .iter()
                    .enumerate()
                    .filter_map(|(n, k)| match k {
                        #enum_name :: #name { .. } => Some(#name {
                            idx: n,
                            state: m.state.clone(),
                        }),
                        _ => None,
                    })
                    .collect();
                result.into_iter()
        }
        impl #name {
            #getters
            #agent_creator_functions
        }
        impl PartialEq for #name{
            fn eq(&self, other: &Self) -> bool{
                self.idx == other.idx
            }
        }
        impl Eq for #name{}
        impl Hash for #name{
            fn hash<H: Hasher>(&self, state: &mut H) {
                self.idx.hash(state);
            }
        }
    }
}

fn get_match_pattern(variant: &Variant) -> TokenStream2 {
    let name = &variant.ident;
    let getters = variant.fields.iter().map(|f| f.ident.as_ref().unwrap());
    quote! {#name{ #(#getters), * }}
}

#[proc_macro_derive(AgentEnum)]
pub fn my_derive_for_enum(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name_of_enum = input.ident;

    let adding_rules = match input.data {
        Data::Struct(_s) => {
            unreachable!("AgentEnum can only be derived for Enums, not structs!")
        }
        Data::Enum(e) => {
            /*let max_num_attributes = match e.variants.iter().map(|k| k.fields.iter().count()).max()
            {
                None => 0,
                Some(y) => y,
            };*/
            let max_num_fields_per_variant =
                match e.variants.iter().map(|f| f.fields.iter().count()).max() {
                    None => 0,
                    Some(y) => y,
                };
            let all_variant_names: Vec<&Ident> = e.variants.iter().map(|f| &f.ident).collect();
            let all_variant_fields: Vec<_> = e.variants.iter().map(|f| &f.fields).collect();
            let y = e.variants.iter().map(|f| {
                let variant_name = &f.ident;
                let match_pattern = get_match_pattern(f);
                //let name_mut = Ident::new(&format!("{}_mut", variant_name), Span::call_site());
                let variant_struct = generate_struct(f, &name_of_enum, &match_pattern,max_num_fields_per_variant,&all_variant_names,&all_variant_fields);
                //let variant_struct_mut = generate_struct_mut(f, &name_of_enum, &name_mut,&match_pattern,max_num_fields_per_variant,&all_variant_names);

                let getter_name = Ident::new(
                    &format!("add_transition_for_{}", variant_name),
                    Span::call_site(),
                );
                let trait_name =
                    Ident::new(&format!("TraitAddRule{}", variant_name), Span::call_site());
                quote! {
                        #variant_struct
                        //#variant_struct_mut
                        trait #trait_name{
                            fn #getter_name<
                    FGUAR: 'static + Fn(&#variant_name) -> bool,
                    FRATE: 'static + Fn(&#variant_name) -> f64,
                    FEFFECT: 'static + Fn(&mut #variant_name) ,
                >( &mut self,
                    guard: FGUAR,
                    rate: FRATE,
                    effect: FEFFECT,
                );
                        }
                    // TODO add known number here
                  impl #trait_name for Model<#name_of_enum,#max_num_fields_per_variant>{
                    fn #getter_name<
                    FGUAR: 'static + Fn(&#variant_name) -> bool,
                    FRATE: 'static + Fn(&#variant_name) -> f64,
                    FEFFECT: 'static + Fn(&mut #variant_name),
                >( &mut self,
                    guard: FGUAR,
                    rate: FRATE,
                    effect: FEFFECT,
                ){
                    self.transition_rule_prototypes.push(
                     TransitionRule{
                        rate : Box::new(move |idx: usize, state : Rc<ReactionState<#name_of_enum,#max_num_fields_per_variant>>| {
                          // println!("New rate calculation for {}!",idx);
                          let object =#variant_name { idx,state};
                          if (guard)(&object){
                            return (rate)(&object);
                          } else {
                          return 0.;
                          }
                        }),
                        matches_type : Box::new(|ag : &#name_of_enum| match ag {
                            #name_of_enum::#match_pattern => {true},
                            _ => false,
                        }),
                        effect : Box::new(move |idx: usize, state : Rc<ReactionState<#name_of_enum,#max_num_fields_per_variant>>|{
                          let mut object =#variant_name { idx,state};
                            return (effect)(&mut object);
                        })
                     }
                    );
                }
                  }

                      }
            });
            quote! { #(#y) *
                impl #name_of_enum {
                    pub fn new_model() -> Model<Agent, #max_num_fields_per_variant> {
                        Model::new()
                    }
                }
            }
        }
        Data::Union(_) => {
            unreachable!("AgentEnum can only be derived for Enums, not structs!")
        }
    };

    let tokens = quote! {
        #adding_rules
    };

    tokens.into()
}
