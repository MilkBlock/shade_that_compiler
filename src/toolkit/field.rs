use core::panic;
use std::{any::Any, cell::RefCell, collections::{hash_map::Iter, HashMap}, fmt::Debug, ops::{Add, BitAnd, BitOr, Div, Mul, Neg, Not, Rem, Sub}, rc::Rc, vec };

use ahash::AHashMap;
use itertools::Itertools;
use strum_macros::EnumIs;
use strum_macros::EnumDiscriminants;
use anyhow::*;
use regex::{self, Regex};


use super::{ast_node::AstTree, scope_node::ST_ROOT, symbol, symtab::{SymIdx, WithBorrow}};
use super::symtab::RcSymIdx;
use crate::{debug_info_blue, debug_info_green, debug_info_red, node};

pub type Fields = HashMap<*const u8, Box<dyn Field>>;
pub static TARGET_POINTER_MEM_LEN:usize = 8;

/// 你实现的类型必须继承这个 trait
pub trait Field: Any + Debug {
    fn as_any(&self) -> &dyn Any;
    fn as_any_move(self) -> Box<dyn Any>;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn clone_box(&self) -> Box<dyn Field>;
    fn as_field_move(self) -> Box<dyn Field>;
}
// if you implement this 会栈溢出，很神奇
impl<T:Clone+Any+Debug> Field for Vec<T>{
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_move(self) -> Box<dyn std::any::Any>{
        Box::new(self)
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn as_field_move(self) -> Box<dyn Field>{
        Box::new(self)
    }
    fn clone_box(&self)->Box<dyn crate::toolkit::field::Field> {
        Box::new(self.clone())
    }
}
impl<T:Clone+Any+Debug> Field for Option<T>{
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_move(self) -> Box<dyn std::any::Any>{
        Box::new(self)
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn clone_box(&self)->Box<dyn crate::toolkit::field::Field> {
        Box::new(self.clone())
    }
    
    fn as_field_move(self) -> Box<dyn Field>{
        Box::new(self)
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, EnumIs)]
pub enum Value {
    I32(Option<i32>),
    F32(Option<f32>),
    I1(Option<bool>),
    Ref{rc_symidx:RcSymIdx, ty:Type},
    Void,
    Fn { arg_syms:Vec<RcSymIdx>, rc_ret_sym:RcSymIdx },
    Ptr64{pointed_ty:Box<Type>,op_pointed_symidx:Option<RcSymIdx>,offset:Box<Value>},
    Array {
        value_map:ArrayEleMap,
        dims:Vec<RcSymIdx>,
        ele_ty:Type,
    },
    Unknown,
    // // 这个类型用来表示不确定的值或其代数表达式
    // // 例如 int a = getint() 由于 a取决于用户输入，那么我们不能直接使用 a的值，只能用一个代数符号表示
    // // Unsure {},
}
#[derive(Clone)]
pub struct ArrayEleMap{
    map:AHashMap<usize,Value>
}
impl ArrayEleMap{
    pub fn new()->Self{
        Self { map: AHashMap::new() }
    }
}
impl ArrayEleMap{
    pub fn get_ele_at(&self,offset:usize)->Result<&Value>{
        match self.map.get(&offset){
            Some(ele) => Ok(ele),
            None => {
                panic!("在map:{:?}中找不到元素{}",self,offset)
            },
        }
    }
    pub fn get_mut_ele_from_usize(&mut self,offset:usize) -> Result<&mut Value>{
        match self.map.get_mut(&offset){
            Some(ele) => Ok(ele),
            None => {
                panic!("在map中找不到元素{}",offset)
            },
        }
    }
    /// insert element by `Value` type offset
    pub fn insert_ele_by_value_type_offset(&mut self,offset:&Value,val:Value) -> Result<()>{
        let &offset = match &offset{
            Value::I32(Some(i)) => i,
            _ => {
                panic!("add_ele 中 offset 类型不应为 {:?}",val)
            }
        };
        self.map.insert(offset as usize, val);
        Ok(())
    }
    /// insert element by `usize` offset
    pub fn insert_ele(&mut self,offset:usize,val:Value) {
        self.map.insert(offset, val);
    }
    pub fn iter(&self) -> Iter<usize,Value>{
        self.map.iter()
    }
}
impl Debug for ArrayEleMap{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut text = String::new();
        for (k,v) in self.map.iter().sorted_by_key(|x| x.0){
            // let s:String  = k.iter().map(|&dim_index| format!("[{}]",dim_index)).collect();
            text += format!("offset {} ={:?}\n",k,v).as_str();
        }
        write!(f,"{}",text)
    }
}
impl PartialEq for ArrayEleMap{
    fn eq(&self, other: &Self) -> bool {
        self.map == other.map
    }
} 
impl PartialOrd for ArrayEleMap{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        panic!("不能比较 array {:?} 和 array {:?} 类型",self, other);
    }
}


#[derive(Clone,EnumIs,PartialOrd,PartialEq,Eq, EnumDiscriminants)]
#[strum_discriminants(derive(EnumIs))]
pub enum Type {
    I32,
    F32,
    I1,
    Void,
    Label,
    Ref,
    Ptr64{
        ty:Box<Type>,
    },
    Array{
        dims:Vec<Option<RcSymIdx>>,
        ele_ty:Box<Type>,
    },
    Fn { arg_syms:Vec<RcSymIdx>, ret_sym:RcSymIdx },
    Unknown,
}
impl Clone for Box<dyn Field> {
    fn clone(&self) -> Box<dyn Field> { self.clone_box() }
}

impl Value {
    pub fn new_ref(rc_symidx:RcSymIdx, ty:Type) -> Self{
        Self::Ref { rc_symidx, ty }
    }
    pub fn logical_or(&self,val:&Value) -> Value{
        match (self,val){
            (Value::I32(Some(v1)), Value::I32(Some(v2))) => Value::new_i1((*v1!=0)||(*v2!=0)),
            (Value::I32(Some(v1)), Value::I1(Some(v2))) => Value::new_i1((*v1!=0)||*v2),
            (Value::I1(Some(v1)), Value::I32(Some(v2))) => Value::new_i1(*v1||(*v2!=0)),
            (Value::I1(Some(v1)), Value::I1(Some(v2))) => Value::new_i1(*v1||*v2),

            (Value::I32(Some(v1)), Value::F32(Some(v2))) => Value::new_i1(*v1!=0 || *v2 != 0.0 ),
            (Value::F32(Some(v2)), Value::I32(Some(v1))) => Value::new_i1(*v1!=0 || *v2 != 0.0 ),

            _ => {
                panic!("can't logical or {self:?} with {val:?}")
            }
        }
    }
    pub fn force_to_ty(&self,target_ty:&Type)-> Value{
        match target_ty{
            Type::I32 => {
                match &self{
                    Value::I32(_) => self.clone(),
                    Value::F32(op_f) => match op_f {
                        Some(f) => {
                            Value::I32(Some(*f as i32))
                        },
                        None => Value::I32(None),
                    },
                    _ => todo!()
                }
            },
            Type::F32 => {
                match &self{
                    Value::F32(_) => self.clone(),
                    Value::I32(op_i) => match op_i {
                        Some(i) => {
                            Value::F32(Some(*i as f32))
                        },
                        None => {Value::F32(None)},
                    },
                    _ => todo!()
                }
            },
            _ => todo!()
        }
    }
    pub fn logical_and(&self,val:&Value) -> Value{
        match (self,val){
            (Value::I32(Some(v1)), Value::I32(Some(v2))) => Value::new_i1((*v1!=0)&&(*v2!=0)),
            (Value::I32(Some(v1)), Value::I1(Some(v2))) => Value::new_i1((*v1!=0)&&*v2),
            (Value::I1(Some(v1)), Value::I32(Some(v2))) => Value::new_i1(*v1&&(*v2!=0)),
            (Value::I32(Some(v1)), Value::F32(Some(v2))) => Value::new_i1(*v1!=0 && *v2 != 0.0),
            (Value::F32(Some(v2)), Value::I32(Some(v1))) => Value::new_i1(*v1!=0 && *v2 != 0.0),
            (Value::I1(Some(v1)), Value::I1(Some(v2))) => Value::new_i1(*v1&&*v2),
            _ => {
                panic!("can't logical and {self:?} with {val:?}")
            }
        }
    }
    pub fn logical_eq(&self, val:&Value) -> Value{
        match (self,val){
            (Value::I32(Some(v1)), Value::I32(Some(v2))) => Value::new_i1(*v1==*v2),
            (Value::I1(Some(v1)), Value::I1(Some(v2))) => Value::new_i1(*v1==*v2),
            (Value::I1(Some(v1)), Value::I32(Some(v2))) => Value::new_i1(*v1 as i32==*v2),
            (Value::I32(Some(v1)), Value::I1(Some(v2))) => Value::new_i1(*v1 ==*v2 as i32),
            (Value::F32(Some(v1)), Value::F32(Some(v2))) => Value::new_i1(*v1==*v2),
            _ => {
                panic!("can't logical eq {self:?} with {val:?}")
            }
        }
    }
    pub fn logical_neq(&self, val:&Value) -> Value{
        match (self,val){
            (Value::I32(Some(v1)), Value::I32(Some(v2))) => Value::new_i1(*v1!=*v2),
            (Value::I1(Some(v1)), Value::I1(Some(v2))) => Value::new_i1(*v1!=*v2),
            (Value::I1(Some(v1)), Value::I32(Some(v2))) => Value::new_i1(*v1 as i32!=*v2),
            (Value::I32(Some(v1)), Value::I1(Some(v2))) => Value::new_i1(*v1 !=*v2 as i32),
            (Value::F32(Some(v1)), Value::F32(Some(v2))) => Value::new_i1(*v1!=*v2),
            _ => {
                panic!("can't logical neq {self:?} with {val:?}")
            }
        }
    }
    pub fn less_than(&self,val:&Value) -> Value{
        match (self,val){
            (Value::I32(Some(v1)), Value::I32(Some(v2))) => Value::new_i1(*v1<*v2),
            (Value::I32(Some(v1)), Value::F32(Some(v2))) => Value::new_i1((*v1 as f32)<*v2),
            (Value::F32(Some(v1)), Value::I32(Some(v2))) => Value::new_i1(*v1<(*v2 as f32)),
            (Value::F32(Some(v1)), Value::F32(Some(v2))) => Value::new_i1(*v1<*v2),
            _ => {
                panic!("can't lessthan {self:?} with {val:?}")
            }
        }
    }
    pub fn greater_than(&self,val:&Value) -> Value{
        match (self,val){
            (Value::I32(Some(v1)), Value::I32(Some(v2))) => Value::new_i1(*v1>*v2),
            (Value::I32(Some(v1)), Value::F32(Some(v2))) => Value::new_i1((*v1 as f32)>*v2),
            (Value::F32(Some(v1)), Value::I32(Some(v2))) => Value::new_i1(*v1>(*v2 as f32)),
            (Value::F32(Some(v1)), Value::F32(Some(v2))) => Value::new_i1(*v1>*v2),
            _ => {
                panic!("can't lessthan {self:?} with {val:?}")
            }
        }
    }
    pub fn less_than_or_equal(&self,val:&Value) -> Value{
        match (self,val){
            (Value::I32(Some(v1)), Value::I32(Some(v2))) => Value::new_i1(*v1<=*v2),
            (Value::I32(Some(v1)), Value::F32(Some(v2))) => Value::new_i1((*v1 as f32)<=*v2),
            (Value::F32(Some(v1)), Value::I32(Some(v2))) => Value::new_i1(*v1<=(*v2 as f32)),
            (Value::F32(Some(v1)), Value::F32(Some(v2))) => Value::new_i1(*v1<=*v2),
            _ => {
                panic!("can't lessthan {self:?} with {val:?}")
            }
        }
    }
    pub fn equal(&self,val:&Value) -> Value{
        match (self,val){
            (Value::I32(Some(v1)), Value::I32(Some(v2))) => Value::new_i1(*v1==*v2),
            (Value::I32(Some(v1)), Value::F32(Some(v2))) => Value::new_i1((*v1 as f32)==*v2),
            (Value::F32(Some(v1)), Value::I32(Some(v2))) => Value::new_i1(*v1==(*v2 as f32)),
            (Value::F32(Some(v1)), Value::F32(Some(v2))) => Value::new_i1(*v1==*v2),

            (Value::I1(Some(v1)), Value::I32(Some(v2))) => Value::new_i1(*v1 as i32 ==*v2),
            (Value::I32(Some(v2)), Value::I1(Some(v1))) => Value::new_i1(*v1 as i32 ==*v2),
            _ => {
                panic!("can't equal {self:?} with {val:?}")
            }
        }
    }
    pub fn greater_than_or_equal(&self,val:&Value) -> Value{
        match (self,val){
            (Value::I32(Some(v1)), Value::I32(Some(v2))) => Value::new_i1(*v1>=*v2),
            (Value::I32(Some(v1)), Value::F32(Some(v2))) => Value::new_i1((*v1 as f32)>=*v2),
            (Value::F32(Some(v1)), Value::I32(Some(v2))) => Value::new_i1(*v1>=(*v2 as f32)),
            (Value::F32(Some(v1)), Value::F32(Some(v2))) => Value::new_i1(*v1>=*v2),
            _ => {
                panic!("can't greaterthan {self:?} with {val:?}")
            }
        }
    }
    pub fn new_i32(value:i32) -> Self { Value::I32(Some(value)) }
    pub fn new_f32(value:f32) -> Self { Value::F32(Some(value)) }
    pub fn new_unsure_from_specific_type(specified_ty:&Type) -> Self {
        match specified_ty{
            Type::I32 => Value::I32(None),
            Type::F32 => Value::F32(None),
            Type::I1 => Value::I1(None),
            Type::Void => Value::Void,
            Type::Label => todo!(),
            Type::Array { dims, ele_ty } => Value::Array { value_map: ArrayEleMap::new(), dims:unwrap_vec(dims), ele_ty: *ele_ty.clone()},
            Type::Fn { arg_syms, ret_sym } => {
                Value::Fn { arg_syms: arg_syms.clone(), rc_ret_sym: ret_sym.clone() }
            },
            Type::Ptr64 { ty: _ } => {
                 Value::Ptr64 { op_pointed_symidx: None, offset: Box::new(Value::new_unsure_from_specific_type(&Type::I32)), pointed_ty: Box::new(specified_ty.clone()) }
            },
            Type::Unknown => {
                Value::Unknown
            },
            Type::Ref => {
                Value::Unknown
            },
        }
    }
    pub fn new_ptr64_from_array_with_offset(rc_array_symidx:RcSymIdx,pointed_ty:Type,offset:Value) -> Self{
        Value::Ptr64 { pointed_ty:Box::new(pointed_ty), op_pointed_symidx: Some(rc_array_symidx), offset:Box::new(offset) }
    }
    pub fn new_ptr64_to_variable(rc_pointed_symidx:RcSymIdx,pointed_ty:Type) -> Self{
        Value::Ptr64 { pointed_ty:Box::new(pointed_ty), op_pointed_symidx: Some(rc_pointed_symidx), offset:Box::new(Value::I32(None)) }
    }
    pub fn is_unsure(&self)->Result<bool>{
        match self{
            Value::I32(op) => Ok(op.is_none()),
            Value::F32(op) => Ok(op.is_none()),
            Value::I1(op) => Ok(op.is_none()),
            _ => panic!("无法确认 value:{:?} 是否 unsure ",self),
        }
    }
    pub fn new_i1(value:bool) -> Self { Value::I1(Some(value)) }
    pub fn new_void() -> Self { Value::Void }
    pub fn new_array(value_map:ArrayEleMap, dims:Vec<RcSymIdx>, ele_ty: Type) -> Self{
        Value::Array { value_map, dims, ele_ty }
    }
    pub fn trans_to_specific_type(&self,ty:&Type) -> Value{
        match (&self,&ty) {
            (Value::I32(Some(v)), Type::I32) => Value::new_i32(*v as i32),
            (Value::I32(Some(v)), Type::F32) => Value::new_f32(*v as f32),
            (Value::I32(Some(v)), Type::I1) => todo!(),
            (Value::I32(Some(v)), Type::Void) => todo!(),
            (Value::I32(Some(v)), Type::Label) => todo!(),
            (Value::F32(Some(v)), Type::I32) => Value::new_i32(*v as i32),
            (Value::F32(Some(v)), Type::F32) => Value::new_f32(*v as f32),
            (Value::F32(Some(v)), Type::I1) => todo!(),
            (Value::F32(Some(v)), Type::Void) => todo!(),
            (Value::F32(Some(v)), Type::Label) => todo!(),
            (Value::I1(Some(v)), Type::I32) => Value::new_i32(Into::into(*v)),
            (Value::I1(Some(v)), Type::F32) => Value::new_f32((*v).into()),
            (Value::I1(Some(v)), Type::I1) => Value::new_i1((*v).into()),
            (Value::Void, t) => panic!("void 类型不能转化为 {:?} 类型",t),
            (Value::Fn { arg_syms: _, rc_ret_sym: _ }, _t) => todo!(),
            _ => panic!("不能将 {:?} 转化为 {:?}",self,ty ),
        }
    }
    // pub fn as_specific_type(self) -> Self {

    // }
    pub fn from_string_with_specific_type(s:&str,ty:&Type)->Value{
        match &ty{
            Type::I32 => Value::new_i32(
                match (s.parse().with_context(||format!("when parsing {} to i32",s)),s.starts_with("0x"), s.starts_with("0")) {
                    (Result::Ok(i), false, false) => {
                        i
                    },
                    (_, true, _) => {
                        i32::from_str_radix(&s.trim_start_matches("0x"), 16).expect("err trans radix 16")
                    },
                    (Result::Ok(i), false, true) => {
                        if i!=0{
                            i32::from_str_radix(&s.trim_start_matches("0"), 8).expect("err trans radix 8")
                        }else {
                            0
                        }
                    },
                    (Err(_), false, false) => {
                        panic!()
                    },
                    (Err(_), false, true) => todo!(),
                    
                }
            ),

            Type::F32 => Value::new_f32(s.parse().with_context(||format!("when parsing {} to f32",s)).unwrap_or_else(|_ |Value::parse_hex_float(s))),
            Type::I1 => Value::new_i1(s.parse().with_context(||format!("when parsing {} to i1",s)).expect("")),
            Type::Void => panic!("不能从string 转化为 Void 类型的value"),
            Type::Label => panic!("不能从string 转化为 Label 类型的value"),
            Type::Fn { arg_syms: _, ret_sym: _ } => panic!("不能从string 转化为 Fn 类型的value"),
            // Type::Unsure {  } => panic!("不能从string 转化为 Unsure 类型的value"),
            Type::Array { dims, ele_ty} => Value::new_array(ArrayEleMap::new(), unwrap_vec(dims), *ele_ty.clone()),
            Type::Ptr64 { ty: _ } => panic!("不能从string 转化为 Ptr64 类型的value"),
            Type::Ref => panic!("不能从string 转化为 Ref 类型的value"),
            Type::Unknown => panic!("不能从string 转化为 Unknown 类型的value"),
        }
    }
    fn parse_hex_float(s: &str) -> f32 {
        let mut parts: Vec<&str> = s.split('p').collect();
        // debug_info_red!("split into {parts:?}");
        if parts.len() !=2{
            parts= s.split('P').collect();
            // debug_info_red!("split into {parts:?}");
        }
        if parts.len() != 2 {
            panic!("float part have multiple power {}", s)
        }

        let float_parts: Vec<&str> = parts[0].trim_start_matches("0x").split('.').collect();
        if float_parts.len() > 2 {
            panic!("float part have multiple dot")// 处理非法格式，如多个小数
        }

        let int_part = if float_parts[0] != ""{ i32::from_str_radix(float_parts[0], 16).unwrap() as f32}else{0.};

        let frac_part = if float_parts.len() == 2 {
            let frac_part_str = float_parts[1];
            let mut frac_part = 0f32;
            for (i, ch) in frac_part_str.chars().enumerate() {
                let digit = ch.to_digit(16).unwrap() as f32;
                frac_part += digit * 16f32.powi(-(i as i32 + 1));
            }
            frac_part
        } else {
            0f32
        };

        let base = int_part + frac_part;
        let exp: i32 = parts[1].parse().unwrap();
        base * 2f32.powi(exp)
    }
    pub fn as_i32_to_min_2_power(self) -> Result<Self>{
        match self{
            Value::I32(Some(num)) => {
                if num < 1 {
                    panic!("you can't get the min 2 power of negative num {}",num)
                }
                let mut i = 0;
                while num > 2_i32.pow(i){
                    i=i+1;
                }
                Ok(Value::new_i32(2_i32.pow(i)))
            },
            _ => {panic!("you can't get the min 2 power of {:?}",self)}
        }
    }
    pub fn as_i32(&self) -> i32{
        match self{
            Value::I32(Some(num)) => {
                *num
            },
            _ => {panic!("{:?} can't cast to i32",self)}
        }
    }
    pub fn as_usize(&self) -> usize{
        match &self{
            Value::I32(Some(num)) => {
                *num as usize
            },
            _ => {panic!("{:?} can't cast to u32",self)}
        }
    }

    pub fn log_if_is_pow_of_two(self) -> Result<isize>{
        match self{
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
                        panic!("can't log {:?} of 2",self)
                    }
                }else {
                    panic!("negative num")
                }
            },
            _ => {panic!("you can't get the min 2 power of {:?}",self)}
        }
    }

    pub fn to_type(&self)->Type{
        match self{
            Value::I32(_) => Type::I32,
            Value::F32(_) => Type::F32,
            Value::I1(_) => Type::I1,
            Value::Void => Type::Void,
            Value::Fn { arg_syms, rc_ret_sym: ret_sym } => Type::Fn { arg_syms: arg_syms.clone(), ret_sym: ret_sym.clone() },
            Value::Array { value_map: _, dims , ele_ty: ele_type } => Type::Array { dims: dims.clone().into_iter().map(|x| Some(x)).collect_vec(), ele_ty:Box::new(ele_type.clone())  },
            Value::Ptr64 { pointed_ty: ty, op_pointed_symidx: _, offset: _  } => Type::Ptr64 { ty: Box::new(*ty.clone()) },
            Value::Ref{ rc_symidx: symidx, ty } => Type::Ref,
            Value::Unknown => Type::Unknown,
            // Value::Unsure {  } => Type::Unsure {  },
        }
    }
    pub fn adapt(&self, value2:&Value) -> Type {
        Type::arith_adapt(&self.to_type() ,&value2.to_type())
    }

    pub fn from_symidx(symidx:&SymIdx) -> Value{
        Self::from_string_with_specific_type(&symidx.symbol_name, &TypeDiscriminants::new_from_const_str(&symidx.symbol_name).into())
    }
    pub fn try_from_symidx(symidx:&SymIdx) -> Value{
        Self::from_string_with_specific_type(&symidx.symbol_name, &TypeDiscriminants::new_from_const_str(&symidx.symbol_name).into())
    }
    pub fn to_symidx(&self)->SymIdx{
        match self{
            Value::I32(op_i32) => {
                if let Some(i32_value) = op_i32{
                    SymIdx::new(ST_ROOT,i32_value.to_string().leak())
                }else{
                    panic!("i32 {:?} unsure 无法转化为 symidx",self)
                }
            },
            Value::F32(op_f32) => {
                if let Some(f32_value) = op_f32{
                    let mut f32_str = f32_value.to_string();
                    if !f32_str.contains("."){
                        f32_str.push('.');
                    }
                    SymIdx::new(ST_ROOT,f32_str.to_string().leak())
                }else{
                    panic!("f32 {:?} unsure 无法转化为 symidx",self)
                }

            },
            Value::Ref { rc_symidx, ty } => {
                rc_symidx.as_ref_borrow().clone()
            },
            Value::I1(op_i1) => {
                if let Some(i1_value) = op_i1{
                    match i1_value{
                        true => {
                            SymIdx::new(ST_ROOT, "true")
                        },
                        false => {
                            SymIdx::new(ST_ROOT,"false")
                        },
                    }
                }else{
                    panic!("f1 {:?} unsure 无法转化为 symidx",self)
                }
            }
            _ => panic!("{:?}无法转化为 symidx",self)
        }
    }
    pub fn try_to_symidx(&self)->Option<SymIdx>{
        match self{
            Value::I32(op_i32) => {
                if let Some(i32_value) = op_i32{
                    Some(SymIdx::new(ST_ROOT,i32_value.to_string().leak()))
                }else{
                    None
                }
            },
            Value::F32(op_f32) => {
                if let Some(f32_value) = op_f32{
                    let mut f32_str = f32_value.to_string();
                    if !f32_str.contains("."){
                        f32_str.push('.');
                    }
                    Some(SymIdx::new(ST_ROOT,f32_str.leak()))
                }else{
                    None
                }

            },
            Value::Ref { rc_symidx, ty } => {
                Some(rc_symidx.as_ref_borrow().clone())
            },
            Value::I1(op_i1) => {
                if let Some(i1_value) = op_i1{
                    match i1_value{
                        true => {
                            Some(SymIdx::new(ST_ROOT, "true"))
                        },
                        false => {
                            Some(SymIdx::new(ST_ROOT,"false"))
                        },
                    }
                }else{
                    panic!("f1 {:?} unsure 无法转化为 symidx",self)
                }
            }
            _ => panic!("{:?}无法转化为 symidx",self)
        }
    }
    pub fn index_array(&self,offset:usize) -> Result<Value> {
        match self{
            Value::Array { value_map, dims: _, ele_ty: _ } => {
                value_map.get_ele_at(offset).cloned()
            },
            _ => {
                panic!("index_array 无法对 非数组类型 使用 {:?}",&self)
            }
        }
    }

    pub fn get_ele_size(&self) -> usize{
        self.to_type().get_ele_size()
    }
    pub fn get_mem_len(&self) -> usize{
        self.to_type().get_mem_len()
    }
}
impl Type {
    /// 这个函数接受一个ast_node 和 ast_tree 通过识别 ast_node 来完成基本类型的识别  
    /// 但是无法识别数组类型(只能识别数组的元素类型)
    pub fn new(ast_node:u32, ast_tree:&AstTree) -> Self {
        // 在asttree中找到node的u32所在节点的类型,返回I32或F32
        let text = node!(at ast_node in ast_tree).op_text.as_ref().unwrap().as_str();
        match text {
            "int" => Type::I32,
            "float" => Type::F32,
            "bool" => Type::I1,
            "double" => Type::F32,
            "void" => Type::Void,
            _ => panic!("text中类型错误 找到不支持的类型 {}", text),
        }
    }
    /// 这个函数接受一个元素类型和各个维度的大小来构建一个数组类型
    /// 但是禁止创建数组的数组
    pub fn new_array_dims_known(ele_ty:Type,dims:Vec<RcSymIdx>)->Self{
        match &ele_ty{
            Type::Fn { arg_syms: _, ret_sym: _ } => panic!("无法新建函数类型的数组"),
            _=>{}
        }
        Type::Array { dims:dims.into_iter().map(|x| Some(x)).collect_vec(), ele_ty: Box::new(ele_ty) }
    }
    pub fn new_array_dims_may_unknown(ele_ty:Type,dims:Vec<Option<RcSymIdx>>)->Self{
        match &ele_ty{
            Type::Fn { arg_syms: _, ret_sym: _ } => panic!("无法新建函数类型的数组"),
            _=>{}
        }
        Type::Array { dims, ele_ty: Box::new(ele_ty) }
    }
    pub fn new_array_dims_may_unknown_with_dims_2_pow(ele_ty:Type,mut dims:Vec<Option<RcSymIdx>>)->Result<Self>{
        match &ele_ty{
            Type::Fn { arg_syms: _, ret_sym: _ } => panic!("无法新建函数类型的数组"),
            _=>{}
        }
        for (idx,op_dim) in &mut dims.iter_mut().rev().enumerate(){
            match op_dim{
                Some(dim) => {
                    if idx < 2 {
                        let symidx:SymIdx = Value::from_symidx(&dim.as_ref_borrow()).as_i32_to_min_2_power()?.to_symidx();
                        *dim =  symidx.as_rc();
                    }
                },
                None => {
                    // do nothing to unknown dim
                },
            }
        }
        Ok(Type::Array { dims, ele_ty: Box::new(ele_ty) })
    }
    pub fn direct_suits(&self, another_type:&Type) -> bool{
        match (self,another_type){
            (Type::Array { dims, ele_ty },Type::Array { dims:dims2, ele_ty:ele_ty2 }) => {
                dims.len() == dims2.len() && ele_ty == ele_ty2
            },
            (Type::Array { dims, ele_ty },Type::Ptr64 { ty }) if !ty.is_array() => {
                ele_ty == ty // && dims.len()==1
            },
            (Type::Array { dims, ele_ty },Type::Ptr64 { ty }) if ty.is_array() => {
                match ty.as_ref(){
                    Type::Array { dims: another_dims, ele_ty } => {
                        ele_ty.as_ref() == &ty.get_ele_ty() // && another_dims.len()==  dims.len() -1
                    },
                    _ =>panic!()
                }
            },
            (Type::Ptr64 { ty:ty1 }, Type::Ptr64 { ty:ty2 }) => {
                true
            }
            _ => {
                self == another_type
            }
        }
    }
    /// get align of the type, it will return ele align if it is a array
    pub fn get_align(&self)->usize{
        match &self{
            Type::I32 => 4,
            Type::F32 => 4,
            Type::I1 => 1,
            Type::Void => panic!("can't get alignment of void type {:?}",self),
            Type::Label => panic!("can't get alignment of label type {:?}",self),
            Type::Array { dims: _, ele_ty: ty } => ty.get_align(),
            Type::Fn { arg_syms: _, ret_sym: _ } =>panic!("can't get alignment of func type {:?}",self),
            Type::Ptr64 { ty: _ } => 8,
            Type::Ref => panic!("can't get align of Ref ty"),
            Type::Unknown => panic!("can't get align of unknown"),
        }
    }

    /// return the size of element if it is an array or else its size 
    pub fn get_ele_size(&self) -> usize{
        match &self{
            Type::Array { dims: _, ele_ty } => {
                ele_ty.get_mem_len()
            },
            Type::Ptr64 { ty } => {
                ty.get_ele_size()
            }
            _ => self.get_mem_len(),
        }
    }
    pub fn get_size(&self) -> usize{
        match &self{
            Type::I32 => 4,
            Type::F32 => 4,
            Type::I1 => 1,
            Type::Void => panic!("can't get alignment of void type {:?}",self),
            Type::Label => panic!("can't get alignment of label type {:?}",self),
            Type::Array { dims: _, ele_ty: ty } => ty.get_align(),
            Type::Fn { arg_syms: _, ret_sym: _ } =>panic!("can't get alignment of func type {:?}",self),
            Type::Ptr64 { ty: _ } => 8,
            Type::Ref => panic!("can't get align of Ref ty"),
            Type::Unknown => panic!("can't get align of unknown"),
        }
    }
    pub fn get_ele_ty(&self) -> Type{
        match &self{
            Type::Array { dims: _, ele_ty } => {
                *ele_ty.clone()
            },
            _ => {
                self.clone()
            }
        }

    }

    pub fn push_dim(&mut self,dim_symidx:RcSymIdx){
        match self{
            Type::Fn { arg_syms: _, ret_sym: _ } => panic!("无法新建函数类型的数组"),
            Type::Void => panic!("无法新建void类型的数组"),
            Type::Label => panic!("无法新建label类型的数组"),
            Type::I32 => *self = Type::new_array_dims_known(self.clone(), vec![dim_symidx]),
            Type::F32 => *self =Type::new_array_dims_known(self.clone(), vec![dim_symidx]),
            Type::I1 => *self =Type::new_array_dims_known(self.clone(), vec![dim_symidx]),
            Type::Array { dims, ele_ty: _ty } => {
                dims.push(Some(dim_symidx));
            },
            Type::Ptr64 { ty } => {
                ty.push_dim(dim_symidx);
            }
            Type::Ref => todo!(),
            Type::Unknown => todo!(),
        }
    }

    pub fn pop_dim(&mut self){
        match self{
            Type::Array { dims, ele_ty: ty } => {
                if dims.len()>1{
                    dims.remove(0);
                }else{
                    *self = *ty.clone();
                    // debug_info_red!("after popped dim : {:?}",self)
                }
            },
            Type::Ptr64 { ty } => {
                if ty.is_array(){
                    ty.pop_dim();
                }else {
                    *self = *ty.clone()
                }
            },
            _ => {panic!("{:?} 无法 pop_dim ",self)}
        }
    }
    pub fn ptr2arr(&self) -> Type{
        match self{
            Type::Ptr64 { ty } => {
                Type::Array { dims: 
                    match ty.as_ref(){
                        Type::Array { dims, ele_ty } => {
                            let mut dims = dims.clone();
                            dims.insert(0, None);
                            dims
                        },
                        Type::Ptr64{ ty } => {
                            let arr = self.ptr2arr();
                            match arr{
                                Type::Array { mut dims, ele_ty } => {
                                    dims.insert(0, None);
                                    dims
                                },
                                _ => {panic!()}
                            }
                        }
                        _=>{
                            vec![None]
                        }
                    }
                    , ele_ty: Box::new(ty.get_ele_ty())
                    }
            },
            _ => {
                panic!("you can only transform a ptr 2 arr")
            }
        }
    }
    pub fn arr2ptr(&self) -> Type{
        match self{
            Type::Array { dims, ele_ty } => {
                let mut ty = self.clone();
                ty.pop_dim();
                Self::Ptr64 { ty:Box::new(ty)}
            },
            _ => panic!()
        }
    }
    pub fn try_arr2ptr(&self) -> Result<Type>{
        match self{
            Type::Array { dims, ele_ty } => {
                let mut ty = self.clone();
                ty.pop_dim();
                Ok(Self::Ptr64 { ty:Box::new(ty)  })
            },
            Type::Ptr64 { ty } => {
                Ok(self.clone())
            }
            _ => panic!()
        }
    }

    /// self could be an array or ptr
    pub fn get_array_dim_stride_symidx_vec(&self)->Vec<SymIdx>{
        let ty = if self.is_ptr_64(){
            self.ptr2arr()
        }else {
            self.clone()
        };
        match ty{
            Self::Array { dims, ele_ty: _ty }=>{
                let mut v1 = Value::new_i32(1);
                let mut weighted_dims = vec![v1.to_symidx()];
                for dim_symidx in dims.get(1..dims.len()).unwrap().iter().rev(){
                    let v2 = Value::from_string_with_specific_type(&dim_symidx.as_ref().unwrap().as_ref_borrow().symbol_name, &Type::I32);
                    debug_info_blue!(" v2 is  {:?}",v2);
                    v1 = v1*v2;
                    weighted_dims.push(v1.to_symidx())
                }
                weighted_dims.reverse();
                debug_info_blue!("weight vec calcuated is {:?}",weighted_dims);
                weighted_dims
            },
            _=> {panic!("get_array_dim_weight_vec 仅能对 array type 使用，无法根据给定type:{:?}给出",self)}
        }
    }
    pub fn get_array_dim_stride_usize_vec(&self)->Vec<usize>{
        let ty = if self.is_ptr_64(){
            self.ptr2arr()
        }else {
            self.clone()
        };
        match ty{
            Self::Array { dims, ele_ty: _ty }=>{
                let mut v1 = Value::new_i32(1);
                let mut weighted_dims = vec![v1.as_usize()];
                for dim_symidx in dims.get(1..dims.len()).unwrap().iter().rev(){
                    let v2 = Value::from_string_with_specific_type(&dim_symidx.as_ref().unwrap().as_ref_borrow().symbol_name, &Type::I32);
                    debug_info_blue!(" v2 is  {:?}",v2);
                    v1 = v1*v2;
                    weighted_dims.push(v1.as_usize())
                }
                weighted_dims.reverse();
                debug_info_blue!("weight vec calcuated is {:?}",weighted_dims);
                weighted_dims
            },
            _=> {panic!("get_array_dim_weight_vec 仅能对 array type 使用，无法根据给定type:{:?}给出",self)}
        }

    }
    pub fn get_array_dim(&self)->Result<&Vec<Option<RcSymIdx>>>{
        match &self{
            Type::I32 => panic!("can't get dim from i32"),
            Type::F32 => panic!("can't get dim from f32"),
            Type::I1 => panic!("can't get dim from i1"),
            Type::Void => panic!("can't get dim from void"),
            Type::Label => panic!("can't get dim from label"),
            Type::Ref => panic!("can't get dim from ref"),
            Type::Ptr64 { ty } => panic!("can't get dim from ptr64"),
            Type::Array { dims, ele_ty } => Ok(dims),
            Type::Fn { arg_syms, ret_sym } => panic!("can't get dim from fn"),
            Type::Unknown => panic!("can't get dim from unknown"),
        }
    }


    pub fn new_from_string(ty_str:&str) -> Result<Self>{
        match ty_str{
            "i32" => {
                Ok(Type::I32)
            },
            "f32" => {
                Ok(Type::F32)
            },
            _ => {
                todo!()
                // let re = Regex::new(r"^array:(\w+)((?:\[\d+\])+)").unwrap();
                // // 匹配输入字符串
                //     // 提取维度
                // if let Some(captures) = re.captures(ty_str) {
                //     let ele_ty = Box::new(Type::new_from_string(&captures[1])?);
                //     let dims = captures[2]
                //         .split(|c| c == '[' || c == ']')
                //         .map(|s| Rc::new(RefCell::new(SymIdx::new(ST_ROOT,s.to_string())))).into_iter()
                //         .collect_vec();
                //     Ok(Type::Array { dims:dims.into_iter().map(|x| Some(x)).collect_vec(), ele_ty  })
                // } else {
                //     panic!("无法识别为 type: {:?}",ty_str)
                // }
            }
        }
    }
    /// return the length of ty if it's an array or else 1
    pub fn get_ele_len(&self) -> usize{
        match self{
            Type::Array { dims, ele_ty: _ } => {
                let array_size:usize = dims.iter()
                    .map(|d|{let ans:usize = d.as_ref().unwrap().as_ref_borrow().symbol_name.parse().unwrap();ans}).product() ;
                array_size
            },
            _ => {
                1
            }
        }
    }

    pub fn get_mem_len(&self)->usize{
        match &self{
            Type::I32 => 4,
            Type::F32 => 4,
            Type::I1 => 1,
            Type::Void => todo!(),
            Type::Label => todo!(),
            Type::Array { dims: _, ele_ty: ty } => self.get_ele_len()*self.get_ele_size(),
            Type::Fn { arg_syms: _, ret_sym: _ } => todo!(),
            Type::Ptr64 { ty: _ } => TARGET_POINTER_MEM_LEN,
            Type::Ref => panic!("can't get mem_len of Ref"),
            Type::Unknown =>panic!("can't get mem_len of Unknown"),
        }
    }
    pub fn arith_adapt(ty1:&Type, ty2:&Type) -> Self {
        match (ty1, ty2) {
            (Type::I32, Type::I32) => Type::I32,
            (Type::I32, Type::F32) => Type::F32,
            (Type::F32, Type::I32) => Type::F32,
            (Type::F32, Type::F32) => Type::F32,
            (Type::I32, Type::I1) => Type::I32,
            (Type::F32, Type::I1) => Type::F32,
            (Type::I1, Type::I32) => Type::I32,
            (Type::I1, Type::F32) => Type::F32,
            (Type::I1, Type::I1) => Type::I1,
            (Type::Ptr64 { ty:ty1 }, Type::Ptr64 { ty:ty2 }) => Type::arith_adapt(ty1, ty2),
            (Type::Ptr64 { ty:ty1 },  ty2) => Type::arith_adapt(ty1, ty2),
            (ty1, Type::Ptr64 { ty:ty2 }) => Type::arith_adapt(ty1, ty2),
            _ => {
                panic!("{:?} and {:?} can't arith_adpat", ty1, ty2)
            }
        }
    }
    pub fn to_ref_ptr_type(&self) -> Self{
        match self{
            Type::I32 => Type::Ptr64 { ty: Box::new(Type::I32)},
            Type::F32 => Type::Ptr64 { ty: Box::new(Type::F32)},
            Type::I1 => Type::Ptr64 { ty: Box::new(Type::I1)},
            Type::Void => Type::Ptr64 { ty: Box::new(Type::Void)},
            Type::Label => todo!(),
            Type::Ptr64 { ty: _ } => Type::Ptr64 { ty: Box::new(self.clone())},
            Type::Array { dims: _, ele_ty: _ty } => {let mut poped_array = self.clone();poped_array.pop_dim();Type::Ptr64 { ty: Box::new(poped_array)}},
            Type::Fn { arg_syms: _, ret_sym: _ } => Type::Ptr64 { ty: Box::new(self.clone())},
            Type::Ref => panic!("ref type can't trans to ptr type"),
            Type::Unknown => todo!(),
        }
    }
    pub fn to_deref_ptr_type(&self) -> Self{
        match self{
            Type::I32 => panic!("{:?} can't be deref_type",self),
            Type::F32 => panic!("{:?} is not  deref_type",self),
            Type::I1 => panic!("{:?} is not deref_type",self),
            Type::Label => panic!("{:?} is not deref_type",self),
            Type::Void => panic!("{:?} is not deref_type",self),
            Type::Ptr64 { ty } => *ty.clone(),
            Type::Array { dims: _, ele_ty: _ty } => panic!("{:?}无法被deref_type",self),
            Type::Fn { arg_syms: _, ret_sym: _ } => panic!("{:?}无法被deref_type",self),
            _ => {
                panic!("can't trans to ref")
            }
        }
    }
    // pub fn type_when_use(&self) -> Self{
    //     match self{
    //         Type::I32 => Type::Ptr64 { ty: Box::new(Type::I32)},
    //         Type::F32 => Type::Ptr64 { ty: Box::new(Type::F32)},
    //         Type::I1 => Type::Ptr64 { ty: Box::new(Type::I1)},
    //         Type::Void => todo!(),
    //         Type::Label => todo!(),
    //         Type::Ptr64 { ty } => Type::Ptr64 { ty: Box::new(self.clone())},
    //         Type::Array { dims, ty } => Type::Ptr64 { ty: Box::new(self.clone())},
    //         Type::Fn { arg_syms, ret_sym } => Type::Ptr64 { ty: Box::new(self.clone())},
    //     }
    // }
}

impl Debug for Type {
    fn fmt(&self, f:&mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::I32 => write!(f, "i32"),
            Type::F32 => write!(f, "f32"),
            Type::I1 => write!(f, "i1"),
            Type::Fn { arg_syms: args_types, ret_sym: ret_type } => {
                write!(f, "Fn{:?}->{:?}", args_types, ret_type)
            }
            Type::Void => write!(f, "void"),
            Type::Label => write!(f, "label"),
            // Type::Unsure {  } => write!(f, "unsure"),
            Type::Array { dims, ele_ty: ty } => write!(f,"Array:{:?}:{:?}",ty,dims),
            Type::Ptr64 { ty } => write!(f,"ptr->{:?}",ty),
            Type::Ref => write!(f,"ref"),
            Type::Unknown => write!(f,"unknown"),
        }
    }
}
impl Add for Value{
    type Output=Value;

    fn add(self, rhs: Self) -> Self::Output {
        let pub_ty=self.adapt(&rhs);
        let l_val=self.trans_to_specific_type(&pub_ty);
        let r_val=rhs.trans_to_specific_type(&pub_ty);
        match (l_val,r_val) {
            (Value::I32(Some(v1)), Value::I32(Some(v2))) => Value::new_i32(v1+v2),
            (Value::F32(Some(v1)), Value::F32(Some(v2))) => Value::new_f32(v1+v2),
            (Value::F32(Some(v1)), Value::I32(Some(v2))) => Value::new_f32(v1 + (v2 as f32)),
            (Value::I32(Some(v1)), Value::F32(Some(v2))) => Value::new_f32((v1 as f32) + v2),
            (Value::I1(Some(_v1)), Value::I1(Some(_v2))) => panic!("I1 can't add"),
            (Value::Void, Value::Void) => panic!("Void can't add"),
            (_,_) => panic!("can't add"),
        }
    }
}
impl Sub for Value{
    type Output=Value;

    fn sub(self, rhs: Self) -> Self::Output {
        let pub_ty=self.adapt(&rhs);
        let l_val=self.trans_to_specific_type(&pub_ty);
        let r_val=rhs.trans_to_specific_type(&pub_ty);
        match (l_val,r_val) {
            (Value::I32(Some(v1)), Value::I32(Some(v2))) => Value::new_i32(v1 - v2),
            (Value::F32(Some(v1)), Value::F32(Some(v2))) => Value::new_f32(v1 - v2),
            (Value::F32(Some(v1)), Value::I32(Some(v2))) => Value::new_f32(v1 - (v2 as f32)),
            (Value::I32(Some(v1)), Value::F32(Some(v2))) => Value::new_f32((v1 as f32 - v2)),
            (Value::I1(Some(_v1)), Value::I1(Some(_v2))) => panic!("I1 can't sub"),
            (Value::Void, Value::Void) => panic!("Void can't sub"),
            (_,_) => panic!("can't sub"),
        }
    }
}
impl Mul for Value{
    type Output = Value;

    fn mul(self, rhs: Self) -> Self::Output {
        let pub_ty=self.adapt(&rhs);
        let l_val=self.trans_to_specific_type(&pub_ty);
        let r_val=rhs.trans_to_specific_type(&pub_ty);
        match (l_val,r_val) {
            (Value::I32(Some(v1)), Value::I32(Some(v2))) => Value::new_i32(v1 * v2),
            (Value::F32(Some(v1)), Value::F32(Some(v2))) => Value::new_f32(v1 * v2),
            (Value::F32(Some(v1)), Value::I32(Some(v2))) => Value::new_f32(v1 * (v2 as f32)),
            (Value::I32(Some(v1)), Value::F32(Some(v2))) => Value::new_f32((v1 as f32 * v2)),
            (Value::I1(Some(_v1)), Value::I1(Some(_v2))) => panic!("I1 can't mul"),
            (Value::Void, Value::Void) => panic!("Void can't mul"),
            (_,_) => panic!("can't mul"),
        }
    }
}
impl Div for Value{
    type Output = Value;

    fn div(self, rhs: Self) -> Self::Output {
        let pub_ty=self.adapt(&rhs);
        let l_val=self.trans_to_specific_type(&pub_ty);
        let r_val=rhs.trans_to_specific_type(&pub_ty);
        match (l_val,r_val) {
            (Value::I32(Some(v1)), Value::I32(Some(v2))) => Value::new_i32(v1 / v2),
            (Value::F32(Some(v1)), Value::F32(Some(v2))) => Value::new_f32(v1 / v2),
            (Value::F32(Some(v1)), Value::I32(Some(v2))) => Value::new_f32(v1 / (v2 as f32)),
            (Value::I32(Some(v1)), Value::F32(Some(v2))) => Value::new_f32((v1 as f32 / v2)),
            (Value::I1(Some(_v1)), Value::I1(Some(_v2))) => panic!("I1 can't div"),
            (Value::Void, Value::Void) => panic!("Void can't div"),
            (_,_) => panic!("can't div"),
        }
    }
}
impl Rem for Value{
    type Output = Value;

    fn rem(self, rhs: Self) -> Self::Output {
        let pub_ty=self.adapt(&rhs);
        let l_val=self.trans_to_specific_type(&pub_ty);
        let r_val=rhs.trans_to_specific_type(&pub_ty);
        match (l_val,r_val) {
            (Value::I32(Some(v1)), Value::I32(Some(v2))) => Value::new_i32(v1 % v2),
            (Value::F32(Some(v1)), Value::F32(Some(v2))) => Value::new_f32(v1 % v2),
            (Value::I1(Some(_v1)), Value::I1(Some(_v2))) => panic!("I1 can't Rem"),
            (Value::Void, Value::Void) => panic!("Void can't Rem"),
            (_,_) => panic!("can't rem"),
        }
    }
}
impl BitAnd for Value{
    type Output = Value;
    fn bitand(self, rhs: Self) -> Self::Output {
        let pub_ty=self.adapt(&rhs);
        let l_val=self.trans_to_specific_type(&pub_ty);
        let r_val=rhs.trans_to_specific_type(&pub_ty);
        match (l_val,r_val) {
            (Value::I32(Some(v1)), Value::I32(Some(v2))) => Value::new_i32(v1 & v2),
            (Value::F32(Some(_v1)), Value::F32(Some(_v2))) => panic!("F32 can't bitand"),
            (Value::I1(Some(v1)), Value::I1(Some(v2))) => Value::new_i1(v1 & v2),
            (Value::Void, Value::Void) => panic!("Void can't bitand"),
            (_,_) => panic!("can't bitand"),
        }
    }
}
impl BitOr for Value{
    type Output = Value;
    fn bitor(self, rhs: Self) -> Self::Output {
        let pub_ty=self.adapt(&rhs);
        let l_val=self.trans_to_specific_type(&pub_ty);
        let r_val=rhs.trans_to_specific_type(&pub_ty);
        match (l_val,r_val) {
            (Value::I32(Some(v1)), Value::I32(Some(v2))) => Value::new_i32(v1 | v2),
            (Value::F32(Some(_v1)), Value::F32(Some(_v2))) => panic!("F32 can't bitor"),
            (Value::I1(Some(v1)), Value::I1(Some(v2))) => Value::new_i1(v1 | v2),
            (Value::Void, Value::Void) => panic!("Void can't bitor"),
            (_,_) => panic!("can't bitor "),
        }
    }
}
impl Not for Value{
    type Output = Value;
    fn not(self) -> Self::Output {
        // let pub_ty=self.adapt(&self)?;
        // let l_val=self.to_specific_type(&pub_ty)?;
        match &self {
            Value::I32(Some(v1)) => Value::new_i32({
                if *v1 == 0 { 1 }else{ 0 } }
            ),
            Value::F32(Some(v1)) => Value::new_i1(
                if *v1 == 0.0 { true }else{ false }
            ),
            Value::I1(Some(v1)) => Value::new_i1(!v1),
            Value::Void => panic!("Void can't logical not"),
            _ => panic!("can't logical not"),
        }
    }
}
impl Neg for Value{
    type Output = Value;
    
    fn neg(self) -> Self::Output {
        match &self {
            Value::I32(Some(v1)) => Value::new_i32(-v1),
            Value::F32(Some(v1)) => Value::new_f32(-v1),
            Value::I1(Some(v1)) => panic!("I1 can't neg"),
            Value::Void => panic!("Void 类型无法进行按位非运算"),
            _ => panic!("其他类型无法进行按位非运算"),
        }
    }
}

#[derive(Clone)]
pub struct UseCounter {
    pub use_count:u32,
}

impl Debug for UseCounter {
    fn fmt(&self, f:&mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{}", self.use_count) }
}

pub fn unwrap_vec<T:Clone>(v:&Vec<Option<T>>)  -> Vec<T>{
    v.clone().into_iter().map(|x| x.unwrap()).collect_vec()
}
impl TypeDiscriminants{
    pub fn new_from_const_str(const_str:&str) -> Self {
        if const_str.contains("true") || const_str.contains("false"){
            TypeDiscriminants::I1
        }else if const_str.contains(".") && (const_str.chars().next().map_or(false, |x|x.is_numeric()|| x=='-' || x=='.')) {
            TypeDiscriminants::F32
        } else if const_str.chars().next().map_or(false, |x|x.is_numeric() || x=='-') && (const_str.contains("e")|| const_str.contains("E")){
            TypeDiscriminants::F32
        }else if const_str.chars().next().map_or(false, |x|x.is_numeric() || x=='-'){
            TypeDiscriminants::I32
        }else if const_str.starts_with("{"){
            TypeDiscriminants::Array 
        }else{
            TypeDiscriminants::Unknown
        } 
    }
}
impl From<TypeDiscriminants> for Type{
    fn from(value: TypeDiscriminants) -> Self {
        match value {
            TypeDiscriminants::I32 => Type::I32,
            TypeDiscriminants::F32 => Type::F32,
            TypeDiscriminants::I1 => Type::I1,
            TypeDiscriminants::Void => Type::Void,
            TypeDiscriminants::Label => Type::Label,
            TypeDiscriminants::Ref => Type::Ref,
            TypeDiscriminants::Ptr64 => Type::Unknown,
            TypeDiscriminants::Array => Type::Array { dims: vec![], ele_ty: Box::new(Type::Unknown) },
            TypeDiscriminants::Fn => panic!(),
            TypeDiscriminants::Unknown => Type::Unknown,
        }
    }
}
// impl From<&Type> for TypeDiscriminants{
//     fn from(value: &Type) -> Self {
//         match value {
//             Type::I32 => TypeDiscriminants::I32,
//             Type::F32 => TypeDiscriminants::F32 ,
//             Type::I1 => TypeDiscriminants::I1 ,
//             Type::Void => TypeDiscriminants::Void ,
//             Type::Label => TypeDiscriminants::Label,
//             Type::Ref => TypeDiscriminants::Ref,
//             Type::Ptr64 { ty } => TypeDiscriminants::Ptr64,
//             Type::Array { dims, ele_ty } => TypeDiscriminants::Array,
//             Type::Fn { arg_syms, ret_sym } => TypeDiscriminants::Fn,
//             Type::Unknown => TypeDiscriminants::Unknown,
//         }
//     }
// 