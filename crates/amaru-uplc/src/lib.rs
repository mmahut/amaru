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

pub mod arena;
pub mod binder;
pub mod bls;
pub mod builtin;
pub mod constant;
pub mod data;
pub mod flat;
pub mod ledger_value;
pub mod machine;
pub mod program;
pub mod syn;
pub mod term;
pub mod typ;

pub use bumpalo;

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::{arena::Arena, program::Program, term::Term};
    use crate::{binder::DeBruijn, program::Version};

    #[test]
    fn add_integer() {
        let arena = Arena::new();

        let term = Term::add_integer(&arena)
            .apply(&arena, Term::integer_from(&arena, 1))
            .apply(&arena, Term::integer_from(&arena, 3));

        let version = Version::plutus_v3(&arena);

        let program = Program::<DeBruijn>::new(&arena, version, term);

        let result = program.eval(&arena);

        assert_eq!(result.term.unwrap(), Term::integer_from(&arena, 4));
    }

    #[test]
    fn fibonacci() {
        let arena = &Arena::new();

        let double_force = Term::var(arena, DeBruijn::new(arena, 1))
            .apply(arena, Term::var(arena, DeBruijn::new(arena, 1)))
            .lambda(arena, DeBruijn::zero(arena))
            .delay(arena)
            .force(arena)
            .apply(
                arena,
                Term::var(arena, DeBruijn::new(arena, 3))
                    .apply(
                        arena,
                        Term::var(arena, DeBruijn::new(arena, 1))
                            .apply(arena, Term::var(arena, DeBruijn::new(arena, 1)))
                            .lambda(arena, DeBruijn::zero(arena))
                            .delay(arena)
                            .force(arena)
                            .apply(arena, Term::var(arena, DeBruijn::new(arena, 2))),
                    )
                    .apply(arena, Term::var(arena, DeBruijn::new(arena, 1)))
                    .lambda(arena, DeBruijn::zero(arena))
                    .lambda(arena, DeBruijn::zero(arena)),
            )
            .lambda(arena, DeBruijn::zero(arena))
            .delay(arena)
            .delay(arena)
            .force(arena)
            .force(arena);

        let if_condition = Term::if_then_else(arena)
            .force(arena)
            .apply(arena, Term::var(arena, DeBruijn::new(arena, 3)))
            .apply(arena, Term::var(arena, DeBruijn::new(arena, 2)))
            .apply(arena, Term::var(arena, DeBruijn::new(arena, 1)))
            .apply(arena, Term::unit(arena))
            .lambda(arena, DeBruijn::zero(arena))
            .lambda(arena, DeBruijn::zero(arena))
            .lambda(arena, DeBruijn::zero(arena))
            .delay(arena)
            .force(arena);

        let add = Term::add_integer(arena)
            .apply(
                arena,
                Term::var(arena, DeBruijn::new(arena, 3)).apply(
                    arena,
                    Term::subtract_integer(arena)
                        .apply(arena, Term::var(arena, DeBruijn::new(arena, 2)))
                        .apply(arena, Term::integer_from(arena, 1)),
                ),
            )
            .apply(
                arena,
                Term::var(arena, DeBruijn::new(arena, 3)).apply(
                    arena,
                    Term::subtract_integer(arena)
                        .apply(arena, Term::var(arena, DeBruijn::new(arena, 2)))
                        .apply(arena, Term::integer_from(arena, 2)),
                ),
            )
            .lambda(arena, DeBruijn::zero(arena));

        let term = double_force
            .apply(
                arena,
                if_condition
                    .apply(
                        arena,
                        Term::less_than_equals_integer(arena)
                            .apply(arena, Term::var(arena, DeBruijn::new(arena, 1)))
                            .apply(arena, Term::integer_from(arena, 1)),
                    )
                    .apply(arena, Term::var(arena, DeBruijn::new(arena, 2)).lambda(arena, DeBruijn::zero(arena)))
                    .apply(arena, add)
                    .lambda(arena, DeBruijn::zero(arena))
                    .lambda(arena, DeBruijn::zero(arena)),
            )
            .apply(arena, Term::var(arena, DeBruijn::new(arena, 1)))
            .lambda(arena, DeBruijn::zero(arena))
            .apply(arena, Term::integer_from(arena, 15));

        let version = Version::plutus_v3(arena);

        let program = Program::new(arena, version, term);

        let result = program.eval(arena);

        assert_eq!(result.term.unwrap(), Term::integer_from(arena, 610));
    }
}
