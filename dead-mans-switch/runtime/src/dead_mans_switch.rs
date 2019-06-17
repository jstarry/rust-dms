use super::SuperCall;
use parity_codec::{Decode, Encode};
use support::dispatch::{Dispatchable, Result};
use support::{decl_event, decl_module, decl_storage, StorageValue};
use system::{ensure_signed, RawOrigin};

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Contract<AccountId, BlockNumber> {
    benificiary: AccountId,
    block_delay: BlockNumber,
    execution_block: BlockNumber,
}

pub trait Trait: balances::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_event!(
	pub enum Event<T>
	where
		<T as system::Trait>::AccountId,
		<T as system::Trait>::BlockNumber
	{
		ActedAs(AccountId, AccountId),
		CreatedContract(AccountId, AccountId, BlockNumber),
		BeneficiaryUpdated(AccountId, AccountId),
		PingedAlive(AccountId),
	}
);

decl_storage! {
    trait Store for Module<T: Trait> as DeadMansSwitchModule {
        Contracts: map T::AccountId => Contract<T::AccountId, T::BlockNumber>;

        TrusteesArray get(trustees_by_index): map (T::AccountId, u64) => T::AccountId;
        TrusteesCount get(trustees_count): map T::AccountId => u64;
        TrusteesIndex: map T::AccountId => u64;
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

        pub fn create_contract(origin, beneficiary: T::AccountId, delay: T::BlockNumber) -> Result {
            unimplemented!()
        }

        pub fn update_beneficiary(origin, beneficiary: T::AccountId) -> Result {
            unimplemented!()
        }

        pub fn ping_alive(origin) -> Result {
            unimplemented!()
        }
    }
}

/// tests for this module
#[cfg(test)]
mod tests {
    use super::*;

    use primitives::{Blake2Hasher, H256};
    use runtime_io::with_externalities;
    use runtime_primitives::{
        testing::{Digest, DigestItem, Header},
        traits::{BlakeTwo256, IdentityLookup},
        BuildStorage,
    };
    use support::{assert_ok, impl_outer_origin};

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
        let mut t = system::GenesisConfig::<Test>::default()
            .build_storage()
            .unwrap()
            .0;
        t.extend(
            balances::GenesisConfig::<Test> {
                balances: vec![(0, 50), (1, 100)],
                vesting: Default::default(),
                existential_deposit: Default::default(),
                creation_fee: Default::default(),
                transaction_base_fee: Default::default(),
                transaction_byte_fee: Default::default(),
                transfer_fee: Default::default(),
            }
            .build_storage()
            .unwrap()
            .0,
        );
        t.into()
    }

    #[test]
    fn it_works_for_default_value() {
        with_externalities(&mut new_test_ext(), || {
            let super_call = SuperCall::Balances(balances::Call::transfer(0, 100));
            assert_ok!(DeadMansSwitchModule::act_as(
                Origin::signed(0),
                1,
                super_call
            ));
        });
    }
}
