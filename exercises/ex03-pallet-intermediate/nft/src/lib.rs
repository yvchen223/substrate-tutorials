#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/v3/runtime/frame>
pub use pallet::*;

#[cfg(test)]
mod tests;
pub mod types;

use frame_support::ensure;
use types::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	#[pallet::config]
	pub trait Config: frame_system::Config + scale_info::TypeInfo {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		#[pallet::constant]
		type MaxLength: Get<u32>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn unique_asset)]
	pub(super) type UniqueAsset<T: Config> =
		StorageMap<_, Blake2_128Concat, UniqueAssetId, UniqueAssetDetails<T, T::MaxLength>>;

	#[pallet::storage]
	#[pallet::getter(fn account)]
	/// The holdings of a specific account for a specific asset.
	pub(super) type Account<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		UniqueAssetId,
		Blake2_128Concat,
		T::AccountId,
		u128,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn nonce)]
	/// Nonce for id of the next created asset
	pub(super) type Nonce<T: Config> = StorageValue<_, UniqueAssetId, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// New unique asset created
		Created {
			creator: T::AccountId,
			asset_id: UniqueAssetId,
		},
		/// Some assets have been burned
		Burned {
			asset_id: UniqueAssetId,
			owner: T::AccountId,
			total_supply: u128,
		},
		/// Some assets have been transferred
		Transferred {
			asset_id: UniqueAssetId,
			from: T::AccountId,
			to: T::AccountId,
			amount: u128,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The asset ID is unknown
		UnknownAssetId,
		/// The signing account does not own any amount of this asset
		NotOwned,
		/// Supply must be positive
		NoSupply,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(0)]
		pub fn mint(
			origin: OriginFor<T>,
			metadata: BoundedVec<u8, T::MaxLength>,
			supply: u128,
		) -> DispatchResult {
			// Must be signed.
			let origin = ensure_signed(origin)?;

			// Must have positive supply.
			ensure!(supply > 0, Error::<T>::NoSupply);

			let asset_id = Self::nonce();
			Nonce::<T>::set(asset_id.saturating_add(1));
			let unique_asset_details = UniqueAssetDetails::<T, T::MaxLength>::new(
				origin.clone(),
				metadata.clone(),
				supply,
			);
			UniqueAsset::<T>::insert(asset_id, unique_asset_details);
			Account::<T>::insert(asset_id, origin.clone(), supply);

			Self::deposit_event(Event::<T>::Created {
				creator: origin.clone(),
				asset_id,
			});

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn burn(origin: OriginFor<T>, asset_id: UniqueAssetId, amount: u128) -> DispatchResult {
			let origin = ensure_signed(origin)?;

			if let Some(asset_details) = UniqueAsset::<T>::get(asset_id) {
				if asset_details.creator() != origin.clone() {
					return Err(Error::<T>::NotOwned.into())
				}
			} else {
				return Err(Error::<T>::UnknownAssetId.into())
			}

			let mut burn_amount = 0;
			let mut total_supply = 0;

			let _ = UniqueAsset::<T>::try_mutate(asset_id, |details| -> DispatchResult {
				let details = details.as_mut().ok_or(Error::<T>::UnknownAssetId)?;
				if details.creator() != origin {
					return Err(Error::<T>::NotOwned.into())
				}
				let old_supply = details.supply;
				details.supply = details.supply.saturating_sub(amount);
				burn_amount = old_supply - details.supply;
				total_supply = details.supply;

				Ok(())
			});
			Account::<T>::mutate(asset_id, origin.clone(), |balance| {
				*balance -= burn_amount;
			});

			Self::deposit_event(Event::Burned {
				asset_id,
				owner: origin.clone(),
				total_supply,
			});

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn transfer(
			origin: OriginFor<T>,
			asset_id: UniqueAssetId,
			amount: u128,
			to: T::AccountId,
		) -> DispatchResult {
			let origin = ensure_signed(origin)?;

			if let Some(asset_details) = UniqueAsset::<T>::get(asset_id) {
				if asset_details.creator() != origin.clone() {
					return Err(Error::<T>::NotOwned.into())
				}
			} else {
				return Err(Error::<T>::UnknownAssetId.into())
			}

			let mut transfer_amount = 0;
			Account::<T>::mutate(asset_id, origin.clone(), |balance| {
				let old_balance = *balance;
				*balance = balance.saturating_sub(amount);
				transfer_amount = old_balance - *balance;
			});
			Account::<T>::mutate(asset_id, to.clone(), |balance| {
				*balance += transfer_amount;
			});

			Self::deposit_event(Event::<T>::Transferred {
				asset_id,
				from: origin.clone(),
				to: to.clone(),
				amount: transfer_amount,
			});

			Ok(())
		}
	}
}
