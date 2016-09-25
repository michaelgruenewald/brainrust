# brainrust

*brainrust* is a somewhat optimizing Brainfuck interpreter written in Rust.

## Structure

When interpreting a file, it first parses the whole file (`parser.rs`),
generating an in-memory representation of the program. This representation is
entirely unoptimized, e.g. for the program `+++` it would contain three `Add`
instructions. Comments are stripped out though and the correct structure of the
program is checked, e.g. for unbalanced loops.

After parsing an optimizer (`optimizer.rs`) runs. It aggregates individual
instructions into groups (e.g. changes `Add(1), Add(1), Add(1)` into `Add(3)`)
and removes instructions without effects (e.g. `+-`). It also introduces new
specialized instructions.

The resulting program is interpreted (`runner.rs`) afterwards.
