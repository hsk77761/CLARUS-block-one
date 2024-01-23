#![cfg_attr(not(feature = "std"), no_std)]

mod functions;
/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://substrate.dev/docs/en/knowledgebase/runtime/frame>

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

// #[cfg(feature = "runtime-benchmarks")]
// mod benchmarking;

use codec::HasCompact;
use frame_support::pallet_prelude::MaxEncodedLen;
pub use pallet::*;
use scale_info::TypeInfo;
use sp_runtime::traits::Member;
use sp_std::{convert::TryInto, fmt::Debug, prelude::*};
pub mod weights;
use serde::{de::DeserializeOwned, Serialize};
use sp_runtime::traits::Zero;
use weights::WeightInfo;

/// Trait with callbacks that are executed after successfull asset creation or destruction.
pub trait AssetsCallback<AssetId, AccountId> {
    /// Indicates that asset with `id` was successfully created by the `owner`
    fn created(_id: &AssetId, _owner: &AccountId) -> Result<(), ()> {
        Ok(())
    }

    /// Indicates that asset with `id` has just been destroyed
    fn destroyed(_id: &AssetId) -> Result<(), ()> {
        Ok(())
    }
}

/// Empty implementation in case no callbacks are required.
impl<AssetId, AccountId> AssetsCallback<AssetId, AccountId> for () {}

#[derive(codec::Encode, codec::Decode, Default, Clone, PartialEq, Eq, Debug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct AssetDetails<T: Config> {
    /// Creator of the token
    pub admin: T::AccountId,
    /// Can mint/burn tokens.
    pub issuer: T::AccountId,
    /// The total number token in circulation.
    pub supply: T::Balance,
    /// The total number of accounts.
    pub accounts: u32,
    /// Asset name
    pub name: Vec<u8>,
    /// Asset symbol
    pub symbol: Vec<u8>,
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_support::traits::EnsureOriginWithArg;
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::AtLeast32BitUnsigned;
    use sp_runtime::traits::StaticLookup;
    use sp_runtime::traits::{CheckedAdd, CheckedSub};
    use sp_std::vec;

    type AccountIdLookupOf<T> = <<T as frame_system::Config>::Lookup as StaticLookup>::Source;
    /// Configure the pallet by specifying the parameters and types on which it depends.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        // The overarching dispatch call type.
        // type Call: From<Call<Self>>;

        /// The units in which we record balances.
        type Balance: Member
            + Parameter
            + AtLeast32BitUnsigned
            + Default
            + Copy
            + MaybeSerializeDeserialize
            + MaxEncodedLen
            + TypeInfo;

        /// Identifier for the class of asset.
        type AssetId: Member
            + Parameter
            + Default
            + Copy
            + HasCompact
            + MaxEncodedLen
            + TypeInfo
            + Serialize
            + DeserializeOwned;

        /// Wrapper around `Self::AssetId` to use in dispatchable call signatures. Allows the use
        /// of compact encoding in instances of the pallet, which will prevent breaking changes
        /// resulting from the removal of `HasCompact` from `Self::AssetId`.
        ///
        /// This type includes the `From<Self::AssetId>` bound, since tightly coupled pallets may
        /// want to convert an `AssetId` into a parameter for calling dispatchable functions
        /// directly.
        type AssetIdParameter: Parameter
            + Copy
            + From<Self::AssetId>
            + Into<Self::AssetId>
            + MaxEncodedLen;

        // /// Standard asset class creation is only allowed if the origin attempting it and the
        // /// asset class are in this set.
        // type CreateOrigin: EnsureOriginWithArg<
        //     Self::RuntimeOrigin,
        //     Self::AssetId,
        //     Success = Self::AccountId,
        // >;
        /// The overarching WeightInfo type
        type WeightInfo: WeightInfo;

        // Callback methods for asset state change (e.g. asset created or destroyed)
        //type CallbackHandle: AssetsCallback<Self::AssetId, Self::AccountId>;
    }

    const STORAGE_VERSION: StorageVersion = StorageVersion::new(10);
    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn balance_of)]
    pub type BalanceOf<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::AssetId,
        Blake2_128Concat,
        T::AccountId,
        T::Balance,
        ValueQuery,
        GetDefault,
    >;

    // allowance for an account and token
    #[pallet::storage]
    #[pallet::getter(fn allowance)]
    pub type Allowance<T: Config> = StorageNMap<
        _,
        (
            NMapKey<Blake2_128Concat, T::AssetId>,
            NMapKey<Blake2_128Concat, T::AccountId>, // owner
            NMapKey<Blake2_128Concat, T::AccountId>, // delegate
        ),
        T::Balance,
        ValueQuery,
        GetDefault,
        ConstU32<300_000>,
    >;

    #[pallet::storage]
    #[pallet::getter(fn assets)]
    /// Details of an asset.
    pub type Asset<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AssetId, AssetDetails<T>, OptionQuery, GetDefault>;

    // Pallets use events to inform users when important changes are made.
    // https://substrate.dev/docs/en/knowledgebase/runtime/events
    #[pallet::event]
    //#[pallet::metadata(T::AccountId = "AccountId")]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Some asset class was created.
        Created {
            asset_id: T::AssetId,
            creator: T::AccountId,
            issuer: T::AccountId,
        },
        // event for transfer of tokens
        // tokenid, from, to, value
        Transferred {
            asset_id: T::AssetId,
            from: T::AccountId,
            to: T::AccountId,
            amount: T::Balance,
        },
        // event when an approval is made
        // tokenid, owner, spender, value
        Approval {
            asset_id: T::AssetId,
            owner: T::AccountId,
            spender: T::AccountId,
            amount: T::Balance,
        },

        /// Some assets were issued. \[asset_id, owner, total_supply\]
        Issued {
            asset_id: T::AssetId,
            owner: T::AccountId,
            balance: T::Balance,
        },

        /// Some assets were destroyed. \[asset_id, owner, balance\]
        Burned {
            asset_id: T::AssetId,
            owner: T::AccountId,
            balance: T::Balance,
        },
    }

    // Errors inform users that something went wrong.
    #[pallet::error]
    pub enum Error<T> {
        /// Error thrown when account does not have permission
        NoPermission,
        /// Error thrown when account does not hold  token
        AccountDoesNotOwnThisToken,
        /// Error thrown when asset detail is unknown
        UnknownAsset,
        /// Error thrown when transfer falied due to  balance underflow
        TransferUnderFlow,
        /// Error thrown when transfer falied due to  balance overflow
        TransferOverFlow,
        /// Error thrown when transfer falied due to insufficient  balance
        InsufficientTransfer,
        /// Error thrown when burn falied due to  balance underflow
        BurnUnderflow,
        /// Error thrown when mint falied due to  balance overflow
        MintOverFlow,
        /// Error thrown when balance update falied due to  balance underflow
        BalanceUnderflow,
        /// Error thrown when balance update falied due to  balance overflow
        BalanceOverflow,
        /// Error thrown when burn falied due to insufficient  balance
        InsufficientBurn,
        /// Error thrown when deleting accounts failed due to underflow
        AccountsUnderflow,
        /// Error thrown when creating new accounts failed due to overflow
        AccountsOverflow,
        /// Callback action resulted in error
        CallbackFailed,
        /// Minimum balance should be non-zero.
        MinBalanceZero,
        /// The asset ID is already taken.
        InUse,
    }

    // Dispatchable functions allows users to interact with the pallet and invoke state changes.
    // These functions materialize as "extrinsics", which are often compared to transactions.
    // Dispatchable functions must be annotated with a weight and must return a DispatchResult.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Issue a new class of fungible assets from a public origin.
        ///
        /// This new asset class has no assets initially and its owner is the origin.
        ///
        /// The origin must conform to the configured `CreateOrigin` and have sufficient funds free.
        ///
        /// Funds of sender are reserved by `AssetDeposit`.
        ///
        /// Parameters:
        /// - `id`: The identifier of the new asset. This must not be currently in use to identify
        /// an existing asset.
        /// - `admin`: The admin of this class of assets. The admin is the initial address of each
        /// member of the asset class's admin team.
        /// - `min_balance`: The minimum balance of this new asset that any single account must
        /// have. If an account's balance is reduced below this, then it collapses to zero.
        ///
        /// Emits `Created` event when successful.
        ///
        /// Weight: `O(1)`
        #[pallet::call_index(0)]
        #[pallet::weight((<T as Config>::WeightInfo::transfer(), Pays::No))]
        pub fn create(
            origin: OriginFor<T>,
            id: T::AssetIdParameter,
            issuer: AccountIdLookupOf<T>,
            min_balance: T::Balance,
            name: Vec<u8>,
            symbol: Vec<u8>,
        ) -> DispatchResult {
            let id: T::AssetId = id.into();

            let admin = ensure_signed(origin)?; //T::CreateOrigin::ensure_origin(origin, &id)?; //done by TC
            let issuer = T::Lookup::lookup(issuer)?;

            ensure!(!Asset::<T>::contains_key(id), Error::<T>::InUse);
            ensure!(!min_balance.is_zero(), Error::<T>::MinBalanceZero);

            //let deposit = T::AssetDeposit::get();
            //T::Currency::reserve(&owner, deposit)?;

            Asset::<T>::insert(
                id,
                AssetDetails {
                    admin: admin.clone(),
                    issuer: issuer.clone(),
                    name,
                    symbol,
                    supply: Zero::zero(),
                    accounts: 0,
                    // deposit,
                    // min_balance,
                    // is_sufficient: false,
                },
            );
            // ensure!(
            //     T::CallbackHandle::created(&id, &owner).is_ok(),
            //     Error::<T>::CallbackFailed
            // );
            Self::deposit_event(Event::Created {
                asset_id: id,
                creator: admin.clone(),
                issuer,
            });

            Ok(())
        }

        // transfer tokens from one account to another
        // origin is assumed as sender
        #[pallet::call_index(1)]
        #[pallet::weight((<T as Config>::WeightInfo::transfer(), Pays::No))]
        pub fn transfer(
            origin: OriginFor<T>,
            #[pallet::compact] id: T::AssetId,
            to: T::AccountId,
            value: T::Balance,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            Self::_transfer(id, sender, to, value)
        }

        // approve token transfer from one account to another
        // once this is done, transfer_from can be called with corresponding values
        #[pallet::call_index(2)]
        #[pallet::weight((<T as Config>::WeightInfo::approve(), Pays::No))]
        pub fn approve(
            origin: OriginFor<T>,
            #[pallet::compact] token_id: T::AssetId,
            spender: T::AccountId,
            value: T::Balance,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            ensure!(
                <BalanceOf<T>>::contains_key(token_id, sender.clone()),
                "Account does not own this token"
            );

            let allowance = Self::allowance((token_id, sender.clone(), spender.clone()));
            let updated_allowance = allowance
                .checked_add(&value)
                .ok_or("overflow in calculating allowance")?;
            <Allowance<T>>::insert(
                (token_id, sender.clone(), spender.clone()),
                updated_allowance,
            );

            Self::deposit_event(Event::Approval {
                asset_id: token_id,
                owner: sender,
                spender,
                amount: value,
            });

            Ok(())
        }

        // the ERC20 standard transfer_from function
        // implemented in the open-zeppelin way - increase/decrease allownace
        // if approved, transfer from an account to another account without owner's signature
        #[pallet::call_index(3)]
        #[pallet::weight((<T as Config>::WeightInfo::transfer_from(), Pays::No))]
        pub fn transfer_from(
            origin: OriginFor<T>,
            token_id: T::AssetId,
            from: T::AccountId,
            to: T::AccountId,
            value: T::Balance,
        ) -> DispatchResult {
            let spender = ensure_signed(origin)?;
            ensure!(
                <Allowance<T>>::contains_key((token_id, from.clone(), spender.clone())),
                "Allowance does not exist."
            );
            let allowance = Self::allowance((token_id, from.clone(), spender.clone()));
            ensure!(allowance >= value, "Not enough allowance.");

            // using checked_sub (safe math) to avoid overflow
            let updated_allowance = allowance
                .checked_sub(&value)
                .ok_or("overflow in calculating allowance")?;
            <Allowance<T>>::insert((token_id, from.clone(), spender.clone()), updated_allowance);

            Self::deposit_event(Event::Approval {
                asset_id: token_id,
                owner: from.clone(),
                spender,
                amount: updated_allowance,
            });

            Self::_transfer(token_id, from, to, value)
        }

        // / Mint assets of a particular class.
        // /
        // / The origin must be Signed and the sender must be the Issuer of the asset `id`.
        // /
        // / - `id`: The identifier of the asset to have some amount minted.
        // / - `beneficiary`: The account to be credited with the minted assets.
        // / - `amount`: The amount of the asset to be minted.
        // /
        // / Emits `Issued` event when successful.
        // /
        // / Weight: `O(1)`
        // / Modes: Pre-existing balance of `beneficiary`; Account pre-existence of `beneficiary`.
        #[pallet::call_index(4)]
        #[pallet::weight((<T as Config>::WeightInfo::mint(), Pays::No))]
        pub fn mint(
            origin: OriginFor<T>,
            #[pallet::compact] id: T::AssetId,
            beneficiary: T::AccountId,
            amount: T::Balance,
        ) -> DispatchResult {
            let origin = ensure_signed(origin)?;
            Self::do_mint(id, &beneficiary, amount, Some(origin))?;
            Ok(())
        }

        // / Reduce the balance of `who` by as much as possible up to `amount` assets of `id`.
        // /
        // / Origin must be Signed and the sender should be the Manager of the asset `id`.
        // /
        // / Bails with `BalanceZero` if the `who` is already dead.
        // /
        // / - `id`: The identifier of the asset to have some amount burned.
        // / - `who`: The account to be debited from.
        // / - `amount`: The maximum amount by which `who`'s balance should be reduced.
        // /
        // / Emits `Burned` with the actual amount burned. If this takes the balance to below the
        // / minimum for the asset, then the amount burned is increased to take it to zero.
        // /
        // / Weight: `O(1)`
        // / Modes: Post-existence of `who`; Pre & post Zombie-status of `who`.
        #[pallet::call_index(5)]
        #[pallet::weight((<T as Config>::WeightInfo::burn(), Pays::No))]
        pub fn burn(
            origin: OriginFor<T>,
            #[pallet::compact] id: T::AssetId,
            who: T::AccountId,
            amount: T::Balance,
        ) -> DispatchResult {
            let origin = ensure_signed(origin)?;
            Self::do_burn(id, &who, amount, Some(origin))?;
            Ok(())
        }
    }

    //todo genesis
    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub asset: Vec<(
            T::AssetId,
            (
                T::AccountId,
                T::AccountId,
                T::Balance,
                u32,
                Vec<u8>,
                Vec<u8>,
            ),
        )>,
        pub balances: Vec<(T::AssetId, T::AccountId, T::Balance)>,
        pub init__erc20_token: bool,
    }

    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                asset: vec![],
                balances: vec![],
                init__erc20_token: false,
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            for (asset_id, (admin, issuer, supply, accounts, name, symbol)) in self.asset.iter() {
                let asset_details: AssetDetails<T> = AssetDetails {
                    admin: admin.clone(),
                    issuer: issuer.clone(),
                    name: name.to_vec(),
                    symbol: symbol.to_vec(),
                    supply: *supply,
                    accounts: *accounts,
                };

                Asset::<T>::insert(asset_id, asset_details);
            }

            for (asset_id, account_id, balance) in self.balances.iter() {
                BalanceOf::<T>::insert(asset_id, account_id, balance);
            }
        }
    }
}
