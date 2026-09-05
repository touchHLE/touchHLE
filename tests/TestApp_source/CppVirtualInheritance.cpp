/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

// Behavioral test for C++ virtual inheritance.
//
// This exercises vtable/VTT-dependent construction and virtual dispatch to
// verify that the object layout is correct. It was written alongside a fix
// for dropped external relocation addends in mach_o.rs, where VTT entries
// of the form `vtable_symbol + offset` lost their offset.
//
// Note: whether this test actually hits the fixed code path depends on the
// toolchain. Modern linkers resolve intra-binary symbol+offset references
// at static link time, so the external relocation path in mach_o.rs is
// never reached. Older toolchains (like the one that built DOOM
// Resurrection 1.0.1) emit these as external relocations, which is where
// the bug manifested. This test still provides end-to-end coverage of
// virtual inheritance correctness and would catch a regression if the
// toolchain ever changes its relocation strategy.

extern "C" int test_cpp_virtual_inheritance(void);

// Virtual base class with a virtual function to ensure vtable emission.
struct VBase {
  int base_val;
  virtual int get_val();
};

int VBase::get_val() { return base_val; }

// Intermediate class using virtual inheritance from VBase.
// During construction, the VTT provides a construction vtable whose
// vbase_offset is used to locate the VBase subobject.
struct Middle : virtual VBase {
  int mid_val;

  Middle() {
    // This store uses the vbase_offset from the construction vtable
    // (provided via VTT) to find VBase within the object.
    // If the VTT addend was dropped, vbase_offset is wrong and this
    // writes to the wrong memory location.
    base_val = 0xABCD;
    mid_val = 42;
  }
};

// Most-derived class. Its constructor sets up the VTT for Middle's
// construction and installs the final vtable afterward.
struct Derived : Middle {
  int der_val;

  Derived() {
    der_val = 99;
    // Overwrite base_val. Same dependency on correct vbase_offset.
    base_val = 0xCAFE;
  }
};

int test_cpp_virtual_inheritance(void) {
  Derived d;

  // If VTT addend was dropped, the constructors wrote base_val to the
  // wrong address. Reading d.base_val (at the compiler-known correct
  // offset for the most-derived class) finds uninitialized memory.
  if (d.base_val != (int)0xCAFE)
    return -1;
  if (d.mid_val != 42)
    return -2;
  if (d.der_val != 99)
    return -3;

  // Verify pointer adjustment from Derived* to VBase* uses the correct
  // vbase_offset from the final vtable.
  VBase *bp = &d;
  if (bp->base_val != (int)0xCAFE)
    return -4;

  // Verify virtual function dispatch through the vtable works.
  if (bp->get_val() != (int)0xCAFE)
    return -5;

  bp->base_val = 0xBEEF;
  if (d.base_val != (int)0xBEEF)
    return -6;

  return 0;
}
