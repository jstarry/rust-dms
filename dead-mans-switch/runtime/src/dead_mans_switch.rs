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
		BeneficiaryUpdated(AccountId, AccountId, AccountId),
		BlockDelayUpdated(AccountId, BlockNumber, BlockNumber),
		PingedAlive(AccountId, BlockNumber),
	}
);

decl_storage! {
    trait Store for Module<T: Trait> as DeadMansSwitchModule {
        Contracts get(contract): map T::AccountId => Contract<T::AccountId, T::BlockNumber>;

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

            <TrustorsArray<T>>::insert((beneficiary.clone(), trustors_count), &sender);
            <TrustorsCount<T>>::insert(&beneficiary, new_trustors_count);
            <TrustorsIndex<T>>::insert(&sender, trustors_count);

            Self::deposit_event(RawEvent::CreatedContract(sender, beneficiary, block_delay));

            Ok(())
        }

        pub fn update_beneficiary(origin, beneficiary: T::AccountId) -> Result {
            let sender = ensure_signed(origin)?;

            ensure!(<Contracts<T>>::exists(&sender), "You do not have a current contract");
            ensure!(sender != beneficiary, "You cannot use yourself as your beneficiary");
            ensure!(<TrustorsIndex<T>>::exists(&sender), "Your account is in a bad state");

            let mut current_contract = Self::contract(&sender);
            let prev_beneficiary = current_contract.beneficiary;
            ensure!(prev_beneficiary != beneficiary, "Your beneficiary is already set to this account");

            let trustors_count = Self::trustors_count(&beneficiary);
            let trustors_index = trustors_count;
            let new_trustors_count = trustors_count.checked_add(1)
                .ok_or("Overflow adding a new trustor for this beneficiary")?;

            let prev_beneficiary_trustors_count = Self::trustors_count(&prev_beneficiary);
            let new_prev_beneficiary_trustors_count = prev_beneficiary_trustors_count.checked_sub(1)
                .ok_or("Underflow removing trustor for previous beneficiary")?;

            current_contract.beneficiary = beneficiary.clone();
            <Contracts<T>>::insert(&sender, &current_contract);

            // prepare to remove the last trustor from the previous beneficiary's list
            let mut prev_trustor_index = <TrustorsIndex<T>>::get(&sender);
            if prev_trustor_index != new_prev_beneficiary_trustors_count {
                let last_trustor_id = <TrustorsArray<T>>::get((prev_beneficiary.clone(), new_prev_beneficiary_trustors_count));
                <TrustorsArray<T>>::insert((prev_beneficiary.clone(), prev_trustor_index), &last_trustor_id);
                <TrustorsIndex<T>>::insert(last_trustor_id, prev_trustor_index);
                prev_trustor_index = new_prev_beneficiary_trustors_count;
            }

            <TrustorsIndex<T>>::insert(&sender, trustors_index);
            <TrustorsArray<T>>::remove((prev_beneficiary.clone(), prev_trustor_index));
            <TrustorsArray<T>>::insert((beneficiary.clone(), trustors_index), &sender);

            <TrustorsCount<T>>::insert(&prev_beneficiary, new_prev_beneficiary_trustors_count);
            <TrustorsCount<T>>::insert(&beneficiary, new_trustors_count);

            Self::deposit_event(RawEvent::BeneficiaryUpdated(sender, prev_beneficiary, beneficiary));

            Ok(())
        }

        pub fn update_block_delay(origin, block_delay: T::BlockNumber) -> Result {
            let sender = ensure_signed(origin)?;

            ensure!(<Contracts<T>>::exists(&sender), "You do not have a current contract");

            let min_block_delay = <MinBlockDelay<T>>::get();
            ensure!(block_delay >= min_block_delay, "Your block delay is too short");

            let current_block = <system::Module<T>>::block_number();
            let execution_block = current_block + block_delay;

            let mut current_contract = Self::contract(&sender);
            let prev_block_delay = current_contract.block_delay;
            current_contract.block_delay = block_delay.clone();
            current_contract.execution_block = execution_block.clone();
            <Contracts<T>>::insert(&sender, &current_contract);

            Self::deposit_event(RawEvent::BlockDelayUpdated(sender, prev_block_delay, block_delay));

            Ok(())
        }

        pub fn ping_alive(origin) -> Result {
            let sender = ensure_signed(origin)?;

            ensure!(<Contracts<T>>::exists(&sender), "You do not have a current contract");

            let mut current_contract = Self::contract(&sender);
            let current_block = <system::Module<T>>::block_number();
            let execution_block = current_block + current_contract.block_delay;
            current_contract.execution_block = execution_block.clone();
            <Contracts<T>>::insert(&sender, &current_contract);

            Self::deposit_event(RawEvent::PingedAlive(sender, execution_block));

            Ok(())
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
    type System = system::Module<Test>;

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
            // create a contract to give access to account #2 after 10 blocks of inactivity
            assert_ok!(DMS::create_contract(Origin::signed(1), 2, 10));

            let contract = DMS::contract(1);
            assert_eq!(contract.block_delay, 10);
            assert_eq!(contract.execution_block, 11);

            // check that account #2 has one trustor
            assert_eq!(DMS::trustors_count(2), 1);

            // check that account #1 does not have a trustor
            assert_eq!(DMS::trustors_count(1), 0);

            // check that account #1 is trustor of account #2
            assert_eq!(DMS::trustors_by_index((2, 0)), 1);
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

    #[test]
    fn update_beneficiary_should_work() {
        with_externalities(&mut build_ext(), || {
            // create contracts to give access to account #1 after 10 blocks of inactivity
            assert_ok!(DMS::create_contract(Origin::signed(10), 1, 10));
            assert_ok!(DMS::create_contract(Origin::signed(20), 1, 10));

            // update beneficiary from account #1 to account #2
            assert_ok!(DMS::update_beneficiary(Origin::signed(20), 2));

            // check that account #2 has a trustor
            assert_eq!(DMS::trustors_count(2), 1);

            // check that account #1 only has one trustor
            assert_eq!(DMS::trustors_count(1), 1);

            // check that account #20 is a trustor of account #2
            assert_eq!(DMS::trustors_by_index((2, 0)), 20);

            // check that account #10 is a trustor of account #1
            assert_eq!(DMS::trustors_by_index((1, 0)), 10);
        });
    }

    #[test]
    fn update_beneficiary_should_fail() {
        with_externalities(&mut build_ext(), || {
            // create contracts to give access to account #1 after 10 blocks of inactivity
            assert_ok!(DMS::create_contract(Origin::signed(10), 1, 10));
            assert_ok!(DMS::create_contract(Origin::signed(20), 1, 10));

            // check that the updated beneficiary needs to be different
            assert_noop!(
                DMS::update_beneficiary(Origin::signed(20), 1),
                "Your beneficiary is already set to this account"
            );

            // check that trustors without beneficiaries cannot update
            assert_noop!(
                DMS::update_beneficiary(Origin::signed(30), 1),
                "You do not have a current contract"
            );

            // check that beneficiaries cannot be set to be the same as the trustor
            assert_noop!(
                DMS::update_beneficiary(Origin::signed(10), 10),
                "You cannot use yourself as your beneficiary"
            );
        });
    }

    #[test]
    fn update_block_delay_should_work() {
        with_externalities(&mut build_ext(), || {
            // create contract to give access to account #1 after 10 blocks of inactivity
            assert_ok!(DMS::create_contract(Origin::signed(10), 1, 10));

            // update block delay from 10 to 20
            assert_ok!(DMS::update_block_delay(Origin::signed(10), 20));

            let contract = DMS::contract(10);
            assert_eq!(contract.block_delay, 20);
            assert_eq!(contract.execution_block, 21);
        });
    }

    #[test]
    fn update_block_delay_should_fail() {
        with_externalities(&mut build_ext(), || {
            // check that trustors without beneficiaries cannot update block delay
            assert_noop!(
                DMS::update_block_delay(Origin::signed(10), 10),
                "You do not have a current contract"
            );
        });
    }

    #[test]
    fn ping_alive_should_work() {
        with_externalities(&mut build_ext(), || {
            // create contract to give access to account #1 after 10 blocks of inactivity
            assert_ok!(DMS::create_contract(Origin::signed(10), 1, 10));

            System::set_block_number(2);

            assert_ok!(DMS::ping_alive(Origin::signed(10)));

            let contract = DMS::contract(10);
            assert_eq!(contract.execution_block, 12);
        });
    }

    #[test]
    fn ping_alive_should_fail() {
        with_externalities(&mut build_ext(), || {
            // check that trustors without beneficiaries cannot ping
            assert_noop!(
                DMS::ping_alive(Origin::signed(10)),
                "You do not have a current contract"
            );
        });
    }
}
