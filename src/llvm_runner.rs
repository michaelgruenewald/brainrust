#![cfg(feature = "llvm")]
use inkwell::types::IntType;
use std::io::{Read, Write};

use inkwell::builder::{Builder, BuilderError};
use inkwell::context::Context;
use inkwell::values::{FunctionValue, IntValue, PointerValue};
use inkwell::IntPredicate;
use inkwell::OptimizationLevel;

use crate::structs::Op::*;
use crate::structs::{Op, OpStream};

const MEMSIZE: usize = 30000;

pub struct LlvmState<'a, R: Read, W: Write> {
    memory: [i8; MEMSIZE],
    input: &'a mut R,
    output: &'a mut W,
    optimize: bool,
}

struct Compiler<'ctx, 'a> {
    context: &'ctx Context,
    builder: &'a Builder<'ctx>,
    function: FunctionValue<'ctx>,

    memory: PointerValue<'ctx>,
    state: PointerValue<'ctx>,

    getcharfn: FunctionValue<'ctx>,
    putcharfn: FunctionValue<'ctx>,

    size_t: IntType<'ctx>,
    byte: IntType<'ctx>,
}

impl<'ctx, 'a> Compiler<'ctx, 'a> {
    fn compile_mov(&self, ptr: IntValue<'ctx>, i: &isize) -> Result<IntValue<'_>, BuilderError> {
        self.builder
            .build_int_add(ptr, self.size_t.const_int((*i) as u64, true), "ptr")
    }

    fn compile_add(&self, ptr: IntValue<'ctx>, i: &u8) -> Result<IntValue<'_>, BuilderError> {
        let mem_ptr = unsafe {
            self.builder
                .build_gep(self.byte, self.memory, &[ptr], "mem_ptr")?
        };
        self.builder.build_store(
            mem_ptr,
            self.builder.build_int_add(
                self.builder
                    .build_load(self.byte, mem_ptr, "v")?
                    .into_int_value(),
                self.byte.const_int((*i).into(), true),
                "v",
            )?,
        )?;
        Ok(ptr)
    }

    fn compile_in(&self, ptr: IntValue<'ctx>) -> Result<IntValue<'_>, BuilderError> {
        let mem_ptr = unsafe {
            self.builder
                .build_gep(self.byte, self.memory, &[ptr], "mem_ptr")?
        };
        let result = self
            .builder
            .build_call(self.getcharfn, &[mem_ptr.into(), self.state.into()], "call")
            .unwrap();
        let exit_block = self.context.append_basic_block(self.function, "exit");
        let next_block = self.context.append_basic_block(self.function, "next");
        self.builder.build_conditional_branch(
            self.builder.build_int_compare(
                IntPredicate::EQ,
                result.try_as_basic_value().left().unwrap().into_int_value(),
                self.context.bool_type().const_zero(),
                "eof",
            )?,
            exit_block,
            next_block,
        )?;
        self.builder.position_at_end(exit_block);
        self.builder.build_return(None).unwrap();
        self.builder.position_at_end(next_block);
        Ok(ptr)
    }

    fn compile_out(&self, ptr: IntValue<'ctx>) -> Result<IntValue<'_>, BuilderError> {
        let mem_ptr = unsafe {
            self.builder
                .build_gep(self.byte, self.memory, &[ptr], "mem_ptr")?
        };
        self.builder.build_call(
            self.putcharfn,
            &[
                self.builder
                    .build_load(self.byte, mem_ptr, "v")?
                    .into_int_value()
                    .into(),
                self.state.into(),
            ],
            "call",
        )?;
        Ok(ptr)
    }

    fn compile_loop(
        &self,
        ptr: IntValue<'ctx>,
        ops: &OpStream,
    ) -> Result<IntValue<'_>, BuilderError> {
        let current_block = self.builder.get_insert_block().unwrap();
        let test_block = self.context.append_basic_block(self.function, "test");
        let body_block = self.context.append_basic_block(self.function, "body");
        let next_block = self.context.append_basic_block(self.function, "next");

        self.builder.build_unconditional_branch(test_block)?;
        self.builder.position_at_end(test_block);

        let test_ptr_phi = self.builder.build_phi(self.size_t, "ptr")?;
        test_ptr_phi.add_incoming(&[(&ptr, current_block)]);
        let new_ptr = test_ptr_phi.as_basic_value().into_int_value();

        let mem_ptr = unsafe {
            self.builder
                .build_gep(self.byte, self.memory, &[new_ptr], "mem_ptr")
        }?;
        self.builder.build_conditional_branch(
            self.builder.build_int_compare(
                IntPredicate::EQ,
                self.builder
                    .build_load(self.byte, mem_ptr, "v")?
                    .into_int_value(),
                self.byte.const_int(0, false),
                "iszero",
            )?,
            next_block,
            body_block,
        )?;
        self.builder.position_at_end(body_block);
        let ptrreg_loop = self.compile(ops.get(), new_ptr);
        test_ptr_phi.add_incoming(&[(&ptrreg_loop, self.builder.get_insert_block().unwrap())]);

        self.builder.build_unconditional_branch(test_block)?;
        self.builder.position_at_end(next_block);

        Ok(new_ptr)
    }

    fn compile(&self, ops: &[Op], start_ptr: IntValue<'ctx>) -> IntValue<'_> {
        ops.iter().fold(start_ptr, |ptr, op| {
            match op {
                Mov(i) => self.compile_mov(ptr, i),
                Add(i) => self.compile_add(ptr, i),
                In => self.compile_in(ptr),
                Out => self.compile_out(ptr),
                Loop(ref ops) => self.compile_loop(ptr, ops),
                Transfer(_, _) => {
                    unimplemented!("Transfer is not implemented for the LLVM backend.")
                }
            }
            .unwrap()
        })
    }
}

impl<'a, R: Read, W: Write> LlvmState<'a, R, W> {
    pub fn new<'b>(input: &'b mut R, output: &'b mut W, optimize: bool) -> LlvmState<'b, R, W> {
        LlvmState {
            memory: [0; MEMSIZE],
            input,
            output,
            optimize,
        }
    }

    pub fn run(&mut self, ops: &[Op]) -> bool {
        let context = Context::create();
        let module = context.create_module("program");
        let execution_engine = module
            .create_jit_execution_engine(if self.optimize {
                OptimizationLevel::Aggressive
            } else {
                OptimizationLevel::None
            })
            .unwrap();
        let builder = context.create_builder();

        let byte = context.i8_type();
        let size_t =
            context.ptr_sized_int_type(execution_engine.get_target_data(), Default::default());
        let getcharfn = module.add_function(
            "getchar",
            context.bool_type().fn_type(
                &[
                    byte.ptr_type(Default::default()).into(),
                    byte.ptr_type(Default::default()).into(),
                ],
                false,
            ),
            None,
        );
        let putcharfn = module.add_function(
            "putchar",
            context.void_type().fn_type(
                &[byte.into(), byte.ptr_type(Default::default()).into()],
                false,
            ),
            None,
        );

        let function = module.add_function(
            "run",
            context.void_type().fn_type(
                &[
                    byte.ptr_type(Default::default()).into(),
                    byte.ptr_type(Default::default()).into(),
                ],
                false,
            ),
            None,
        );
        let entry_block = context.append_basic_block(function, "entry");
        builder.position_at_end(entry_block);

        let compiler = Compiler {
            context: &context,
            builder: &builder,
            function,

            memory: function.get_nth_param(0).unwrap().into_pointer_value(),
            state: function.get_nth_param(1).unwrap().into_pointer_value(),

            getcharfn,
            putcharfn,

            size_t: context
                .ptr_sized_int_type(execution_engine.get_target_data(), Default::default()),
            byte: context.i8_type(),
        };

        compiler.compile(ops, size_t.const_zero());
        builder.build_return(None).unwrap();

        module.verify().unwrap();

        execution_engine.add_global_mapping(&getcharfn, LlvmState::<R, W>::getchar as usize);
        execution_engine.add_global_mapping(&putcharfn, LlvmState::<R, W>::putchar as usize);

        unsafe {
            execution_engine
                .get_function::<unsafe extern "C" fn(*mut [i8; MEMSIZE], *mut std::ffi::c_void) -> u64>(
                    "run",
                )
                .unwrap()
                .call(&mut self.memory as *mut [i8; MEMSIZE], /* FIXME: evil */ std::mem::transmute(self));
        };

        true
    }

    extern "C" fn getchar(ch: &mut u8, state: &mut LlvmState<R, W>) -> bool {
        let mut c = [0u8];
        if state.input.read(&mut c).unwrap() == 0 {
            return false;
        }
        *ch = c[0];
        true
    }

    extern "C" fn putchar(ch: u8, state: &mut LlvmState<R, W>) {
        state.output.write_all(&[ch]).unwrap();
    }
}
