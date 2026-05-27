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

use crate::{ComputeHash, HasScriptHash, Hash, MemoizedScript, NativeScript, PlutusScript, cbor, size::SCRIPT};

/// A borrowed reference to a script.
///
/// The by-reference counterpart of the owned [`MemoizedScript`], flattened to its four
/// kinds: a native script, or a Plutus script whose language version is carried in the
/// type ([`PlutusScript`]`<1>`/`<2>`/`<3>`). The version travels with the script because
/// execution depends on it. The available builtins, the cost model, and the
/// script-context encoding all differ by Plutus version.
#[derive(Debug, Clone)]
pub enum Script<'a> {
    Native(&'a NativeScript),
    PlutusV1(&'a PlutusScript<1>),
    PlutusV2(&'a PlutusScript<2>),
    PlutusV3(&'a PlutusScript<3>),
}

impl Script<'_> {
    /// Unwraps a layer of CBOR, returning the flat-encoded bytes
    /// that are passed to the CEK machine for evaluation.
    ///
    /// a `Script::Native` is treated `unreachable` since there are no redeemers for NativeScripts
    /// and they are not flat-encoded bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, cbor::decode::Error> {
        fn decode_cbor_bytes(cbor: &[u8]) -> Result<Vec<u8>, cbor::decode::Error> {
            cbor::decode::Decoder::new(cbor).bytes().map(|b| b.to_vec())
        }

        match self {
            Script::PlutusV1(s) => decode_cbor_bytes(s.0.as_ref()),
            Script::PlutusV2(s) => decode_cbor_bytes(s.0.as_ref()),
            Script::PlutusV3(s) => decode_cbor_bytes(s.0.as_ref()),
            Script::Native(_) => unreachable!("a redeemer should never point to a native_script"),
        }
    }
}

impl<'a> From<&'a MemoizedScript> for Script<'a> {
    fn from(value: &'a MemoizedScript) -> Self {
        match value {
            MemoizedScript::NativeScript(script) => Script::Native(script.as_ref()),
            MemoizedScript::PlutusV1Script(script) => Script::PlutusV1(script),
            MemoizedScript::PlutusV2Script(script) => Script::PlutusV2(script),
            MemoizedScript::PlutusV3Script(script) => Script::PlutusV3(script),
        }
    }
}

impl<'a> HasScriptHash for Script<'a> {
    fn script_hash(&self) -> Hash<SCRIPT> {
        match self {
            Script::Native(native_script) => native_script.compute_hash(),
            Script::PlutusV1(plutus_script) => plutus_script.compute_hash(),
            Script::PlutusV2(plutus_script) => plutus_script.compute_hash(),
            Script::PlutusV3(plutus_script) => plutus_script.compute_hash(),
        }
    }
}
