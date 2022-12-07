use indextree::NodeId;

use crate::namespace::NamespaceId;
use crate::prefix::PrefixId;
use crate::xmldata::{Node, XmlArena};
use crate::xmlvalue::Value;

pub struct Document {
    pub(crate) root: NodeId,
}

pub(crate) fn prefix_by_namespace(
    node_id: NodeId,
    namespace_id: NamespaceId,
    arena: &XmlArena,
) -> Option<PrefixId> {
    for ancestor in node_id.ancestors(arena) {
        let xml_node = arena.get(ancestor).unwrap().get();
        if let Value::Element(element) = xml_node {
            if let Some(prefix_id) = element.namespace_info.to_prefix.get(&namespace_id) {
                return Some(*prefix_id);
            }
        }
    }
    None
}

pub(crate) fn namespace_by_prefix(
    node_id: NodeId,
    prefix_id: PrefixId,
    arena: &XmlArena,
) -> Option<NamespaceId> {
    for ancestor in node_id.ancestors(arena) {
        let xml_node = arena.get(ancestor).unwrap().get();
        if let Value::Element(element) = xml_node {
            if let Some(namespace_id) = element.namespace_info.to_namespace.get(&prefix_id) {
                return Some(*namespace_id);
            }
        }
    }
    None
}

impl Document {
    pub fn root(&self) -> Node {
        Node::new(self.root)
    }

    // XXX probably break this into convenience methods
    // to lookup prefix. Getting the prefix is only handy when doing
    // tree manipulation in rare cases, as usually namespace is
    // fine. During serialization we use a special stack for
    // performance reasons.
    // fn fullname(&self, node_id: NodeId, name_id: NameId) -> Result<String, Error> {
    //     let name = self.data.name_lookup.get_value(name_id);
    //     if name.namespace_id == self.data.no_namespace_id {
    //         return Ok(name.name.to_string());
    //     }
    //     // XXX this is relatively slow
    //     let prefix_id = prefix_by_namespace(node_id, name.namespace_id, &self.data.arena);
    //     // if prefix_id cannot be found, then that's an error: we have removed
    //     // a prefix declaration even though it is still in use
    //     let prefix_id = prefix_id.ok_or_else(|| {
    //         Error::NoPrefixForNamespace(
    //             self.data
    //                 .namespace_lookup
    //                 .get_value(name.namespace_id)
    //                 .to_string(),
    //         )
    //     })?;
    //     if prefix_id == self.data.empty_prefix_id {
    //         Ok(format!("{}", name.name))
    //     } else {
    //         let prefix = self.data.prefix_lookup.get_value(prefix_id);
    //         Ok(format!("{}:{}", prefix, name.name))
    //     }
    // }
}

// impl<'a> Debug for Document<'a> {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         self.root_node_id()
//             .debug_pretty_print(&self.data.arena)
//             .fmt(f)
//     }
// }
