#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::v1::{
	account, benchmarks_instance_pallet, impl_benchmark_test_suite, whitelisted_caller,
};
use frame_system::RawOrigin;
use pallet_nfts::CollectionConfig;
use sp_runtime::traits::StaticLookup;

const SEED: u32 = 0;

fn assert_last_event<T: Config<I>, I: 'static>(generic_event: <T as Config<I>>::RuntimeEvent) {
	let events = frame_system::Pallet::<T>::events();
	let system_event: <T as frame_system::Config>::RuntimeEvent = generic_event.into();
	// compare to the last event record
	let frame_system::EventRecord { event, .. } = &events[events.len() - 1];
	assert_eq!(event, &system_event);
}

// Helper function to create collection and mint NFT
fn setup_nft<T: Config<I>, I: 'static>(
) -> Result<(T::CollectionId, T::ItemId, T::AccountId), &'static str> {
	let caller: T::AccountId = whitelisted_caller();
	let caller_lookup = T::Lookup::unlookup(caller.clone());

	// Use a numeric collection ID
	let collection = T::CollectionId::from(0u32);
	let item = T::ItemId::from(0u32);

	let config = CollectionConfig {
		settings: Default::default(),
		max_supply: None,
		mint_settings: Default::default(),
	};

	// Create collection
	pallet_nfts::Pallet::<T, I>::create(
		RawOrigin::Signed(caller.clone()).into(),
		caller_lookup.clone(),
		config,
	)?;

	// Mint NFT
	pallet_nfts::Pallet::<T, I>::mint(
		RawOrigin::Signed(caller.clone()).into(),
		collection,
		item,
		caller_lookup,
		None,
	)?;

	Ok((collection, item, caller))
}

benchmarks_instance_pallet! {
	where_clause {
		where
			T: pallet_nfts::Config<I>,
			T::CollectionId: From<u32>,
			T::ItemId: From<u32>,
			T::AccountId: AsRef<[u8]>
	}

	create_nftaa {
		let (collection, item, caller) = setup_nft::<T, I>()?;
	}: _(RawOrigin::Signed(caller.clone()), collection, item)
	verify {
		assert_last_event::<T, I>(Event::NFTAACreated {
			collection,
			item,
			nft_account: Pallet::<T, I>::generate_nft_address(collection, item)
		}.into());
	}

	transfer_nftaa {
		let (collection, item, caller) = setup_nft::<T, I>()?;
		let recipient: T::AccountId = account("recipient", 0, SEED);
		let recipient_lookup = T::Lookup::unlookup(recipient.clone());

		// First create the NFTAA
		Pallet::<T, I>::create_nftaa(
			RawOrigin::Signed(caller.clone()).into(),
			collection,
			item
		)?;
	}: _(RawOrigin::Signed(caller.clone()), collection, item, recipient_lookup)
	verify {
		assert_last_event::<T, I>(Event::NFTAATransferred {
			collection,
			item,
			from: caller,
			to: recipient,
		}.into());
	}
}

impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test,);
