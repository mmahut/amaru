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

use std::borrow::Cow;

use super::{script::Script, value::Value};
use crate::{Address, MemoizedDatum, MemoizedTransactionOutput};

/// The Plutus-facing view of a transaction output.
///
/// A borrowed representation of [`MemoizedTransactionOutput`] with each part already in its
/// phase-two form: the [`Value`], the borrowed [`MemoizedDatum`] (none/hash/inline), and an
/// optional reference [`Script`]. The address is held as a [`Cow`] so it can borrow from
/// the source output rather than clone.
#[derive(Debug, Clone)]
pub struct TransactionOutput<'a> {
    pub address: Cow<'a, Address>,
    pub value: Value<'a>,
    pub datum: &'a MemoizedDatum,
    pub script: Option<Script<'a>>,
}

impl<'a> From<&'a MemoizedTransactionOutput> for TransactionOutput<'a> {
    fn from(output: &'a MemoizedTransactionOutput) -> Self {
        Self {
            address: Cow::Borrowed(&output.address),
            value: output.value.as_ref().into(),
            datum: &output.datum,
            script: output.script.as_ref().map(Script::from),
        }
    }
}
