/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
#include <cstdint>
#include <cstdio>

#include "dynarmic/interface/A32/a32.h"
#include "dynarmic/interface/A32/config.h"
#include "dynarmic/interface/A32/context.h"

namespace touchHLE::cpu {

using VAddr = std::uint32_t;

// Types and functions defined in Rust
extern "C" {
struct touchHLE_Mem;
std::uint8_t touchHLE_cpu_read_u8(touchHLE_Mem *mem, VAddr addr, bool *error);
std::uint16_t touchHLE_cpu_read_u16(touchHLE_Mem *mem, VAddr addr, bool *error);
std::uint32_t touchHLE_cpu_read_u32(touchHLE_Mem *mem, VAddr addr, bool *error);
std::uint64_t touchHLE_cpu_read_u64(touchHLE_Mem *mem, VAddr addr, bool *error);
bool touchHLE_cpu_write_u8(touchHLE_Mem *mem, VAddr addr, std::uint8_t value);
bool touchHLE_cpu_write_u16(touchHLE_Mem *mem, VAddr addr, std::uint16_t value);
bool touchHLE_cpu_write_u32(touchHLE_Mem *mem, VAddr addr, std::uint32_t value);
bool touchHLE_cpu_write_u64(touchHLE_Mem *mem, VAddr addr, std::uint64_t value);
}

const auto HaltReasonSvc = Dynarmic::HaltReason::UserDefined1;
const auto HaltReasonUndefinedInstruction = Dynarmic::HaltReason::UserDefined2;
const auto HaltReasonBreakpoint = Dynarmic::HaltReason::UserDefined3;

class Environment final : public Dynarmic::A32::UserCallbacks {
public:
  Dynarmic::A32::Jit *cpu = nullptr;
  touchHLE_Mem *mem = nullptr;
  std::uint64_t ticks_remaining;
  uint32_t halting_svc;

private:
  std::uint8_t MemoryRead8(VAddr vaddr) override {
    bool error;
    auto value = touchHLE_cpu_read_u8(mem, vaddr, &error);
    if (error) {
      cpu->HaltExecution(Dynarmic::HaltReason::MemoryAbort);
    }
    return value;
  }
  std::uint16_t MemoryRead16(VAddr vaddr) override {
    bool error;
    auto value = touchHLE_cpu_read_u16(mem, vaddr, &error);
    if (error) {
      cpu->HaltExecution(Dynarmic::HaltReason::MemoryAbort);
    }
    return value;
  }
  std::uint32_t MemoryRead32(VAddr vaddr) override {
    bool error;
    auto value = touchHLE_cpu_read_u32(mem, vaddr, &error);
    if (error) {
      cpu->HaltExecution(Dynarmic::HaltReason::MemoryAbort);
    }
    return value;
  }
  std::uint64_t MemoryRead64(VAddr vaddr) override {
    bool error;
    auto value = touchHLE_cpu_read_u64(mem, vaddr, &error);
    if (error) {
      cpu->HaltExecution(Dynarmic::HaltReason::MemoryAbort);
    }
    return value;
  }

  std::optional<std::uint32_t> MemoryReadCode(VAddr vaddr) override {
    bool error;
    auto value = touchHLE_cpu_read_u32(mem, vaddr, &error);
    if (error) {
      return std::nullopt;
    } else {
      return value;
    }
  }

  void MemoryWrite8(VAddr vaddr, std::uint8_t value) override {
    if (touchHLE_cpu_write_u8(mem, vaddr, value)) {
      cpu->HaltExecution(Dynarmic::HaltReason::MemoryAbort);
    }
  }
  void MemoryWrite16(VAddr vaddr, std::uint16_t value) override {
    if (touchHLE_cpu_write_u16(mem, vaddr, value)) {
      cpu->HaltExecution(Dynarmic::HaltReason::MemoryAbort);
    }
  }
  void MemoryWrite32(VAddr vaddr, std::uint32_t value) override {
    if (touchHLE_cpu_write_u32(mem, vaddr, value)) {
      cpu->HaltExecution(Dynarmic::HaltReason::MemoryAbort);
    }
  }
  void MemoryWrite64(VAddr vaddr, std::uint64_t value) override {
    if (touchHLE_cpu_write_u64(mem, vaddr, value)) {
      cpu->HaltExecution(Dynarmic::HaltReason::MemoryAbort);
    }
  }

  void InterpreterFallback(std::uint32_t, size_t) override {
    abort(); // TODO
  }
  void CallSVC(std::uint32_t svc) override {
    halting_svc = svc;
    cpu->HaltExecution(HaltReasonSvc);
  }
  void ExceptionRaised(VAddr pc, Dynarmic::A32::Exception exception) override {
    // MemoryReadCode returned nullopt
    if (exception == Dynarmic::A32::Exception::NoExecuteFault) {
      cpu->HaltExecution(Dynarmic::HaltReason::MemoryAbort);
    } else if (exception == Dynarmic::A32::Exception::UndefinedInstruction) {
      cpu->HaltExecution(HaltReasonUndefinedInstruction);
    } else if (exception == Dynarmic::A32::Exception::Breakpoint) {
      cpu->HaltExecution(HaltReasonBreakpoint);
    } else {
      std::fprintf(stderr, "ExceptionRaised: unexpected exception %u at %x\n",
                   unsigned(exception), pc);
      abort();
    }
  }
  void AddTicks(std::uint64_t ticks) override {
    if (ticks > ticks_remaining) {
      ticks_remaining = 0;
      return;
    }
    ticks_remaining -= ticks;
  }
  std::uint64_t GetTicksRemaining() override { return ticks_remaining; }
};

class DynarmicWrapper {
  Environment env;
  std::unique_ptr<Dynarmic::A32::Jit> cpu;
  std::array<std::uint8_t *, Dynarmic::A32::UserConfig::NUM_PAGE_TABLE_ENTRIES>
      page_table;

public:
  DynarmicWrapper(void *direct_memory_access_ptr, size_t null_page_count) {
    Dynarmic::A32::UserConfig user_config;
    user_config.callbacks = &env;
    // TODO: only do this in debug builds? it's probably expensive
    user_config.check_halt_on_memory_access = true;
    if (direct_memory_access_ptr) {
      // Allow fast accesses to all pages other than the null page, which will
      // fall back to a memory callback, which will then abort execution.
      // TODO: Eventually we should use dynarmic's true fastmem mode, but that
      // requires using mmap/mprotect/etc on the host OS so we can still catch
      // null pointer accesses.
      page_table.fill((std::uint8_t *)direct_memory_access_ptr);
      // Note that the null page size is also defined in src/mem.rs.
      static_assert(1 << Dynarmic::A32::UserConfig::PAGE_BITS == 0x1000);

      if (null_page_count > page_table.size()) {
        printf("Too many null pages, %zu requested but maximum is %zu.",
               null_page_count, page_table.size());
        abort();
      }
      for (size_t i = 0; i < null_page_count; i++) {
        page_table[i] = nullptr;
      }
      user_config.page_table = &page_table;
      user_config.absolute_offset_page_table = true;
    }
    cpu = std::make_unique<Dynarmic::A32::Jit>(user_config);
    env.cpu = cpu.get();
  }

  const std::uint32_t *regs() const { return &cpu->Regs().front(); }
  std::uint32_t *regs() { return &cpu->Regs().front(); }

  std::uint32_t cpsr() const { return cpu->Cpsr(); }
  void set_cpsr(std::uint32_t cpsr) { cpu->SetCpsr(cpsr); }

  void invalidate_cache_range(VAddr start, std::uint32_t size) {
    cpu->InvalidateCacheRange(start, size);
  }

  void swap_context(void *context) {
    Dynarmic::A32::Context tmp = cpu->SaveContext();
    cpu->LoadContext(*(Dynarmic::A32::Context *)context);
    *(Dynarmic::A32::Context *)context = tmp;
  }

  std::int32_t run_or_step(touchHLE_Mem *mem, std::uint64_t *ticks) {
    env.mem = mem;
    Dynarmic::HaltReason hr;
    if (ticks) {
      env.ticks_remaining = *ticks;
      hr = cpu->Run();
    } else {
      hr = cpu->Step();
    }
    std::int32_t res;
    if ((!hr && ticks) || (hr == Dynarmic::HaltReason::Step && !ticks)) {
      res = -1;
    } else if (Dynarmic::Has(hr, Dynarmic::HaltReason::MemoryAbort)) {
      res = -2;
    } else if (Dynarmic::Has(hr, HaltReasonUndefinedInstruction)) {
      res = -3;
    } else if (Dynarmic::Has(hr, HaltReasonBreakpoint)) {
      res = -4;
    } else if (Dynarmic::Has(hr, HaltReasonSvc)) {
      res = std::int32_t(env.halting_svc);
    } else {
      printf("unhandled halt reason %u\n", unsigned(hr));
      abort();
    }
    env.mem = nullptr;
    if (ticks) {
      *ticks = env.ticks_remaining;
    }
    return res;
  }
};

extern "C" {

DynarmicWrapper *touchHLE_DynarmicWrapper_new(void *direct_memory_access_ptr,
                                              size_t null_page_count) {
  return new DynarmicWrapper(direct_memory_access_ptr, null_page_count);
}
void touchHLE_DynarmicWrapper_delete(DynarmicWrapper *cpu) { delete cpu; }

const std::uint32_t *
touchHLE_DynarmicWrapper_regs_const(const DynarmicWrapper *cpu) {
  return cpu->regs();
}
std::uint32_t *touchHLE_DynarmicWrapper_regs_mut(DynarmicWrapper *cpu) {
  return cpu->regs();
}

std::uint32_t touchHLE_DynarmicWrapper_cpsr(const DynarmicWrapper *cpu) {
  return cpu->cpsr();
}
void touchHLE_DynarmicWrapper_set_cpsr(DynarmicWrapper *cpu,
                                       std::uint32_t cpsr) {
  cpu->set_cpsr(cpsr);
}

void touchHLE_DynarmicWrapper_swap_context(DynarmicWrapper *cpu,
                                           void *context) {
  cpu->swap_context(context);
}

void touchHLE_DynarmicWrapper_invalidate_cache_range(DynarmicWrapper *cpu,
                                                     VAddr start,
                                                     std::uint32_t size) {
  cpu->invalidate_cache_range(start, size);
}

std::int32_t touchHLE_DynarmicWrapper_run_or_step(DynarmicWrapper *cpu,
                                                  touchHLE_Mem *mem,
                                                  std::uint64_t *ticks) {
  return cpu->run_or_step(mem, ticks);
}

void *touchHLE_DynarmicWrapper_Context_new() {
  return (void *)new Dynarmic::A32::Context();
}
void touchHLE_DynarmicWrapper_Context_delete(void *context) {
  delete (Dynarmic::A32::Context *)context;
}
}

} // namespace touchHLE::cpu
