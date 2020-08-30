mod error;

use dusk_jubjub::{GENERATOR_EXTENDED, AffinePoint, ExtendedPoint, Fr};
use dusk_bls12_381::Scalar;
use poseidon252::sponge::sponge::sponge_hash;
use subtle::ConstantTimeEq;
use crate::error::Error;

pub struct Message(Scalar);

/// An EdDSA secret key, consisting of two JubJub scalars.
pub struct SecretKey {
    p1: Fr,
    p2: Fr,
}

impl SecretKey {
    /// This will create a new [`SecretKey`] from a scalar 
    /// of the Field Fr.
    pub fn new() -> Result<SecretKey, Error> {
        let scalar = Fr::random(&mut rand::thread_rng());
        if scalar.ct_eq(&Fr::zero()).unwrap_u8() == 1u8 {
            return Err(Error::InvalidParameters);
        }

        let p1_bls = sponge_hash(&[scalar.into(), Scalar::zero()]);
        let p2_bls = sponge_hash(&[scalar.into(), Scalar::one()]);
        let p1 = Fr::from_raw(*p1_bls.reduce().internal_repr());
        let p2 = Fr::from_raw(*p2_bls.reduce().internal_repr());

        Ok(SecretKey{
            p1,
            p2,
        })
    }

    /// Returns the [`PublicKey`] of the [`SecretKey`].
    pub fn to_public(&self) -> PublicKey {
        let point = AffinePoint::from(GENERATOR_EXTENDED * &self.p1);
        PublicKey(point)
    }

    /// Sign a [`Message`] with the [`SecretKey`], outputting a [`Signature`].
    #[allow(non_snake_case)]
    pub fn sign(&self, m: &Message) -> Signature {
        let pk = PublicKey::from_secret(self);

        let r = sponge_hash(&[self.p2.into(), m.0]);
        let r_j = Fr::from_raw(*r.reduce().internal_repr());

        let R = AffinePoint::from(GENERATOR_EXTENDED * r_j);

        let h = sponge_hash(&[R.get_x(), R.get_y(), pk.0.get_x(), pk.0.get_y(), m.0]);
        let h_j = Fr::from_raw(*h.reduce().internal_repr());
        let h_pk = h_j * self.p1;
        let s = h_pk + r_j;
        
        Signature{s, R}
    }
}

/// An EdDSA public key, internally represented by a point
/// on the JubJub curve.
pub struct PublicKey(AffinePoint);

impl From<&SecretKey> for PublicKey {
    fn from(sk: &SecretKey) -> Self {
        PublicKey::from_secret(sk)
    }
}

impl PublicKey {
    /// This will create a new [`PublicKey`] from a [`SecretKey`].
    pub fn from_secret(secret: &SecretKey) -> PublicKey {
        let point = AffinePoint::from(GENERATOR_EXTENDED * secret.p1);
        PublicKey(point)
    }

    /// This creates a new random [`PublicKey`].
    /// Note that this function does not return the [`SecretKey`]
    /// associated to this public key.
    pub fn new() -> Result<PublicKey, Error> {
        let sk = SecretKey::new();
        let pk = SecretKey::to_public(&sk.unwrap());
        Ok(pk)
    }
}

/// A [`KeyPair`] contains both the [`SecretKey`] and the
/// associated [`PublicKey`].
pub struct KeyPair {
    pub secret_key: SecretKey,
    pub public_key: PublicKey,
}

impl KeyPair {
    pub fn new() -> Result<Self, Error> {
        let sk = SecretKey::new()?;
        let pk = SecretKey::to_public(&sk);
        
        Ok(KeyPair {
            secret_key: sk,
            public_key: pk,
        })
    }

    pub fn new_from_secret(sk: SecretKey) -> Self {
        let pk = SecretKey::to_public(&sk);
        
        KeyPair {
            secret_key: sk,
            public_key: pk,
        }
    }

    pub fn sign(&self, m: &Message) -> Signature {
        self.secret_key.sign(m)
    }
}

/// An EdDSA signature, produced by signing a [`Message`] with a
/// [`SecretKey`].
#[allow(non_snake_case)]
pub struct Signature {
    s: Fr,
    R: AffinePoint,
}

impl Signature {
    /// Verify the correctness of a [`Signature`], given a [`Message`]
    /// and a [`PublicKey`].
    pub fn verify(&self, m: &Message, pk: &PublicKey) -> bool {
        let h = sponge_hash(&[self.R.get_x(), self.R.get_y(), pk.0.get_y(), pk.0.get_x(), m.0]);
        let h_j = Fr::from_raw(*h.reduce().internal_repr());
        let p1 = GENERATOR_EXTENDED * self.s;
        let h_pk = AffinePoint::from(ExtendedPoint::from(pk.0) * h_j);
        let p2 = ExtendedPoint::from(self.R) + ExtendedPoint::from(h_pk);

        p1.eq(&p2)
    }
}

#[cfg(test)]
mod integrations {
    use super::*;

    #[test]
    // TestSignVerify
    fn sign_verify() {  
        let keypair = KeyPair::new().unwrap();
        let mut rng = rand::thread_rng();

        let message = Message(Scalar::random(&mut rng));
    
        let a = keypair.sign(&message);
        let b = a.verify(&message, &keypair.public_key);

        assert!(b);
    }

    #[test]
    fn test_wrong_keys() {
        let keypair = KeyPair::new().unwrap();
        let mut rng = rand::thread_rng();

        let message = Message(Scalar::random(&mut rng));
    
        let a = keypair.sign(&message);
        let b = a.verify(&message, &PublicKey::new().unwrap());

        assert!(!b);
    }
}
