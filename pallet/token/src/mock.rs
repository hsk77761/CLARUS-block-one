use frame_support::{
    sp_runtime::{self, testing::*, traits::*},
    traits::*,
    WeakBoundedVec,
};
use frame_system::mocking::*;
use pallet_balances::BalanceLock;

use crate as pallet_token;

type Block = MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test
    {
        System: frame_system,
        Balances: pallet_balances,
        Token: pallet_token,
    }
);

pub type UInt = u64;
pub type Balance = u128;
pub type AccountId = UInt;

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
}
impl frame_system::Config for Test {
    type BaseCallFilter = Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
    type DbWeight = ();
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
    type OnSetCode = ();
    type MaxConsumers = ConstU32<16>;
    type Block = Block;
    type Nonce = UInt;
}

parameter_types! {
    pub const ExistentialDeposit: Balance = 1;
    pub const MaxLocks: u32 = 10;
    pub const MaxReserves: u32 = 10;
    pub const MaxHolds: u32 = 10;
    pub const MaxFreezes: u32 = 10;
}
impl pallet_balances::Config for Test {
    type Balance = Balance;
    type DustRemoval = ();
    type RuntimeEvent = RuntimeEvent;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxLocks = MaxLocks;
    type MaxReserves = MaxReserves;
    type ReserveIdentifier = [u8; 8];
    type RuntimeHoldReason = RuntimeHoldReason;
    type FreezeIdentifier = ();
    type MaxHolds = MaxHolds;
    type MaxFreezes = MaxFreezes;
}

// Provides access to `pallet_balances::Locks` storage.
pub struct LocksStore<T>(PhantomData<T>);
pub type WeakBoundecVecOf<T> = WeakBoundedVec<
    BalanceLock<<T as pallet_balances::Config>::Balance>,
    <T as pallet_balances::Config>::MaxLocks,
>;

impl<T> StoredMap<T::AccountId, WeakBoundecVecOf<T>> for LocksStore<T>
where
    T: pallet_balances::Config,
{
    fn get(k: &T::AccountId) -> WeakBoundecVecOf<T> {
        pallet_balances::Locks::<T>::get(k)
    }

    fn try_mutate_exists<R, E: From<sp_runtime::DispatchError>>(
        k: &T::AccountId,
        f: impl FnOnce(&mut Option<WeakBoundecVecOf<T>>) -> Result<R, E>,
    ) -> Result<R, E> {
        pallet_balances::Locks::<T>::try_mutate_exists(k, f)
    }
}

impl pallet_token::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type AssetId = u64;
    type WeightInfo = pallet_token::weights::SubstrateWeight<Test>;
}
