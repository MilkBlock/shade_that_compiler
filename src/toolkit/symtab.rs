use super::{context::COMPILATION_UNIT, field::{Type, TypeDiscriminants, Value}, symbol::Symbol};
use crate::{add_node, add_node_with_edge };
use core::fmt::Debug;
use ahash::{AHashMap};
use anyhow::{anyhow,Result};
use delegate::delegate;
use itertools::Itertools;
use petgraph::stable_graph::StableDiGraph;
use std::{borrow::Cow, cell::{Ref, RefCell, RefMut}, collections::hash_map::{Iter, IterMut}, fmt::{Display, Formatter}, hash::Hash, num::{NonZero, NonZeroU32}, rc::Rc};

pub type SymTabGraph = StableDiGraph<SymTab, SymTabEdge, u32>;

#[derive(Clone,PartialEq,Eq, PartialOrd, Ord)]
pub struct RcSymIdx { 
    rc_symidx:Rc<RefCell<SymIdx>>
}
impl Hash for RcSymIdx{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_ref_borrow().symbol_name.hash(state);
        self.as_ref_borrow().scope_node.hash(state);
    }
}
impl RcSymIdx{
    pub fn new(symidx:SymIdx) -> Self{
        Self { rc_symidx: Rc::new(RefCell::new(symidx)) }
    }
}

impl Debug for RcSymIdx{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{:?}",self.as_ref_borrow())
    }
}

pub trait WithBorrow {
    // fn with_borrow<'a>(&'a self) -> (RcSymIdx,Ref<'a,SymIdx>);
    // fn with_borrow_mut<'a>(&'a self) -> (&'a RcSymIdx,RefMut<'a,SymIdx>);
    fn as_ref_borrow<'a>(&'a self) -> Ref<'a,SymIdx>;
    fn as_ref_borrow_mut<'a>(&'a self) -> RefMut<'a,SymIdx>;
}
impl WithBorrow for RcSymIdx{
    fn as_ref_borrow<'a >(&'a self) -> Ref<'a,SymIdx> {
        self.rc_symidx.as_ref().borrow()
    }
    
    fn as_ref_borrow_mut<'a >(&'a self) -> RefMut<'a,SymIdx> {
        self.rc_symidx.as_ref().borrow_mut()
    }
}

#[derive(Clone)]
pub struct SymTab {
    // map:BTreeMap<SymIdx, Symbol>,
    pub map:AHashMap<SymIdx, Symbol>,
    text:String,
}
#[derive(Clone)]
pub struct SymTabEdge {
    text:String,
}
impl Debug for SymTabEdge {
    fn fmt(&self, f:&mut Formatter<'_>) -> std::fmt::Result { write!(f, "{}", self.text) }
}
impl SymTabEdge {
    pub fn new(text:String) -> Self { SymTabEdge { text } }
}

pub type NzU32Op = Option<NonZero<u32>>;
/// 由于我们对 Symbol 的索引必须同时考虑 symbol 所在的scope 的层级以及 symbol的名字，不如直接改成结构体SymbolIndex
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SymIdx {
    pub scope_node:u32,
    pub symbol_name:&'static str,
    pub ssa_idx:NzU32Op,
    pub temp_idx:NzU32Op,
}
impl From<usize> for SymIdx{
    fn from(value: usize) -> Self {
        SymIdx { scope_node: 0, symbol_name: value.to_string().leak(), ssa_idx: None, temp_idx: None }
    }
}
impl From<isize> for SymIdx{
    fn from(value: isize) -> Self {
        SymIdx { scope_node: 0, symbol_name: value.to_string().leak(), ssa_idx: None, temp_idx: None }
    }
}
impl SymIdx {
    pub fn as_rc(self)-> RcSymIdx{
        RcSymIdx::new(self)
    }
    pub fn new(scope_node:u32, symbol_name:&'static str) -> Self { SymIdx { scope_node, symbol_name: symbol_name, ssa_idx:None, temp_idx: None } }
    pub fn new_verbose(scope_node:u32, symbol_name:&'static str, temp_idx: NzU32Op ,ssa_idx:NzU32Op) -> Self { SymIdx { scope_node, symbol_name, temp_idx,ssa_idx } }
    pub fn from_str(s:&'static str) -> Self{
        Self::new(0, s)
    }
    pub fn as_global_ptr(mut self) -> Result<Self>{
        if self.symbol_name.starts_with('*'){
            return Err(anyhow!("can't transform symbol_name to be global ptr twice"))
        }
        let mut temp_string = self.symbol_name.to_string();
        temp_string.insert(0, '*');
        self.symbol_name = temp_string.leak();
        Ok(self)
    }
    /// check if this symidx is ptr to global variables
    pub fn is_global_ptr(&self) -> bool{
        self.symbol_name.starts_with('*')
    }
    pub fn is_literal(&self) -> bool{
        !TypeDiscriminants::new_from_const_str(&self.symbol_name).is_unknown()
    }
    pub fn as_deglobal_ptr(mut self) -> Result<Self>{
        if !self.is_global_ptr(){
            let mut temp_string = self.symbol_name.to_string();
            temp_string.remove(0);
            self.symbol_name = temp_string.leak();
        }else{
            return Err(anyhow!("can't transform deglobal a symbol_name that is not global ptr twice"))
        }
        Ok(self)
    }
    pub fn to_deglobal_ptr(&self) -> Self{
        let mut cloned = self.clone();
        if self.is_global_ptr(){
            let mut temp_string = self.symbol_name.to_string();
            temp_string.remove(0);
            cloned.symbol_name = temp_string.leak();
        }else{
            panic!("can't deglobal a symbol_name or deglobal global ptr twice")
        }
        cloned
    }
    pub fn to_globl_ptr(&self) -> Self{
        let mut cloned = self.clone();
        if cloned.symbol_name.starts_with('*'){
            panic!("can't transform symbol_name to be global ptr twice")
        }
        let mut temp_string = cloned.symbol_name.to_string();
        temp_string.insert(0, '*');
        cloned.symbol_name = temp_string.leak();
        cloned
    }
    pub fn to_src_symidx(&self)-> SymIdx{
        let mut cloned = self.clone();
        cloned.ssa_idx = None;
        cloned
    }
    pub fn is_src_symidx(&self) -> bool{
        self.ssa_idx.is_none()
    }
    pub fn as_src_symidx(mut self)-> SymIdx{
        self.ssa_idx = None;
        self
    }
    pub fn to_ssa_symidx(&self,ssa_idx:u32)->SymIdx{
        let mut cloned = self.clone();
        cloned.ssa_idx = Some(NonZeroU32::new(ssa_idx).unwrap());
        cloned
    }
    pub fn as_ssa_symidx(mut self,ssa_idx:u32)-> SymIdx{
        self.ssa_idx = Some(NonZeroU32::new(ssa_idx).unwrap());
        self
    }
    pub fn into_symbol(self)->Symbol{
        Symbol::new_verbose(self.scope_node, self.symbol_name, self.temp_idx,self.ssa_idx)
    }
    pub fn get_next_ssa_symidx(&self)->SymIdx{
        if self.ssa_idx == None{
            self.to_ssa_symidx(1)
        }else{
            self.to_ssa_symidx(self.ssa_idx.unwrap().get()+1)
        }
    }
    pub fn try_log_two_as_i32(&self) -> Result<isize>{
        if self.is_literal(){
            match Value::from_symidx(&self){
                Value::I32(Some(num)) => {
                    if num > 0 {
                        let num  = num as usize;
                        let mut i = 0;
                        while num > 2_usize.pow(i){
                            i=i+1;
                        }
                        if 2_usize.pow(i) == num {
                            Ok(i as isize)
                        }else {
                            Err(anyhow!("can't log {:?} of 2",self))
                        }
                    }else {
                        Err(anyhow!("negative num"))
                    }
                },
                _ => {Err(anyhow!("you can't get the min 2 power of {:?}",self))}
            }
        }else {
            Err(anyhow!("can't turn {:?} into log of two",self))
        }
    }
    pub fn get_ty<'a>(&self,symtab:&'a SymTab) -> Cow<'a,Type>{
        if self.is_literal(){
            Cow::Owned(Value::from_symidx(&self).to_type())
        }else {
            Cow::Borrowed(symtab.get(&self.to_src_symidx()).get_type())
        }
    }
    pub fn is_temp(&self) -> bool{
        self.temp_idx.is_some()
    }
}
// macro_rules! make_get_field_func {
//     ($($functionname:ident $field_name:ident:$field_type:ident),*) => {

//     };
// }

impl SymTab {
    // 创建一个新的符号表
    // pub fn new() -> SymTab { SymTab { map:BTreeMap::new(), text: String::new() } }
    pub fn new() -> SymTab { SymTab { map:AHashMap::new(), text: String::new() } }
    
    // 添加或更新符号，如果是更新，那么返回旧的符号
    pub fn add_symbol(&mut self, sym:Symbol) -> RcSymIdx {
        let rc_symidx = sym.rc_symidx.clone();
        if self.map.insert(rc_symidx.as_ref_borrow().clone(), sym).is_none() { rc_symidx }
            else { panic!("symtab插入失败,你这个表中已经存在同名称同scope的符号{:?}了,你必须先remove 掉它", rc_symidx)} // do nothing , 插入成功，里面没有同scope的同名符号
    }

    // 查找符号
    pub fn get(&self, symidx:&SymIdx) -> &Symbol { self.map.get(symidx).expect(concat!("找不到对应的symbol")) }
    pub fn try_get(&self, symidx:&SymIdx) -> Result<&Symbol> { self.map.get(symidx).ok_or(anyhow::anyhow!(concat!("找不到对应的symbol"))) }
    pub fn get_mut(&mut self, symidx:&SymIdx) -> &mut Symbol { self.map.get_mut(symidx).unwrap_or_else(||panic!("找不到{:?}对应的symbol",symidx)) }
    pub fn try_get_mut(&mut self, symidx:&SymIdx) -> Result<&mut Symbol> { self.map.get_mut(symidx).ok_or(anyhow!("找不到{:?}对应的symbol",symidx)) }
    pub fn get_symidx_cor_rc(&self,symidx:&SymIdx) -> RcSymIdx{
        self.get(symidx).rc_symidx.clone()
    }

    delegate!{
        to self.map {
            pub fn iter(&self)->Iter<SymIdx,Symbol>;
            pub fn iter_mut(&mut self)->IterMut<SymIdx,Symbol>;
        }
    }

    // 删除符号
    pub fn remove_symbol(&mut self, symbol_index:&SymIdx) { self.map.remove(symbol_index); }
    pub fn remove_symbol_verbose(&mut self, symbol_name:&'static str, scope_node:u32, temp_idx:NzU32Op) { self.map.remove(&SymIdx { scope_node, symbol_name, ssa_idx:None, temp_idx }); }

    pub fn has_symbol(&self, symidx:&SymIdx)->bool{
        self.map.contains_key(symidx)
    }

    pub fn get_mut_global_info(&mut self) -> &mut Symbol{
        self.get_mut(&SymIdx { scope_node: 0, symbol_name: COMPILATION_UNIT, ssa_idx: None, temp_idx: None })
    }
    pub fn get_global_info(&self) -> &Symbol{
        self.get(&SymIdx { scope_node: 0, symbol_name: COMPILATION_UNIT, ssa_idx: None, temp_idx: None })
    }

    pub fn debug_symtab_graph(&mut self,desc:String, symtab_graph:&mut SymTabGraph,symidx_vec:Vec<&SymIdx>){
        let mut idx = symtab_graph.node_count() as u32;
        if idx==0{
            add_node!({let mut s = self.clone(); s.load_symtab_text(symidx_vec);s } to symtab_graph);
        }else {//如果已经有节点了,在最后一个节点上加点加边
            idx-=1;
            add_node_with_edge!({let mut s = self.clone(); s.load_symtab_text(symidx_vec);s } with_edge {SymTabEdge::new(desc)} from idx in symtab_graph);
        }

    }

    pub fn load_symtab_text(&mut self,symidx_vec:Vec<&SymIdx>){
        if symidx_vec.len()!=0{
            self.text+= "#sym_name@fields$";
            for symidx in &symidx_vec{
                let sym = self.get(symidx);
                self.text += format!("@ # {:?} @ {:#?} $", symidx, sym.fields).as_str();
            }
        }else{
            let mut s = "#sym_name@fields$".to_string();
            for (symidx,sym) in self.iter().sorted_by_key(|( k,v )| k.symbol_name){
                s += format!("@ # {:?} @ {:#?} $", symidx, sym.fields).as_str();
            }
            self.text += s.as_str();
        }
    }

    
}
impl Default for SymTab {
    fn default() -> Self { Self { map:Default::default(), text: String::new() } }
}
impl Debug for SymIdx {
    fn fmt(&self, f:&mut Formatter<'_>) -> std::fmt::Result {
        match self.ssa_idx {
            Some(index_ssa) => write!(f, "{}_{}_{}", self.symbol_name, self.scope_node, index_ssa),
            // Some(index_ssa) => write!(f, "{}*{}*{}", self.symbol_name,  ,index_ssa),
            // None => write!(f, "{} _s{}", self.symbol_name, self.scope_node),
            None => write!(f, "{}_{}", self.symbol_name, self.scope_node),
        }
    }
}
impl Display for SymIdx{
    fn fmt(&self, f:&mut Formatter<'_>) -> std::fmt::Result {
        match self.ssa_idx {
            Some(index_ssa) => write!(f, "{} _s{} _i{}", self.symbol_name, self.scope_node, index_ssa),
            // Some(index_ssa) => write!(f, "{}_i{}", self.symbol_name, index_ssa),
            // None => write!(f, "{}_s{}", self.symbol_name, self.scope_node),
            None => write!(f, "{}", self.symbol_name),
        }
    }
}
impl Debug for SymTab {
    fn fmt(&self, f:&mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{}",self.text)
    }
}
