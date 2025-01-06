
#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use core::marker::PhantomData;

/// Weight functions needed for pallet_template.
pub trait NftaaWeightInfo {
    fn create_nftaa() -> Weight;
    fn transfer_nftaa() -> Weight;
}

/// Weights for pallet_template using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> NftaaWeightInfo for SubstrateWeight<T> {
    fn create_nftaa() -> Weight {
        // TODO: Replace with actual benchmarked values
        Weight::from_parts(10_000, 0)
            .saturating_add(T::DbWeight::get().reads(1))
            .saturating_add(T::DbWeight::get().writes(1))
    }

    fn transfer_nftaa() -> Weight {
        // TODO: Replace with actual benchmarked values
        Weight::from_parts(10_000, 0)
            .saturating_add(T::DbWeight::get().reads(2))
            .saturating_add(T::DbWeight::get().writes(2))
    }
}

// For backwards compatibility and tests
impl NftaaWeightInfo for () {
    fn create_nftaa() -> Weight {
        Weight::from_parts(10_000, 0)
    }

    fn transfer_nftaa() -> Weight {
        Weight::from_parts(10_000, 0)
    }
}
