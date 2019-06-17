use super::SuperCall;
use parity_codec::{Decode, Encode};
use runtime_primitives::traits::As;
use support::dispatch::{Dispatchable, Result};
use support::{decl_event, decl_module, decl_storage, ensure, StorageMap, StorageValue};
use system::{ensure_signed, RawOrigin};

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Contract<AccountId, BlockNumber> {
    beneficiary: AccountId,
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

        // Common way of implementing vectors with maps in substrate
        TrustorsArray get(trustors_by_index): map (T::AccountId, u64) => T::AccountId;
        TrustorsCount get(trustors_count): map T::AccountId => u64;
        TrustorsIndex: map T::AccountId => u64;

        MinBlockDelay: T::BlockNumber = T::BlockNumber::sa(10);
    }
}

decl_module! {
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

        pub fn create_contract(origin, beneficiary: T::AccountId, block_delay: T::BlockNumber) -> Result {
            let sender = ensure_signed(origin)?;

            ensure!(!<Contracts<T>>::exists(&sender), "You can only have one contract");
            ensure!(sender != beneficiary, "You cannot use yourself as your beneficiary");

            let min_block_delay = <MinBlockDelay<T>>::get();
            ensure!(block_delay >= min_block_delay, "Your block delay is too short");

            let trustors_count = Self::trustors_count(&beneficiary);
            let new_trustors_count = trustors_count.checked_add(1)
                .ok_or("Overflow adding a new trustor for this beneficiary")?;

            let current_block = <system::Module<T>>::block_number();
            let execution_block = current_block + block_delay;
            let contract = Contract {
                beneficiary: beneficiary.clone(),
                block_delay,
                execution_block,
            };
            <Contracts<T>>::insert(&sender, &contract);

            <TrustorsArray<T>>::insert((sender.clone(), trustors_count), &sender);
            <TrustorsCount<T>>::insert(&beneficiary, new_trustors_count);
            <TrustorsIndex<T>>::insert(&sender, trustors_count);

            Self::deposit_event(RawEvent::CreatedContract(sender, beneficiary, block_delay));

            Ok(())
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
    use support::{assert_noop, assert_ok, impl_outer_origin};

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

    type DMS = Module<Test>;

    fn build_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
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
    fn act_as_should_work() {
        with_externalities(&mut build_ext(), || {
            let super_call = SuperCall::Balances(balances::Call::transfer(0, 100));
            assert_ok!(DMS::act_as(Origin::signed(0), 1, super_call));
        });
    }

    #[test]
    fn create_contract_should_work() {
        with_externalities(&mut build_ext(), || {
            // create a contract to give access to account #1 after 10 blocks of inactivity
            assert_ok!(DMS::create_contract(Origin::signed(0), 1, 10));

            // check that account #1 has one trustor
            assert_eq!(DMS::trustors_count(1), 1);

            // check that account #0 does not have a trustor
            assert_eq!(DMS::trustors_count(0), 0);

            // check that account #0 is trustor of account #1
            assert_eq!(DMS::trustors_by_index((1, 0)), 0);
        });
    }

    #[test]
    fn create_contract_should_fail() {
        with_externalities(&mut build_ext(), || {
            // create a contract to give access to account #1 after 10 blocks of inactivity
            assert_ok!(DMS::create_contract(Origin::signed(0), 1, 10));

            // check that account cannot create another contract
            assert_noop!(
                DMS::create_contract(Origin::signed(0), 2, 10),
                "You can only have one contract"
            );

            // check that short delay is disallowed
            assert_noop!(
                DMS::create_contract(Origin::signed(1), 2, 0),
                "Your block delay is too short"
            );

            // check that account cannot set themselves as beneficiary
            assert_noop!(
                DMS::create_contract(Origin::signed(1), 1, 0),
                "You cannot use yourself as your beneficiary"
            );
        });
    }
}
