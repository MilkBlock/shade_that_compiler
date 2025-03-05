use std::panic::{self, catch_unwind, AssertUnwindSafe};

use crate::Args;

use anyhow::Result;
use colored::Colorize;
use log::{debug, error};

use super::context::NhwcCtx;

pub trait Pass {
    fn run(&mut self, ctx:&mut super::context::NhwcCtx) -> Result<()>;
    fn get_desc(&self) -> String;
    fn get_pass_name(&self) -> String;
    fn when_finish_or_panic(&mut self, ctx:&mut crate::toolkit::context::NhwcCtx);
}
pub struct PassManager {
    /// 其中放置 所有pass 的运行顺序的string
    passes:Vec<Box<dyn Pass>>,
    pub ctx:super::context::NhwcCtx,
}
impl PassManager {
    pub fn new(args:Args) -> Self { PassManager { passes:vec![], ctx:super::context::NhwcCtx::new(args).unwrap() } }
    pub fn add_pass(&mut self, pass:Box<dyn Pass>) { self.passes.push(pass); }
    /// 调用这个函数运行 PassManager 中的所有函数
    pub fn execute_passes(&mut self) -> Result<()>{
        for pass in &mut self.passes {
            let name = pass.get_pass_name();
            let may_err = panic::catch_unwind(AssertUnwindSafe(||{
                pass.run(&mut self.ctx).unwrap();
            }));
            pass.when_finish_or_panic(&mut self.ctx);
            if may_err.is_err(){
                may_err.unwrap();
                return Err(anyhow::anyhow!("Error occurred when running Pass {}",name ));
            }
        }
            //println!("{}", format!("Pass {} run successfully", pass.get_pass_name()).green());
        // if errs.len()>0{
        //     println!("All errors:");
        // }
        // for e in &errs{
        //     error!("{}", format!("{:?}", e).red());
        // }
        // errs.len() > 0
        Ok(())
    }
    pub fn await_all_io_tasks(&mut self){
        for handle in self.ctx.io_task_list.drain(..){
            match handle.join() {
                Ok(_) => {
                }
                Err(e) => {
                    // println!("{}", format!("{:?}", e).red());
                    error!("{}", format!("{:?}", e).red())
                }
            }
        }

    }
}

