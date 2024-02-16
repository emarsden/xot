use std::borrow::Cow;

use crate::id::{NameId, NamespaceId, PrefixId};
use crate::xotdata::Xot;
use crate::{Error, Node};

use super::state::XmlNameState;

pub trait NameIdInfo {
    /// Access the underlying name id
    fn name_id(&self) -> NameId;

    /// Access the underlying namespace id
    fn namespace_id(&self) -> NamespaceId;

    /// Access the prefix id in this context.
    fn prefix_id(&self) -> Result<PrefixId, Error>;
}

pub trait Lookup {
    fn prefix_id_for_namespace_id(&self, namespace_id: NamespaceId) -> Option<PrefixId>;
    fn namespace_id_for_prefix_id(&self, prefix_id: PrefixId) -> Option<NamespaceId>;
}

/// A structure that helps you access names in a Xot tree.
///
/// Has a reference to Xot and a node so that name and prefix information can be
/// retrieved.
#[derive(Debug, Clone)]
pub struct XmlNameRef<'a, L: Lookup> {
    /// Looking up string information for names, namespaces and prefixes.
    xot: &'a Xot,
    // A way to look up prefix information.
    lookup: L,
    // This identifies the name and namespace. This is the only thing that
    // identifies the xml name and is used for hashing and comparison.
    name_id: NameId,
}

impl<'a, L: Lookup> std::hash::Hash for XmlNameRef<'a, L> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name_id.hash(state);
    }
}

impl<'a, L: Lookup> PartialEq for XmlNameRef<'a, L> {
    fn eq(&self, other: &Self) -> bool {
        self.name_id == other.name_id
    }
}

impl<'a, L: Lookup> Eq for XmlNameRef<'a, L> {}

struct NodeLookup<'a> {
    xot: &'a Xot,
    node: Node,
}

impl<'a> Lookup for NodeLookup<'a> {
    fn prefix_id_for_namespace_id(&self, namespace_id: NamespaceId) -> Option<PrefixId> {
        self.xot.prefix_for_namespace(self.node, namespace_id)
    }
    fn namespace_id_for_prefix_id(&self, prefix_id: PrefixId) -> Option<NamespaceId> {
        self.xot.namespace_for_prefix(self.node, prefix_id)
    }
}

impl<'a, L: Lookup> NameIdInfo for XmlNameRef<'a, L> {
    /// Access the underlying name id
    fn name_id(&self) -> NameId {
        self.name_id
    }

    /// Access the underlying namespace id
    fn namespace_id(&self) -> NamespaceId {
        self.xot.namespace_for_name(self.name_id)
    }

    /// Access the prefix id in this context.
    fn prefix_id(&self) -> Result<PrefixId, Error> {
        self.lookup
            .prefix_id_for_namespace_id(self.namespace_id())
            .ok_or_else(|| Error::MissingPrefix(self.namespace_id()))
    }
}

impl<'a, L: Lookup> XmlNameRef<'a, L> {
    /// Create a new XmlName
    pub fn new(xot: &'a Xot, lookup: L, name_id: NameId) -> Self {
        Self {
            xot,
            lookup,
            name_id,
        }
    }

    /// Create an XmlName from a local name and namespace.
    ///
    /// If namespace is the empty string, the name isn't in a namespace.
    pub fn from_name_ns(xot: &'a mut Xot, lookup: L, local_name: &str, namespace: &str) -> Self {
        let namespace_id = xot.add_namespace(namespace);
        let name_id = xot.add_name_ns(local_name, namespace_id);
        Self {
            xot,
            lookup,
            name_id,
        }
    }

    /// Given prefix, and name, create an XmlName in context
    pub fn from_prefix_name(
        xot: &'a mut Xot,
        lookup: L,
        prefix: &str,
        local_name: &str,
    ) -> Result<Self, Error> {
        let prefix_id = xot.add_prefix(prefix);
        let namespace_id = lookup
            .namespace_id_for_prefix_id(prefix_id)
            .ok_or_else(|| Error::UnknownPrefix(prefix.to_string()))?;
        let name_id = xot.add_name_ns(local_name, namespace_id);
        Ok(Self {
            xot,
            lookup,
            name_id,
        })
    }

    /// Given a fullname (with potentially a prefix), construct an XmlName
    pub fn from_fullname(xot: &'a mut Xot, lookup: L, fullname: &str) -> Result<Self, Error> {
        let (prefix, local_name) = match fullname.find(':') {
            Some(pos) => {
                let (prefix, local_name) = fullname.split_at(pos);
                (prefix, &local_name[1..])
            }
            None => ("", fullname),
        };
        Self::from_prefix_name(xot, lookup, prefix, local_name)
    }

    pub fn to_state(&self) -> Result<XmlNameState, Error> {
        Ok(XmlNameState::new(
            self.name_id,
            self.namespace_id(),
            self.prefix_id()?,
        ))
    }

    /// Get the local name as a str reference.
    pub fn local_name(&self) -> &'a str {
        self.xot.local_name_str(self.name_id)
    }

    /// Get the namespace as a str reference.
    ///
    /// If there is no namespace, this is the empty string.
    pub fn namespace(&self) -> &'a str {
        self.xot.namespace_str(self.namespace_id())
    }

    /// Get the prefix for the name in the context of a node.
    ///
    /// If this is in the default namespace, this is the empty string.
    ///
    /// If the prefix cannot be found, return an [`Error::MissingPrefix`].
    pub fn prefix(&self) -> Result<&'a str, Error> {
        let prefix_id = self.prefix_id()?;
        Ok(self.xot.prefix_str(prefix_id))
    }

    /// Get the full name in the context of a node.
    ///
    /// This may include a prefix.
    pub fn fullname(&self) -> Result<Cow<'a, str>, Error> {
        let prefix = self.prefix()?;
        if !prefix.is_empty() {
            Ok(Cow::Owned(format!("{}:{}", prefix, self.local_name())))
        } else {
            Ok(Cow::Borrowed(self.local_name()))
        }
    }
}
