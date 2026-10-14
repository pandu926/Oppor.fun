use crate::error::{ApiError, Result};
use aes_gcm::{
    Aes256Gcm, KeyInit, Nonce,
    aead::{Aead, Payload},
};
use alloy_primitives::{Address, B256, U256, keccak256};
use alloy_sol_types::{SolType, SolValue, sol_data};
use rand::{RngCore, rngs::OsRng};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use zeroize::Zeroizing;

pub fn random32() -> [u8; 32] {
    let mut b = [0; 32];
    OsRng.fill_bytes(&mut b);
    b
}
pub fn canonical<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    serde_jcs::to_vec(value).map_err(|_| ApiError::internal())
}
pub fn json_hash<T: Serialize>(value: &T) -> Result<B256> {
    Ok(keccak256(canonical(value)?))
}
pub fn encrypt_seed(key: &[u8; 32], campaign: B256, seed: &[u8; 32]) -> Result<Vec<u8>> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| ApiError::internal())?;
    let mut nonce = [0; 12];
    OsRng.fill_bytes(&mut nonce);
    let encrypted = cipher
        .encrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: seed,
                aad: campaign.as_slice(),
            },
        )
        .map_err(|_| ApiError::internal())?;
    let mut bytes = nonce.to_vec();
    bytes.extend(encrypted);
    Ok(bytes)
}
pub fn decrypt_seed(
    key: &[u8; 32],
    campaign: B256,
    encrypted: &[u8],
) -> Result<Zeroizing<[u8; 32]>> {
    if encrypted.len() != 60 {
        return Err(ApiError::internal());
    }
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| ApiError::internal())?;
    let plain = Zeroizing::new(
        cipher
            .decrypt(
                Nonce::from_slice(&encrypted[..12]),
                Payload {
                    msg: &encrypted[12..],
                    aad: campaign.as_slice(),
                },
            )
            .map_err(|_| ApiError::internal())?,
    );
    Ok(Zeroizing::new(
        plain
            .as_slice()
            .try_into()
            .map_err(|_| ApiError::internal())?,
    ))
}
fn label(text: &str) -> B256 {
    let mut b = [0; 32];
    b[..text.len()].copy_from_slice(text.as_bytes());
    B256::new(b)
}
pub fn seed_commitment(campaign: B256, seed: B256) -> B256 {
    keccak256((label("OPPOR_RAFFLE_SEED_V1"), campaign, seed).abi_encode())
}
pub fn draw_seed(campaign: B256, snapshot: B256, seed: B256) -> B256 {
    keccak256((label("OPPOR_RAFFLE_DRAW_V1"), campaign, snapshot, seed).abi_encode())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Allocation {
    pub index: String,
    pub recipient: Address,
    pub token_id: String,
    pub quantity: String,
    pub proof: Vec<B256>,
}
pub fn leaf(
    chain: u64,
    escrow: Address,
    campaign: B256,
    kind: u8,
    token: Address,
    allocation: &Allocation,
) -> Result<B256> {
    let tuple = (
        U256::from(chain),
        escrow,
        campaign,
        crate::domain::amount(&allocation.index)?,
        allocation.recipient,
        kind,
        token,
        crate::domain::amount(&allocation.token_id)?,
        crate::domain::positive(&allocation.quantity)?,
    );
    type LeafTuple = (
        sol_data::Uint<256>,
        sol_data::Address,
        sol_data::FixedBytes<32>,
        sol_data::Uint<256>,
        sol_data::Address,
        sol_data::Uint<8>,
        sol_data::Address,
        sol_data::Uint<256>,
        sol_data::Uint<256>,
    );
    Ok(keccak256(keccak256(LeafTuple::abi_encode(&tuple))))
}
fn pair(a: B256, b: B256) -> B256 {
    let mut bytes = [0; 64];
    let (l, r) = if a < b { (a, b) } else { (b, a) };
    bytes[..32].copy_from_slice(l.as_slice());
    bytes[32..].copy_from_slice(r.as_slice());
    keccak256(bytes)
}
pub fn merkle(leaves: &[B256]) -> (B256, Vec<Vec<B256>>) {
    if leaves.is_empty() {
        return (B256::ZERO, vec![]);
    }
    let mut sorted: Vec<_> = leaves.iter().copied().enumerate().collect();
    sorted.sort_by_key(|(_, h)| *h);
    let mut tree = vec![B256::ZERO; 2 * leaves.len() - 1];
    let mut positions = vec![0; leaves.len()];
    for (i, (index, hash)) in sorted.into_iter().enumerate() {
        let pos = tree.len() - 1 - i;
        tree[pos] = hash;
        positions[index] = pos;
    }
    for i in (0..leaves.len() - 1).rev() {
        tree[i] = pair(tree[2 * i + 1], tree[2 * i + 2]);
    }
    let proofs = positions
        .into_iter()
        .map(|mut i| {
            let mut proof = vec![];
            while i > 0 {
                let sibling = if i % 2 == 0 { i - 1 } else { i + 1 };
                proof.push(tree[sibling]);
                i = (i - 1) / 2;
            }
            proof
        })
        .collect();
    (tree[0], proofs)
}
pub fn verify_proof(root: B256, leaf: B256, proof: &[B256]) -> bool {
    proof.iter().fold(leaf, |h, p| pair(h, *p)) == root
}
pub fn raffle(seed: B256, n: usize, k: usize) -> (Vec<usize>, u64) {
    let mut ordinals: Vec<_> = (0..n).collect();
    let mut counter = 0u64;
    for i in 0..k.min(n) {
        let bound = U256::from(n - i);
        let threshold = (U256::MAX % bound + U256::from(1)) % bound;
        let r = loop {
            let x = U256::from_be_bytes(keccak256((seed, U256::from(counter)).abi_encode()).0);
            counter += 1;
            if x >= threshold {
                break usize::try_from(x % bound).expect("Bounded random index");
            }
        };
        ordinals.swap(i, i + r);
    }
    ordinals.truncate(k.min(n));
    (ordinals, counter)
}
pub fn parse_hash(v: &[u8]) -> Result<B256> {
    if v.len() != 32 {
        Err(ApiError::internal())
    } else {
        Ok(B256::from_slice(v))
    }
}
pub fn hash_string(v: Option<&[u8]>) -> Value {
    v.map(|b| Value::String(format!("0x{}", hex::encode(b))))
        .unwrap_or(Value::Null)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn seed_authentication() {
        let k = [2; 32];
        let campaign = B256::repeat_byte(3);
        let seed = [4; 32];
        let e = encrypt_seed(&k, campaign, &seed).unwrap();
        assert_eq!(*decrypt_seed(&k, campaign, &e).unwrap(), seed);
        assert!(decrypt_seed(&k, B256::ZERO, &e).is_err());
        let mut bad = e;
        bad[20] ^= 1;
        assert!(decrypt_seed(&k, campaign, &bad).is_err());
    }
    #[test]
    fn canonical_json_order() {
        assert_eq!(
            canonical(&serde_json::json!({"b":1,"a":2})).unwrap(),
            br#"{"a":2,"b":1}"#
        );
    }
    #[test]
    fn all_merkle_sizes() {
        for n in [0, 1, 2, 3, 5, 100] {
            let leaves: Vec<_> = (0..n).map(|i| keccak256([i as u8])).collect();
            let (root, proofs) = merkle(&leaves);
            for (i, p) in proofs.iter().enumerate() {
                assert!(verify_proof(root, leaves[i], p));
                assert!(!verify_proof(root, keccak256(b"foreign"), p));
            }
        }
    }
    proptest::proptest! {#[test]fn draw_is_unique(n in 0usize..500,k in 0usize..500){let(a,c)=raffle(B256::repeat_byte(7),n,k);let(b,d)=raffle(B256::repeat_byte(7),n,k);proptest::prop_assert_eq!(&a,&b);proptest::prop_assert_eq!(c,d);let mut s=a.clone();s.sort();s.dedup();proptest::prop_assert_eq!(s.len(),k.min(n));proptest::prop_assert!(a.iter().all(|x|*x<n));}}
}
