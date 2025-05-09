///  返回下一层找到的第一个rule_id符合的节点，使用这个宏的时候必须确保语境中有ast_tree,node
/// ```rust    
/// let node =3;  // 三号节点是一个 function def
/// let node_ids= find!(rule RULE_compoundStatement
///                    finally RULE_blockItemList
///                    at node in ast_tree).unwrap();
/// assert_eq!(node_ids , vec![119,24,12] ,"找到的 node id 不对");
/// ```
#[macro_export]
macro_rules! find {
    (rule $id:ident at $node:ident in $ast_tree:ident) => {
        {
            let mut iter  = crate::toolkit::ast_node::find_neighbors_rule_ast($ast_tree,$node,Some($id));
            iter.next()
        }
    } ;
    (rule $($id:ident)then+ finally $fin_id:ident at $node:ident in $ast_tree:ident) => {
        {
            let new_node = $node;
            $(let new_node = find!(rule $id at new_node in $ast_tree).unwrap();)+
            find!(rule $fin_id at new_node in $ast_tree)
        }
    };
    (rule $($id:ident)then+ finally term $fin_id:ident at $node:ident in $ast_tree:ident) => {
        {
            let new_node = $node;
            $(let new_node = find!(rule $id at new_node in $ast_tree).unwrap();)+
            find!(term $fin_id at new_node in $ast_tree)
        }
    };
    (term $term_name:ident at $node:ident in $ast_tree:ident) => {
        {
            let mut iter  = crate::toolkit::ast_node::find_neighbors_term_ast($ast_tree,$node,Some($term_name));
            iter.next()
        }
    };
    (term at $node:ident in $ast_tree:ident) => {
        {
            let mut iter  = crate::toolkit::ast_node::find_neighbors_term_ast($ast_tree,$node,None);
            iter.next()
        }
    };
}

/// ? 返回下一层找到的第一个rule_id符合的节点，使用这个宏的时候必须确保语境中有ast_tree和node
/// ```rust    
/// let node =3;  // 三号节点是一个 function def
/// let node_ids:Vec<u32>=
/// find_nodes!(rule RULE_compoundStatement
///             then RULE_blockItemList
///             finally RULE_blockItem
///             at node in ast_tree);
/// assert_eq!(node_ids , vec![119,24,12] ,"找到的 node id 不对");
/// ```
#[macro_export]
macro_rules! find_nodes {
    (rule $id:ident at $node:ident in $ast_tree:ident) => {
        {
            let iter = crate::toolkit::ast_node::find_neighbors_rule_ast($ast_tree,$node,Some($id));
            let mut nodes:Vec<u32> = iter.collect();
            nodes.reverse();
            nodes
        }
    };
    (rule $id:ident at $node:ident in $ast_tree:ident iter_reversed) => {
        {
            let iter = crate::toolkit::ast_node::find_neighbors_rule_ast($ast_tree,$node,Some($id));
            iter
        }
    };
    (rule at $node:ident in $ast_tree:ident) => {
        {
            let iter = crate::toolkit::ast_node::find_neighbors_rule_ast($ast_tree,$node,None);
            let nodes:Vec<u32> = iter.collect();
            nodes.reverse();
            nodes
        }
    };
    (term $id:ident at $node:ident in $ast_tree:ident) => {
        {
            let iter = crate::toolkit::ast_node::find_neighbors_term_ast($ast_tree,$node,Some($id));
            let mut nodes:Vec<u32> = iter.collect();
            nodes.reverse();
            nodes
        }
    };
    (term $id:ident at $node:ident in $ast_tree:ident iter_reversed) => {
        {
            let iter = crate::toolkit::ast_node::find_neighbors_term_ast($ast_tree,$node,Some($id));
            iter
        }
    };
    (term at $node:ident in $ast_tree:ident) => {
        {
            let iter = crate::toolkit::ast_node::find_neighbors_term_ast($ast_tree,$node,None);
            let mut nodes:Vec<u32> = iter.collect();
            nodes.reverse();
            nodes
        }
    };
    (term at $node:ident in $ast_tree:ident iter_reversed) => {
        {
            let iter = crate::toolkit::ast_node::find_neighbors_term_ast($ast_tree,$node,None);
            iter
        }
    };
    (rule $($id:ident)then+ finally $fin_id:ident at $node:ident in $ast_tree:ident) => {
        {
            let new_node = $node;
            $(let new_node = find!(rule $id at new_node in $ast_tree).unwrap();)+
            let nodes:Vec<u32> = find_nodes!(rule $fin_id at new_node in $ast_tree);
            nodes
        }
    };
    (rule $($id:ident)then+ finally at $node:ident in $ast_tree:ident) => {
        {
            let new_node = $node;
            $(let new_node = find!(rule $id at new_node in $ast_tree).unwrap();)+
            let nodes:Vec<u32> = find_nodes!(rule at new_node in $ast_tree);
            nodes
        }
    };
}

/// 这个宏返回指定节点下dfs的遍历筛选结果迭代器
#[macro_export]
macro_rules! find_nodes_by_dfs {
    (rule $id:ident at $node:ident in $ast_tree:ident) => {{
        let iter = crate::toolkit::ast_node::find_dfs_rule_ast($ast_tree, $node, $id);
        iter.collect()
    }};
}

/// 这个宏返回指定节点直接附属的节点，你必须保证这个节点下只有一个节点
#[macro_export]
macro_rules! direct_child_node {
    (at $node:ident in $graph:ident) => {{
        $graph
            .neighbors(petgraph::matrix_graph::NodeIndex::from($node))
            .next()
            .unwrap_or_else(||panic!("no direct child node of {:?} in {:?}", $graph.node_weight(petgraph::matrix_graph::NodeIndex::from($node)), $graph))
            .index() as u32
    }};
    (at $node:ident in $graph:ident with_predicate $f:block )=> {{
        let v = $crate::direct_child_nodes!(at $node in $graph with_predicate $f);
        if v.len() >1 {
            panic!("node {} have more than one direct_child_node", $node)
        } else if v.len() ==1 {
            v[0]
        }else {
            panic!("node {} have no direct_child_node", $node)
        }
    }};
    (at $node:ident in $graph:ident ret_option) => {{
        let node_index_option = $graph.neighbors(petgraph::matrix_graph::NodeIndex::from($node)).next();
        node_index_option.map(|node_index| node_index.index() as u32)
    }};
}
#[macro_export]
macro_rules! direct_parent_node {
    (at $node:ident in $graph:ident) => {{
        let nodes = $crate::direct_parent_nodes!(at $node in $graph );
        if nodes.len() ==1 {
            nodes[0]
        }else{
            panic!("this node have multiple or zero parent_node, direct_parent_node failed")
        }
    }};
    (at $node:ident in $graph:ident ret_option) => {{
        let nodes = $crate::direct_parent_nodes!(at $node in $graph );
        if nodes.len() ==1 {
            Some(nodes[0])
        }else{
            None
        }
    }};
    (at $node:ident in $graph:ident with_predicate $f:block ) => {{
        let nodes = $crate::direct_parent_nodes!(at $node in $graph with_predicate $f);
        if nodes.len() ==1 {
            nodes[0]
        }else{
            panic!("this node have multiple or zero parent_node, direct_parent_node failed for {} with ans {:?}",$node,nodes)
        }
    }};
    (at $node:ident in $graph:ident with_predicate $f:block ret_option) => {{
        let nodes = $crate::direct_parent_nodes!(at $node in $graph with_predicate $f);
        if nodes.len() ==1 {
            Some(nodes[0])
        }else{
            None
        }
    }};
}
#[macro_export]
/// 用法:   
/// ```
/// direct_parent_nodes!(at $node in $graph)
/// ```
macro_rules! direct_parent_nodes {
    (at $node:ident in $graph:ident) => {{
        use petgraph::visit::EdgeRef;
        let edges_vec:Vec<_> = $graph.edges_directed(petgraph::matrix_graph::NodeIndex::from($node), petgraph::Direction::Incoming)
            .map(|e|e.source().index() as u32).collect();
        edges_vec
    }};
    (at $node:ident in $graph:ident with_predicate $f:block )=> {{
        use petgraph::visit::EdgeRef;
        let edges_vec:Vec<_> = $graph.edges_directed(petgraph::matrix_graph::NodeIndex::from($node), petgraph::Direction::Incoming)
            .filter($f)
            .map(|e|e.source().index() as u32).collect();
        edges_vec
    }};
}

#[macro_export]
/// 找到这个点的所有出边 EdgeRef  
/// # Example
/// ```
/// outgoing_edges(at $node in $graph)
/// ```
macro_rules! outgoing_edges {
    (at $node:ident in $graph:ident) => {{
        let edges = $graph.edges_directed(petgraph::matrix_graph::NodeIndex::from($node), petgraph::Direction::Outgoing).collect();
        edges
    }};
}
#[macro_export]
/// 找到这个点的唯一出边的weight
/// # Example
/// ```
/// outoing_edge_weight(at $node in $graph)
/// ```
macro_rules! outgoing_edge_weight {
    (at $node:ident in $graph:ident) => {{
        use petgraph::visit::EdgeRef;
        let mut edges:Vec<_> = $graph.edges_directed(petgraph::matrix_graph::NodeIndex::from($node), petgraph::Direction::Outgoing);
        edges.next().expect("这个node 没有边").weight()
    }};
}
#[macro_export]
/// 找到这个点的唯一出边的weight
/// # Example
/// ```
/// outoing_edge_weight(at $node in $graph)
/// ```
macro_rules! outgoing_edge_weights {
    (at $node:ident in $graph:ident) => {{
        use petgraph::visit::EdgeRef;
        let mut edges:Vec<_> = $graph.edges_directed(petgraph::matrix_graph::NodeIndex::from($node), petgraph::Direction::Outgoing).map(|edge| edge.weight());
        edges
    }};
}
#[macro_export]
/// 找到这个点的所有出边 EdgeRef
/// 用法 incoming_edges(at $node in $graph)
macro_rules! incoming_edges {
    (at $node:ident in $graph:ident) => {{
        let edges:Vec<_> = $graph.edges_directed(petgraph::matrix_graph::NodeIndex::from($node), petgraph::Direction::Incoming).collect();
        edges
    }};
}
#[macro_export]
/// 找到这个点的所有入边weight
/// 用法 incoming_edges(at $node in $graph)
macro_rules! incoming_edge_weights {
    (at $node:ident in $graph:ident) => {{
        use petgraph::visit::EdgeRef;
        let edges:Vec<_> = $graph.edges_directed(petgraph::matrix_graph::NodeIndex::from($node), petgraph::Direction::Incoming).map(|edge| edge.weight()).collect();
        edges
    }};
}
#[macro_export]
/// 找到这个点的唯一出边weight
/// 用法 outgoing_edges(at $node in $graph)
macro_rules! incoming_edge_weight {
    (at $node:ident in $graph:ident) => {{
        use petgraph::visit::EdgeRef;
        let mut edges = $graph.edges_directed(petgraph::matrix_graph::NodeIndex::from($node), petgraph::Direction::Incoming);
        edges.next().expect("这个node 没有边").weight()
    }};
}

#[macro_export]
macro_rules! direct_child_nodes {
    (at $node:ident in $graph:ident iter_reversed) => {{
        let iter = $graph.neighbors(petgraph::matrix_graph::NodeIndex::from($node)).map(|x| x.index() as u32);
        iter
    }};
    (at $node:ident in $graph:ident) => {{
        let iter = $graph.neighbors(petgraph::matrix_graph::NodeIndex::from($node)).map(|x| x.index() as u32);
        let mut nodes:Vec<u32> = iter.collect();
        nodes.reverse();
        nodes
    }};
    (at $node:ident in $graph:ident with_predicate $f:block )=> {{
        use petgraph::visit::EdgeRef;
        let mut edges_vec:Vec<_> = $graph.edges_directed(petgraph::matrix_graph::NodeIndex::from($node), petgraph::Direction::Outgoing)
            .filter($f)
            .map(|e|e.target().index() as u32).collect();
        edges_vec.reverse();
        edges_vec
    }};
    (at $node:ident in $graph:ident with_predicate $f:block iter_reversed)=> {{
        use petgraph::visit::EdgeRef;
        let iter = $graph.edges_directed(petgraph::matrix_graph::NodeIndex::from($node), petgraph::Direction::Outgoing)
            .filter($f)
            .map(|e|e.target().index() as u32);
        iter
    }};
    (at $node:ident in $graph:ident with_predicate $f:ident )=> {{
        use petgraph::visit::EdgeRef;
        let mut edges_vec:Vec<_> = $graph.edges_directed(petgraph::matrix_graph::NodeIndex::from($node), petgraph::Direction::Outgoing)
            .filter($f)
            .map(|e|e.target().index() as u32).collect();
        edges_vec.reverse();
        edges_vec
    }};
    (at $node:ident in $graph:ident with_priority $f:block )=> {{
        use petgraph::visit::EdgeRef;
        let mut edges_vec:Vec<_> = $graph.edges_directed(petgraph::matrix_graph::NodeIndex::from($node), petgraph::Direction::Outgoing)
            .filter(|e|$f(e)>0).collect();
        edges_vec.sort_by_key(|e|$f(e));
        let edges_vec:Vec<_> = edges_vec
            .iter()
            .map(|e|e.target().index() as u32).collect();
        edges_vec
    }};
}
/// 这个宏返回指定节点的outgoing 边，你必须保证这个节点的出边只有一条
#[macro_export]
macro_rules! direct_edge {
    (at $node:ident in $graph:ident) => {{
        $graph
            .edges(petgraph::matrix_graph::NodeIndex::from($node))
            .next()
            .unwrap_or_else(||panic!("no direct edge of {:?} in {:?}", $graph.node_weight(petgraph::matrix_graph::NodeIndex::from($node)), $graph))
            .index() as u32
    }};
}
#[macro_export]
/// 用法 add_edge!($edge from $a to $b in $graph)
macro_rules! add_edge {
    ($edge_struct:block from  $a:ident to $b:ident in $cfg_graph:ident) => {
        $cfg_graph.add_edge(petgraph::matrix_graph::NodeIndex::from($a), petgraph::matrix_graph::NodeIndex::from($b), $edge_struct).index() as u32
    };
    ($a:ident to $b:ident in $ast_tree:ident) => {
        $ast_tree.add_edge(petgraph::matrix_graph::NodeIndex::from($a), petgraph::matrix_graph::NodeIndex::from($b), ()).index() as u32
    };
    (from $a:ident to $b:ident in $scope_tree:ident) => {
        $scope_tree.add_edge(petgraph::matrix_graph::NodeIndex::from($a), petgraph::matrix_graph::NodeIndex::from($b), ()).index() as u32
    };
}
#[macro_export]
/// 有两种用法，一种是  
/// add_node($node_struct:ident to $graph:ident)  
/// 另一种则是  
/// add_node($node_struct:block to $graph:ident)  
/// 一个允许block 一个 允许 ident  
macro_rules! add_node {
    ($node_struct:ident to $graph:ident) => {
        $graph.add_node($node_struct).index() as u32
    };
    ($node_struct:block to $graph:ident) => {
        $graph.add_node($node_struct).index() as u32
    };
}

#[macro_export]
/// 用法 add_node_with_edge!($node_struct with_edge $edgestruct from $node in $graph)
/// 或你可以省略Edge(当你这个图中Edge确实定义是空的时候) add_node_with_edge!($node_struct $edgestruct from $node in $graph)
macro_rules! add_node_with_edge {
    ($node_struct:ident from $from_node:ident in $graph:ident) => {{
        let node_id = $graph.add_node($node_struct).index() as u32;
        $graph.add_edge(petgraph::matrix_graph::NodeIndex::from($from_node), petgraph::matrix_graph::NodeIndex::from(node_id), ());
        node_id
    }};
    ($node_struct:ident with_edge $edgestruct:ident from $from_node:ident in $graph:ident) => {{
        let node_id = $graph.add_node($node_struct).index() as u32;
        $graph.add_edge(petgraph::matrix_graph::NodeIndex::from($from_node), petgraph::matrix_graph::NodeIndex::from(node_id), $edgestruct);
        node_id
    }};
    ($node_struct:block from $from_node:ident in $graph:ident) => {{
        let node_id = $graph.add_node($node_struct).index() as u32;
        $graph.add_edge(petgraph::matrix_graph::NodeIndex::from($from_node), petgraph::matrix_graph::NodeIndex::from(node_id), ());
        node_id
    }};
    ($node_struct:block with_edge $edgestruct:block from $from_node:ident in $graph:ident) => {{
        let node_id = $graph.add_node($node_struct).index() as u32;
        $graph.add_edge(petgraph::matrix_graph::NodeIndex::from($from_node), petgraph::matrix_graph::NodeIndex::from(node_id), $edgestruct);
        node_id
    }};
}
/// add symbol with Some Fields to symtab
/// 比如说 将一个 I32 类型的 变量符号 添加到 符号表中，并且返回这个symbol在符号表中的 index
/// 要求 field_value 必须实现 Field trait
/// add_symbol(x with field field_name_A:field_value_A to some_symtab)
#[macro_export]
macro_rules! add_symbol {
    ($sym:ident $(with_field $field_name:ident:$field_value:block )* to $symtab:ident ) => {
        {
            let symidx = $symtab.add_symbol($sym);
            $(
                $(if $if_block )? {
                    let sym =  $symtab.get_mut_symbol(&symidx);
                    paste::paste!{
                    sym.[<add_ $field_name:lower>]($field_value);
                    };
                }
            )*
            symidx
        }
    };
    ($sym:block $(with_field $field_name:ident:$field_value:block $(if $if_block:block)?)* to $symtab:ident ) => {
        {
            let symidx = $symtab.add_symbol($sym);
            $(
            $(if $if_block )? {
                let sym =  $symtab.get_mut(&symidx.as_ref_borrow());
                paste::paste!{
                    sym.[<add_ $field_name:lower>]($field_value);
                };
            }
            )*
            symidx
        }
    };
    // hello
    ($sym_name:block of scope $scope:block $(with_field $field_name:ident:{$field_value:expr})* to $symtab:ident) => {
        {
            let mut sym = Symbol::new($scope,$sym_name);
            $(
            paste::paste!{
                sym.[<add_ $field_name:lower>]($field_value);
            };)*
            let symidx = $symtab.add_symbol(sym);
            symidx
        }
    };
}
/// 
/// 如果发现你想添加的field 已经存在则会直接 panic
///($field_name:ident:{$field:expr} to $symidx:ident in $symtab:ident ) => {
#[macro_export]
macro_rules! add_field {
    (with_field $field_name:ident:{$field:expr} to $symidx:ident in $symtab:ident ) => {
        paste::paste!{
            $symtab.get_mut(&$symidx.as_ref_borrow()).[<add_ $field_name:lower>]($field);
        };
    };
    ($(with_field $field_name:ident:{$field:expr})+ to $symidx:ident in $symtab:ident ) => {
        $(
        $crate::add_field!(with_field $field_name:{$field} to $symidx in $symtab);
        )+
    };
    (with_field $field_name:ident:{$field:expr} to $fields:ident ) => {
        $fields.insert($field_name,$field);
    };
}

#[macro_export]
macro_rules! get_ast_from_symidx {
    (find $symidx:ident with $scope_tree:ident) => {{
        let sym_scope = $symidx.scope_node;
        let sym_ast = node!(at sym_scope in $scope_tree).ast_node;
        sym_ast
    }}
}

#[macro_export]
macro_rules! node {
    (at $node:ident in $graph:ident) => {{
        $graph.node_weight(petgraph::matrix_graph::NodeIndex::from($node)).expect("找不到 index 对应的 node ")
    }};
    (at $node:block in $graph:ident) => {{
        $graph.node_weight(petgraph::matrix_graph::NodeIndex::from($node)).expect("找不到 index 对应的 node ")
    }};
}

#[macro_export]
macro_rules! node_mut {
    (at $node:ident in $graph:ident) => {{
        $graph.node_weight_mut(petgraph::matrix_graph::NodeIndex::from($node)).expect("找不到 index 对应的 node ")
    }};
}

/// 这个宏返回指定节点直接附属的节点，你必须保证这个节点下只有一个节点
#[macro_export]
macro_rules! rule_id {
    (at $node:ident in $ast_tree:ident) => {
        {
            if node!(at $node in $ast_tree).is_terminal{
                panic!("can't use rule_id macro to a terminal node")
            }
            node!(at $node in $ast_tree).rule_id
        }
    };
}
#[macro_export]
macro_rules! term_id {
    (at $node:ident in $ast_tree:ident) => {
        {
            if !node!(at $node in $ast_tree).is_terminal{
                panic!("can't use term_id macro to a rule node")
            }
            (node!(at $node in $ast_tree).rule_id as isize).try_into().expect("无法将此 term_id 转化为 isize 类型")
        }
    };
}

/// insert_instr($instr to $node at $idx in $graph slab $instrslab)  
#[macro_export]
macro_rules! insert_instr {
    ($instr:ident to $node:ident instr_list $instr_list:ident at $idx:block in $graph:ident slab $instrslab:ident) =>{
        {
            let cfg_node_struct = node_mut!(at $node in $graph);
            let instr = $instrslab.insert_instr($instr);
            cfg_node_struct.$instr_list.insert($idx,instr);
            // $instrslab.get_mut_instr(instr)?.add_cfg_instr_idx(CfgInstrIdx::new($node,cfg_node_struct.instrs.len()-1, false));
            instr
        }
    };
}

#[macro_export]
macro_rules! add_passes {
    ($first_pass:ident $(then $next_pass:ident)* to $pass_manager:ident) => {
        $pass_manager.add_pass(Box::new($first_pass));
        $($pass_manager.add_pass(Box::new($next_pass));)*
    };
}

// #[macro_export]
// macro_rules! make_field_owner_trait_for_struct {
//     ($($struct_name:ident),+ with fields $fields:ident) => {
//         $(
//         impl FieldsOwner for $struct_name {
//             fn add_field(&mut self, key: &'static str, sf: Box<dyn Field>) {
//                 self.$fields.insert(key, sf);
//             }
//             fn get_field(&self, key: &str) -> Option<&Box<dyn Field>> {
//                 self.$fields.get(key)
//             }
//             fn remove_field(&mut self, field_name: &'static str) {
//                 self.$fields.remove(field_name);
//             }
//             fn get_field_mut(&mut self, key: &str) -> Option<&mut Box<dyn Field>> {
//                 self.$fields.get_mut(key)
//             }
//         }
//         )*
//     };
// }
#[macro_export]
macro_rules! downcast_op_any {
    (ref $field_type:ty,$op_field:ident) => {{
        match $op_field {
            None => None,
            Some(field) => field.as_any().downcast_ref::<$field_type>(),
        }
    }};
    (mut $field_type:ty,$op_field_mut:ident) => {{
        match $op_field_mut {
            None => None,
            Some(field) => field.as_any_mut().downcast_mut::<$field_type>(),
        }
    }};
}

#[macro_export]
/// 生成特化的get  get_mut remove add 函数  
/// 类似于 get_cfg_node_type() 这种函数
/// 用法: makespecialized_get_field_fn_for_struct{struct_name
///     upper_field_name1:field_type1,
///     upper_field_name2:field_type2
///     with fields member_of_fields_type}
macro_rules! reg_field_for_struct {
    ($struct_name:ident {$($upper_field_name:ident:$field_type:ty,)*} with_fields $fields:ident with_prefix $prefix:ident) => {
        paste::paste!{
        $(
            reg_field_for_struct!{$struct_name
                {
                    [<$prefix _ $upper_field_name>]:$field_type,
                }
                with_fields $fields
            }
        )+
        }
    };
    ($struct_name:ident {$($upper_field_name:ident:$field_type:ty,)*} with_fields $fields:ident) => {
        $($crate::_reg_field_name!($upper_field_name);)*
        impl $struct_name {
            paste::paste!{
            $(
                pub fn [<get_ $upper_field_name:lower>](&self) -> &$field_type{
                    let op_field = self.$fields.get(&$upper_field_name.as_ptr());
                    $crate::downcast_op_any!(ref $field_type,op_field).unwrap_or_else(||panic!("can't find field {} type:{}",stringify!($field_type),stringify!($op_field)))
                }
                pub fn [<get_mut_ $upper_field_name:lower>](&mut self) -> &mut $field_type{
                    // if stringify!($upper_field_name) == "CUR_REG" {
                    //     println!("mut acess cur_reg of {:?}",self);
                    // }
                    let op_field_mut = self.$fields.get_mut(&$upper_field_name.as_ptr());
                    $crate::downcast_op_any!(mut $field_type,op_field_mut).unwrap_or_else(||panic!("can't find field {} type:{}",stringify!($field_type),stringify!($op_field)))
                }
                pub fn [<get_op_ $upper_field_name:lower>](&self) -> Option<&$field_type>{
                    // if stringify!($upper_field_name) == "CUR_REG" {
                    //     println!("mut acess cur_reg of {:?}",self);
                    // }
                    let op_field_mut = self.$fields.get(&$upper_field_name.as_ptr());
                    $crate::downcast_op_any!(ref $field_type,op_field_mut)
                }
                pub fn [<get_op_mut_ $upper_field_name:lower>](&mut self) -> Option<&mut $field_type>{
                    // if stringify!($upper_field_name) == "CUR_REG" {
                    //     println!("mut acess cur_reg of {:?}",self);
                    // }
                    let op_field_mut = self.$fields.get_mut(&$upper_field_name.as_ptr());
                    $crate::downcast_op_any!(mut $field_type,op_field_mut)
                }
                pub fn [<add_ $upper_field_name:lower>](&mut self, field:$field_type) {
                    // if stringify!($upper_field_name) == "CUR_REG" {
                    //     println!("add mut acess cur_reg of {:?}",self);
                    // }
                    let _op_field = self.$fields.insert($upper_field_name.as_ptr(),Box::new(field));
                    // let op_field_ref = op_field.as_ref();
                    // $crate::downcast_op_any!($field_type,op_field)
                }
                pub fn [<has_ $upper_field_name:lower>](&self)->bool{
                    self.$fields.get(&$upper_field_name.as_ptr()).is_some()
                }
                pub fn [<remove_ $upper_field_name:lower >](&mut self) {
                    self.$fields.remove(&$upper_field_name.as_ptr());
                }

            )*
            }
        }
    };
}
#[macro_export]
macro_rules! make_field_trait_for_struct {
    ($($struct_name:ty),+) => {
        $(
        impl Field for $struct_name {
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
            fn as_any_move(self) -> Box<dyn std::any::Any>{
                Box::new(self)
            }
            fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
                self
            }
            fn clone_box(&self)->Box<dyn Field> {
                Box::new(self.clone())
            }
            fn as_field_move(self) -> Box<dyn Field> {
                Box::new(self)
            }
        }
        )*
    };
}

#[macro_export]
macro_rules! instr {
    (at $idx:ident in $slab:ident) => {
        $slab.get_instr($idx)
    };
}
#[macro_export]
macro_rules! instr_mut {
    (at $idx:ident in $slab:ident) => {
        $slab.get_mut_instr($idx)
    };
}

/// 注册一个 field_name，这个name 用于注册一个
#[macro_export]
macro_rules! _reg_field_name {
    ($upper_field_name:ident) => {
        paste::paste!{
            // pub static $upper_field_name:&str = concat!(stringify!([<$upper_field_name:lower>])," ","(",file!(),")");
            pub static $upper_field_name:&str = stringify!([<$upper_field_name:lower>]);
        }
    };
}
#[macro_export]
macro_rules! create_lower_string_from_two_ident {
    ($ident1:ident,$ident2:ident) => {
        stringify!([<$ident1:lower _ $file:lower>]);       
    };
}

#[macro_export]
macro_rules! debug_info_blue{
    ($($t:tt)*) => {{
        use log::trace;
        // // println!("\x1B[34m debuginfo {}\x1B[0m",format!($($t)*))
        trace!("{}",format!($($t)*))
    }};
}
#[macro_export]
macro_rules! debug_info_yellow{
    ($($t:tt)*) => {{
        use log::info;
        // println!("\x1B[33m debuginfo {}\x1B[0m",format!($($t)*))
        info!("{}",format!($($t)*))
    }};
}
#[macro_export]
macro_rules! debug_info_green{
    ($($t:tt)*) => {{
        use log::debug;
        // // println!("\x1B[32m debuginfo {}\x1B[0m",format!($($t)*))
        debug!("{}",format!($($t)*))
    }};
}
#[macro_export]
macro_rules! debug_info_red{
    ($($t:tt)*) => {{
        use log::warn;
        // // println!("\x1B[31m debuginfo {}\x1B[0m",format!($($t)*))
        warn!("{}",format!($($t)*))
    }};
}

#[macro_export]
macro_rules! code_pos {
    ($msg:expr) => {
        format!("{}:{}:{} - {}", file!(), line!(), column!())
        
    };
}
// 这是一个计时宏
#[macro_export]
macro_rules! timeit {
    ($ret:block , $word:expr) => {{
        use log::info;
        let start_time = Instant::now();
        let ret = $ret;
        let duration = Instant::now() - start_time;
        info!("{} 耗时: {:?}", $word, duration);
        println!("{} 耗时: {:?}", $word, duration);
        ret
    }};
}