// Copyright 2026 PRAGMA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::cmp::Ordering;

use crate::{Network, StakePayload};

/// A stake address with ordering the way Plutus expects withdrawal keys to be sorted.
///
/// A wrapper around [`crate::StakeAddress`] to provide a custom [`Ord`] implementation.
/// Wrapping the address makes a `BTreeMap<StakeAddress, _>` iterate, and therefore serialize,
/// in the order a script expects. Equality is defined to agree with this ordering.
#[derive(Clone, Debug)]
pub struct StakeAddress(pub crate::StakeAddress);

impl From<StakeAddress> for crate::StakeAddress {
    fn from(value: StakeAddress) -> Self {
        value.0
    }
}

impl Ord for StakeAddress {
    /// Plutus canonically expects stake addresses to be sorted by network,
    /// then script credentials > public key credentials,
    /// and finally lexicographical ordering of hash bytes.
    ///
    ///
    /// [Aiken reference implementation](https://github.com/aiken-lang/aiken/blob/a8c032935dbaf4a1140e9d8be5c270acd32c9e8c/crates/uplc/src/tx/script_context.rs#L1112)
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        fn network_tag(network: Network) -> u8 {
            match network {
                Network::Testnet => 0,
                Network::Mainnet => 1,
                Network::Other(tag) => tag,
            }
        }

        if self.0.network() != other.0.network() {
            return network_tag(self.0.network()).cmp(&network_tag(other.0.network()));
        }

        match (self.0.payload(), other.0.payload()) {
            (StakePayload::Script(..), StakePayload::Stake(..)) => Ordering::Less,
            (StakePayload::Stake(..), StakePayload::Script(..)) => Ordering::Greater,
            (StakePayload::Script(hash_a), StakePayload::Script(hash_b)) => hash_a.cmp(hash_b),
            (StakePayload::Stake(hash_a), StakePayload::Stake(hash_b)) => hash_a.cmp(hash_b),
        }
    }
}

impl PartialOrd for StakeAddress {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for StakeAddress {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for StakeAddress {}

#[cfg(test)]
mod tests {
    use proptest::{
        prelude::{Just, Strategy, any, prop},
        prop_assert, prop_oneof, proptest,
    };

    use super::*;
    use crate::new_stake_address;

    fn network_strategy() -> impl Strategy<Value = Network> {
        prop_oneof![Just(Network::Testnet), Just(Network::Mainnet), any::<u8>().prop_map(Network::from),]
    }

    fn stake_address_strategy() -> impl Strategy<Value = StakeAddress> {
        (prop::bool::ANY, any::<[u8; 28]>(), network_strategy()).prop_map(|(is_script, hash_bytes, network)| {
            let delegation: StakePayload = if is_script {
                StakePayload::Script(hash_bytes.into())
            } else {
                StakePayload::Stake(hash_bytes.into())
            };

            StakeAddress(new_stake_address(network, delegation))
        })
    }

    #[test]
    fn proptest_stake_address_ordering() {
        proptest!(|(addresses in prop::collection::vec(stake_address_strategy(), 20..100))| {
            let mut sorted = addresses.clone();
            sorted.sort();


            for window in sorted.windows(2) {
                let a = &window[0];
                let b = &window[1];

                fn network_tag(network: Network) -> u8 {
                    match network {
                        Network::Testnet => 0,
                        Network::Mainnet => 1,
                        Network::Other(tag) => tag,
                    }
                }

                let net_a = a.0.network();
                let net_b = b.0.network();


                // We sort by network first (testnet, mainnet, other by tag)
                if net_a != net_b {
                    prop_assert!(
                        network_tag(net_a) < network_tag(net_b),
                        "Network ordering violated: {:?} should be < {:?}",
                        network_tag(net_a),
                        network_tag(net_b)
                    );
                } else {
                    match (a.0.payload(), b.0.payload()) {
                        // Script < Stake
                        (StakePayload::Script(_), StakePayload::Stake(_)) => {
                            // This is correct
                        }
                        (StakePayload::Stake(_), StakePayload::Script(_)) => {
                            prop_assert!(false, "Payload type ordering violated: Stake should not come before Script");
                        }
                        // Same payload compare bytes
                        (StakePayload::Script(h1), StakePayload::Script(h2)) => {
                            prop_assert!(
                                h1 <= h2,
                                "Script hash ordering violated: {:?} should be <= {:?}",
                                h1, h2
                            );
                        }
                        (StakePayload::Stake(h1), StakePayload::Stake(h2)) => {
                            prop_assert!(
                                h1 <= h2,
                                "Stake hash ordering violated: {:?} should be <= {:?}",
                                h1, h2
                            );
                        }
                    }
                }
            }
        });
    }
}
