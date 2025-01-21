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

	mint {
        let caller: T::AccountId = whitelisted_caller();

        let collection = T::Helper::collection(0);
        let item = T::Helper::item(0);

        // Create collection with basic settings
        let collection_config = CollectionConfig {
            settings: CollectionSettings::default(),  // Just use default settings
            max_supply: None,
            mint_settings: Default::default(),
        };

        // Create the collection first
        assert_ok!(Pallet::<T, I>::create(
            RawOrigin::Signed(caller.clone()).into(),
            T::Lookup::unlookup(caller.clone()),
            collection_config
        ));

    }: _(RawOrigin::Signed(caller.clone()), collection, item, T::Lookup::unlookup(caller.clone()), None)
    verify {
        assert!(NftAccounts::<T, I>::contains_key((collection, item)));
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

		assert_ok!(Pallet::<T, I>::create(
			RawOrigin::Signed(caller.clone()).into(),
			T::Lookup::unlookup(caller.clone()),
			collection_config
		));

		assert_ok!(Pallet::<T, I>::mint(
			RawOrigin::Signed(caller.clone()).into(),
			collection,
			item,
			T::Lookup::unlookup(caller.clone()),
			None
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
