use crate::toolkit::{dot::Config};
use anyhow::Result;

use crate::toolkit::{context::NhwcCtx, etc::generate_png_by_graph_multi_tasks, gen_nhwc_cfg::parse_cfg_into_nhwc_cfg, pass_manager::Pass};
#[derive(Debug)]
pub struct Cfg2NcfgPass {
    is_gen_ncfg_png:bool,
    is_gen_symtab_png:bool,
}
impl Cfg2NcfgPass {
    pub fn new(is_gen_ncfg_png:bool, is_gen_symtab_png:bool) -> Self { Cfg2NcfgPass { is_gen_ncfg_png, is_gen_symtab_png } }
}

impl Pass for Cfg2NcfgPass {
    // 运行这个pass
    fn run(&mut self, ctx:&mut NhwcCtx) -> Result<()> {
        // 不要在函数中对context进行解包,解包应在pass阶段完成
        let (cfg_graph, scope_tree, ast_tree, symtab, et_tree, ast2scope, symtab_graph) =
            (&mut ctx.cfg_graph, &mut ctx.scope_tree, &mut ctx.ast_tree, &mut ctx.symtab, &mut ctx.et_tree, &mut ctx.ast2scope, &mut ctx.symtab_graph);

        // let rst = parse_cfg_into_nhwc_cfg(cfg_graph, scope_tree, ast_tree, symtab, et_tree, ast2scope, 0, &mut ctx.instr_slab, &mut Some(symtab_graph));
        let rst = parse_cfg_into_nhwc_cfg(cfg_graph, scope_tree, ast_tree, symtab, et_tree, ast2scope, &mut ctx.nhwc_instr_slab, &mut self.is_gen_symtab_png.then(||symtab_graph));

        //4.1可视化
        // if self.is_gen_symtab_png {
        //     generate_png_by_graph_multi_tasks(&ctx.symtab_graph.clone(), "symtab_graph".to_string(), &[Config::Record, Config::Rounded, Config::SymTab, Config::Title("symtab_graph".to_string()),Config::CfgBlock],&mut ctx.io_task_list)?;
        // }
        // generate_png_by_graph(&et_tree, format!("et_tree"), &[Config::EdgeNoLabel, Config::Record, Config::Title("et_tree".to_string()),Config::NodeIndexLabel])?;
        rst?;
        Ok(())
    }
    // 返回pass的描述，具体作用
    fn get_desc(&self) -> String { return "pass Cfg2NhwcCfgPass description".to_string(); }
    // 返回pass的名称
    fn get_pass_name(&self) -> String { return "Cfg2NhwcCfgPass".to_string(); }
    
    fn when_finish_or_panic(&mut self, ctx:&mut crate::toolkit::context::NhwcCtx) {
        if self.is_gen_ncfg_png {
            for cfg_node in ctx.cfg_graph.node_weights_mut() {
                cfg_node.clear_text();
                cfg_node.load_instrs_text(&ctx.nhwc_instr_slab);
            }
            generate_png_by_graph_multi_tasks(&ctx.cfg_graph.clone(), "nhwc_cfg_graph".to_string(), 
            &[
                Config::Record, 
                Config::Rounded, Config::CfgBlock,Config::Title("nhwc_cfg_graph".to_string()), Config::NodeIndexLabel],&mut ctx.io_task_list).unwrap();
        }
    }
}
