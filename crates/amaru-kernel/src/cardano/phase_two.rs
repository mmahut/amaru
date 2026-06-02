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

//! Intermediate representation types used during phase-two (Plutus) validation.
//!
//! These types are an in-memory representation of a Cardano transaction tailored for
//! Plutus script execution. They are not exact mirrors of the on-chain ledger types,
//! and may carry phase-two-specific simplifications (e.g. bootstrap addresses are
//! skipped, mints carry a different value type, etc.).
//!
//! The corresponding `ToPlutusData` serialization logic lives in the `amaru-plutus`
//! crate.

pub mod certificate;
pub mod datums;
pub mod mint;
pub mod output_reference;
pub mod redeemers;
pub mod required_signers;
pub mod script;
pub mod script_context;
pub mod script_info;
pub mod stake_address;
pub mod time_range;
pub mod transaction_output;
pub mod tx_info;
pub mod utxos;
pub mod value;
pub mod votes;
pub mod withdrawals;
