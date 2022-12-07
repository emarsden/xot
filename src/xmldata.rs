use indextree::{Arena, NodeEdge, NodeId};

use crate::document::Document;
use crate::error::Error;
use crate::name::{Name, NameId, NameLookup};
use crate::namespace::{Namespace, NamespaceId, NamespaceLookup};
use crate::prefix::{Prefix, PrefixId, PrefixLookup};
use crate::xmlvalue::{Comment, Element, ProcessingInstruction, Text, ValueType, XmlValue};

pub type XmlArena = Arena<XmlValue>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct XmlNode(NodeId);

impl XmlNode {
    pub(crate) fn new(node_id: NodeId) -> Self {
        XmlNode(node_id)
    }
    pub(crate) fn get(&self) -> NodeId {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum XmlNodeEdge {
    Start(XmlNode),
    End(XmlNode),
}

pub struct XmlData {
    pub(crate) arena: XmlArena,
    pub(crate) namespace_lookup: NamespaceLookup,
    pub(crate) prefix_lookup: PrefixLookup,
    pub(crate) name_lookup: NameLookup,
    pub(crate) no_namespace_id: NamespaceId,
    pub(crate) empty_prefix_id: PrefixId,
}

impl XmlData {
    pub fn new() -> Self {
        let mut namespace_lookup = NamespaceLookup::new();
        let no_namespace_id = namespace_lookup.get_id_mut(Namespace::new("".into()));
        let mut prefix_lookup = PrefixLookup::new();
        let empty_prefix_id = prefix_lookup.get_id_mut(Prefix::new("".into()));
        XmlData {
            arena: XmlArena::new(),
            namespace_lookup,
            prefix_lookup,
            name_lookup: NameLookup::new(),
            no_namespace_id,
            empty_prefix_id,
        }
    }

    #[inline]
    pub(crate) fn arena(&self) -> &XmlArena {
        &self.arena
    }

    #[inline]
    pub(crate) fn arena_mut(&mut self) -> &mut XmlArena {
        &mut self.arena
    }

    #[inline]
    pub fn xml_value(&self, node_id: XmlNode) -> &XmlValue {
        self.arena[node_id.0].get()
    }

    #[inline]
    pub fn xml_value_mut(&mut self, node_id: XmlNode) -> &mut XmlValue {
        self.arena[node_id.0].get_mut()
    }

    // manipulators

    pub(crate) fn new_node(&mut self, xml_node: XmlValue) -> XmlNode {
        XmlNode(self.arena.new_node(xml_node))
    }

    pub fn new_text(&mut self, text: &str) -> XmlNode {
        let text_node = XmlValue::Text(Text::new(text.to_string()));
        self.new_node(text_node)
    }

    pub fn new_element(&mut self, name_id: NameId) -> XmlNode {
        let element_node = XmlValue::Element(Element::new(name_id));
        self.new_node(element_node)
    }

    pub fn new_comment(&mut self, comment: &str) -> XmlNode {
        let comment_node = XmlValue::Comment(Comment::new(comment.to_string()));
        self.new_node(comment_node)
    }

    pub fn new_processing_instruction(&mut self, target: &str, data: Option<&str>) -> XmlNode {
        let pi_node = XmlValue::ProcessingInstruction(ProcessingInstruction::new(
            target.to_string(),
            data.map(|s| s.to_string()),
        ));
        self.new_node(pi_node)
    }

    pub fn append(&mut self, parent: XmlNode, child: XmlNode) -> Result<(), Error> {
        self.add_structure_check(Some(parent), child)?;
        if self.add_consolidate_text_nodes(child, self.last_child(parent), None) {
            return Ok(());
        }
        parent.0.checked_append(child.0, self.arena_mut())?;
        Ok(())
    }

    pub fn append_text(&mut self, parent: XmlNode, text: &str) -> Result<(), Error> {
        let text_node_id = self.new_text(text);
        self.append(parent, text_node_id)?;
        Ok(())
    }

    pub fn append_element(&mut self, parent: XmlNode, name_id: NameId) -> Result<(), Error> {
        let element_node_id = self.new_element(name_id);
        self.append(parent, element_node_id)?;
        Ok(())
    }

    pub fn append_comment(&mut self, parent: XmlNode, comment: &str) -> Result<(), Error> {
        let comment_node_id = self.new_comment(comment);
        self.append(parent, comment_node_id)?;
        Ok(())
    }

    pub fn append_processing_instruction(
        &mut self,
        parent: XmlNode,
        target: &str,
        data: Option<&str>,
    ) -> Result<(), Error> {
        let pi_node_id = self.new_processing_instruction(target, data);
        self.append(parent, pi_node_id)?;
        Ok(())
    }

    pub fn prepend(&mut self, parent: XmlNode, child: XmlNode) -> Result<(), Error> {
        self.add_structure_check(Some(parent), child)?;
        if self.add_consolidate_text_nodes(child, None, self.first_child(parent)) {
            return Ok(());
        }
        parent.0.checked_prepend(child.0, self.arena_mut())?;
        Ok(())
    }

    pub fn insert_after(
        &mut self,
        reference_node: XmlNode,
        new_sibling: XmlNode,
    ) -> Result<(), Error> {
        self.add_structure_check(self.parent(reference_node), new_sibling)?;
        if self.add_consolidate_text_nodes(
            new_sibling,
            Some(reference_node),
            self.next_sibling(reference_node),
        ) {
            return Ok(());
        }
        reference_node
            .0
            .checked_insert_after(new_sibling.0, self.arena_mut())?;
        Ok(())
    }

    pub fn insert_before(
        &mut self,
        reference_node: XmlNode,
        new_sibling: XmlNode,
    ) -> Result<(), Error> {
        self.add_structure_check(self.parent(reference_node), new_sibling)?;
        if self.add_consolidate_text_nodes(
            new_sibling,
            self.previous_sibling(reference_node),
            Some(reference_node),
        ) {
            return Ok(());
        }
        reference_node
            .0
            .checked_insert_before(new_sibling.0, self.arena_mut())?;
        Ok(())
    }

    pub fn detach(&mut self, node: XmlNode) -> Result<(), Error> {
        self.remove_structure_check(node)?;
        let prev_node = self.previous_sibling(node);
        let next_node = self.next_sibling(node);
        node.0.detach(self.arena_mut());
        self.remove_consolidate_text_nodes(prev_node, next_node);
        Ok(())
    }

    pub fn remove(&mut self, node: XmlNode) -> Result<(), Error> {
        self.remove_structure_check(node)?;
        let prev_node = self.previous_sibling(node);
        let next_node = self.next_sibling(node);
        node.0.remove_subtree(self.arena_mut());
        self.remove_consolidate_text_nodes(prev_node, next_node);
        Ok(())
    }

    fn add_structure_check(&self, parent: Option<XmlNode>, child: XmlNode) -> Result<(), Error> {
        let parent = parent.ok_or_else(|| {
            Error::InvalidOperation("Cannot create siblings for document root".into())
        })?;
        match self.value_type(child) {
            ValueType::Root => {
                return Err(Error::InvalidOperation("Cannot move document root".into()));
            }
            ValueType::Element => {
                if self.is_under_root(child) {
                    return Err(Error::InvalidOperation("Cannot move root element".into()));
                }
                if self.is_root(parent) {
                    return Err(Error::InvalidOperation(
                        "Cannot move extra element under document root".into(),
                    ));
                }
            }
            ValueType::Text => {
                if self.is_root(parent) {
                    return Err(Error::InvalidOperation(
                        "Cannot move text under document root".into(),
                    ));
                }
            }
            ValueType::ProcessingInstruction | ValueType::Comment => {
                // these can exist everywhere
            }
        }
        Ok(())
    }

    fn remove_structure_check(&self, node: XmlNode) -> Result<(), Error> {
        match self.value_type(node) {
            ValueType::Root => {
                return Err(Error::InvalidOperation(
                    "Cannot remove document root".into(),
                ));
            }
            ValueType::Element => {
                if self.is_under_root(node) {
                    return Err(Error::InvalidOperation("Cannot remove root element".into()));
                }
            }
            ValueType::Text | ValueType::ProcessingInstruction | ValueType::Comment => {
                // these have no removal constraints
            }
        }
        Ok(())
    }

    fn add_consolidate_text_nodes(
        &mut self,
        node: XmlNode,
        prev_node: Option<XmlNode>,
        next_node: Option<XmlNode>,
    ) -> bool {
        let added_text = if let XmlValue::Text(t) = self.xml_value(node) {
            Some(t.get().to_string())
        } else {
            None
        };
        if added_text.is_none() {
            return false;
        }
        let added_text = added_text.unwrap();

        // due to consolidation, two text nodes can never be adjacent,
        // so consolidate with the previous node or next node is fine
        if let Some(prev_node) = prev_node {
            if let XmlValue::Text(prev) = self.xml_value_mut(prev_node) {
                let mut s = prev.get().to_string();
                s.push_str(&added_text);
                prev.set(s);
                // remove the text node we wanted to insert as it's now consolidated
                node.0.remove(self.arena_mut());
                true
            } else {
                false
            }
        } else if let Some(next_node) = next_node {
            if let XmlValue::Text(next) = self.xml_value_mut(next_node) {
                let mut s = added_text;
                s.push_str(next.get());
                next.set(s);
                // remove the text node we wanted to insert as it's now consolidated
                node.0.remove(self.arena_mut());
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    fn remove_consolidate_text_nodes(
        &mut self,
        prev_node: Option<XmlNode>,
        next_node: Option<XmlNode>,
    ) -> bool {
        if prev_node.is_none() {
            return false;
        }
        let prev_node = prev_node.unwrap();
        if next_node.is_none() {
            return false;
        }
        let next_node = next_node.unwrap();
        let prev_text = self.text(prev_node);
        let next_text = self.text(next_node);
        if prev_text.is_none() || next_text.is_none() {
            return false;
        }
        let to_add = next_text.unwrap().get().to_string();

        let prev_text_mut = self.text_mut(prev_node).unwrap();
        let mut s = prev_text_mut.get().to_string();
        s.push_str(&to_add);
        prev_text_mut.set(s);
        next_node.0.remove(self.arena_mut());
        true
    }

    // accessors

    pub fn root_element(&self, document: &Document) -> XmlNode {
        for child in self.children(document.root()) {
            if let XmlValue::Element(_) = self.xml_value(child) {
                return child;
            }
        }
        unreachable!("Document should always have a single root node")
    }

    pub fn parent(&self, node: XmlNode) -> Option<XmlNode> {
        self.arena()[node.0].parent().map(XmlNode)
    }

    pub fn first_child(&self, node: XmlNode) -> Option<XmlNode> {
        self.arena()[node.0].first_child().map(XmlNode)
    }

    pub fn last_child(&self, node: XmlNode) -> Option<XmlNode> {
        self.arena()[node.0].last_child().map(XmlNode)
    }

    pub fn next_sibling(&self, node: XmlNode) -> Option<XmlNode> {
        self.arena()[node.0].next_sibling().map(XmlNode)
    }

    pub fn previous_sibling(&self, node: XmlNode) -> Option<XmlNode> {
        self.arena()[node.0].previous_sibling().map(XmlNode)
    }

    pub fn ancestors(&self, node: XmlNode) -> impl Iterator<Item = XmlNode> + '_ {
        node.0.ancestors(self.arena()).map(XmlNode)
    }

    pub fn children(&self, node: XmlNode) -> impl Iterator<Item = XmlNode> + '_ {
        node.0.children(self.arena()).map(XmlNode)
    }

    pub fn reverse_children(&self, node: XmlNode) -> impl Iterator<Item = XmlNode> + '_ {
        node.0.reverse_children(self.arena()).map(XmlNode)
    }

    pub fn descendants(&self, node: XmlNode) -> impl Iterator<Item = XmlNode> + '_ {
        node.0.descendants(self.arena()).map(XmlNode)
    }

    pub fn following_siblings(&self, node: XmlNode) -> impl Iterator<Item = XmlNode> + '_ {
        node.0.following_siblings(self.arena()).map(XmlNode)
    }

    pub fn preceding_siblings(&self, node: XmlNode) -> impl Iterator<Item = XmlNode> + '_ {
        node.0.preceding_siblings(self.arena()).map(XmlNode)
    }

    pub fn is_removed(&self, node: XmlNode) -> bool {
        self.arena()[node.0].is_removed()
    }

    pub fn traverse(&self, node: XmlNode) -> impl Iterator<Item = XmlNodeEdge> + '_ {
        node.0.traverse(self.arena()).map(|edge| match edge {
            NodeEdge::Start(node_id) => XmlNodeEdge::Start(XmlNode(node_id)),
            NodeEdge::End(node_id) => XmlNodeEdge::End(XmlNode(node_id)),
        })
    }

    pub fn reverse_traverse(&self, node: XmlNode) -> impl Iterator<Item = XmlNodeEdge> + '_ {
        node.0
            .reverse_traverse(self.arena())
            .map(|edge| match edge {
                NodeEdge::Start(node_id) => XmlNodeEdge::Start(XmlNode(node_id)),
                NodeEdge::End(node_id) => XmlNodeEdge::End(XmlNode(node_id)),
            })
    }

    pub fn text(&self, node: XmlNode) -> Option<&Text> {
        let xml_node = self.xml_value(node);
        if let XmlValue::Text(text) = xml_node {
            Some(text)
        } else {
            None
        }
    }

    pub fn text_str(&self, node: XmlNode) -> Option<&str> {
        self.text(node).map(|n| n.get())
    }

    pub fn text_mut(&mut self, node: XmlNode) -> Option<&mut Text> {
        let xml_node = self.xml_value_mut(node);
        if let XmlValue::Text(text) = xml_node {
            Some(text)
        } else {
            None
        }
    }

    pub fn element(&self, node: XmlNode) -> Option<&Element> {
        let xml_node = self.xml_value(node);
        if let XmlValue::Element(element) = xml_node {
            Some(element)
        } else {
            None
        }
    }

    pub fn element_mut(&mut self, node: XmlNode) -> Option<&mut Element> {
        let xml_node = self.xml_value_mut(node);
        if let XmlValue::Element(element) = xml_node {
            Some(element)
        } else {
            None
        }
    }

    pub fn value_type(&self, node: XmlNode) -> ValueType {
        self.xml_value(node).value_type()
    }

    pub fn is_under_root(&self, node: XmlNode) -> bool {
        if let Some(parent_id) = self.parent(node) {
            self.value_type(parent_id) == ValueType::Root
        } else {
            false
        }
    }

    pub fn is_root(&self, node: XmlNode) -> bool {
        self.value_type(node) == ValueType::Root
    }

    pub fn is_element(&self, node: XmlNode) -> bool {
        self.value_type(node) == ValueType::Element
    }

    pub fn is_text(&self, node: XmlNode) -> bool {
        self.value_type(node) == ValueType::Text
    }

    pub fn is_comment(&self, node: XmlNode) -> bool {
        self.value_type(node) == ValueType::Comment
    }

    pub fn is_processing_instruction(&self, node: XmlNode) -> bool {
        self.value_type(node) == ValueType::ProcessingInstruction
    }

    // name & namespace
    pub fn name(&self, name: &str) -> Option<NameId> {
        self.name_ns(name, self.no_namespace_id)
    }

    pub fn name_mut(&mut self, name: &str) -> NameId {
        self.name_ns_mut(name, self.no_namespace_id)
    }

    pub fn name_ns(&self, name: &str, namespace_id: NamespaceId) -> Option<NameId> {
        self.name_lookup
            .get_id(Name::new(name.to_string(), namespace_id))
    }

    pub fn name_ns_mut(&mut self, name: &str, namespace_id: NamespaceId) -> NameId {
        self.name_lookup
            .get_id_mut(Name::new(name.to_string(), namespace_id))
    }

    pub fn namespace(&self, namespace: &str) -> Option<NamespaceId> {
        self.namespace_lookup
            .get_id(Namespace::new(namespace.to_string()))
    }

    pub fn namespace_mut(&mut self, namespace: &str) -> NamespaceId {
        self.namespace_lookup
            .get_id_mut(Namespace::new(namespace.to_string()))
    }
}

impl Default for XmlData {
    fn default() -> Self {
        Self::new()
    }
}
