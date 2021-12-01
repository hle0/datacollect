use kuchiki::NodeRef;

/// An `itemscope` as per the [schema.org] specification.
///
/// [schema.org]: https://schema.org/
pub struct Scope {
    node: NodeRef,
}

impl From<NodeRef> for Scope {
    fn from(node: NodeRef) -> Self {
        Self { node }
    }
}

impl Scope {
    pub fn find(node: NodeRef, item_type: &str) -> Option<Self> {
        Self::from(node).select_type(item_type)
    }

    /// Gets the value of a given [`NodeRef`]'s DOM attribute (given by `key`), if it exists.
    fn get_node_property(node: &NodeRef, key: &'static str) -> Option<String> {
        node.as_element()
            .and_then(|e| e.attributes.borrow().get(key).map(|s| s.to_string()))
    }

    /// Checks whether a given [`NodeRef`] has a DOM attribute `key` which equals `value`.
    fn node_property_eq(node: &NodeRef, key: &'static str, value: &str) -> bool {
        Self::get_node_property(node, key)
            .filter(|s| s.as_str() == value)
            .is_some()
    }

    /// Select all descendant [`NodeRef`]'s where an attribute (given by `key`) exists
    /// and equals `value`.
    fn select_nodes_by_property_and_value<'x>(
        &self,
        key: &'static str,
        value: &'x str,
    ) -> impl Iterator<Item = NodeRef> + 'x {
        self.node
            .descendants()
            .filter(move |d| Self::node_property_eq(d, key, value))
    }

    /// Get an [`Iterator`] of descendant [`Scope`]'s where the `itemtype` attribute equals `item_type`.
    ///
    /// Note that these are descendant scopes, not just child scopes - children of children (and so on)
    /// are included in the returned [`Iterator`].
    pub fn select_types<'x>(&self, item_type: &'x str) -> impl Iterator<Item = Self> + 'x {
        self.select_nodes_by_property_and_value("itemtype", item_type)
            .map(Self::from)
    }

    /// Get the first descendant [`Scope`] where the `itemtype` attribute equals `item_type`.
    pub fn select_type(&self, item_type: &str) -> Option<Self> {
        self.select_types(item_type).next()
    }

    /// Get an [`Iterator`] of descendant [`Scope`]'s where the `itemprop` attribute equals `prop`.
    ///
    /// Note that these are descendant scopes, not just child scopes - children of children (and so on)
    /// are included in the returned [`Iterator`].
    pub fn select_props<'x>(&self, prop: &'x str) -> impl Iterator<Item = Self> + 'x {
        self.select_nodes_by_property_and_value("itemprop", prop)
            .map(Self::from)
    }

    /// Get the first descendant [`Scope`] where the `itemprop` attribute equals `prop`.
    pub fn select_prop(&self, prop: &str) -> Option<Self> {
        self.select_props(prop).next()
    }

    /// Get an [`Iterator`] of the values of descendants where the `itemprop` attribute equals `prop`.
    ///
    /// This is equivalent to the `content` attribute if it exists, otherwise the concatenated text contents of the node.
    ///
    /// Note that these are descendant values, not just child values - values of children of children (and so on)
    /// are included in the returned [`Iterator`].
    pub fn get_values<'x>(&self, prop: &'x str) -> impl Iterator<Item = String> + 'x {
        self.select_nodes_by_property_and_value("itemprop", prop)
            .map(|n| Self::get_node_property(&n, "content").unwrap_or_else(|| n.text_contents()))
    }

    /// Get the value of the first descendant where the `itemprop` attribute equals `prop`.
    ///
    /// This is equivalent to the `content` attribute if it exists, otherwise the concatenated text contents of the node.
    pub fn get_value(&self, prop: &str) -> Option<String> {
        self.get_values(prop).next()
    }
}

#[cfg(test)]
mod tests {
    use super::Scope;
    use kuchiki::{parse_html, traits::TendrilSink};

    #[test]
    fn do_tests() {
        let node = parse_html().one(r#"
            <html>
                <head></head>
                <body>
                    <!-- from https://schema.org/docs/gs.html -->
                    <div itemscope itemtype="https://schema.org/Offer">
                        <span itemprop="name">Blend-O-Matic</span>
                        <span itemprop="price">$19.95</span>
                        <div itemprop="reviews" itemscope itemtype="https://schema.org/AggregateRating">
                            <img src="four-stars.jpg" />
                            <meta itemprop="ratingValue" content="4" />
                            <meta itemprop="bestRating" content="5" />
                            Based on <span itemprop="ratingCount">25</span> user ratings
                        </div>
                    </div>
                </body>
            </html>
        "#);

        let scope = Scope::find(node, "https://schema.org/Offer").unwrap();

        assert_eq!(scope.get_value("name").unwrap(), "Blend-O-Matic");
        assert_eq!(scope.get_value("price").unwrap(), "$19.95");

        let inner_scope = scope.select_prop("reviews").unwrap();
        assert_eq!(
            inner_scope
                .get_value("ratingValue")
                .unwrap()
                .parse::<u32>()
                .unwrap(),
            4
        );
        assert_eq!(
            inner_scope
                .get_value("bestRating")
                .unwrap()
                .parse::<u32>()
                .unwrap(),
            5
        );
        assert_eq!(
            inner_scope
                .get_value("ratingCount")
                .unwrap()
                .parse::<u32>()
                .unwrap(),
            25
        );
    }
}
