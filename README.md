# Design

## Spec

Knowing that Dock is moving over to the Substrate framework, I decided to implement my solution as a Substrate Runtime Module.

This module exposes an api which allows users to setup a "contract" which specifies how long they want to delay the switch execution and to whom they wish to hand over control of their account.

Allowing users to act as other users was tricky due to my inexperience with Substrate but after perusing the "contract" module, I realized that I could create valid "signed" transactions on behalf of users within my module. So, in order to act as someone else, a user can specify 1) another user's address and 2) an unsigned transaction while calling my module.

This module is intended to be ready as-is for a UI or CLI to interact with it. Most notably, the module maintains data structures to allow beneficiaries to look up their corresponding trustors.

## Assumptions

1. Control over an account is sufficient (private key knowledge is not required)

1. The network is not able to be compromised such that a user's ping alive transactions are ignored. If this were possible, a beneficiary could maliciously stifle their trustor's transactions so that they could take over the trustor's account.

1. Even after the switch is expired, it is still possible for the original user (trustor) to regain exclusive access to their account but they cannot revert any transactions their beneficiary may have made.

1. Min/max block delay will not be updated. I could certainly handle this case but felt it was out of scope.

1. Trustors cannot assign themselves to be their own beneficiaries.

1. Only one beneficiary can be chosen at a time. Handling multiple beneficiaries sounded fun but out of scope.

1. Only calls to the `balances` module can be made by the beneficiary. I initially added the groundwork for supporting other module calls but decided to decrease the scope to make the code cleaner and simpler.

1. UI is out of scope. Unfortunately this means there is no way (that I know of) to interact with my module. I hope that the tests are sufficient to show the logic and operation of the module. But I would honestly be really happy to take on the task of hacking on a simple UI to make this interactable if that would be helpful.

## Notes

I originally intended to implement the task in a way to actually hand over the private key to the beneficiary. But I was unable to come up with a satisfactory solution. I would either need to trust some external service to store private keys (ideally encrypted with the beneficiaries keys) or make a cryptographic puzzle that would be hard enough such that it would take some large amount of compute power (and thus some rough estimate of time delay) to crack open a encrypted payload containing a beneficiary encrypted trustor private key.

## Building

Install Rust:

```bash
curl https://sh.rustup.rs -sSf | sh
```

Install required tools:

```bash
./scripts/init.sh
```

Build the WebAssembly binary:

```bash
./scripts/build.sh
```

Build all native code:

```bash
cargo build -- release
```

## Run

You can start a development chain with:

```bash
cargo run -- --dev
```

You can run tests with:

```bash
cargo test -p dead-mans-switch-runtime
```

You can generate docs with:

```bash
cargo doc -p dead-mans-switch-runtime
open target/doc/dead_mans_switch_runtime/dead_mans_switch/index.html
```
