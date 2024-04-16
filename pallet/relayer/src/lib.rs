// Copyright 2023 Capsule Corp (France) SAS.
// This file is part of Ternoa.

// Ternoa is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Ternoa is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Ternoa.  If not, see <http://www.gnu.org/licenses/>.

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

use frame_support::{
    dispatch::DispatchResultWithPostInfo,
    pallet_prelude::*,
    traits::{StorageVersion, UnfilteredDispatchable},
};
use frame_system::pallet_prelude::*;
use sp_std::prelude::*;

const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    /// Configure the pallet by specifying the parameters and types on which it depends.
    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_token::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// A sudo-able call.
        type RuntimeCall: From<Call<Self>>;

        #[pallet::constant]
        type WBtcAssetId: Get<Self::AssetId>;
        // Someone who can call the mandate extrinsic.
        // type ExternalOrigin: EnsureOrigin<Self::RuntimeOrigin>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    #[pallet::storage_version(STORAGE_VERSION)]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    pub type Relayer<T: Config> =
        StorageValue<_, <T as frame_system::Config>::AccountId, OptionQuery>;

    #[pallet::storage]
    pub type BitcoinTranxId<T: Config> = 
    StorageMap<_, Blake2_128Concat, Vec<u8>, T::Balance, ValueQuery>;

    #[pallet::storage]
    pub type User<T: Config> = 
    StorageMap<_, Blake2_128Concat, <T as frame_system::Config>::AccountId, Vec<u8>, OptionQuery>;
    
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        // #[pallet::weight({
        // 	let dispatch_info = call.get_dispatch_info();
        // 	(dispatch_info.weight, dispatch_info.class)
        // })]
        #[pallet::weight(10000000)]
        pub fn register_relayer(
            origin: OriginFor<T>,
            address: <T as frame_system::Config>::AccountId,
        ) -> DispatchResult {
            ensure_signed(origin)?; //TC

            Relayer::<T>::mutate(|user| {
                *user = Some(address.clone());
            });

            Self::deposit_event(Event::NewRegistration {
                address,
                role: "relayer".as_bytes().to_vec(),
            });

            Ok(())
        }

        #[pallet::weight(10000000)]
        pub fn update_relayer(
            origin: OriginFor<T>,
            address: <T as frame_system::Config>::AccountId,
        ) -> DispatchResult {
            ensure_signed(origin)?; //TC

            Relayer::<T>::try_mutate_exists(|user_opt| {
                if let Some(user) = user_opt {
                    Self::deposit_event(Event::RelayerUpdated {
                        old_address: user.clone(),
                        new_address: address.clone(),
                    });

                    *user = address;
                }
                Ok(())
            })
        }

        #[pallet::weight(10000000)]
        pub fn remove_relayer(origin: OriginFor<T>) -> DispatchResult {
            ensure_signed(origin)?; //TC

            Relayer::<T>::try_mutate_exists(|user_opt| {
                if let Some(user) = user_opt {
                    Self::deposit_event(Event::RelayersRemoved {
                        address: user.clone(),
                        role: "relayer".as_bytes().to_vec(),
                    });
                }
                *user_opt = None;
                Ok(())
            })
        }

        #[pallet::weight(10000000)]
        pub fn mint_wrapper_token(
            origin: OriginFor<T>,
            assetid: T::AssetId,
            address: <T as frame_system::Config>::AccountId,
            amount: T::Balance,
            bitcoin_address: Vec<u8>,
            transaction_id: Vec<u8>
        ) -> DispatchResult {
            let rel = ensure_signed(origin.clone())?;
            //Storing btc address, persistent
            // User::<T>::try_mutate(address, |maybe_details| {
            //     // TODO: return some meaningful error here.
            //     if let None = maybe_details {
            //         Some(bitcoin_address)
            //     }
            //     else {
            //         None
            //     }
            // });
            //Extrinsic call to mint token
            User::<T>::insert(address.clone(), bitcoin_address.clone());
            <pallet_token::Pallet<T>>::mint(origin, assetid, address.clone(), amount)?;
            BitcoinTranxId::<T>::insert(transaction_id, amount);
            Self::deposit_event(Event::WBtcAdded {
                relayer: rel,
                user: address,
                amount,
                bitcoin_address,
            });

            Ok(())
        }

        #[pallet::weight(10000000)]
        pub fn burn_wrapper_token(
            origin: OriginFor<T>,
            assetid: T::AssetId,
            address: <T as frame_system::Config>::AccountId,
            amount: T::Balance,
            bitcoin_address: Vec<u8>,
        ) -> DispatchResult {
            let rel = ensure_signed(origin.clone())?;
            let assetid: T::AssetId = T::WBtcAssetId::get();
            <pallet_token::Pallet<T>>::burn(origin, assetid, address.clone(), amount)?;
            Self::deposit_event(Event::WBtcDeleted {
                relayer: rel,
                user: address,
                amount,
                bitcoin_address,
            });
            Ok(())
        }
    }
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A root operation was executed, show result
        NewRegistration {
            address: <T as frame_system::Config>::AccountId,
            role: Vec<u8>,
        },
        /// A relayer is removed
        RelayersRemoved {
            address: <T as frame_system::Config>::AccountId,
            role: Vec<u8>,
        },
        /// A relayer address is updated
        RelayerUpdated {
            old_address: <T as frame_system::Config>::AccountId,
            new_address: <T as frame_system::Config>::AccountId,
        },
        /// Relayer minted wrapped btc successfully
        WBtcAdded {
            relayer: <T as frame_system::Config>::AccountId,
            user: <T as frame_system::Config>::AccountId,
            amount: T::Balance,
            bitcoin_address: Vec<u8>,
        },
        /// Relayer burn wrapped btc successfully
        WBtcDeleted {
            relayer: <T as frame_system::Config>::AccountId,
            user: <T as frame_system::Config>::AccountId,
            amount: T::Balance,
            bitcoin_address: Vec<u8>,
        },
    }
}
