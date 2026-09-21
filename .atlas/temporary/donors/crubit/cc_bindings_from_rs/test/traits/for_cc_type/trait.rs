// Part of the Crubit project, under the Apache License v2.0 with LLVM
// Exceptions. See /LICENSE for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

pub trait Trait {
    fn get_value(&self) -> i32;
}

impl Trait for cc_type::CcType {
    fn get_value(&self) -> i32 {
        self.value
    }
}
