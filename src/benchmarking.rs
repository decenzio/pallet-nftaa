#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_support::traits::Get;
use frame_system::RawOrigin;
use pallet_nfts::{CollectionConfig, Config as NftsConfig};
use sp_runtime::traits::StaticLookup;

benchmarks! {
    create_nftaa {
        let caller: T::AccountId = whitelisted_caller();
        let collection: T::CollectionId = Zero::zero();
        let item: T::ItemId = Zero::zero();

        // Create collection
        let config = CollectionConfig {
            settings: Default::default(),
            max_supply: None,
            mint_settings: Default::default(),
        };
        pallet_nfts::Pallet::<T, I>::create(
            RawOrigin::Signed(caller.clone()).into(),
            T::Lookup::unlookup(caller.clone()),
            config,
        )?;

        // Mint NFT
        pallet_nfts::Pallet::<T, I>::mint(
            RawOrigin::Signed(caller.clone()).into(),
            collection,
            item,
            T::Lookup::unlookup(caller.clone()),
            None,
        )?;

    }: _(RawOrigin::Signed(caller), collection, item)

    transfer_nftaa {
        let caller: T::AccountId = whitelisted_caller();
        let recipient: T::AccountId = account("recipient", 0, 0);
        let collection: T::CollectionId = Zero::zero();
        let item: T::ItemId = Zero::zero();

        // Setup: Create collection and mint NFT
        let config = CollectionConfig {
            settings: Default::default(),
            max_supply: None,
            mint_settings: Default::default(),
        };
        pallet_nfts::Pallet::<T, I>::create(
            RawOrigin::Signed(caller.clone()).into(),
            T::Lookup::unlookup(caller.clone()),
            config,
        )?;
        pallet_nfts::Pallet::<T, I>::mint(
            RawOrigin::Signed(caller.clone()).into(),
            collection,
            item,
            T::Lookup::unlookup(caller.clone()),
            None,
        )?;

        // Create NFTAA
        Pallet::<T, I>::create_nftaa(
            RawOrigin::Signed(caller.clone()).into(),
            collection,
            item,
        )?;

    }: _(RawOrigin::Signed(caller), collection, item, T::Lookup::unlookup(recipient))
}

impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test,);
