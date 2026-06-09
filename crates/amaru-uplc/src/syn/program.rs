use chumsky::{Parser, prelude::*};

use super::{term, types::Extra, utils::comments, version};
use crate::{binder::DeBruijn, program::Program};

pub fn parser<'a>() -> impl Parser<'a, &'a str, &'a Program<'a, DeBruijn>, Extra<'a>> {
    text::keyword("program")
        .padded()
        .ignore_then(version::parser().padded())
        .then(term::parser().padded())
        .delimited_by(just('('), just(')'))
        .padded()
        .padded_by(comments())
        .then_ignore(end())
        .map_with(|(version, term), e| {
            let state = e.state();

            Program::new(state.arena, version, term)
        })
}
