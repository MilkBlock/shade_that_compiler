use std::fs;
use std::io::Write;
use std::rc::Rc;


use crate::toolkit::cfg_node::CFG_ROOT;
use crate::toolkit::gen_nhwc_cfg;
use crate::toolkit::nhwc_instr::NhwcInstrType;
use crate::toolkit::scope_node::ST_ROOT;
use crate::toolkit::symtab::{RcSymIdx, WithBorrow};
use crate::{direct_child_node, direct_child_nodes, instr, instr_mut, node_mut};

use crate::{node, toolkit::{cfg_edge::CfgEdgeType, context::NhwcCtx, etc::dfs_with_priority, pass_manager::Pass}};
use anyhow::*;
use log::error;
/// 定义额外的信息，这样我们就可以把 add_field 宏加入到符号表或者任何实现了 Fields trait 的地方
/// 任何一个Pass 都有一个pass_run函数 来进行这个pass 相关的工作，比如说对于 SSAPass 我们要对 一个BasicBlock 中的ExprTree做出转换。
/// 因为实际上 一个 ExprTree 最终会对应一个BasicBlock。
/// 可能会有人问，那我们为什么还要生成 nhwc_ir ？ 因为 nhwc_ir 有如下优点
/// 1. 便于debug，到时候这个pass 写完生成这个 cfg 对应的 llvm_ir 我就能更清楚的知道我们到底做了哪些更改
/// 2. nhwc_ir 是线性的结构，而 汇编语言也是线性的结构 ，这样就可以 从 nhwc_ir 转换成 汇编代码了
///
///
///
/// 总上，pass 的主要操作对象是 每个basic block 的expr_tree以及 cfg node。这点我们大概不会变
/// 这个结构体，用于存储与Pass 相关的数据
///
#[derive(Debug)]
pub struct NhwcDumpPass {
    is_gen_nhwc_ir_file:bool,
    nhwc_ir_vec : Vec<(usize,i32)>
}
impl NhwcDumpPass {
    pub fn new(is_gen_nhwc_ir_file:bool) -> Self { NhwcDumpPass {  is_gen_nhwc_ir_file, nhwc_ir_vec: Vec::new() } }
}

impl Pass for NhwcDumpPass {
    // 运行这个pass
    fn run(&mut self, ctx:&mut NhwcCtx) -> Result<()> { 

        let (args,symtab,instr_slab,cfg_graph,nhwc_ir_list) = (&ctx.args,&mut ctx.symtab,&mut ctx.nhwc_instr_slab,&mut ctx.cfg_graph, &mut ctx.collected_nhwc_ir);

        let dfs_node_vec = dfs_with_priority(cfg_graph,CFG_ROOT,|e| match &e.weight().cfg_edge_type{
            CfgEdgeType::BodyHead {  } => 1,
            CfgEdgeType::IfFalse {  } => 2,
            CfgEdgeType::Direct {  } => 2,
            CfgEdgeType::IfTrue {  } => 1,
            CfgEdgeType::BodyTail {  } => 1,
        });

        let mut anonymous_label_count = 0;
        for &cfg_node in dfs_node_vec.iter(){
            let cfg_node_struct = node_mut!(at cfg_node in cfg_graph);
            if !( cfg_node_struct.cfg_node_type.is_entry() ||
                cfg_node_struct.cfg_node_type.is_root()) && cfg_node_struct.op_label_instr.is_none(){
                    let anonymous_label_symidx = gen_nhwc_cfg::process_label_symbol(cfg_node,ST_ROOT,format!("L{}",anonymous_label_count).leak(),symtab);
                    let anonymous_label= NhwcInstrType::new_label(anonymous_label_symidx).into();
                    node_mut!(at cfg_node in cfg_graph).push_nhwc_instr(anonymous_label, instr_slab);
                    anonymous_label_count +=1;
            }
        }
        for &cfg_node in dfs_node_vec.iter(){
            if node!(at cfg_node in cfg_graph).cfg_node_type.is_basic_block() ||  node!(at cfg_node in cfg_graph).cfg_node_type.is_gather(){
                if let Some(cfg_node_to_jump) =direct_child_node!(at cfg_node in cfg_graph ret_option){
                    if let Some(label_instr_to_jump) =node!(at cfg_node_to_jump in cfg_graph).op_label_instr{
                        match &instr!(at label_instr_to_jump in instr_slab).instr_type{
                            NhwcInstrType::Label { label_symidx } => {
                                if node!(at cfg_node in cfg_graph).op_jump_instr.is_none(){
                                    let jump_instr_struct = NhwcInstrType::new_jump(label_symidx.clone()).into();
                                    node_mut!(at cfg_node in cfg_graph).push_nhwc_instr(jump_instr_struct, instr_slab);
                                }
                            },
                            _=>{
                                return Err(anyhow!("cfg_node 的 label_instr 不可能为 除了label 以外的类型"))}
                        }
                    }
                }
            }
        }
        for &cfg_node in dfs_node_vec.iter(){
            if cfg_node == CFG_ROOT|| node!(at cfg_node in cfg_graph).cfg_node_type.is_entry() || direct_child_nodes!(at cfg_node in cfg_graph).len()==0{continue;}
            // println!("{:?}",node!(at cfg_node in cfg_graph));
            let &jump_instr = node!(at cfg_node in cfg_graph).op_jump_instr.as_ref().unwrap();
            match &instr!(at jump_instr in instr_slab).instr_type{
                NhwcInstrType::Jump { jump_op } => {
                    match jump_op{
                        crate::toolkit::nhwc_instr::JumpOp::Br { cond, t1, t2 } => {
                            // if cond is constant then replace it 
                            if cond.as_ref_borrow().symbol_name == "false"{
                                let jump_instr_struct = NhwcInstrType::new_jump(t2.clone()).into();
                                let jump_instr = instr_slab.insert_instr(jump_instr_struct);
                                node_mut!(at cfg_node in cfg_graph).op_jump_instr = Some(jump_instr);
                            }else if cond.as_ref_borrow().symbol_name == "true"{
                                let jump_instr_struct = NhwcInstrType::new_jump(t1.clone()).into();
                                let jump_instr = instr_slab.insert_instr(jump_instr_struct);
                                node_mut!(at cfg_node in cfg_graph).op_jump_instr = Some(jump_instr);
                            }
                        },
                        _ => {}
                    }
                },
                _ => {}
            }
        }
        for cfg_node in dfs_node_vec{
            for &instr in node!(at cfg_node in cfg_graph).iter_all_instrs(){
                let mut cur_tab ;
                if instr!(at instr in instr_slab).instr_type.is_define_func(){
                    cur_tab = 0;
                }else if instr!(at instr in instr_slab).instr_type.is_label(){
                    cur_tab = 1;
                }
                else{
                    cur_tab = 2;
                }
                if matches!(&node!(at cfg_node in cfg_graph).cfg_node_type,crate::toolkit::cfg_node::CfgNodeType::Root { static_ast_nodes: _ }){
                    cur_tab = 0;
                }
                self.nhwc_ir_vec.push((instr,cur_tab));
                nhwc_ir_list.push(instr)
            }
        }
        
        Ok(()) 
    }
    // 返回pass的描述，具体作用
    fn get_desc(&self) -> String { return "pass nhwc_collect description".to_string(); }

    // 返回pass的名称
    fn get_pass_name(&self) -> String { return "NhwcCollectPass".to_string(); }
    
    fn when_finish_or_panic(&mut self, ctx:&mut crate::toolkit::context::NhwcCtx) {
        if self.is_gen_nhwc_ir_file{
            
            let (args,symtab,instr_slab,cfg_graph,nhwc_ir_list) = (&ctx.args,&mut ctx.symtab,&mut ctx.nhwc_instr_slab,&mut ctx.cfg_graph, &mut ctx.collected_nhwc_ir);

            // let mut f =fs::File::create(args.input.file_stem().unwrap().to_string_lossy().to_string() + ".nhwc")?;
            let mut f =fs::File::create(args.output.file_stem().unwrap().to_string_lossy().to_string() + ".nhwc").unwrap();
            writeln!(f,"# input: {:?}",args.input);
            for &(instr,cur_tab) in self.nhwc_ir_vec.iter(){
                // instr_mut!(at instr in instr_slab)?.text.clear();
                
                writeln!(f,"{}{:?}","    ".repeat(cur_tab as usize),instr!(at instr in instr_slab)).unwrap();
                // if let InstrType::Jump{ jump_op } = &instr!(at instr in instr_slab)?.instr_type {
                //     if jump_op.is_ret(){
                //         cur_tab -= 1;
                //     }
                // }
            }
        }
    }
}

