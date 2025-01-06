use frame_support::traits::Currency;
use frame_system::pallet_prelude::BlockNumberFor;
use pallet_nfts::{CollectionConfig, Config};

use super::*;

pub(super) type BalanceOf<T, I = ()> =
    <<T as Config<I>>::Currency as Currency<<T as SystemConfig>::AccountId>>::Balance;

pub(super) type CollectionConfigFor<T, I = ()> =
    CollectionConfig<BalanceOf<T, I>, BlockNumberFor<T>, <T as Config<I>>::CollectionId>;

pub(super) type DepositBalanceOf<T, I = ()> =
    <<T as Config<I>>::Currency as Currency<<T as SystemConfig>::AccountId>>::Balance;

pub(super) type ItemPrice<T, I = ()> = BalanceOf<T, I>;
