use super::*;

pub(super) trait Builder {
    fn with_parent(self, parent: Parent) -> Tree;
}

impl<T: FnOnce(Parent) -> Tree> Builder for T {
    fn with_parent(self, parent: Parent) -> Tree {
        self(parent)
    }
}

fn leaf(commitment: Insert<Commitment>) -> impl Builder {
    move |parent| Tree {
        inner: Arc::new(Wrapped::with_parent_and_height(
            parent,
            0,
            Inner::Leaf(Leaf { commitment }),
        )),
    }
}

fn node(height: u8, children: impl IntoIterator<Item = impl Builder>) -> impl Builder {
    move |parent| Tree {
        inner: Arc::new_cyclic(|this| {
            let children: Vec<_> = children
                .into_iter()
                .map(|child| child.with_parent(this.clone()))
                .collect();
            assert!(
                !children.is_empty() && children.len() <= 4,
                "nodes must have between 1 and 4 children"
            );

            Wrapped::with_parent_and_height(parent, height, Inner::Node(Node { children }))
        }),
    }
}

mod tier {
    use super::*;

    pub(super) fn empty(parent: Parent, height: u8) -> Tree {
        Tree {
            inner: Arc::new(Wrapped::with_parent_and_height(
                parent,
                height,
                Inner::Tier(Tier { root: None }),
            )),
        }
    }

    fn non_empty(parent: Parent, height: u8, contents: Insert<impl Builder>) -> Tree {
        Tree {
            inner: Arc::new_cyclic(|this| {
                Wrapped::with_parent_and_height(
                    parent,
                    height,
                    Inner::Tier(Tier {
                        root: Some(contents.map(|contents| contents.with_parent(this.clone()))),
                    }),
                )
            }),
        }
    }

    pub(super) fn containing(parent: Parent, height: u8, contents: impl Builder) -> Tree {
        non_empty(parent, height, Insert::Keep(contents))
    }

    pub(super) fn hashed(parent: Parent, height: u8, hash: Hash) -> Tree {
        non_empty(parent, height, Insert::Hash::<fn(Parent) -> Tree>(hash))
    }
}

fn tier(base_height: u8, level_0: Insert<impl IntoIterator<Item = impl Builder>>) -> impl Builder {
    fn level(height: u8, mut level: List<impl Builder>) -> List<impl Builder> {
        let mut next_level = List::new();

        while !level.is_empty() {
            let mut children = List::new();

            for _ in 0..4 {
                if let Some(child) = level.pop_front() {
                    children.push_back(child);
                }
            }

            // We always keep nodes during construction; pruning unneeded nodes is only possible
            // after hashes are computed, but this can't be done until the full tree is constructed.
            next_level.push_back(node(height + 1, children));
        }

        next_level
    }

    move |parent| {
        let level_0 = match level_0 {
            Insert::Hash(hash) => return tier::hashed(parent, base_height + 8, hash),
            Insert::Keep(level_0) => level_0,
        };

        let level_0: List<_> = level_0.into_iter().collect();

        if level_0.is_empty() {
            return tier::empty(parent, base_height + 8);
        }

        let level_1 = level(base_height, level_0);
        let level_2 = level(base_height + 1, level_1);
        let level_3 = level(base_height + 2, level_2);
        let level_4 = level(base_height + 3, level_3);
        let level_5 = level(base_height + 4, level_4);
        let level_6 = level(base_height + 5, level_5);
        let level_7 = level(base_height + 6, level_6);
        let level_8 = level(base_height + 7, level_7);

        assert_eq!(
            level_8.len(),
            1,
            "tiers must contain less than 4^8 elements"
        );

        let root = level_8.into_iter().next().unwrap();

        tier::containing(parent, base_height + 8, root)
    }
}

pub(super) fn block(block: Insert<impl IntoIterator<Item = Insert<Commitment>>>) -> impl Builder {
    tier(0, block.map(|leaves| leaves.into_iter().map(self::leaf)))
}

pub(super) fn epoch(
    epoch: Insert<impl IntoIterator<Item = Insert<impl IntoIterator<Item = Insert<Commitment>>>>>,
) -> impl Builder {
    tier(8, epoch.map(|blocks| blocks.into_iter().map(self::block)))
}

pub(super) fn eternity(
    eternity: impl IntoIterator<
        Item = Insert<
            impl IntoIterator<Item = Insert<impl IntoIterator<Item = Insert<Commitment>>>>,
        >,
    >,
) -> impl Builder {
    tier(16, Insert::Keep(eternity.into_iter().map(self::epoch)))
}
