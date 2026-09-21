// Part of the Crubit project, under the Apache License v2.0 with LLVM
// Exceptions. See /LICENSE for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

// Contains a field because C++ doesn't support zero sized types (see b/258259459)
#[derive(Default)]
#[repr(transparent)]
pub struct CppMovable(pub i32);

impl Drop for CppMovable {
    fn drop(&mut self) {}
}

pub fn takes_val_movable(_val: CppMovable) {}

#[repr(transparent)]
pub struct NotCppMovable(pub i32);

impl Drop for NotCppMovable {
    fn drop(&mut self) {}
}

pub fn takes_val_unmovable(_val: NotCppMovable) {}
