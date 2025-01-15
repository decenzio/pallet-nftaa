//! Benchmarking setup for pallet-nftaa

use super::*;
use frame_benchmarking::v2::*;
use frame_support::assert_ok;
use frame_system::RawOrigin;
use pallet_nfts::{BenchmarkHelper, CollectionConfig, CollectionSettings};
use sp_runtime::traits::StaticLookup;
use alloc::vec;

benchmarks_instance_pallet! {
	where_clause {
		where <T as Config<I>>::RuntimeCall: From<frame_system::Call<T>>,
		frame_system::Call<T>: Into<<T as Config<I>>::RuntimeCall>
	}

	create_nftaa {
		let caller: T::AccountId = whitelisted_caller();

		let collection = T::Helper::collection(0);
		let item = T::Helper::item(0);

		let collection_config = CollectionConfig {
			settings: CollectionSettings::default(),
			max_supply: None,
			mint_settings: Default::default(),
		};

		assert_ok!(pallet_nfts::Pallet::<T, I>::create(
			RawOrigin::Signed(caller.clone()).into(),
			T::Lookup::unlookup(caller.clone()),
			collection_config
		));

		assert_ok!(pallet_nfts::Pallet::<T, I>::mint(
			RawOrigin::Signed(caller.clone()).into(),
			collection,
			item,
			T::Lookup::unlookup(caller.clone()),
			None
		));
	}: _(RawOrigin::Signed(caller.clone()), collection, item)
	verify {
		assert!(NftAccounts::<T, I>::contains_key((collection, item)));
	}

	transfer_nftaa {
		let caller: T::AccountId = whitelisted_caller();
		let recipient: T::AccountId = account("recipient", 0, 0);

		let collection = T::Helper::collection(0);
		let item = T::Helper::item(0);

		let collection_config = CollectionConfig {
			settings: CollectionSettings::default(),
			max_supply: None,
			mint_settings: Default::default(),
		};

		assert_ok!(pallet_nfts::Pallet::<T, I>::create(
			RawOrigin::Signed(caller.clone()).into(),
			T::Lookup::unlookup(caller.clone()),
			collection_config
		));

		assert_ok!(pallet_nfts::Pallet::<T, I>::mint(
			RawOrigin::Signed(caller.clone()).into(),
			collection,
			item,
			T::Lookup::unlookup(caller.clone()),
			None
		));

		assert_ok!(Pallet::<T, I>::create_nftaa(
			RawOrigin::Signed(caller.clone()).into(),
			collection,
			item
		));
	}: _(RawOrigin::Signed(caller), collection, item, T::Lookup::unlookup(recipient.clone()))
	verify {
		assert_eq!(
			pallet_nfts::Pallet::<T, I>::owner(collection, item),
			Some(recipient)
		);
	}

	proxy_call {
		let caller: T::AccountId = whitelisted_caller();
		let collection = T::Helper::collection(0);
		let item = T::Helper::item(0);

		let collection_config = CollectionConfig {
			settings: CollectionSettings::default(),
			max_supply: None,
			mint_settings: Default::default(),
		};

		assert_ok!(pallet_nfts::Pallet::<T, I>::create(
			RawOrigin::Signed(caller.clone()).into(),
			T::Lookup::unlookup(caller.clone()),
			collection_config
		));

		assert_ok!(pallet_nfts::Pallet::<T, I>::mint(
			RawOrigin::Signed(caller.clone()).into(),
			collection,
			item,
			T::Lookup::unlookup(caller.clone()),
			None
		));

		assert_ok!(Pallet::<T, I>::create_nftaa(
			RawOrigin::Signed(caller.clone()).into(),
			collection,
			item
		));

		// Create a dummy call to use in the proxy
		let dummy_call: <T as Config<I>>::RuntimeCall = frame_system::Call::<T>::remark {
			remark: vec![1, 2, 3].try_into().unwrap()
		}.into();
	}: _(RawOrigin::Signed(caller), collection, item, Box::new(dummy_call))
	verify {
		// We could add verification if needed
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
