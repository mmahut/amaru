// Copyright 2025 PRAGMA
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

use amaru_uplc::{arena::Arena, machine::PlutusVersion, syn::parse_program};

fn run_conformance(file_contents: &str, expected_output: &str, expected_budget: &str) {
    let arena = Arena::new();

    let Ok(program) = parse_program(&arena, file_contents).into_result() else {
        pretty_assertions::assert_eq!("parse error", expected_output);
        pretty_assertions::assert_eq!("parse error", expected_budget);

        return;
    };

    let result = program.eval_version(&arena, PlutusVersion::V3);

    let info = result.info;

    let Ok(term) = result.term else {
        pretty_assertions::assert_eq!("evaluation failure", expected_output);
        pretty_assertions::assert_eq!("evaluation failure", expected_budget);

        return;
    };

    let expected = parse_program(&arena, expected_output).into_result().unwrap();

    pretty_assertions::assert_eq!(expected.term, term);

    let consumed_budget = format!("({{cpu: {}\n| mem: {}}})", info.consumed_budget.cpu, info.consumed_budget.mem);

    pretty_assertions::assert_eq!(consumed_budget, expected_budget);
}

include!(concat!(env!("OUT_DIR"), "/generated_tests.rs"));
