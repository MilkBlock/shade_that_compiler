use crate::antlr_parser::cparser::{
    RULE_blockItem, RULE_blockItemList, RULE_breakpointStatement, RULE_declaration, RULE_expression, RULE_expressionStatement, RULE_forAfterExpression, RULE_forBeforeExpression, RULE_forCondition, RULE_forIterationStatement, RULE_forMidExpression, RULE_ifSelection, RULE_iterationStatement, RULE_jumpStatement, RULE_labeledStatement, RULE_selectionStatement, RULE_statement, RULE_switchSelection, RULE_whileIterationStatement
};
use crate::toolkit::ast_node::AstTree;
use crate::toolkit::cfg_edge::CfgEdge;
use crate::{add_edge, add_node, node, reg_field_for_struct, rule_id, RULE_compoundStatement, RULE_functionDefinition};
use crate::{find, find_nodes};
use anyhow::Result;
use petgraph::stable_graph::{EdgeIndex, NodeIndex};
use petgraph::visit::EdgeRef;
use syn::token::Loop;

use super::cfg_node::{CfgGraph, CfgNode, CfgNodeType};


use super::symtab::SymTab;
// 这个文件中没有在命名中提到是哪一中图中的节点，那么统一是 scope_node

reg_field_for_struct!(CfgNode{
    // CFG_COR_GATHER:u32,
} with_fields info);

pub fn process_while(cfg_graph:&mut CfgGraph, ast_tree:&AstTree, symtab:&mut SymTab, current_while_node:u32,loop_level:usize) -> Result<Option<(u32, u32)>> {
    let loop_level = loop_level+1;
    //expression做成whileloop节点
    let expr_node = find!(rule RULE_expression at current_while_node in ast_tree).unwrap();
    let statement_node = find!(rule RULE_statement at current_while_node in ast_tree).unwrap();
    let op_head_tail_nodes = process_stmt(cfg_graph, ast_tree, symtab, statement_node,loop_level)?;
    let loop_struct = CfgNode::new_while(expr_node,loop_level);
    let cfg_while_node = add_node!(loop_struct to cfg_graph);
    if let Some((st_head_node, st_tail_node)) = op_head_tail_nodes {
        add_edge!({CfgEdge::new_body_head()} from cfg_while_node to st_head_node in cfg_graph);
        add_edge!({CfgEdge::new_body_tail()} from st_tail_node to cfg_while_node in cfg_graph);
    } else {
        let empty_bb_struct = CfgNode::new_bb(vec![],loop_level);
        let empty_bb = add_node!(empty_bb_struct to cfg_graph);
        add_edge!({CfgEdge::new_body_head()} from cfg_while_node to empty_bb in cfg_graph);
        add_edge!({CfgEdge::new_body_tail()} from empty_bb to cfg_while_node in cfg_graph);
    }
    Ok(Some((cfg_while_node, cfg_while_node)))
}

///处理循环过程的cfg节点处理和连接，返回branch和statement的idx
pub fn process_iteration(cfg_graph:&mut CfgGraph, ast_tree:&AstTree, symtab:&mut SymTab, current_iteration_node:u32,loop_level:usize) -> Result<Option<(u32, u32)>> {
    //处理branch的构造
    let which_iteration_node = node!(at current_iteration_node in ast_tree).child_vec[0];
    match (rule_id!(at which_iteration_node in ast_tree), which_iteration_node) {
        (RULE_whileIterationStatement, while_node) => Ok(process_while(cfg_graph, ast_tree, symtab, while_node,loop_level)?),
        (RULE_forIterationStatement, for_node) => Ok(process_for(cfg_graph, ast_tree, symtab, for_node,loop_level)?),
        (_, _) => panic!("循环不属于for或while循环"),
    }
}
pub fn process_for(cfg_graph:&mut CfgGraph, ast_tree:&AstTree, symtab:&mut SymTab, current_for_node:u32,loop_level:usize) -> Result<Option<(u32, u32)>> {
    let loop_level = loop_level + 1;
    //forconditioin做成branch节点
    let for_condition_node = find!(rule RULE_forCondition at current_for_node in ast_tree).unwrap();
    let for_before_node = find!(rule RULE_forBeforeExpression at for_condition_node in ast_tree).unwrap();
    let for_mid_node = find!(rule RULE_forMidExpression at for_condition_node in ast_tree).unwrap();
    let for_after_node = find!(rule RULE_forAfterExpression at for_condition_node in ast_tree).unwrap();

    let statement_node = find!(rule RULE_statement at current_for_node in ast_tree).unwrap();
    let op_head_tail = process_stmt(cfg_graph, ast_tree, symtab, statement_node,loop_level)?;
    let branch_struct = CfgNode::new_for(for_before_node, for_mid_node, for_after_node,loop_level);
    let cfg_for_node = add_node!(branch_struct to cfg_graph);
    if let Some((st_head_node, st_tail_node)) = op_head_tail {
        add_edge!({CfgEdge::new_body_head()} from cfg_for_node to st_head_node in cfg_graph);
        add_edge!({CfgEdge::new_body_tail()} from st_tail_node to cfg_for_node in cfg_graph);
    } else {
        // 说明这个 for body 里面是空的，可即便是空的我们也要加上一个空的basic block 便于后续处理
        let empty_bb_struct = CfgNode::new_bb(vec![],loop_level);
        let empty_bb = add_node!(empty_bb_struct to cfg_graph);
        add_edge!({CfgEdge::new_body_head()} from cfg_for_node to empty_bb in cfg_graph);
        add_edge!({CfgEdge::new_body_tail()} from empty_bb to cfg_for_node in cfg_graph);
    }
    Ok(Some((cfg_for_node, cfg_for_node)))
}

/// 返回 生成CFG nodes 的 Option<(头,尾)>
pub fn process_stmt(cfg_graph:&mut CfgGraph, ast_tree:&AstTree, symtab:&mut SymTab, current_statement_node:u32,loop_level:usize) -> Result<Option<(u32, u32)>> {
    let which_statement = node!(at current_statement_node in ast_tree).child_vec[0];
    //匹配循环体内部的大括号，单语句，分支语句情况
    match (rule_id!(at which_statement in ast_tree), which_statement) {
        (RULE_compoundStatement, compoundstatement_node) => Ok(process_compound(cfg_graph, ast_tree, symtab, compoundstatement_node,loop_level)?),
        (RULE_expressionStatement, expressionstatement_node) => Ok(process_expression(cfg_graph, ast_tree, symtab, expressionstatement_node,loop_level)?),
        (RULE_iterationStatement, iter_node) => Ok(process_iteration(cfg_graph, ast_tree, symtab, iter_node,loop_level)?),
        (RULE_selectionStatement, select_node) => Ok(process_selection(cfg_graph, ast_tree, symtab, select_node,loop_level)?),
        (RULE_jumpStatement, jump_node) => {
            let bb_struct = CfgNode::new_bb(vec![jump_node],loop_level);
            let cfg_basicblock_node = add_node!(bb_struct to cfg_graph);
            Ok(Some((cfg_basicblock_node, cfg_basicblock_node)))
        }
        (RULE_breakpointStatement, breakpoint_node) => {
            let bb_struct = CfgNode::new_bb(vec![breakpoint_node],loop_level);
            let cfg_basicblock_node = add_node!(bb_struct to cfg_graph);
            Ok(Some((cfg_basicblock_node, cfg_basicblock_node)))
        }
        (RULE_labeledStatement, labeled_node) => {
            let bb_struct = CfgNode::new_bb(vec![labeled_node],loop_level);
            let cfg_basicblock_node = add_node!(bb_struct to cfg_graph);
            Ok(Some((cfg_basicblock_node, cfg_basicblock_node)))
        }
        (RULE_declaration, declare_node) => Ok(process_declartion(cfg_graph, ast_tree, symtab, declare_node,loop_level)?),
        (_, _) => panic!("不属于任何statement下等语句,ast树有错误 node:{}", which_statement),
    }
}
pub fn process_declartion(cfg_graph:&mut CfgGraph, _ast_tree:&AstTree, _symtab:&mut SymTab, current_declare_node:u32,loop_level:usize) -> Result<Option<(u32, u32)>> {
    let bb_struct = CfgNode::new_bb(vec![current_declare_node],loop_level);
    let cfg_basicblock_node = add_node!(bb_struct to cfg_graph);
    Ok(Some((cfg_basicblock_node, cfg_basicblock_node)))
}
pub fn process_expression(cfg_graph:&mut CfgGraph, _ast_tree:&AstTree, _symtab:&mut SymTab, current_expression_node:u32,loop_level:usize) -> Result<Option<(u32, u32)>> {
    let bb_struct = CfgNode::new_bb(vec![current_expression_node],loop_level);
    let cfg_basicblock_node = add_node!(bb_struct to cfg_graph);
    Ok(Some((cfg_basicblock_node, cfg_basicblock_node)))
}
///处理选择分支节点，内部区分if，switch，head_node连接到branch，gather连接到tail_node
pub fn process_selection(cfg_graph:&mut CfgGraph, ast_tree:&AstTree, symtab:&mut SymTab, current_selection_node:u32,loop_level:usize) -> Result<Option<(u32, u32)>> {
    //处理if和switch
    let which_selection_node = node!(at current_selection_node in ast_tree).child_vec[0];
    match (rule_id!(at which_selection_node in ast_tree), which_selection_node) {
        (RULE_ifSelection, if_node) => Ok(process_if(cfg_graph, ast_tree, symtab, if_node,loop_level)?),
        (RULE_switchSelection, switch_node) => Ok(process_switch(cfg_graph, ast_tree, symtab, switch_node,loop_level)?),
        (_, _) => panic!("不属于if和switch的循环"),
    }
}

pub fn process_if(cfg_graph:&mut CfgGraph, ast_tree:&AstTree, symtab:&mut SymTab, if_node:u32,loop_level:usize) -> Result<Option<(u32, u32)>> {
    //将expression做成branch
    let expression_node = find!(rule RULE_expression at if_node in ast_tree).unwrap();

    //处理statement
    let statement_nodes:Vec<u32> = find_nodes!(rule RULE_statement at if_node in ast_tree);
    if statement_nodes.len() == 1 {
        let p0 = process_stmt(cfg_graph, ast_tree, symtab, statement_nodes[0],loop_level)?;

        let branch_struct = CfgNode::new_branch(expression_node,loop_level);
        let cfg_branch_node = add_node!(branch_struct to cfg_graph);

        let cfg_gather_struct = CfgNode::new_gather(loop_level);
        let cfg_gather_node = add_node!(cfg_gather_struct to cfg_graph);

        match p0 {
            Some((st_head_node, st_tail_node)) => {
                add_edge!({CfgEdge::new_if_true()} from cfg_branch_node to st_head_node in cfg_graph);
                add_edge!({CfgEdge::new_direct()} from st_tail_node to cfg_gather_node in cfg_graph);
            }
            None => {
                add_edge!({CfgEdge::new_if_true()} from cfg_branch_node to cfg_gather_node in cfg_graph);
            }
        }
        add_edge!({CfgEdge::new_if_false()} from cfg_branch_node to cfg_gather_node in cfg_graph);
        Ok(Some((cfg_branch_node, cfg_gather_node)))
    } else if statement_nodes.len() == 2 {
        let p0 = process_stmt(cfg_graph, ast_tree, symtab, statement_nodes[0],loop_level)?;
        let p1 = process_stmt(cfg_graph, ast_tree, symtab, statement_nodes[1],loop_level)?;

        let branch_struct = CfgNode::new_branch(expression_node,loop_level);
        let cfg_branch_node = add_node!(branch_struct to cfg_graph);

        let cfg_gather_struct = CfgNode::new_gather(loop_level);
        let cfg_gather_node = add_node!(cfg_gather_struct to cfg_graph);

        match p0 {
            Some((st_head_node, st_tail_node)) => {
                add_edge!({CfgEdge::new_if_true()} from cfg_branch_node to st_head_node in cfg_graph);
                add_edge!({CfgEdge::new_direct()} from st_tail_node to cfg_gather_node in cfg_graph);
            }
            None => {
                // 加入一个空的bb
                let cfg_new_bb = add_node!({CfgNode::new_bb(vec![],loop_level)} to cfg_graph);
                add_edge!({CfgEdge::new_if_true()} from cfg_branch_node to cfg_new_bb in cfg_graph);
                add_edge!({CfgEdge::new_if_true()} from cfg_new_bb to cfg_gather_node in cfg_graph);
            }
        }
        match p1 {
            Some((st_head_node, st_tail_node)) => {
                add_edge!({CfgEdge::new_if_false()} from cfg_branch_node to st_head_node in cfg_graph);
                add_edge!({CfgEdge::new_direct()} from st_tail_node to cfg_gather_node in cfg_graph);
            }
            None => {
                let cfg_new_bb = add_node!({CfgNode::new_bb(vec![],loop_level)} to cfg_graph);
                add_edge!({CfgEdge::new_if_false()} from cfg_branch_node to cfg_new_bb in cfg_graph);
                add_edge!({CfgEdge::new_if_false()} from cfg_new_bb to cfg_gather_node in cfg_graph);
            }
        }
        Ok(Some((cfg_branch_node, cfg_gather_node)))
    } else {
        panic!("不对, selectionStatement 下面不可能有两个以上的Statement");
    }
}
pub fn process_switch(cfg_graph:&mut CfgGraph, ast_tree:&AstTree, symtab:&mut SymTab, switch_node:u32,loop_level:usize) -> Result<Option<(u32, u32)>> {
    //将expression做成branch
    let expression_node = find!(rule RULE_expression at switch_node in ast_tree).unwrap();

    let branch_struct = CfgNode::new_switch(expression_node,loop_level);
    let cfg_branch_node = add_node!(branch_struct to cfg_graph);

    let cfg_gather_struct = CfgNode::new_gather(loop_level);
    let cfg_gather_node = add_node!(cfg_gather_struct to cfg_graph);

    //处理statement
    let compound_stmt_node = find!(rule RULE_statement finally RULE_compoundStatement at switch_node in ast_tree).unwrap();
    if let Some((st_head_node, st_tail_node)) = process_compound(cfg_graph, ast_tree, symtab, compound_stmt_node,loop_level)? {
        add_edge!({CfgEdge::new_direct()} from cfg_branch_node to st_head_node in cfg_graph);
        add_edge!({CfgEdge::new_direct()} from st_tail_node to cfg_gather_node in cfg_graph);
    }
    Ok(Some((cfg_branch_node, cfg_gather_node)))
}

/// 首先确保这两个节点直接相连,如果合并成功，返回合并后的 node ，如果合并失败，返回None
/// 需要注意的是 一个gather 和一个空的 BasicBlock 不可以合并，其次我们需要保留边的属性
/// 两个BaiscBlock 如果 前者的最后一个 ast_stmt 是 jumpStatement 那么也不可以合并
pub fn try_unite(opt_node1:Option<u32>, opt_node2:Option<u32>, cfg_graph:&mut CfgGraph, ast_tree:&AstTree) -> Result<Option<u32>> {
    // return Ok(None);
    match (opt_node1, opt_node2) {
        (Some(node1), Some(node2)) => {
            let (node_struct1, node_struct2) = cfg_graph.index_twice_mut(NodeIndex::from(node1), NodeIndex::from(node2));
            match ( &mut node_struct1.cfg_node_type,&mut node_struct2.cfg_node_type) {
                (CfgNodeType::BasicBlock { ast_nodes: ast_nodes1 }, CfgNodeType::BasicBlock { ast_nodes: ast_nodes2 }) => {
                    if let Some(&ast_node) = ast_nodes1.last(){
                        if rule_id!(at ast_node in ast_tree) == RULE_jumpStatement{
                            // jump stmt can't merge 
                            return Ok(None)
                        }
                    }
                    ast_nodes1.extend_from_slice(&ast_nodes2);
                    let edges:Vec<_> = cfg_graph.edges(NodeIndex::from(node2)).map(|x| (x.id().index() as u32, x.weight().clone())).collect();
                    for (edge, weight) in edges {
                        let dst = cfg_graph.edge_endpoints(EdgeIndex::from(edge)).unwrap().1;
                        add_edge!({weight} from node1 to dst in cfg_graph);
                    }
                    cfg_graph.remove_node(NodeIndex::from(node2));
                    Ok(Some(node1))
                }
                // ( _, CfgNodeType::BasicBlock { ast_nodes: ast_nodes2, },) => {
                //     // 检测如果 node2 里面 bb 内容为空 就删掉这个bb 并且转移它的边
                //     let edges: Vec<u32> = cfg_graph
                //         .edges(NodeIndex::from(node2))
                //         .map(|x| x.id().index() as u32)
                //         .collect();
                //     if edges.len() == 0 {
                //         for edge in edges {
                //             let dst = cfg_graph
                //                 .edge_endpoints(EdgeIndex::from(edge))
                //                 .unwrap()
                //                 .1
                //                 .index() as u32;
                //             add_edge!({CfgEdge::new_direct()} from node1 to dst in cfg_graph);
                //         }
                //         cfg_graph.remove_node(NodeIndex::from(node2));
                //     }
                //     Ok(Some(node1))
                // }
                _ => {
                    Ok(None)
                    // 如果其中都不是BasicBlock ，那么合并不了一点
                }
            }
        }
        (Some(_node1), None) => Ok(opt_node1),
        (None, Some(_node2)) => Ok(opt_node2),
        (None, None) => Ok(None),
    }
}

pub fn process_compound(cfg_graph:&mut CfgGraph, ast_tree:&AstTree, symtab:&mut SymTab, current_compound_node:u32,loop_level:usize) -> Result<Option<(u32, u32)>> {
    // 一开始 没有head也没有 tail
    let mut opt_current_cfg_head_and_tail = None;
    //查找compoundstatement,下找blocklist以及blockitem
    let blockitemlist_node = find!(rule RULE_blockItemList at current_compound_node in ast_tree).unwrap_or_else(|| panic!("unwrap failed at ast_node:{}", current_compound_node));
    let blockitem_nodes:Vec<u32> = find_nodes!(rule RULE_blockItem at blockitemlist_node in ast_tree);

    // 这里 rev 是因为 adj 只能返回  rev 的部分
    for blockitem_node in blockitem_nodes {
        let declare_or_statement_node = node!(at blockitem_node in ast_tree).child_vec[0];
        match (rule_id!(at declare_or_statement_node in ast_tree), declare_or_statement_node) {
            (RULE_statement, stmt_node) => {
                opt_current_cfg_head_and_tail = {
                    let (opt_stmt_head, opt_stmt_tail) = process_stmt(cfg_graph, ast_tree, symtab, stmt_node,loop_level)?.unzip();
                    let (opt_current_cfg_head, opt_current_cfg_tail) = opt_current_cfg_head_and_tail.unzip();
                    let opt_unite_bb_node = try_unite(opt_current_cfg_tail, opt_stmt_head, cfg_graph, ast_tree)?;
                    match (opt_unite_bb_node, opt_stmt_head, opt_current_cfg_tail) {
                        (None, Some(stmt_head), Some(current_cfg_tail)) => {
                            add_edge!({CfgEdge::new_direct()} from current_cfg_tail to stmt_head in cfg_graph);
                        }
                        _ => {}
                    };
                    // 这里考虑的是如果 unite 成功，那么如何转移 cfg_current_head_and_tail 并且要考虑到如果 process_stmt
                    match (opt_stmt_head, opt_stmt_tail) {
                        (Some(x), Some(y)) if x == y => Option::zip(opt_current_cfg_head.or(opt_unite_bb_node).or(opt_stmt_head), opt_unite_bb_node.or(opt_stmt_tail)),
                        _ => Option::zip(opt_current_cfg_head.or(opt_unite_bb_node).or(opt_stmt_head), opt_stmt_tail.or(opt_current_cfg_tail)),
                    }
                }
            }
            (RULE_declaration, declare_node) => {
                opt_current_cfg_head_and_tail = {
                    let (opt_declare_head, opt_declare_tail) = process_declartion(cfg_graph, ast_tree, symtab, declare_node,loop_level)?.unzip();
                    let (opt_current_cfg_head, opt_current_cfg_tail) = opt_current_cfg_head_and_tail.unzip();
                    let opt_unite_bb_node = try_unite(opt_current_cfg_tail, opt_declare_head, cfg_graph, ast_tree)?;
                    match (opt_unite_bb_node, opt_declare_head, opt_current_cfg_tail) {
                        (None, Some(declare_head), Some(current_cfg_tail)) => {
                            add_edge!({CfgEdge::new_direct()} from current_cfg_tail to declare_head in cfg_graph);
                        }
                        _ => {}
                    }
                    Option::zip(opt_current_cfg_head.or(opt_unite_bb_node).or(opt_declare_head), opt_unite_bb_node.or(opt_declare_tail).or(opt_current_cfg_tail))
                }
            }
            _ => {
                panic!("ast_tree 树错了  node:{}", declare_or_statement_node)
            }
        }
    }
    Ok(opt_current_cfg_head_and_tail)
}

pub static AST_ROOT:u32 = 0;
/// 这个函数依赖 ast
pub fn parse_ast_to_cfg(ast_tree:&AstTree, cfg_graph:&mut CfgGraph, symtab:&mut SymTab) -> Result<()> {
    let compilation_unit_node = node!(at AST_ROOT in ast_tree).child_vec[0];
    let static_nodes:&Vec<u32> = &node!(at compilation_unit_node in ast_tree).child_vec;
    let mut static_decl_nodes = vec![];
    let mut funcdef_nodes = vec![];
    for &static_node in static_nodes{
        let gloabl_var_ast_node = node!(at static_node in ast_tree).child_vec[0];
        match (rule_id!(at gloabl_var_ast_node in ast_tree),gloabl_var_ast_node){
            (RULE_declaration,static_decl_node) => {
                static_decl_nodes.push(static_decl_node);
            },
            (RULE_functionDefinition,funcdef_node) => {
                funcdef_nodes.push(funcdef_node)
            },
            (_,_) => {},
        }
    }
    let loop_level:usize = 0;
    let cfg_func_parent = CfgNode::new_root(static_decl_nodes,0);
    // 为每一个function 创建一个共享的根节点
    let cfg_func_parent_node = cfg_graph.add_node(cfg_func_parent);
    for func_def_node in funcdef_nodes {
        let entry_struct = CfgNode::new_entry(func_def_node, 0,loop_level);
        let cfg_entry_node = add_node!(entry_struct to cfg_graph);
        add_edge!( {CfgEdge::new_direct()} from  cfg_func_parent_node to cfg_entry_node in cfg_graph);
        let exit_struct = CfgNode::new_exit(func_def_node,loop_level);
        let cfg_exit_node = add_node!(exit_struct to cfg_graph);
        // println!("funcdef {}",funcdef_node);
        let current_compound_node = find!(rule RULE_compoundStatement at func_def_node in ast_tree ).unwrap();

        match process_compound(cfg_graph, ast_tree, symtab, current_compound_node,loop_level)? {
            Some((cfg_head_node, cfg_tail_node)) => {
                add_edge!( {CfgEdge::new_direct()} from  cfg_entry_node to cfg_head_node in cfg_graph);
                add_edge!( {CfgEdge::new_direct()} from  cfg_tail_node to cfg_exit_node in cfg_graph);
            }
            None => {
                add_edge!( {CfgEdge::new_direct()} from  cfg_entry_node to cfg_exit_node in cfg_graph);
            }
        }
    }
    Ok(())
}

// /// 这个函数用于将cfg生成为 bbcfg 意思是这个cfg 里面只有bb
// pub fn parse_cfg2cfgbb(ast_tree:&AstTree,cfg_graph:&mut CfgGraph,symtab:&mut SymTab,scope_tree:&ScopeTree){
//     // 建立一个新的cfg
//     let mut bb_cfg = CfgGraph::new();
//     let root_node = add_node!({CfgNode::Root {  } } to bb_cfg);
//     process_bbcfg_root(ast_tree, cfg_graph, &mut bb_cfg, root_node, root_node, symtab, scope_tree)
// }

// pub fn process_bbcfg_root(ast_tree:&AstTree,cfg_graph:&mut CfgGraph,bb_cfg:&mut CfgGraph,current_root_node_in_cfg:u32,current_root_node_in_bbcfg:u32,symtab:&mut SymTab,scope_tree:&ScopeTree){
//     for node in direct_nodes!(at current_root_node_in_cfg in cfg_graph).iter(){

//     }

// }
