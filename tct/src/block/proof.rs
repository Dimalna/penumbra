use penumbra_proto::transparent_proofs as pb;

pub use thiserror::Error;

use crate::{
    internal::{hash, index},
    Commitment, Hash,
};

pub use super::{Block, Position, Root};

/// An as-yet-unverified proof of the inclusion of some [`Commitment`] in a [`Block`].
#[derive(Derivative)]
#[derivative(
    Debug(bound = ""),
    Clone(bound = ""),
    PartialEq(bound = ""),
    Eq(bound = "")
)]
pub struct Proof<Hasher: hash::Hasher>(pub(super) crate::proof::Proof<Block<Hasher>, Hasher>);

impl<Hasher: hash::Hasher> Proof<Hasher> {
    /// Construct a new [`Proof`] of inclusion for a given [`Commitment`], index, and authentication
    /// path from root to leaf.
    pub fn new(
        commitment: Commitment,
        Position(index): Position,
        auth_path: [[Hash<Hasher>; 3]; 8],
    ) -> Self {
        use crate::internal::path::{Leaf, Node};
        let [a, b, c, d, e, f, g, h] = auth_path;
        let path = Leaf;
        let path = Node {
            siblings: h,
            child: path,
        };
        let path = Node {
            siblings: g,
            child: path,
        };
        let path = Node {
            siblings: f,
            child: path,
        };
        let path = Node {
            siblings: e,
            child: path,
        };
        let path = Node {
            siblings: d,
            child: path,
        };
        let path = Node {
            siblings: c,
            child: path,
        };
        let path = Node {
            siblings: b,
            child: path,
        };
        let path = Node {
            siblings: a,
            child: path,
        };
        Self(crate::proof::Proof {
            leaf: commitment,
            position: u16::from(index) as u64,
            auth_path: path,
        })
    }

    /// Verify a [`Proof`] of inclusion against the [`Root`] of an [`Block`].
    ///
    /// # Errors
    ///
    /// Returns [`VerifyError`] if the proof is invalid for that [`Root`].
    pub fn verify(&self, root: Root<Hasher>) -> Result<(), crate::VerifyError<Hasher>> {
        self.0.verify(root.0)
    }

    /// Get the commitment whose inclusion is witnessed by the proof.
    pub fn commitment(&self) -> Commitment {
        self.0.leaf
    }

    /// Get the position of the witnessed commitment.
    pub fn position(&self) -> crate::block::Position {
        crate::epoch::block::Position(index::within::Block::from(self.0.index() as u16))
    }

    /// Get the authentication path for this proof, order from root to leaf.
    pub fn auth_path(&self) -> [&[Hash<Hasher>; 3]; 8] {
        use crate::internal::path::{Leaf, Node};
        let path = self.0.auth_path();
        let Node {
            siblings: a,
            child: path,
        } = path;
        let Node {
            siblings: b,
            child: path,
        } = path;
        let Node {
            siblings: c,
            child: path,
        } = path;
        let Node {
            siblings: d,
            child: path,
        } = path;
        let Node {
            siblings: e,
            child: path,
        } = path;
        let Node {
            siblings: f,
            child: path,
        } = path;
        let Node {
            siblings: g,
            child: path,
        } = path;
        let Node {
            siblings: h,
            child: path,
        } = path;
        let Leaf = path;
        [a, b, c, d, e, f, g, h]
    }
}

impl<Hasher: hash::Hasher> From<Proof<Hasher>> for pb::MerkleProof {
    fn from(proof: Proof<Hasher>) -> Self {
        proof.0.into()
    }
}

impl<Hasher: hash::Hasher> TryFrom<pb::MerkleProof> for Proof<Hasher> {
    type Error = crate::ProofDecodeError;

    fn try_from(value: pb::MerkleProof) -> Result<Self, Self::Error> {
        Ok(Proof(crate::internal::proof::Proof::try_from(value)?))
    }
}

impl<Hasher: hash::Hasher> penumbra_proto::Protobuf<pb::MerkleProof> for Proof<Hasher> {}
