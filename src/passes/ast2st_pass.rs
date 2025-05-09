use crate::toolkit::{context::NhwcCtx, dot::Config, etc::generate_png_by_graph_multi_tasks, gen_scope::parse_ast_to_scope, pass_manager::Pass};
use anyhow::Result;
#[derive(Debug)]
pub struct Ast2StPass {
    is_gen_png:bool,
}
impl Ast2StPass {
    pub fn new(is_gen_png:bool) -> Self { Ast2StPass { is_gen_png} }
}

impl Pass for Ast2StPass {
    // 运行这个pass
    fn run(&mut self, ctx:&mut NhwcCtx) -> Result<()> {
        parse_ast_to_scope(&ctx.ast_tree,&ctx.cfg_graph,&mut ctx.scope_tree,&mut ctx.ast2scope);
        // 3.1 生成对应的pn
        Ok(())
    }
    // 返回pass的描述，具体作用
    fn get_desc(&self) -> String { return "pass ast2scope_tree pass description".to_string(); }
    // 返回pass的名称
    fn get_pass_name(&self) -> String { return "Ast2ScopeTree".to_string(); }
    
    fn when_finish_or_panic(&mut self, ctx:&mut crate::toolkit::context::NhwcCtx) {
        if self.is_gen_png {
            // let symt = self.op_cfg_graph.unwrap();
            for scope_node in ctx.scope_tree.node_weights_mut() {
                scope_node.load_ast_node_text(&ctx.ast_tree);
            }
            generate_png_by_graph_multi_tasks(&ctx.scope_tree.clone(), "scope_tree".to_string(), &[Config::EdgeNoLabel, Config::Record, Config::Title("scope_tree".to_string()),Config::NodeIndexLabel],&mut ctx.io_task_list).unwrap();
        }
    }
}
