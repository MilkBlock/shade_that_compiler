use core::fmt::Debug;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;


use crate::reg_field_for_struct;
use crate::toolkit::symtab::WithBorrow;

use super::field::{Fields, Type, Value};
use super::symtab::{NzU32Op, RcSymIdx, SymIdx};
use super::{field::Field};
use anyhow::Context;

#[derive(Clone)]
pub struct Symbol {
    pub fields:Fields,
    pub rc_symidx:RcSymIdx,
}
impl Debug for Symbol {
    fn fmt(&self, f:&mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{{{:?} fields:{:?}}}", self.rc_symidx.as_ref_borrow(), self.fields) }
}
/* 引用计数 */
/* 符号的类型 */
/* 常量符号的值 */
reg_field_for_struct! {Symbol { 
    // USE_COUNTER:UseCounter,
    TYPE:Type, 
    VALUE:Value,
} with_fields fields}

impl Symbol {
    pub fn new_verbose(scope_node:u32, symbol_name:&'static str, temp_idx:NzU32Op,ssa_idx:NzU32Op) -> Symbol { Symbol { fields:HashMap::new(), rc_symidx:SymIdx::new_verbose(scope_node, symbol_name, temp_idx,ssa_idx).as_rc() } }
    pub fn new(scope_node:u32, symbol_name:&'static str) -> Symbol { Symbol { fields:HashMap::new(), rc_symidx:SymIdx::new(scope_node, symbol_name).as_rc() } }
    pub fn new_from_symidx(symidx:&SymIdx)->Self{
        Symbol::new_verbose(symidx.scope_node, symidx.symbol_name,symidx.temp_idx ,symidx.ssa_idx)
    }
}
