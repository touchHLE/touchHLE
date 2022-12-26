#include <cstdint>

#include "dynarmic/interface/A32/a32.h"
#include "dynarmic/interface/A32/config.h"

namespace touchHLE::cpu {

using VAddr = std::uint32_t;

// Types and functions defined in Rust
extern "C" {
struct touchHLE_Mem;
std::uint8_t touchHLE_cpu_read_u8(touchHLE_Mem *mem, VAddr addr);
std::uint16_t touchHLE_cpu_read_u16(touchHLE_Mem *mem, VAddr addr);
std::uint32_t touchHLE_cpu_read_u32(touchHLE_Mem *mem, VAddr addr);
std::uint64_t touchHLE_cpu_read_u64(touchHLE_Mem *mem, VAddr addr);
void touchHLE_cpu_write_u8(touchHLE_Mem *mem, VAddr addr, std::uint8_t value);
void touchHLE_cpu_write_u16(touchHLE_Mem *mem, VAddr addr, std::uint8_t value);
void touchHLE_cpu_write_u32(touchHLE_Mem *mem, VAddr addr, std::uint8_t value);
void touchHLE_cpu_write_u64(touchHLE_Mem *mem, VAddr addr, std::uint8_t value);
}

const auto HaltReasonSvc = Dynarmic::HaltReason::UserDefined1;

class Environment final : public Dynarmic::A32::UserCallbacks {
public:
  Dynarmic::A32::Jit *cpu = nullptr;
  touchHLE_Mem *mem = nullptr;
  std::uint64_t ticks_remaining;
  uint32_t halting_svc;

private:
  std::uint8_t MemoryRead8(VAddr vaddr) override {
    return touchHLE_cpu_read_u8(mem, vaddr);
  }
  std::uint16_t MemoryRead16(VAddr vaddr) override {
    return touchHLE_cpu_read_u16(mem, vaddr);
  }
  std::uint32_t MemoryRead32(VAddr vaddr) override {
    return touchHLE_cpu_read_u32(mem, vaddr);
  }
  std::uint64_t MemoryRead64(VAddr vaddr) override {
    return touchHLE_cpu_read_u64(mem, vaddr);
  }

  void MemoryWrite8(VAddr vaddr, std::uint8_t value) override {
    touchHLE_cpu_write_u8(mem, vaddr, value);
  }
  void MemoryWrite16(VAddr vaddr, std::uint16_t value) override {
    touchHLE_cpu_write_u16(mem, vaddr, value);
  }
  void MemoryWrite32(VAddr vaddr, std::uint32_t value) override {
    touchHLE_cpu_write_u32(mem, vaddr, value);
  }
  void MemoryWrite64(VAddr vaddr, std::uint64_t value) override {
    touchHLE_cpu_write_u64(mem, vaddr, value);
  }

  void InterpreterFallback(std::uint32_t, size_t) override {
    abort(); // TODO
  }
  void CallSVC(std::uint32_t svc) override {
    halting_svc = svc;
    cpu->HaltExecution(HaltReasonSvc);
  }
  void ExceptionRaised(std::uint32_t, Dynarmic::A32::Exception) override {
    abort(); // TODO
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

public:
  DynarmicWrapper() {
    Dynarmic::A32::UserConfig user_config;
    user_config.callbacks = &env;
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

  std::int32_t run(touchHLE_Mem *mem, std::uint64_t *ticks) {
    env.mem = mem;
    env.ticks_remaining = *ticks;
    Dynarmic::HaltReason hr = cpu->Run();
    std::int32_t res;
    if (!hr) {
      res = -1;
    } else if (Dynarmic::Has(hr, HaltReasonSvc)) {
      res = std::int32_t(env.halting_svc);
    } else {
      printf("unhandled halt reason %u\n", hr);
      abort();
    }
    env.mem = nullptr;
    *ticks = env.ticks_remaining;
    return res;
  }
};

extern "C" {

DynarmicWrapper *touchHLE_DynarmicWrapper_new() {
  return new DynarmicWrapper();
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

void touchHLE_DynarmicWrapper_invalidate_cache_range(DynarmicWrapper *cpu,
                                                     VAddr start,
                                                     std::uint32_t size) {
  cpu->invalidate_cache_range(start, size);
}

std::int32_t touchHLE_DynarmicWrapper_run(DynarmicWrapper *cpu,
                                          touchHLE_Mem *mem,
                                          std::uint64_t *ticks) {
  return cpu->run(mem, ticks);
}
}

} // namespace touchHLE::cpu
