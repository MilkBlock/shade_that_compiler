use anyhow::Result;

use crate::toolkit::{context::NhwcCtx, pass_manager::Pass};
#[derive(Debug)]
pub struct Riscv2BinaryPass {}
impl Riscv2BinaryPass {
    pub fn new() -> Self { Riscv2BinaryPass {} }
}

impl Pass for Riscv2BinaryPass {
    // 运行这个pass
    fn run(&mut self, _ctx:&mut NhwcCtx) -> Result<()> {
        todo!();
    }
    // 返回pass的描述，具体作用
    fn get_desc(&self) -> String { return "pass Riscv2Binary description".to_string(); }
    // 返回pass的名称
    fn get_pass_name(&self) -> String { return "Riscv2BinaryPass Pass".to_string(); }
    
    fn when_finish_or_panic(&mut self, ctx:&mut crate::toolkit::context::NhwcCtx) {
        todo!()
    }
}
