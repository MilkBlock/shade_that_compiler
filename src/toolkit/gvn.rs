use std::mem;
use crate::toolkit::field::Field;
use ahash::{HashMap, HashMapExt};
use bimap::{BiHashMap, BiMap};
use itertools::Itertools;
use petgraph::{ graph::node_index, stable_graph::EdgeReference, visit::EdgeRef, Direction::Incoming};
use crate::{debug_info_red, instr};
use crate::{debug_info_blue, instr_mut, make_field_trait_for_struct, node, node_mut, reg_field_for_struct, toolkit::{cfg_node::InstrList, eval_et, gen_instr_et::{parse_instr_list_to_et, }, gen_nhwc_cfg::process_temp_symbol}};

use super::etc::rpo_with_priority;
use super::{cfg_node::{CfgGraph, CfgNode}, context::DjGraph, et_node::{ EtEdge, EtTree}, etc::{self,  dfs_with_priority_enter_exit, rpo, rpo_with_predicate}, gen_instr_et::{first_rc_symidx_in_et_node, first_rc_symidx_in_et_node_may_literal}, gen_ssa::{cfg_is_dominated_by, instr_is_dominated_by, update_ssa_def_instr}, nhwc_instr::{InstrSlab, NhwcInstr, NhwcInstrType}, scope_node::ScopeTree, symtab::{SymTab, WithBorrow}};
use anyhow::*;


reg_field_for_struct!(CfgNode { COR_INSTR_ET_NODE_BIMAP:BiHashMap<usize,u32>,GVN_WHILE_COR_EXPR_HASH_MAP:HashMap<isize,u32>, } with_fields info);
make_field_trait_for_struct!(
    BiHashMap<usize,u32>
);

pub fn gvn(instr_et:&mut EtTree,dom_tree:&mut DjGraph, cfg_graph:&mut CfgGraph, symtab: &mut SymTab, instr_slab: &mut InstrSlab<NhwcInstr>,scope_tree:&mut ScopeTree)-> Result<()>{
    
    for (rc_func_symidx,cfg_entry) in symtab.get_global_info().get_all_cfg_func_symidx_entry_tuples().clone(){
        // debug_info_yellow!("{} :neighbors {:?}", start_node, nodes);

        let mut rc_symidx_et_node_map = HashMap::new();
        let &dj_entry = node!(at cfg_entry in cfg_graph).get_cor_dj_node();
        let mut expr_hash_map: std::collections::HashMap<isize, u32, ahash::RandomState> = HashMap::new();
        for (dom_node,access_state) in dfs_with_priority_enter_exit(dom_tree, dj_entry, |e|match e.weight(){
            crate::toolkit::dj_edge::DjEdge::Join {  } => -1, // don't travel through join edge
            crate::toolkit::dj_edge::DjEdge::Dom {  } => 1,
        }){
            let cor_cfg_node = node!(at dom_node in dom_tree).cor_cfg_node;
            match access_state{
                super::etc::AccessState::Enter => {
                    // also you should hold the expr hash map back when exit this node
                    // if node!(at cor_cfg_node in cfg_graph).cfg_node_type.is_while_loop(){
                    //     node_mut!(at cor_cfg_node in cfg_graph).add_gvn_while_cor_expr_hash_map(mem::take(&mut expr_hash_map))
                    //     // expr_hash_map.clear();
                    // }
                    // println!("enter {}", dom_node);

                    let mut instr_et_node_bimap = BiMap::new();
                    parse_instr_list_to_et(node!(at cor_cfg_node in cfg_graph).iter_all_instrs().cloned(), instr_et, symtab, &mut rc_symidx_et_node_map, &mut instr_et_node_bimap, scope_tree, instr_slab)?;
                    // let mut new_instr_list  = InstrList::new();
                    for &instr in node!(at cor_cfg_node in cfg_graph).phi_instrs.clone().iter()
                    .chain(node!(at cor_cfg_node in cfg_graph).instrs.clone().iter()){
                        // after parsing a instr into dag then try evaluate it and if evaluate to literal then renaming 
                        if let Some(&et_node) = instr!(at instr in instr_slab).get_op_cor_instr_et_node(){
                            // println!("compress et_node {et_node}");
                            // if instr_et.edges_directed(node_index(et_node as usize), Incoming).count() == 0{
                                // println!("compress_et_at {et_node}");
                                eval_et::compress_et_for_gvn(instr_et, et_node, &mut |op_found_et_node,et_node,et_tree|{
                                    true
                                } ,symtab, 0, scope_tree, &mut expr_hash_map)?;
                        }
                    }
                },
                super::etc::AccessState::Exit => {
                    // remove hash of outdated(not dominant after) instr_et_nodes
                    // println!("exit {}", dom_node);
                    for &instr in node!(at cor_cfg_node in cfg_graph).phi_instrs.clone().iter()
                    .chain(node!(at cor_cfg_node in cfg_graph).instrs.clone().iter()){
                        if instr!(at instr in instr_slab).has_cor_instr_et_node(){
                            if let Some(&et_node) = instr!(at instr in instr_slab).get_op_cor_instr_et_node(){
                                if let Some(def_symidx) = instr!(at instr in instr_slab).get_ssa_direct_def_symidx_vec().get(0){
                                    if first_rc_symidx_in_et_node(et_node, instr_et).is_ok(){
                                        if &first_rc_symidx_in_et_node(et_node, instr_et)? == def_symidx{
                                            if let Some(hash ) = &node!(at et_node in instr_et).hash{
                                                expr_hash_map.remove(hash);
                                                // println!("remove hash of {et_node} with symidx:{:?} in expr_hash_map {:?}", first_rc_symidx_in_et_node(et_node,instr_et).unwrap(),expr_hash_map);
                                            }else {
                                                // panic!();
                                            }
                                        }
                                    }
                                }
                                // expr_hash_map.remove(&node!(at et_node in instr_et).hash.unwrap());
                            }
                        }
                    }

                    // if node!(at cor_cfg_node in cfg_graph).cfg_node_type.is_while_loop(){
                    //     debug_info_red!("clear expr hash map {:?}",expr_hash_map);
                    //     expr_hash_map = node!(at cor_cfg_node in cfg_graph).get_gvn_while_cor_expr_hash_map()?.clone();
                    //     // expr_hash_map.clear();
                    // }
                },
            }
            // before visit next node we should ensure the et_node expr_hash_map should be deleted 
        }

        
            // eval_et::_compress_et(instr_et, 0, &mut |et_node,et_tree|true
            //     ,symtab, 0, scope_tree, &mut expr_hash_map,false)?;
            // eval_et::_compress_et(instr_et, 1, &mut |et_node,et_tree|true
            //     ,symtab, 0, scope_tree, &mut expr_hash_map,false)?;
            // eval_et::_compress_et(instr_et, 4, &mut |et_node,et_tree|true
            //     ,symtab, 0, scope_tree, &mut expr_hash_map,false)?;
    }
    update_ssa_def_instr(cfg_graph, symtab, instr_slab)?;
    for node_idx in instr_et.node_indices(){
        let et_node = node_idx.index() as u32;
        let et_node_struct = node!(at et_node in instr_et);
        if et_node_struct.equivalent_symidx_vec.len() == 0 { continue; }
        match &et_node_struct.et_node_type{
            super::et_node::EtNodeType::Operator { op, ast_node, text, op_rc_symidx } => {
                let first_symidx = first_rc_symidx_in_et_node(et_node, instr_et)?.as_ref_borrow().clone();
                // if symidx is temp then rename all equivalent symidx to temp 
                // because temp symbol is never redefined in our compiler's context, it's legal
                // if *symtab.get(&first_symidx.to_src_symidx())?.get_is_temp()?{
                    for rc_symidx in &et_node_struct.equivalent_symidx_vec[1..]{
                        // println!("access {rc_symidx:?}");
                        let &def_instr = symtab.get(&rc_symidx.as_ref_borrow()).get_ssa_def_instr();
                        // println!("delete {:?} into {:?}",instr!(at def_instr in instr_slab)?.get_cfg_instr_idx(),NhwcInstrType::Nope {  });
                        let mut symidx = rc_symidx.as_ref_borrow_mut();
                        mem::swap(&mut symtab.get_mut(&symidx).rc_symidx,&mut symidx.clone().as_rc());
                        // println!("rename {:?} into {:?}",symidx,first_symidx);
                        *instr_mut!(at def_instr in instr_slab) = NhwcInstrType::Nope {  }.into();
                        *symidx = first_symidx.clone()
                    }
                // }
            },
            super::et_node::EtNodeType::Literal { rc_literal_symidx, ast_node, text } => {
                let first_symidx = first_rc_symidx_in_et_node_may_literal(et_node, instr_et)?.as_ref_borrow().clone();
                for rc_symidx in &et_node_struct.equivalent_symidx_vec{
                    let mut symidx = rc_symidx.as_ref_borrow_mut();
                    if !symidx.is_literal(){
                        mem::swap(&mut symtab.get_mut(&symidx).rc_symidx,&mut symidx.clone().as_rc());
                        let &def_instr = symtab.get(&symidx).get_ssa_def_instr();
                        *instr_mut!(at def_instr in instr_slab) = NhwcInstrType::Nope {  }.into();
                        // println!("literal rename {:?} into {:?}",symidx,first_symidx);
                        *symidx = first_symidx.clone()
                    }else {
                        // {symidx;}
                        // println!("find {:?} is literal in eq_symidx_vec at {:?} ",rc_symidx, et_node_struct)
                        // panic!()
                    }
                }
            },
            super::et_node::EtNodeType::Symbol { rc_symidx, ast_node, text, decldef_def_or_use } => {
                let first_symidx = first_rc_symidx_in_et_node(et_node, instr_et)?.as_ref_borrow().clone();
                // if symidx is temp then rename all equivalent symidx to temp 
                // because temp symbol is never redefined in our compiler's context, it's legal

                if *symtab.get(&first_symidx.to_src_symidx()).get_is_temp(){
                    for rc_symidx in &et_node_struct.equivalent_symidx_vec[1..]{
                        let mut symidx = rc_symidx.as_ref_borrow_mut();
                        mem::swap(&mut symtab.get_mut(&symidx).rc_symidx,&mut symidx.clone().as_rc());
                        // println!("rename {:?} into {:?}",symidx,first_symidx);
                        let &def_instr = symtab.get(&symidx).get_ssa_def_instr();
                        *instr_mut!(at def_instr in instr_slab) = NhwcInstrType::Nope {  }.into();
                        *symidx = first_symidx.clone()
                    }
                }
            },
            super::et_node::EtNodeType::Separator { ast_node, text } => {panic!()},
        }
    }
    for (rc_func_symidx,cfg_entry) in symtab.get_global_info().get_all_cfg_func_symidx_entry_tuples().clone(){
        let dfs_vec = etc::dfs(cfg_graph,cfg_entry);
        for cfg_node in dfs_vec{
            for &instr in node!(at cfg_node in cfg_graph).iter_all_instrs(){
                let def_vec =  instr!(at instr in instr_slab).get_ssa_direct_def_symidx_vec();
                if def_vec.len() == 1 && def_vec[0].as_ref_borrow().is_literal(){
                    *instr_mut!(at instr in instr_slab) = NhwcInstrType::Nope {  }.into();
                }
            }
        }
    }
    Ok(())
}