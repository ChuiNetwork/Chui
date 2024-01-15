#![feature(test)]

extern crate test;

use {
    bincode::serialize,
    solana_runtime::status_cache::*,
    solana_sdk::{
        hash::{hash, Hash},
        signature::Signature,
    },
    test::Bencher,
};

type BankStatusCache = StatusCache<()>;

#[bench]
fn test_statuscache_serialize(bencher: &mut Bencher) {
    let mut status_cache = BankStatusCache::default();
    status_cache.add_root(0);
    status_cache.clear();
    for hash_index in 0..100 {
        let blockhash = Hash::new(&vec![hash_index; std::mem::size_of::<Hash>()]);
        let mut id = blockhash;
        for _ in 0..100 {
            id = hash(id.as_ref());
            let mut sigbytes = Vec::from(id.as_ref());
            id = hash(id.as_ref());
            sigbytes.extend(id.as_ref());
            let sig = Signature::new(&sigbytes);
            status_cache.insert(&blockhash, &sig, 0, ());
        }
    }
    bencher.iter(|| {
        let _ = serialize(&status_cache.slot_deltas(&[0])).unwrap();
    });
}
