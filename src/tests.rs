use crate::{mock::*, types::CollectionConfigFor};
use frame_support::assert_ok;
use pallet_nfts::{CollectionConfig, CollectionSettings, MintSettings};
use sp_runtime::AccountId32 as AccountId;

type AccountIdOf<Test> = <Test as frame_system::Config>::AccountId;

pub const ALICE: AccountId = AccountId::new([0u8; 32]);

fn account(id: u8) -> AccountIdOf<Test> {
    [id; 32].into()
}

fn collection_config_with_all_settings_enabled() -> CollectionConfigFor<Test> {
    CollectionConfig {
        settings: CollectionSettings::all_enabled(),
        max_supply: None,
        mint_settings: MintSettings::default(),
    }
}

// fn collections() -> Vec<(AccountIdOf<Test>, u32)> {
//     let mut r: Vec<_> = CollectionAccount::<Test>::iter()
//         .map(|x| (x.0, x.1))
//         .collect();
//     r.sort();
//     let mut s: Vec<_> = Collection::<Test>::iter()
//         .map(|x| (x.1.owner, x.0)) // Can't get owner from CollectionAccount as it's private
//         .collect();
//     s.sort();
//     assert_eq!(r, s);
//     r
// }

#[test]
fn it_creates_collection() {
    new_test_ext().execute_with(|| {
        // Dispatch a signed extrinsic.
        assert_ok!(NFTAA::create(
            RuntimeOrigin::signed(ALICE),
            ALICE,
            collection_config_with_all_settings_enabled()
        ));
    });
}
