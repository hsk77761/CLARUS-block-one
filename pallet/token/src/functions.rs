//! Functions for the Assets pallet.

use super::*;
use frame_support::{dispatch::DispatchResult, ensure};
use sp_runtime::{
    traits::{CheckedAdd, CheckedSub, Zero},
    DispatchError,
};

// implementation of mudule
// utility and private functions
// if marked public, accessible by other modules
impl<T: Config> Pallet<T> {
    // the ERC20 standard transfer function
    // internal
    pub(super) fn _transfer(
        id: T::AssetId,
        from: T::AccountId,
        to: T::AccountId,
        amount: T::Balance,
    ) -> DispatchResult {
        ensure!(
            <BalanceOf<T>>::contains_key(id, from.clone()),
            Error::<T>::AccountDoesNotOwnThisToken
        );

        Asset::<T>::try_mutate(id, |maybe_details| -> DispatchResult {
            let details = maybe_details.as_mut().ok_or(Error::<T>::UnknownAsset)?;

            <BalanceOf<T>>::try_mutate_exists(id, &from, |maybe_account| -> DispatchResult {
                let mut balance = maybe_account.take().unwrap_or_default();
                ensure!(balance >= amount, Error::<T>::InsufficientTransfer);
                // Make the debit.
                balance = balance
                    .checked_sub(&amount)
                    .ok_or(Error::<T>::TransferUnderFlow)?;
                *maybe_account = if balance.is_zero() {
                    details.accounts = details
                        .accounts
                        .checked_sub(1)
                        .ok_or(Error::<T>::AccountsUnderflow)?;
                    None
                } else {
                    Some(balance)
                };
                Ok(())
            })?;

            <BalanceOf<T>>::try_mutate(id, &to, |t| -> DispatchResult {
                let new_balance = t.checked_add(&amount).ok_or(Error::<T>::TransferOverFlow)?;
                if t.is_zero() {
                    Self::new_account(&to, details)?;
                }
                *t = new_balance;
                Ok(())
            })?;

            Ok(())
        })?;

        Self::deposit_event(Event::Transferred {
            asset_id: id,
            from,
            to,
            amount,
        });
        Ok(())
    }

    pub fn _transfer_all(id: T::AssetId, from: T::AccountId, to: T::AccountId) -> DispatchResult {
        if <BalanceOf<T>>::contains_key(id, &from) {
            Asset::<T>::try_mutate(id, |maybe_details| -> DispatchResult {
                let details = maybe_details.as_mut().ok_or(Error::<T>::UnknownAsset)?;

                <BalanceOf<T>>::try_mutate_exists(id, &from, |maybe_account| -> DispatchResult {
                    let mut balance = maybe_account.take().unwrap_or_default();
                    ensure!(!balance.is_zero(), Error::<T>::InsufficientTransfer);
                    <BalanceOf<T>>::try_mutate(id, &to, |t| -> DispatchResult {
                        let new_balance = t
                            .checked_add(&balance)
                            .ok_or(Error::<T>::TransferOverFlow)?;
                        if t.is_zero() {
                            Self::new_account(&to, details)?;
                        }
                        *t = new_balance;
                        Ok(())
                    })?;
                    Self::deposit_event(Event::Transferred {
                        asset_id: id,
                        from: from.clone(),
                        to,
                        amount: balance,
                    });

                    // Make the debit.
                    balance = balance
                        .checked_sub(&balance)
                        .ok_or(Error::<T>::TransferUnderFlow)?;
                    *maybe_account = if balance.is_zero() {
                        details.accounts = details
                            .accounts
                            .checked_sub(1)
                            .ok_or(Error::<T>::AccountsUnderflow)?;
                        None
                    } else {
                        Some(balance)
                    };

                    Ok(())
                })?;

                Ok(())
            })?;
        }

        Ok(())
    }

    /// Increases the asset `id` balance of `beneficiary` by `amount`.
    ///
    /// This alters the registered supply of the asset and emits an event.
    ///
    /// Will return an error or will increase the amount by exactly `amount`.
    pub(super) fn do_mint(
        id: T::AssetId,
        beneficiary: &T::AccountId,
        amount: T::Balance,
        maybe_check_issuer: Option<T::AccountId>,
    ) -> DispatchResult {
        Self::increase_balance(id, beneficiary, amount, |details| -> DispatchResult {
            if let Some(check_issuer) = maybe_check_issuer {
                ensure!(&check_issuer == &details.issuer, Error::<T>::NoPermission);
            }

            details.supply = details
                .supply
                .checked_add(&amount)
                .ok_or(Error::<T>::MintOverFlow)?;
            Ok(())
        })?;
        Self::deposit_event(Event::Issued {
            asset_id: id,
            owner: beneficiary.clone(),
            balance: amount,
        });
        Ok(())
    }

    /// Increases the asset `id` balance of `beneficiary` by `amount`.
    ///
    /// LOW-LEVEL: Does not alter the supply of asset or emit an event. Use `do_mint` if you need
    /// that. This is not intended to be used alone.
    ///
    /// Will return an error or will increase the amount by exactly `amount`.
    pub(super) fn increase_balance(
        id: T::AssetId,
        beneficiary: &T::AccountId,
        amount: T::Balance,
        check: impl FnOnce(&mut AssetDetails<T>) -> DispatchResult,
    ) -> DispatchResult {
        if amount.is_zero() {
            return Ok(());
        }
        Asset::<T>::try_mutate(id, |maybe_details| -> DispatchResult {
            // TODO: return some meaningful error here.
            let details = maybe_details.as_mut().ok_or(Error::<T>::UnknownAsset)?;

            check(details)?;

            <BalanceOf<T>>::try_mutate(id, beneficiary, |t| -> DispatchResult {
                let new_balance = t.checked_add(&amount).ok_or(Error::<T>::BalanceOverflow)?;
                if t.is_zero() {
                    Self::new_account(beneficiary, details)?;
                }
                *t = new_balance;
                Ok(())
            })?;
            Ok(())
        })?;
        Ok(())
    }

    pub(super) fn new_account(_who: &T::AccountId, d: &mut AssetDetails<T>) -> DispatchResult {
        let accounts = d
            .accounts
            .checked_add(1)
            .ok_or(Error::<T>::AccountsOverflow)?;
        d.accounts = accounts;
        Ok(())
    }

    /// Reduces asset `id` balance of `target` by `amount`. Flags `f` can be given to alter whether
    /// it attempts a `best_effort` or makes sure to `keep_alive` the account.
    ///
    /// This alters the registered supply of the asset and emits an event.
    ///
    /// Will return an error and do nothing or will decrease the amount and return the amount
    /// reduced by.
    pub(super) fn do_burn(
        id: T::AssetId,
        target: &T::AccountId,
        amount: T::Balance,
        maybe_check_admin: Option<T::AccountId>,
    ) -> Result<T::Balance, DispatchError> {
        let actual = Self::decrease_balance(id, target, amount, |actual, details| {
            // Check admin rights.
            if let Some(check_admin) = maybe_check_admin {
                ensure!(&check_admin == &details.issuer, Error::<T>::NoPermission);
            }

            details.supply = details
                .supply
                .checked_sub(&actual)
                .ok_or(Error::<T>::BurnUnderflow)?;

            Ok(())
        })?;
        Self::deposit_event(Event::Burned {
            asset_id: id,
            owner: target.clone(),
            balance: actual,
        });
        Ok(actual)
    }

    /// Reduces asset `id` balance of `target` by `amount`. Flags `f` can be given to alter whether
    /// it attempts a `best_effort` or makes sure to `keep_alive` the account.
    ///
    /// LOW-LEVEL: Does not alter the supply of asset or emit an event. Use `do_burn` if you need
    /// that. This is not intended to be used alone.
    ///
    /// Will return an error and do nothing or will decrease the amount and return the amount
    /// reduced by.
    pub(super) fn decrease_balance(
        id: T::AssetId,
        target: &T::AccountId,
        amount: T::Balance,
        check: impl FnOnce(T::Balance, &mut AssetDetails<T>) -> DispatchResult,
    ) -> Result<T::Balance, DispatchError> {
        if amount.is_zero() {
            return Ok(amount);
        }

        Asset::<T>::try_mutate(id, |maybe_details| -> DispatchResult {
            let details = maybe_details.as_mut().ok_or(Error::<T>::UnknownAsset)?;

            check(amount, details)?;

            <BalanceOf<T>>::try_mutate_exists(id, target, |maybe_account| -> DispatchResult {
                let mut balance = maybe_account.take().unwrap_or_default();
                ensure!(balance >= amount, Error::<T>::InsufficientBurn);
                // Make the debit.
                balance = balance
                    .checked_sub(&amount)
                    .ok_or(Error::<T>::BalanceUnderflow)?;
                *maybe_account = if balance.is_zero() {
                    details.accounts = details
                        .accounts
                        .checked_sub(1)
                        .ok_or(Error::<T>::AccountsUnderflow)?;
                    None
                } else {
                    Some(balance)
                };
                Ok(())
            })?;

            Ok(())
        })?;

        Ok(amount)
    }
}
