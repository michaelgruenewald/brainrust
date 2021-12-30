#![cfg(feature = "llvm")]
use inkwell::types::BasicMetadataTypeEnum;
use inkwell::values::BasicMetadataValueEnum;
use std::io::{Read, Write};

use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::execution_engine::ExecutionEngine;
use inkwell::values::{FunctionValue, IntValue, PointerValue};
use inkwell::AddressSpace;
use inkwell::IntPredicate;
use inkwell::OptimizationLevel;

use crate::structs::Op;
use crate::structs::Op::*;

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
    execution_engine: &'a ExecutionEngine<'ctx>,
    function: FunctionValue<'ctx>,

    memory: PointerValue<'ctx>,
    state: PointerValue<'ctx>,

    getcharfn: FunctionValue<'ctx>,
    putcharfn: FunctionValue<'ctx>,
}

impl<'ctx, 'a> Compiler<'ctx, 'a> {
    fn compile(&self, ops: &[Op], start_ptr: IntValue<'ctx>) -> IntValue<'ctx> {
        let byte = self.context.i8_type();
        let size_t = self.context.ptr_sized_int_type(
            self.execution_engine.get_target_data(),
            Some(AddressSpace::Generic),
        );

        let builder = self.builder;
        let mut ptr = start_ptr;

        for op in ops {
            match op {
                Mov(i) => {
                    ptr = builder.build_int_add(ptr, size_t.const_int((*i) as u64, true), "ptr");
                }
                Add(i) => {
                    let mem_ptr = unsafe { builder.build_gep(self.memory, &[ptr], "mem_ptr") };
                    builder.build_store(
                        mem_ptr,
                        builder.build_int_add(
                            builder.build_load(mem_ptr, "v").into_int_value(),
                            byte.const_int((*i).into(), true),
                            "v",
                        ),
                    );
                }
                In => {
                    let mem_ptr = unsafe { builder.build_gep(self.memory, &[ptr], "mem_ptr") };
                    let result = builder.build_call(
                        self.getcharfn,
                        &[
                            BasicMetadataValueEnum::PointerValue(mem_ptr),
                            BasicMetadataValueEnum::PointerValue(self.state),
                        ],
                        "call",
                    );
                    let exit_block = self.context.append_basic_block(self.function, "exit");
                    let next_block = self.context.append_basic_block(self.function, "next");
                    builder.build_conditional_branch(
                        builder.build_int_compare(
                            IntPredicate::EQ,
                            result.try_as_basic_value().left().unwrap().into_int_value(),
                            self.context.bool_type().const_zero(),
                            "eof",
                        ),
                        exit_block,
                        next_block,
                    );
                    builder.position_at_end(exit_block);
                    builder.build_return(None);
                    builder.position_at_end(next_block);
                }
                Out => {
                    let mem_ptr = unsafe { builder.build_gep(self.memory, &[ptr], "mem_ptr") };
                    builder.build_call(
                        self.putcharfn,
                        &[
                            BasicMetadataValueEnum::IntValue(
                                builder.build_load(mem_ptr, "v").into_int_value(),
                            ),
                            BasicMetadataValueEnum::PointerValue(self.state),
                        ],
                        "call",
                    );
                }
                Loop(ref ops) => {
                    let current_block = self.builder.get_insert_block().unwrap();
                    let test_block = self.context.append_basic_block(self.function, "test");
                    let body_block = self.context.append_basic_block(self.function, "body");
                    let next_block = self.context.append_basic_block(self.function, "next");

                    builder.build_unconditional_branch(test_block);
                    builder.position_at_end(test_block);

                    let test_ptr_phi = builder.build_phi(size_t, "ptr");
                    test_ptr_phi.add_incoming(&[(&ptr, current_block)]);
                    ptr = test_ptr_phi.as_basic_value().into_int_value();

                    let mem_ptr = unsafe { builder.build_gep(self.memory, &[ptr], "mem_ptr") };
                    builder.build_conditional_branch(
                        builder.build_int_compare(
                            IntPredicate::EQ,
                            builder.build_load(mem_ptr, "v").into_int_value(),
                            byte.const_int(0, false),
                            "iszero",
                        ),
                        next_block,
                        body_block,
                    );
                    builder.position_at_end(body_block);
                    let ptrreg_loop = self.compile(ops.get(), ptr);
                    test_ptr_phi
                        .add_incoming(&[(&ptrreg_loop, self.builder.get_insert_block().unwrap())]);

                    builder.build_unconditional_branch(test_block);
                    self.builder.position_at_end(next_block);
                }
                Transfer(_, _) => {
                    unimplemented!("Transfer is not implemented for the LLVM backend.");
                }
            }
        }

        ptr
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
        let size_t = context.ptr_sized_int_type(
            execution_engine.get_target_data(),
            Some(AddressSpace::Generic),
        );
        let getcharfn = module.add_function(
            "getchar",
            context.bool_type().fn_type(
                &[
                    byte.ptr_type(AddressSpace::Generic).into(),
                    BasicMetadataTypeEnum::PointerType(byte.ptr_type(AddressSpace::Generic)),
                ],
                false,
            ),
            None,
        );
        let putcharfn = module.add_function(
            "putchar",
            context.void_type().fn_type(
                &[
                    byte.into(),
                    BasicMetadataTypeEnum::PointerType(byte.ptr_type(AddressSpace::Generic)),
                ],
                false,
            ),
            None,
        );

        let function = module.add_function(
            "run",
            context.void_type().fn_type(
                &[
                    BasicMetadataTypeEnum::PointerType(byte.ptr_type(AddressSpace::Generic)),
                    BasicMetadataTypeEnum::PointerType(byte.ptr_type(AddressSpace::Generic)),
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
            execution_engine: &execution_engine,
            function,

            memory: function.get_nth_param(0).unwrap().into_pointer_value(),
            state: function.get_nth_param(1).unwrap().into_pointer_value(),

            getcharfn,
            putcharfn,
        };

        compiler.compile(ops, size_t.const_zero());
        builder.build_return(None);

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
