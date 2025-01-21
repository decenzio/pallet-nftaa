use crate::{mock::*, Error, Event};
use frame_support::{assert_noop, assert_ok};

#[test]
fn create_nftaa_works() {
	new_test_ext().execute_with(|| {
		// Create a collection first
		assert_ok!(NFTAA::create(
			RuntimeOrigin::signed(account(1)),
			account(1),
			default_collection_config()
		));
		// Mint an NFT
		assert_ok!(NFTAA::mint(RuntimeOrigin::signed(account(1)), 0, 0, account(1), None));

		// Verify NFTAA was created
		assert!(NFTAA::nft_accounts((0, 0)).is_some());

		// Event should be emitted
		System::assert_has_event(
			Event::NFTAACreated {
				collection: 0,
				item: 0,
				nft_account: NFTAA::nft_accounts((0, 0)).unwrap()
			}
			.into(),
		);
	});
}

#[test]
fn create_nftaa_fails_if_not_owner() {
	new_test_ext().execute_with(|| {
		// Create collection and mint NFT
		assert_ok!(NFTAA::create(
			RuntimeOrigin::signed(account(1)),
			account(1),
			default_collection_config()
		));

		// Should fail when non-owner tries to create NFTAA
		assert_noop!(NFTAA::mint(RuntimeOrigin::signed(account(2)), 0, 0, account(1), None),
			pallet_nfts::Error::<Test>::NoPermission
		);
	});
}

#[test]
fn create_nftaa_fails_if_already_exists() {
	new_test_ext().execute_with(|| {
		// Setup
		assert_ok!(NFTAA::create(
			RuntimeOrigin::signed(account(1)),
			account(1),
			default_collection_config()
		));
		assert_ok!(NFTAA::mint(RuntimeOrigin::signed(account(1)), 0, 0, account(1), None));

		// Should fail on second attempt
		assert_noop!(
			NFTAA::mint(RuntimeOrigin::signed(account(1)), 0, 0, account(1), None),
			Error::<Test>::NFTAAAlreadyExists
		);
	});
}


#[test]
fn proxy_call_works() {
	new_test_ext().execute_with(|| {
		// Setup
		assert_ok!(NFTAA::create(
			RuntimeOrigin::signed(account(1)),
			account(1),
			default_collection_config()
		));
		assert_ok!(NFTAA::mint(RuntimeOrigin::signed(account(1)), 0, 0, account(1), None));

		// Create a test call (e.g., system remark)
		let call =
			Box::new(RuntimeCall::System(frame_system::Call::remark { remark: vec![1, 2, 3] }));

		// Execute proxy call
		assert_ok!(NFTAA::proxy_call(RuntimeOrigin::signed(account(1)), 0, 0, call));

		// Event should be emitted
		System::assert_has_event(
			Event::ProxyExecuted { collection: 0, item: 0, result: Ok(()) }.into(),
		);
	});
}

#[test]
fn proxy_call_fails_if_nftaa_listed() {
	new_test_ext().execute_with(|| {
		// Setup
		assert_ok!(NFTAA::create(
			RuntimeOrigin::signed(account(1)),
			account(1),
			default_collection_config()
		));
		assert_ok!(NFTAA::mint(RuntimeOrigin::signed(account(1)), 0, 0, account(1), None));

		// List the NFT for sale
		assert_ok!(NFTAA::set_price(RuntimeOrigin::signed(account(1)), 0, 0, Some(1000), None));

		// Try to execute proxy call
		let call =
			Box::new(RuntimeCall::System(frame_system::Call::remark { remark: vec![1, 2, 3] }));

		assert_noop!(
			NFTAA::proxy_call(RuntimeOrigin::signed(account(1)), 0, 0, call),
			Error::<Test>::NFTAAListed
		);
	});
}

#[test]
fn proxy_call_fails_if_not_nftaa_owner() {
	new_test_ext().execute_with(|| {
		// Setup
		assert_ok!(NFTAA::create(
			RuntimeOrigin::signed(account(1)),
			account(1),
			default_collection_config()
		));
		assert_ok!(NFTAA::mint(RuntimeOrigin::signed(account(1)), 0, 0, account(1), None));

		let call =
			Box::new(RuntimeCall::System(frame_system::Call::remark { remark: vec![1, 2, 3] }));

		assert_noop!(
			NFTAA::proxy_call(RuntimeOrigin::signed(account(2)), 0, 0, call),
			Error::<Test>::NotNFTAAOwner
		);
	});
}
