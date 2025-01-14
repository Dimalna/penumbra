//! The tiered commitment tree for Penumbra.
//!
//! ```ascii,no_run
//! Eternity┃           ╱╲ ◀───────────── Anchor
//!     Tree┃          ╱││╲               = Global Tree Root
//!         ┃         * ** *           ╮
//!         ┃      *   *  *   *        │ 8 levels
//!         ┃   *     *    *     *     ╯
//!         ┃  ╱╲    ╱╲    ╱╲    ╱╲
//!         ┃ ╱││╲  ╱││╲  ╱││╲  ╱││╲ ◀─── Global Tree Leaf
//!                         ▲             = Epoch Root
//!                      ┌──┘
//!                      │
//!                      │
//!    Epoch┃           ╱╲ ◀───────────── Epoch Root
//!     Tree┃          ╱││╲
//!         ┃         * ** *           ╮
//!         ┃      *   *  *   *        │ 8 levels
//!         ┃   *     *    *     *     ╯
//!         ┃  ╱╲    ╱╲    ╱╲    ╱╲
//!         ┃ ╱││╲  ╱││╲  ╱││╲  ╱││╲ ◀─── Epoch Leaf
//!                  ▲                    = Block Root
//!                  └───┐
//!                      │
//!                      │
//!    Block┃           ╱╲ ◀───────────── Block Root
//!     Tree┃          ╱││╲
//!         ┃         * ** *           ╮
//!         ┃      *   *  *   *        │ 8 levels
//!         ┃   *     *    *     *     ╯
//!         ┃  ╱╲    ╱╲    ╱╲    ╱╲
//!         ┃ ╱││╲  ╱││╲  ╱││╲  ╱││╲ ◀─── Block Leaf
//!                                       = Note Commitment
//! ```

// Cargo doc complains if the recursion limit isn't higher, even though cargo build succeeds:
#![recursion_limit = "256"]
#![warn(missing_docs)]

#[macro_use]
extern crate derivative;

#[macro_use]
extern crate serde;

mod error;
mod index;
mod proof;
mod serialize;
mod tree;

pub use proof::Proof;
pub use tree::{Position, Root, Tree};

#[cfg(any(doc, feature = "internal"))]
pub mod internal;
#[cfg(not(any(doc, feature = "internal")))]
mod internal;

pub mod builder {
    //! Builders for individual epochs and blocks within a tree.
    //!
    //! This module is only necessary for constructing trees in parallel; in a single-threaded
    //! context, the methods on [`Tree`](super::Tree) suffice.

    pub mod epoch {
        //! [`Epoch`]s within [`Tree`](super::super::Tree)s, and their [`Root`]s.
        pub use crate::tree::epoch::*;
    }

    pub mod block {
        //! [`Block`]s within [`Epoch`](super::epoch::Epoch)s, and their [`Root`]s.
        pub use crate::tree::epoch::block::*;
    }
}

#[doc(inline)]
pub use crate::internal::{
    path::PathDecodeError,
    proof::{ProofDecodeError, VerifyError},
};

mod prelude {
    pub(crate) use super::{
        index,
        internal::{
            complete::{self, Complete, ForgetOwned},
            frontier::{self, Focus, Forget, Frontier, Full, GetPosition, Insert, Item},
            hash::GetHash,
            hash::{CachedHash, Hash, OptionHash},
            height::{Height, IsHeight, Succ, Zero},
            interface::Witness,
            path::{self, AuthPath, Path, WhichWay},
            three::{Elems, ElemsMut, IntoElems, Three},
        },
        Commitment, Position, Proof, Root, Tree, VerifyError,
    };
}

/// When inserting a [`Commitment`] into a [`Tree`], [`Epoch`], or [`Block`], should we
/// [`Keep`] it to allow it to be witnessed later, or [`Forget`] about it after updating the root
/// hash?
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(any(test, feature = "arbitrary"), derive(proptest_derive::Arbitrary))]
pub enum Witness {
    /// When inserting a [`Commitment`] into a [`Tree`], [`Epoch`], or [`Block`], this flag
    /// indicates that we should keep this commitment to allow it to be witnessed later.
    Keep,
    /// When inserting a [`Commitment`] into a [`Tree`], [`Epoch`], or [`Block`], this flag
    /// indicates that we should immediately forget about it to save space, because we will not want to
    /// witness its presence later.
    Forget,
}

pub use penumbra_crypto::note::Commitment;

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn check_eternity_size() {
        static_assertions::assert_eq_size!(Tree, [u8; 608]);
    }

    #[test]
    fn check_eternity_proof_size() {
        static_assertions::assert_eq_size!(Proof, [u8; 2344]);
    }
}
