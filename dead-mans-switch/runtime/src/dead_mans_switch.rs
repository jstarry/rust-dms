use support::{decl_module, decl_storage, decl_event, StorageValue};
use support::dispatch::{Result, Dispatchable};
use system::{ensure_signed, RawOrigin};
use super::SuperCall;


/// The module's configuration trait.
pub trait Trait: balances::Trait {
	// TODO: Add other types and constants required configure this module.

	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

/// This module's storage items.
decl_storage! {
	trait Store for Module<T: Trait> as DeadMansSwitchModule {
	}
}

decl_module! {
	/// The module declaration.
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {

		fn deposit_event<T>() = default;

		pub fn act_as(origin, r#as: T::AccountId, call: SuperCall<T>) -> Result {
			let who = ensure_signed(origin)?;

			// TODO check if who can act as 'as'

			match call {
				super::SuperCall::Balances(c) => c.dispatch(RawOrigin::Signed(r#as.clone()).into()),
			}?;

			Self::deposit_event(RawEvent::ActedAs(who, r#as));
			Ok(())
		}
	}
}

decl_event!(
	pub enum Event<T> where AccountId = <T as system::Trait>::AccountId {
		ActedAs(AccountId, AccountId),
	}
);

/// tests for this module
#[cfg(test)]
mod tests {
	use super::*;

	use runtime_io::with_externalities;
	use primitives::{H256, Blake2Hasher};
	use support::{impl_outer_origin, assert_ok};
	use runtime_primitives::{
		BuildStorage,
		traits::{BlakeTwo256, IdentityLookup},
		testing::{Digest, DigestItem, Header}
	};

	impl_outer_origin! {
		pub enum Origin for Test {}
	}

	// For testing the module, we construct most of a mock runtime. This means
	// first constructing a configuration type (`Test`) which `impl`s each of the
	// configuration traits of modules we want to use.
	#[derive(Clone, Eq, PartialEq)]
	pub struct Test;
	impl system::Trait for Test {
		type Origin = Origin;
		type Index = u64;
		type BlockNumber = u64;
		type Hash = H256;
		type Hashing = BlakeTwo256;
		type Digest = Digest;
		type AccountId = u64;
		type Lookup = IdentityLookup<Self::AccountId>;
		type Header = Header;
		type Event = ();
		type Log = DigestItem;
	}

    impl balances::Trait for Test {
        type Balance = u64;
        type OnFreeBalanceZero = ();
        type OnNewAccount = ();
        type Event = ();
        type TransactionPayment = ();
        type TransferPayment = ();
        type DustRemoval = ();
    }

	impl Trait for Test {
		type Event = ();
	}
	type DeadMansSwitchModule = Module<Test>;

	fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
		let mut t = system::GenesisConfig::<Test>::default().build_storage().unwrap().0;
		t.extend(balances::GenesisConfig::<Test> {
			balances: vec![(0, 50), (1, 100)],
			vesting: Default::default(),
			existential_deposit: Default::default(),
			creation_fee: Default::default(),
			transaction_base_fee: Default::default(),
			transaction_byte_fee: Default::default(),
			transfer_fee: Default::default(),
		}.build_storage().unwrap().0);
		t.into()
	}

	#[test]
	fn it_works_for_default_value() {
		with_externalities(&mut new_test_ext(), || {
			let super_call = SuperCall::Balances(balances::Call::transfer(0, 100));
			assert_ok!(DeadMansSwitchModule::act_as(Origin::signed(0), 1, super_call));
		});
	}
}
