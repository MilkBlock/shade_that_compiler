use crate::toolkit::{context::NhwcCtx, dot::Config, etc::generate_png_by_graph_multi_tasks, gen_loop_tree::parse_cfg2loop_tree, pass_manager::Pass};
use anyhow::*;
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
pub struct Cfg2LptPass {
    is_gen_png:bool,
}
impl Cfg2LptPass {
    pub fn new(is_gen_png:bool) -> Self { Cfg2LptPass{is_gen_png} }
}

impl Pass for Cfg2LptPass {
    // 运行这个pass
    fn run(&mut self, ctx:&mut NhwcCtx) -> Result<()> { 
        let loop_tree = &mut ctx.loop_tree;
        let cfg_graph = &mut ctx.cfg_graph;
        parse_cfg2loop_tree(loop_tree, cfg_graph)?;
        Ok(())
    }
    // 返回pass的描述，具体作用
    fn get_desc(&self) -> String { return "pass Cfg2LptPass description".to_string(); }

    // 返回pass的名称
    fn get_pass_name(&self) -> String { return "Cfg2LptPass".to_string(); }
    
    fn when_finish_or_panic(&mut self, ctx:&mut crate::toolkit::context::NhwcCtx) {
        if self.is_gen_png {
            generate_png_by_graph_multi_tasks(&ctx.loop_tree.clone(), "loop_tree".to_string(), &[Config::Record, Config::Title("loop_tree".to_string()),Config::NodeIndexLabel],&mut ctx.io_task_list).unwrap();
        }
    }
}
