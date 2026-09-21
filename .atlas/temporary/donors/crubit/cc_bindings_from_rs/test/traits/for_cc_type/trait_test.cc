// Part of the Crubit project, under the Apache License v2.0 with LLVM
// Exceptions. See /LICENSE for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

#include "cc_bindings_from_rs/test/traits/for_cc_type/trait.h"

#include "gtest/gtest.h"
#include "cc_bindings_from_rs/test/traits/for_cc_type/cc_type.h"

namespace crubit {
namespace {

TEST(ForCcTypeTraitTest, CcTypeImpl) {
  CcType cc_val = {123};
  EXPECT_EQ(trait::Trait::impl<CcType>::get_value(cc_val), 123);
}

}  // namespace
}  // namespace crubit
