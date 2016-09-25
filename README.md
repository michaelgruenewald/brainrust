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
specialized instructions, as described below.

The resulting program is interpreted (`runner.rs`) afterwards.

## Optimizations

In Brainfuck programs there are certain kinds of loops that, if they adhere to
some restrictions, can be interpreted in a single step. The restrictions they
must fulfil are:

- They must not contain nested loops.
- They must not contain input or output operations.
- The pointer must not change across iterations.

Due to the first two restrictions, the third one can be translated to: the
pointer movement instructions must be balanced within the loop body.

These kinds of loops may be equivalently represented by their effects to cells
relative to the pointer position. E.g., the loop `[->++<]` will decrement the
cell pointed at by one and increment the cell next to by two in every
iteration. The same holds true for all variations of this loop, e.g.,
`[>++<-]`, `[>+<->+<]`, etc.

These kinds of loops are represented using the `Transfer` operation in the
code. They may be executed more efficiently because the number of iterations
can be computed solely by the value of the initial cell and the effect of a
single iteration to it.
