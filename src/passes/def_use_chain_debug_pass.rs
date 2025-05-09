use crate::{ reg_field_for_struct, toolkit::{context::NhwcCtx, dot::Config, etc::{generate_png_by_graph_multi_tasks}, gen_dug::{parse_dug}, nhwc_instr::NhwcInstr, pass_manager::Pass, symtab::{SymTab, SymTabEdge, SymTabGraph}}};
use anyhow::*;
#[derive(Debug)]
pub struct DefUseChainPass {
    is_gen_png:bool
}
impl DefUseChainPass {
    pub fn new(is_gen_png:bool) -> Self { DefUseChainPass { is_gen_png } }
}

impl Pass for DefUseChainPass {
    // 运行这个pass
    fn run(&mut self, ctx:&mut NhwcCtx) -> Result<()> { 
        // 先建立一个图 
        let (instr_slab,cfg_graph,def_use_graph,symtab,dj_graph)= (&mut ctx.nhwc_instr_slab,&mut ctx.cfg_graph,&mut ctx.def_use_graph,&mut ctx.symtab,&ctx.dj_graph);
        
        parse_dug(cfg_graph, instr_slab, symtab, def_use_graph, dj_graph)?;
        

        Ok(()) 
    }
    // 返回pass的描述，具体作用
    fn get_desc(&self) -> String { return "pass def use chain debug description".to_string(); }

    // 返回pass的名称
    fn get_pass_name(&self) -> String { return "DefUseChain Debug Pass".to_string(); }
    
    fn when_finish_or_panic(&mut self, ctx:&mut crate::toolkit::context::NhwcCtx) {
        let (instr_slab,cfg_graph,def_use_graph,symtab,dj_graph)= (&mut ctx.nhwc_instr_slab,&mut ctx.cfg_graph,&mut ctx.def_use_graph,&mut ctx.symtab,&ctx.dj_graph);
        if self.is_gen_png {
            // let symt = self.op_cfg_graph.unwrap();
            for def_use_node in def_use_graph.node_weights_mut(){
                def_use_node.load_instr_text(instr_slab);
            }
            generate_png_by_graph_multi_tasks(&ctx.def_use_graph.clone(), "def_use_graph".to_string(), &[Config::Record, Config::Title("def_use_graph".to_string()),Config::RankDirLR],&mut ctx.io_task_list).unwrap();
        }
    }
}
