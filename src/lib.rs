#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

extern crate alloc;

use alloc::{boxed::Box, vec::Vec};
use frame_support::dispatch::PostDispatchInfo;
use frame_support::weights::Weight;
use frame_system::Config as SystemConfig;
use sp_runtime::traits::Dispatchable;
use sp_runtime::traits::{Hash, StaticLookup};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod weights;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

/// The log target of this pallet.
pub const LOG_TARGET: &'static str = "runtime::nftaa";

/// A type alias for the account ID type used in the dispatchable functions of this pallet.
type AccountIdLookupOf<T> = <<T as SystemConfig>::Lookup as StaticLookup>::Source;

pub trait WeightInfo {
	fn mint() -> Weight;
	fn proxy_call() -> Weight;
}

impl WeightInfo for () {
	fn mint() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `314`
		//  Estimated: `3623`
		// Minimum execution time: 13_000_000 picoseconds.
		Weight::from_parts(14_000_000, 0)
	}
	fn proxy_call() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `395`
		//  Estimated: `3623`
		// Minimum execution time: 19_000_000 picoseconds.
		Weight::from_parts(20_000, 0)
	}
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{
		dispatch::{extract_actual_weight, GetDispatchInfo},
		pallet_prelude::*,
		traits::{nonfungibles_v2::Trading, OriginTrait},
	};
	use frame_system::pallet_prelude::*;
	use pallet_nfts::{
		AttributeNamespace, BalanceOf, BlockNumberFor, CancelAttributesApprovalWitness,
		CollectionConfigFor, CollectionSettings, DepositBalanceOf, DestroyWitness, ItemConfig,
		ItemPrice, ItemTipOf, MintSettings, MintWitness, PreSignedMintOf, PriceWithDirection,
		WeightInfo as NftsWeightInfo,
	};

	use pallet_utility::WeightInfo as UtilityWeightInfo;

	#[pallet::config]
	pub trait Config<I: 'static = ()>:
		frame_system::Config + pallet_nfts::Config<I> + pallet_utility::Config
	{
		/// Runtime event type for pallet
		type RuntimeEvent: From<Event<Self, I>>
			+ IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// The overarching event type.
		type RuntimeCall: From<Call<Self, I>>
			+ Dispatchable<RuntimeOrigin = Self::RuntimeOrigin, PostInfo = PostDispatchInfo>
			+ Encode
			+ Decode
			+ TypeInfo
			+ Into<<Self as pallet_utility::Config>::RuntimeCall>
			+ core::fmt::Debug
			+ GetDispatchInfo
			+ Clone
			+ Eq
			+ PartialEq
			+ From<pallet_utility::Call<Self>>;
		/// A type representing the weights required by the dispatchables of this pallet.
		type NftaaWeightInfo: WeightInfo;
		type NftsWeightInfo: pallet_nfts::WeightInfo;
	}

	#[pallet::pallet]
	pub struct Pallet<T, I = ()>(PhantomData<(T, I)>);

	#[pallet::storage]
	#[pallet::getter(fn nft_accounts)]
	pub type NftAccounts<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Blake2_128Concat, (T::CollectionId, T::ItemId), T::AccountId, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		/// An NFT was converted to an account
		NFTAACreated { collection: T::CollectionId, item: T::ItemId, nft_account: T::AccountId },
		/// An NFTAA's ownership was transferred
		NFTAATransferred {
			collection: T::CollectionId,
			item: T::ItemId,
			from: T::AccountId,
			to: T::AccountId,
		},
		/// A proxy call was executed through an NFTAA
		ProxyExecuted { collection: T::CollectionId, item: T::ItemId, result: DispatchResult },
	}

	#[pallet::error]
	pub enum Error<T, I = ()> {
		/// The NFT has already been converted to an account
		NFTAAAlreadyExists,
		/// The NFTAA does not exist
		NFTAANotFound,
		/// The NFTAA is currently listed for sale and cannot execute proxy calls
		NFTAAListed,
		/// The caller is not the owner of the NFTAA
		NotNFTAAOwner,
	}
	// Helper functions for implementation
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		/// Check if an NFT is listed for sale
		fn is_nft_listed(collection: T::CollectionId, item: T::ItemId) -> bool {
			// Use item_details instead of get_item
			pallet_nfts::Pallet::<T, I>::item_price(&collection, &item).is_some()
		}

		/// Generate a deterministic address for an NFT
		fn generate_nfta_address(collection: T::CollectionId, item: T::ItemId) -> T::AccountId {
			// Encode the chain ID, collection ID, and item ID
			let mut data = T::SS58Prefix::get().encode();
			data.extend(collection.encode());
			data.extend(item.encode());

			let hash = T::Hashing::hash(&data);
			T::AccountId::decode(&mut &hash.encode()[..])
				.expect("Generated account ID is always valid")
		}

		/// Execute a call through an NFTAA
		pub fn _proxy_call(
			origin: OriginFor<T>,
			collection: T::CollectionId,
			item: T::ItemId,
			call: Box<<T as Config<I>>::RuntimeCall>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin.clone())?;

			// Ensure the NFTAA exists
			ensure!(
				NftAccounts::<T, I>::contains_key((collection, item)),
				Error::<T, I>::NFTAANotFound
			);

			// Verify ownership using the parent pallet
			ensure!(
				pallet_nfts::Pallet::<T, I>::owner(collection, item)
					.map_or(false, |owner| owner == who),
				Error::<T, I>::NotNFTAAOwner
			);

			// Check if the NFT is listed for sale
			ensure!(!Self::is_nft_listed(collection, item), Error::<T, I>::NFTAAListed);

			// Get the NFTAA address
			let nft_account = NftAccounts::<T, I>::get((collection, item))
				.expect("We already checked that the NFTAA exists; qed");

			// Reconstruct logic from pallet_utility::Pallet::as_derivative

			// Change origin to the NFTAA account
			let nft_origin = T::RuntimeOrigin::signed(nft_account);
			let info = call.get_dispatch_info();
			let result = call.dispatch(nft_origin);

			// Always take into account the base weight of this call.
			let mut weight = <T as pallet_utility::Config>::WeightInfo::as_derivative()
				.saturating_add(T::DbWeight::get().reads_writes(1, 1));

			// Add the real weight of the dispatch.
			weight = weight.saturating_add(extract_actual_weight(&result, &info));

			// Emit event with the result
			Self::deposit_event(Event::ProxyExecuted {
				collection,
				item,
				result: result.map(|_| ()).map_err(|e| e.error),
			});

			result
				.map_err(|mut err| {
					err.post_info = Some(weight).into();
					err
				})
				.map(|_| Some(weight).into())
		}

		// Mint an NFTAA
		pub fn _nftaa_mint(
			origin: OriginFor<T>,
			collection: T::CollectionId,
			item: T::ItemId,
			mint_to: AccountIdLookupOf<T>,
			witness_data: Option<MintWitness<T::ItemId, DepositBalanceOf<T, I>>>,
		) -> DispatchResult {
			let _who = ensure_signed(origin.clone())?;
			// Check if the NFTAA already exists
			ensure!(
				!NftAccounts::<T, I>::contains_key((collection, item)),
				Error::<T, I>::NFTAAAlreadyExists
			);

			let nft_account = Self::generate_nfta_address(collection, item);
			pallet_nfts::Pallet::<T, I>::mint(
				origin.clone(),
				collection,
				item,
				mint_to,
				witness_data,
			)?;

			let key =
				pallet_nfts::Pallet::<T, I>::construct_attribute_key(b"nftaa_address".to_vec())?;
			let value =
				pallet_nfts::Pallet::<T, I>::construct_attribute_value(nft_account.encode())?;

			pallet_nfts::Pallet::<T, I>::set_attribute(
				origin.clone(),
				collection,
				Some(item),
				AttributeNamespace::CollectionOwner,
				key,
				value,
			)?;

			// Store the NFTAA
			NftAccounts::<T, I>::insert((collection, item), nft_account.clone());

			// Emit event
			Self::deposit_event(Event::NFTAACreated { collection, item, nft_account });

			Ok(())
		}
	}

	/// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	/// These functions materialize as "extrinsics", which are often compared to transactions.
	/// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	/// <https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/guides/your_first_pallet/index.html#dispatchables>

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		/// Execute a call through an NFTAA
		///
		/// The origin must be Signed and must be the owner of the NFTAA.
		/// The NFTAA must not be listed for sale.
		///
		/// Parameters:
		/// - `collection`: The collection ID of the NFTAA
		/// - `item`: The item ID of the NFTAA
		/// - `call`: The call to be executed
		#[pallet::call_index(0)]
		#[pallet::weight({
        let dispatch_info = call.get_dispatch_info();
        (
            dispatch_info.call_weight.saturating_add(T::NftaaWeightInfo::proxy_call()),
            dispatch_info.class,
            dispatch_info.pays_fee
        )
    })]
		pub fn proxy_call(
			origin: OriginFor<T>,
			collection: T::CollectionId,
			item: T::ItemId,
			call: Box<<T as Config<I>>::RuntimeCall>,
		) -> DispatchResultWithPostInfo {
			Self::_proxy_call(origin, collection, item, call)
		}

		/// Mint an item of a particular collection.
		///
		/// The origin must be Signed and the sender must comply with the `mint_settings` rules.
		///
		/// - `collection`: The collection of the item to be minted.
		/// - `item`: An identifier of the new item.
		/// - `mint_to`: Account into which the item will be minted.
		/// - `witness_data`: When the mint type is `HolderOf(collection_id)`, then the owned
		///   item_id from that collection needs to be provided within the witness data object. If
		///   the mint price is set, then it should be additionally confirmed in the `witness_data`.
		///
		/// Note: the deposit will be taken from the `origin` and not the `owner` of the `item`.
		///
		/// Emits `Issued` event when successful.
		///
		/// Weight: `O(1)`
		#[pallet::call_index(1)]
		#[pallet::weight(T::NftaaWeightInfo::mint())]
		pub fn mint(
			origin: OriginFor<T>,
			collection: T::CollectionId,
			item: T::ItemId,
			mint_to: AccountIdLookupOf<T>,
			witness_data: Option<MintWitness<T::ItemId, DepositBalanceOf<T, I>>>,
		) -> DispatchResult {
			Self::_nftaa_mint(origin, collection, item, mint_to, witness_data)
		}

		/// Issue a new collection of non-fungible items from a public origin.
		///
		/// This new collection has no items initially and its owner is the origin.
		///
		/// The origin must be Signed and the sender must have sufficient funds free.
		///
		/// `CollectionDeposit` funds of sender are reserved.
		///
		/// Parameters:
		/// - `admin`: The admin of this collection. The admin is the initial address of each
		/// member of the collection's admin team.
		///
		/// Emits `Created` event when successful.
		///
		/// Weight: `O(1)`
		#[pallet::call_index(2)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn create(
			origin: OriginFor<T>,
			admin: AccountIdLookupOf<T>,
			config: CollectionConfigFor<T, I>,
		) -> DispatchResult {
			pallet_nfts::Pallet::<T, I>::create(origin, admin, config)
		}

		/// Destroy a collection of fungible items.
		///
		/// The origin must conform to `ForceOrigin` or must be `Signed` and the sender must be the
		/// owner of the `collection`.
		///
		/// NOTE: The collection must have 0 items to be destroyed.
		///
		/// - `collection`: The identifier of the collection to be destroyed.
		/// - `witness`: Information on the items minted in the collection. This must be
		/// correct.
		///
		/// Emits `Destroyed` event when successful.
		///
		/// Weight: `O(m + c + a)` where:
		/// - `m = witness.item_metadatas`
		/// - `c = witness.item_configs`
		/// - `a = witness.attributes`
		#[pallet::call_index(3)]
		#[pallet::weight(T::NftsWeightInfo::destroy(
			witness.item_metadatas,
			witness.item_configs,
			witness.attributes,
		))]
		pub fn destroy(
			origin: OriginFor<T>,
			collection: T::CollectionId,
			witness: DestroyWitness,
		) -> DispatchResultWithPostInfo {
			pallet_nfts::Pallet::<T, I>::destroy(origin, collection, witness)
		}

		/// Destroy a single item.
		///
		/// The origin must conform to `ForceOrigin` or must be Signed and the signing account must
		/// be the owner of the `item`.
		///
		/// - `collection`: The collection of the item to be burned.
		/// - `item`: The item to be burned.
		///
		/// Emits `Burned`.
		///
		/// Weight: `O(1)`
		#[pallet::call_index(4)]
		#[pallet::weight(T::NftsWeightInfo::burn())]
		pub fn burn(
			origin: OriginFor<T>,
			collection: T::CollectionId,
			item: T::ItemId,
		) -> DispatchResult {
			pallet_nfts::Pallet::<T, I>::burn(origin, collection, item)
		}

		/// Move an item from the sender account to another.
		///
		/// Origin must be Signed and the signing account must be either:
		/// - the Owner of the `item`;
		/// - the approved delegate for the `item` (in this case, the approval is reset).
		///
		/// Arguments:
		/// - `collection`: The collection of the item to be transferred.
		/// - `item`: The item to be transferred.
		/// - `dest`: The account to receive ownership of the item.
		///
		/// Emits `Transferred`.
		///
		/// Weight: `O(1)`
		#[pallet::call_index(5)]
		#[pallet::weight(T::NftsWeightInfo::transfer())]
		pub fn transfer(
			origin: OriginFor<T>,
			collection: T::CollectionId,
			item: T::ItemId,
			dest: AccountIdLookupOf<T>,
		) -> DispatchResult {
			pallet_nfts::Pallet::<T, I>::transfer(origin, collection, item, dest)
		}

		/// Set an attribute for a collection or item.
		///
		/// Origin must be Signed and must conform to the namespace ruleset:
		/// - `CollectionOwner` namespace could be modified by the `collection` Admin only;
		/// - `ItemOwner` namespace could be modified by the `maybe_item` owner only. `maybe_item`
		///   should be set in that case;
		/// - `Account(AccountId)` namespace could be modified only when the `origin` was given a
		///   permission to do so;
		///
		/// The funds of `origin` are reserved according to the formula:
		/// `AttributeDepositBase + DepositPerByte * (key.len + value.len)` taking into
		/// account any already reserved funds.
		///
		/// - `collection`: The identifier of the collection whose item's metadata to set.
		/// - `maybe_item`: The identifier of the item whose metadata to set.
		/// - `namespace`: Attribute's namespace.
		/// - `key`: The key of the attribute.
		/// - `value`: The value to which to set the attribute.
		///
		/// Emits `AttributeSet`.
		///
		/// Weight: `O(1)`
		#[pallet::call_index(6)]
		#[pallet::weight(T::NftsWeightInfo::set_attribute())]
		pub fn set_attribute(
			origin: OriginFor<T>,
			collection: T::CollectionId,
			maybe_item: Option<T::ItemId>,
			namespace: AttributeNamespace<T::AccountId>,
			key: BoundedVec<u8, T::KeyLimit>,
			value: BoundedVec<u8, T::ValueLimit>,
		) -> DispatchResult {
			pallet_nfts::Pallet::<T, I>::set_attribute(
				origin, collection, maybe_item, namespace, key, value,
			)
		}

		/// Clear an attribute for a collection or item.
		///
		/// Origin must be either `ForceOrigin` or Signed and the sender should be the Owner of the
		/// attribute.
		///
		/// Any deposit is freed for the collection's owner.
		///
		/// - `collection`: The identifier of the collection whose item's metadata to clear.
		/// - `maybe_item`: The identifier of the item whose metadata to clear.
		/// - `namespace`: Attribute's namespace.
		/// - `key`: The key of the attribute.
		///
		/// Emits `AttributeCleared`.
		///
		/// Weight: `O(1)`
		#[pallet::call_index(7)]
		#[pallet::weight(T::NftsWeightInfo::clear_attribute())]
		pub fn clear_attribute(
			origin: OriginFor<T>,
			collection: T::CollectionId,
			maybe_item: Option<T::ItemId>,
			namespace: AttributeNamespace<T::AccountId>,
			key: BoundedVec<u8, T::KeyLimit>,
		) -> DispatchResult {
			pallet_nfts::Pallet::<T, I>::clear_attribute(
				origin, collection, maybe_item, namespace, key,
			)
		}

		/// Allows to buy an item if it's up for sale.
		///
		/// Origin must be Signed and must not be the owner of the `item`.
		///
		/// - `collection`: The collection of the item.
		/// - `item`: The item the sender wants to buy.
		/// - `bid_price`: The price the sender is willing to pay.
		///
		/// Emits `ItemBought` on success.
		#[pallet::call_index(8)]
		#[pallet::weight(T::NftsWeightInfo::buy_item())]
		pub fn buy_item(
			origin: OriginFor<T>,
			collection: T::CollectionId,
			item: T::ItemId,
			bid_price: ItemPrice<T, I>,
		) -> DispatchResult {
			pallet_nfts::Pallet::<T, I>::buy_item(origin, collection, item, bid_price)
		}

		/// Clear the metadata for a collection.
		///
		/// Origin must be either `ForceOrigin` or `Signed` and the sender should be the Admin of
		/// the `collection`.
		///
		/// Any deposit is freed for the collection's owner.
		///
		/// - `collection`: The identifier of the collection whose metadata to clear.
		///
		/// Emits `CollectionMetadataCleared`.
		///
		/// Weight: `O(1)`
		#[pallet::call_index(9)]
		#[pallet::weight(T::NftsWeightInfo::clear_collection_metadata())]
		pub fn clear_collection_metadata(
			origin: OriginFor<T>,
			collection: T::CollectionId,
		) -> DispatchResult {
			pallet_nfts::Pallet::<T, I>::clear_collection_metadata(origin, collection)
		}

		/// Clear the metadata for an item.
		///
		/// Origin must be either `ForceOrigin` or Signed and the sender should be the Admin of the
		/// `collection`.
		///
		/// Any deposit is freed for the collection's owner.
		///
		/// - `collection`: The identifier of the collection whose item's metadata to clear.
		/// - `item`: The identifier of the item whose metadata to clear.
		///
		/// Emits `ItemMetadataCleared`.
		///
		/// Weight: `O(1)`
		#[pallet::call_index(10)]
		#[pallet::weight(T::NftsWeightInfo::clear_metadata())]
		pub fn clear_metadata(
			origin: OriginFor<T>,
			collection: T::CollectionId,
			item: T::ItemId,
		) -> DispatchResult {
			pallet_nfts::Pallet::<T, I>::clear_metadata(origin, collection, item)
		}

		/// Disallows specified settings for the whole collection.
		///
		/// Origin must be Signed and the sender should be the Owner of the `collection`.
		///
		/// - `collection`: The collection to be locked.
		/// - `lock_settings`: The settings to be locked.
		///
		/// Note: it's possible to only lock(set) the setting, but not to unset it.
		///
		/// Emits `CollectionLocked`.
		///
		/// Weight: `O(1)`
		#[pallet::call_index(11)]
		#[pallet::weight(T::NftsWeightInfo::lock_collection())]
		pub fn lock_collection(
			origin: OriginFor<T>,
			collection: T::CollectionId,
			lock_settings: CollectionSettings,
		) -> DispatchResult {
			pallet_nfts::Pallet::<T, I>::lock_collection(origin, collection, lock_settings)
		}

		/// Disallows changing the metadata or attributes of the item.
		///
		/// Origin must be either `ForceOrigin` or Signed and the sender should be the Admin
		/// of the `collection`.
		///
		/// - `collection`: The collection if the `item`.
		/// - `item`: An item to be locked.
		/// - `lock_metadata`: Specifies whether the metadata should be locked.
		/// - `lock_attributes`: Specifies whether the attributes in the `CollectionOwner` namespace
		///   should be locked.
		///
		/// Note: `lock_attributes` affects the attributes in the `CollectionOwner` namespace only.
		/// When the metadata or attributes are locked, it won't be possible the unlock them.
		///
		/// Emits `ItemPropertiesLocked`.
		///
		/// Weight: `O(1)`
		#[pallet::call_index(12)]
		#[pallet::weight(T::NftsWeightInfo::lock_item_properties())]
		pub fn lock_item_properties(
			origin: OriginFor<T>,
			collection: T::CollectionId,
			item: T::ItemId,
			lock_metadata: bool,
			lock_attributes: bool,
		) -> DispatchResult {
			pallet_nfts::Pallet::<T, I>::lock_item_properties(
				origin,
				collection,
				item,
				lock_metadata,
				lock_attributes,
			)
		}

		/// Disallow further unprivileged transfer of an item.
		///
		/// Origin must be Signed and the sender should be the Freezer of the `collection`.
		///
		/// - `collection`: The collection of the item to be changed.
		/// - `item`: The item to become non-transferable.
		///
		/// Emits `ItemTransferLocked`.
		///
		/// Weight: `O(1)`
		#[pallet::call_index(13)]
		#[pallet::weight(T::NftsWeightInfo::lock_item_transfer())]
		pub fn lock_item_transfer(
			origin: OriginFor<T>,
			collection: T::CollectionId,
			item: T::ItemId,
		) -> DispatchResult {
			pallet_nfts::Pallet::<T, I>::lock_item_transfer(origin, collection, item)
		}

		/// Re-evaluate the deposits on some items.
		///
		/// Origin must be Signed and the sender should be the Owner of the `collection`.
		///
		/// - `collection`: The collection of the items to be reevaluated.
		/// - `items`: The items of the collection whose deposits will be reevaluated.
		///
		/// NOTE: This exists as a best-effort function. Any items which are unknown or
		/// in the case that the owner account does not have reservable funds to pay for a
		/// deposit increase are ignored. Generally the owner isn't going to call this on items
		/// whose existing deposit is less than the refreshed deposit as it would only cost them,
		/// so it's of little consequence.
		///
		/// It will still return an error in the case that the collection is unknown or the signer
		/// is not permitted to call it.
		///
		/// Weight: `O(items.len())`
		#[pallet::call_index(14)]
		#[pallet::weight(T::NftsWeightInfo::redeposit(items.len() as u32))]
		pub fn redeposit(
			origin: OriginFor<T>,
			collection: T::CollectionId,
			items: Vec<T::ItemId>,
		) -> DispatchResult {
			pallet_nfts::Pallet::<T, I>::redeposit(origin, collection, items)
		}

		/// Set the maximum number of items a collection could have.
		///
		/// Origin must be either `ForceOrigin` or `Signed` and the sender should be the Owner of
		/// the `collection`.
		///
		/// - `collection`: The identifier of the collection to change.
		/// - `max_supply`: The maximum number of items a collection could have.
		///
		/// Emits `CollectionMaxSupplySet` event when successful.
		#[pallet::call_index(15)]
		#[pallet::weight(T::NftsWeightInfo::set_collection_max_supply())]
		pub fn set_collection_max_supply(
			origin: OriginFor<T>,
			collection: T::CollectionId,
			max_supply: u32,
		) -> DispatchResult {
			pallet_nfts::Pallet::<T, I>::set_collection_max_supply(origin, collection, max_supply)
		}

		/// Set the metadata for a collection.
		///
		/// Origin must be either `ForceOrigin` or `Signed` and the sender should be the Admin of
		/// the `collection`.
		///
		/// If the origin is `Signed`, then funds of signer are reserved according to the formula:
		/// `MetadataDepositBase + DepositPerByte * data.len` taking into
		/// account any already reserved funds.
		///
		/// - `collection`: The identifier of the item whose metadata to update.
		/// - `data`: The general information of this item. Limited in length by `StringLimit`.
		///
		/// Emits `CollectionMetadataSet`.
		///
		/// Weight: `O(1)`
		#[pallet::call_index(16)]
		#[pallet::weight(T::NftsWeightInfo::set_collection_metadata())]
		pub fn set_collection_metadata(
			origin: OriginFor<T>,
			collection: T::CollectionId,
			data: BoundedVec<u8, T::StringLimit>,
		) -> DispatchResult {
			pallet_nfts::Pallet::<T, I>::set_collection_metadata(origin, collection, data)
		}

		/// Set the metadata for an item.
		///
		/// Origin must be either `ForceOrigin` or Signed and the sender should be the Admin of the
		/// `collection`.
		///
		/// If the origin is Signed, then funds of signer are reserved according to the formula:
		/// `MetadataDepositBase + DepositPerByte * data.len` taking into
		/// account any already reserved funds.
		///
		/// - `collection`: The identifier of the collection whose item's metadata to set.
		/// - `item`: The identifier of the item whose metadata to set.
		/// - `data`: The general information of this item. Limited in length by `StringLimit`.
		///
		/// Emits `ItemMetadataSet`.
		///
		/// Weight: `O(1)`
		#[pallet::call_index(17)]
		#[pallet::weight(T::NftsWeightInfo::set_metadata())]
		pub fn set_metadata(
			origin: OriginFor<T>,
			collection: T::CollectionId,
			item: T::ItemId,
			data: BoundedVec<u8, T::StringLimit>,
		) -> DispatchResult {
			pallet_nfts::Pallet::<T, I>::set_metadata(origin, collection, item, data)
		}

		/// Set (or reset) the price for an item.
		///
		/// Origin must be Signed and must be the owner of the `item`.
		///
		/// - `collection`: The collection of the item.
		/// - `item`: The item to set the price for.
		/// - `price`: The price for the item. Pass `None`, to reset the price.
		/// - `buyer`: Restricts the buy operation to a specific account.
		///
		/// Emits `ItemPriceSet` on success if the price is not `None`.
		/// Emits `ItemPriceRemoved` on success if the price is `None`.
		#[pallet::call_index(18)]
		#[pallet::weight(T::NftsWeightInfo::set_price())]
		pub fn set_price(
			origin: OriginFor<T>,
			collection: T::CollectionId,
			item: T::ItemId,
			price: Option<ItemPrice<T, I>>,
			whitelisted_buyer: Option<AccountIdLookupOf<T>>,
		) -> DispatchResult {
			pallet_nfts::Pallet::<T, I>::set_price(
				origin,
				collection,
				item,
				price,
				whitelisted_buyer,
			)
		}

		/// Change the Issuer, Admin and Freezer of a collection.
		///
		/// Origin must be either `ForceOrigin` or Signed and the sender should be the Owner of the
		/// `collection`.
		///
		/// Note: by setting the role to `None` only the `ForceOrigin` will be able to change it
		/// after to `Some(account)`.
		///
		/// - `collection`: The collection whose team should be changed.
		/// - `issuer`: The new Issuer of this collection.
		/// - `admin`: The new Admin of this collection.
		/// - `freezer`: The new Freezer of this collection.
		///
		/// Emits `TeamChanged`.
		///
		/// Weight: `O(1)`
		#[pallet::call_index(19)]
		#[pallet::weight(T::NftsWeightInfo::set_team())]
		pub fn set_team(
			origin: OriginFor<T>,
			collection: T::CollectionId,
			issuer: Option<AccountIdLookupOf<T>>,
			admin: Option<AccountIdLookupOf<T>>,
			freezer: Option<AccountIdLookupOf<T>>,
		) -> DispatchResult {
			pallet_nfts::Pallet::<T, I>::set_team(origin, collection, issuer, admin, freezer)
		}

		/// Change the Owner of a collection.
		///
		/// Origin must be Signed and the sender should be the Owner of the `collection`.
		///
		/// - `collection`: The collection whose owner should be changed.
		/// - `owner`: The new Owner of this collection. They must have called
		///   `set_accept_ownership` with `collection` in order for this operation to succeed.
		///
		/// Emits `OwnerChanged`.
		///
		/// Weight: `O(1)`
		#[pallet::call_index(20)]
		#[pallet::weight(T::NftsWeightInfo::transfer_ownership())]
		pub fn transfer_ownership(
			origin: OriginFor<T>,
			collection: T::CollectionId,
			new_owner: AccountIdLookupOf<T>,
		) -> DispatchResult {
			pallet_nfts::Pallet::<T, I>::transfer_ownership(origin, collection, new_owner)
		}

		/// Re-allow unprivileged transfer of an item.
		///
		/// Origin must be Signed and the sender should be the Freezer of the `collection`.
		///
		/// - `collection`: The collection of the item to be changed.
		/// - `item`: The item to become transferable.
		///
		/// Emits `ItemTransferUnlocked`.
		///
		/// Weight: `O(1)`
		#[pallet::call_index(21)]
		#[pallet::weight(T::NftsWeightInfo::unlock_item_transfer())]
		pub fn unlock_item_transfer(
			origin: OriginFor<T>,
			collection: T::CollectionId,
			item: T::ItemId,
		) -> DispatchResult {
			pallet_nfts::Pallet::<T, I>::unlock_item_transfer(origin, collection, item)
		}

		/// Update mint settings.
		///
		/// Origin must be either `ForceOrigin` or `Signed` and the sender should be the Issuer
		/// of the `collection`.
		///
		/// - `collection`: The identifier of the collection to change.
		/// - `mint_settings`: The new mint settings.
		///
		/// Emits `CollectionMintSettingsUpdated` event when successful.
		#[pallet::call_index(22)]
		#[pallet::weight(T::NftsWeightInfo::update_mint_settings())]
		pub fn update_mint_settings(
			origin: OriginFor<T>,
			collection: T::CollectionId,
			mint_settings: MintSettings<BalanceOf<T, I>, BlockNumberFor<T, I>, T::CollectionId>,
		) -> DispatchResult {
			pallet_nfts::Pallet::<T, I>::update_mint_settings(origin, collection, mint_settings)
		}

		/// Approve item's attributes to be changed by a delegated third-party account.
		///
		/// Origin must be Signed and must be an owner of the `item`.
		///
		/// - `collection`: A collection of the item.
		/// - `item`: The item that holds attributes.
		/// - `delegate`: The account to delegate permission to change attributes of the item.
		///
		/// Emits `ItemAttributesApprovalAdded` on success.
		#[pallet::call_index(23)]
		#[pallet::weight(T::NftsWeightInfo::approve_item_attributes())]
		pub fn approve_item_attributes(
			origin: OriginFor<T>,
			collection: T::CollectionId,
			item: T::ItemId,
			delegate: AccountIdLookupOf<T>,
		) -> DispatchResult {
			pallet_nfts::Pallet::<T, I>::approve_item_attributes(origin, collection, item, delegate)
		}

		/// Approve an item to be transferred by a delegated third-party account.
		///
		/// Origin must be either `ForceOrigin` or Signed and the sender should be the Owner of the
		/// `item`.
		///
		/// - `collection`: The collection of the item to be approved for delegated transfer.
		/// - `item`: The item to be approved for delegated transfer.
		/// - `delegate`: The account to delegate permission to transfer the item.
		/// - `maybe_deadline`: Optional deadline for the approval. Specified by providing the
		/// 	number of blocks after which the approval will expire
		///
		/// Emits `TransferApproved` on success.
		///
		/// Weight: `O(1)`
		#[pallet::call_index(24)]
		#[pallet::weight(T::NftsWeightInfo::approve_transfer())]
		pub fn approve_transfer(
			origin: OriginFor<T>,
			collection: T::CollectionId,
			item: T::ItemId,
			delegate: AccountIdLookupOf<T>,
			maybe_deadline: Option<BlockNumberFor<T, I>>,
		) -> DispatchResult {
			pallet_nfts::Pallet::<T, I>::approve_transfer(
				origin,
				collection,
				item,
				delegate,
				maybe_deadline,
			)
		}

		/// Cancel one of the transfer approvals for a specific item.
		///
		/// Origin must be either:
		/// - the `Force` origin;
		/// - `Signed` with the signer being the Owner of the `item`;
		///
		/// Arguments:
		/// - `collection`: The collection of the item of whose approval will be cancelled.
		/// - `item`: The item of the collection of whose approval will be cancelled.
		/// - `delegate`: The account that is going to loose their approval.
		///
		/// Emits `ApprovalCancelled` on success.
		///
		/// Weight: `O(1)`
		#[pallet::call_index(25)]
		#[pallet::weight(T::NftsWeightInfo::cancel_approval())]
		pub fn cancel_approval(
			origin: OriginFor<T>,
			collection: T::CollectionId,
			item: T::ItemId,
			delegate: AccountIdLookupOf<T>,
		) -> DispatchResult {
			pallet_nfts::Pallet::<T, I>::cancel_approval(origin, collection, item, delegate)
		}

		/// Cancel the previously provided approval to change item's attributes.
		/// All the previously set attributes by the `delegate` will be removed.
		///
		/// Origin must be Signed and must be an owner of the `item`.
		///
		/// - `collection`: Collection that the item is contained within.
		/// - `item`: The item that holds attributes.
		/// - `delegate`: The previously approved account to remove.
		///
		/// Emits `ItemAttributesApprovalRemoved` on success.
		#[pallet::call_index(26)]
		#[pallet::weight(T::NftsWeightInfo::cancel_item_attributes_approval(
			witness.account_attributes
		))]
		pub fn cancel_item_attributes_approval(
			origin: OriginFor<T>,
			collection: T::CollectionId,
			item: T::ItemId,
			delegate: AccountIdLookupOf<T>,
			witness: CancelAttributesApprovalWitness,
		) -> DispatchResult {
			pallet_nfts::Pallet::<T, I>::cancel_item_attributes_approval(
				origin, collection, item, delegate, witness,
			)
		}

		/// Cancel an atomic swap.
		///
		/// Origin must be Signed.
		/// Origin must be an owner of the `item` if the deadline hasn't expired.
		///
		/// - `collection`: The collection of the item.
		/// - `item`: The item an owner wants to give.
		///
		/// Emits `SwapCancelled` on success.
		#[pallet::call_index(27)]
		#[pallet::weight(T::NftsWeightInfo::cancel_swap())]
		pub fn cancel_swap(
			origin: OriginFor<T>,
			offered_collection: T::CollectionId,
			offered_item: T::ItemId,
		) -> DispatchResult {
			pallet_nfts::Pallet::<T, I>::cancel_swap(origin, offered_collection, offered_item)
		}

		/// Claim an atomic swap.
		/// This method executes a pending swap, that was created by a counterpart before.
		///
		/// Origin must be Signed and must be an owner of the `item`.
		///
		/// - `send_collection`: The collection of the item to be sent.
		/// - `send_item`: The item to be sent.
		/// - `receive_collection`: The collection of the item to be received.
		/// - `receive_item`: The item to be received.
		/// - `witness_price`: A price that was previously agreed on.
		///
		/// Emits `SwapClaimed` on success.
		#[pallet::call_index(28)]
		#[pallet::weight(T::NftsWeightInfo::claim_swap())]
		pub fn claim_swap(
			origin: OriginFor<T>,
			send_collection: T::CollectionId,
			send_item: T::ItemId,
			receive_collection: T::CollectionId,
			receive_item: T::ItemId,
			witness_price: Option<PriceWithDirection<ItemPrice<T, I>>>,
		) -> DispatchResult {
			pallet_nfts::Pallet::<T, I>::claim_swap(
				origin,
				send_collection,
				send_item,
				receive_collection,
				receive_item,
				witness_price,
			)
		}

		/// Cancel all the approvals of a specific item.
		///
		/// Origin must be either:
		/// - the `Force` origin;
		/// - `Signed` with the signer being the Owner of the `item`;
		///
		/// Arguments:
		/// - `collection`: The collection of the item of whose approvals will be cleared.
		/// - `item`: The item of the collection of whose approvals will be cleared.
		///
		/// Emits `AllApprovalsCancelled` on success.
		///
		/// Weight: `O(1)`
		#[pallet::call_index(29)]
		#[pallet::weight(T::NftsWeightInfo::clear_all_transfer_approvals())]
		pub fn clear_all_transfer_approvals(
			origin: OriginFor<T>,
			collection: T::CollectionId,
			item: T::ItemId,
		) -> DispatchResult {
			pallet_nfts::Pallet::<T, I>::clear_all_transfer_approvals(origin, collection, item)
		}

		/// Register a new atomic swap, declaring an intention to send an `item` in exchange for
		/// `desired_item` from origin to target on the current blockchain.
		/// The target can execute the swap during the specified `duration` of blocks (if set).
		/// Additionally, the price could be set for the desired `item`.
		///
		/// Origin must be Signed and must be an owner of the `item`.
		///
		/// - `collection`: The collection of the item.
		/// - `item`: The item an owner wants to give.
		/// - `desired_collection`: The collection of the desired item.
		/// - `desired_item`: The desired item an owner wants to receive.
		/// - `maybe_price`: The price an owner is willing to pay or receive for the desired `item`.
		/// - `duration`: A deadline for the swap. Specified by providing the number of blocks
		/// 	after which the swap will expire.
		///
		/// Emits `SwapCreated` on success.
		#[pallet::call_index(30)]
		#[pallet::weight(T::NftsWeightInfo::create_swap())]
		pub fn create_swap(
			origin: OriginFor<T>,
			offered_collection: T::CollectionId,
			offered_item: T::ItemId,
			desired_collection: T::CollectionId,
			maybe_desired_item: Option<T::ItemId>,
			maybe_price: Option<PriceWithDirection<ItemPrice<T, I>>>,
			duration: BlockNumberFor<T, I>,
		) -> DispatchResult {
			pallet_nfts::Pallet::<T, I>::create_swap(
				origin,
				offered_collection,
				offered_item,
				desired_collection,
				maybe_desired_item,
				maybe_price,
				duration,
			)
		}

		/// Change the config of a collection.
		///
		/// Origin must be `ForceOrigin`.
		///
		/// - `collection`: The identifier of the collection.
		/// - `config`: The new config of this collection.
		///
		/// Emits `CollectionConfigChanged`.
		///
		/// Weight: `O(1)`
		#[pallet::call_index(31)]
		#[pallet::weight(T::NftsWeightInfo::force_collection_config())]
		pub fn force_collection_config(
			origin: OriginFor<T>,
			collection: T::CollectionId,
			config: CollectionConfigFor<T, I>,
		) -> DispatchResult {
			pallet_nfts::Pallet::<T, I>::force_collection_config(origin, collection, config)
		}

		/// Change the Owner of a collection.
		///
		/// Origin must be `ForceOrigin`.
		///
		/// - `collection`: The identifier of the collection.
		/// - `owner`: The new Owner of this collection.
		///
		/// Emits `OwnerChanged`.
		///
		/// Weight: `O(1)`
		#[pallet::call_index(32)]
		#[pallet::weight(T::NftsWeightInfo::force_collection_owner())]
		pub fn force_collection_owner(
			origin: OriginFor<T>,
			collection: T::CollectionId,
			owner: AccountIdLookupOf<T>,
		) -> DispatchResult {
			pallet_nfts::Pallet::<T, I>::force_collection_owner(origin, collection, owner)
		}

		/// Issue a new collection of non-fungible items from a privileged origin.
		///
		/// This new collection has no items initially.
		///
		/// The origin must conform to `ForceOrigin`.
		///
		/// Unlike `create`, no funds are reserved.
		///
		/// - `owner`: The owner of this collection of items. The owner has full superuser
		///   permissions over this item, but may later change and configure the permissions using
		///   `transfer_ownership` and `set_team`.
		///
		/// Emits `ForceCreated` event when successful.
		///
		/// Weight: `O(1)`
		#[pallet::call_index(33)]
		#[pallet::weight(T::NftsWeightInfo::force_create())]
		pub fn force_create(
			origin: OriginFor<T>,
			owner: AccountIdLookupOf<T>,
			config: CollectionConfigFor<T, I>,
		) -> DispatchResult {
			pallet_nfts::Pallet::<T, I>::force_create(origin, owner, config)
		}

		/// Mint an item of a particular collection from a privileged origin.
		///
		/// The origin must conform to `ForceOrigin` or must be `Signed` and the sender must be the
		/// Issuer of the `collection`.
		///
		/// - `collection`: The collection of the item to be minted.
		/// - `item`: An identifier of the new item.
		/// - `mint_to`: Account into which the item will be minted.
		/// - `item_config`: A config of the new item.
		///
		/// Emits `Issued` event when successful.
		///
		/// Weight: `O(1)`
		#[pallet::call_index(34)]
		#[pallet::weight(T::NftsWeightInfo::force_mint())]
		pub fn force_mint(
			origin: OriginFor<T>,
			collection: T::CollectionId,
			item: T::ItemId,
			mint_to: AccountIdLookupOf<T>,
			item_config: ItemConfig,
		) -> DispatchResult {
			pallet_nfts::Pallet::<T, I>::force_mint(origin, collection, item, mint_to, item_config)
		}

		/// Force-set an attribute for a collection or item.
		///
		/// Origin must be `ForceOrigin`.
		///
		/// If the attribute already exists and it was set by another account, the deposit
		/// will be returned to the previous owner.
		///
		/// - `set_as`: An optional owner of the attribute.
		/// - `collection`: The identifier of the collection whose item's metadata to set.
		/// - `maybe_item`: The identifier of the item whose metadata to set.
		/// - `namespace`: Attribute's namespace.
		/// - `key`: The key of the attribute.
		/// - `value`: The value to which to set the attribute.
		///
		/// Emits `AttributeSet`.
		///
		/// Weight: `O(1)`
		#[pallet::call_index(35)]
		#[pallet::weight(T::NftsWeightInfo::force_set_attribute())]
		pub fn force_set_attribute(
			origin: OriginFor<T>,
			set_as: Option<T::AccountId>,
			collection: T::CollectionId,
			maybe_item: Option<T::ItemId>,
			namespace: AttributeNamespace<T::AccountId>,
			key: BoundedVec<u8, T::KeyLimit>,
			value: BoundedVec<u8, T::ValueLimit>,
		) -> DispatchResult {
			pallet_nfts::Pallet::<T, I>::force_set_attribute(
				origin, set_as, collection, maybe_item, namespace, key, value,
			)
		}

		/// Mint an item by providing the pre-signed approval.
		///
		/// Origin must be Signed.
		///
		/// - `mint_data`: The pre-signed approval that consists of the information about the item,
		///   its metadata, attributes, who can mint it (`None` for anyone) and until what block
		///   number.
		/// - `signature`: The signature of the `data` object.
		/// - `signer`: The `data` object's signer. Should be an Issuer of the collection.
		///
		/// Emits `Issued` on success.
		/// Emits `AttributeSet` if the attributes were provided.
		/// Emits `ItemMetadataSet` if the metadata was not empty.
		#[pallet::call_index(36)]
		#[pallet::weight(T::NftsWeightInfo::mint_pre_signed(mint_data.attributes.len() as u32))]
		pub fn mint_pre_signed(
			origin: OriginFor<T>,
			mint_data: Box<PreSignedMintOf<T, I>>,
			signature: T::OffchainSignature,
			signer: T::AccountId,
		) -> DispatchResult {
			pallet_nfts::Pallet::<T, I>::mint_pre_signed(origin, mint_data, signature, signer)
		}

		/// Allows to pay the tips.
		///
		/// Origin must be Signed.
		///
		/// - `tips`: Tips array.
		///
		/// Emits `TipSent` on every tip transfer.
		#[pallet::call_index(37)]
		#[pallet::weight(T::NftsWeightInfo::pay_tips(tips.len() as u32))]
		pub fn pay_tips(
			origin: OriginFor<T>,
			tips: BoundedVec<ItemTipOf<T, I>, T::MaxTips>,
		) -> DispatchResult {
			pallet_nfts::Pallet::<T, I>::pay_tips(origin, tips)
		}

		/// Set (or reset) the acceptance of ownership for a particular account.
		///
		/// Origin must be `Signed` and if `maybe_collection` is `Some`, then the signer must have a
		/// provider reference.
		///
		/// - `maybe_collection`: The identifier of the collection whose ownership the signer is
		///   willing to accept, or if `None`, an indication that the signer is willing to accept no
		///   ownership transferal.
		///
		/// Emits `OwnershipAcceptanceChanged`.
		#[pallet::call_index(38)]
		#[pallet::weight(T::NftsWeightInfo::set_accept_ownership())]
		pub fn set_accept_ownership(
			origin: OriginFor<T>,
			maybe_collection: Option<T::CollectionId>,
		) -> DispatchResult {
			pallet_nfts::Pallet::<T, I>::set_accept_ownership(origin, maybe_collection)
		}
	}
}
