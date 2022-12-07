#include <cstdint>

#include "dynarmic/interface/A32/a32.h"
#include "dynarmic/interface/A32/config.h"

namespace touchHLE::cpu {

using VAddr = std::uint32_t;

// Types and functions defined in Rust
extern "C" {
struct touchHLE_Memory;
std::uint8_t touchHLE_cpu_read_u8(touchHLE_Memory *mem, VAddr addr);
std::uint16_t touchHLE_cpu_read_u16(touchHLE_Memory *mem, VAddr addr);
std::uint32_t touchHLE_cpu_read_u32(touchHLE_Memory *mem, VAddr addr);
std::uint64_t touchHLE_cpu_read_u64(touchHLE_Memory *mem, VAddr addr);
void touchHLE_cpu_write_u8(touchHLE_Memory *mem, VAddr addr,
                           std::uint8_t value);
void touchHLE_cpu_write_u16(touchHLE_Memory *mem, VAddr addr,
                            std::uint8_t value);
void touchHLE_cpu_write_u32(touchHLE_Memory *mem, VAddr addr,
                            std::uint8_t value);
void touchHLE_cpu_write_u64(touchHLE_Memory *mem, VAddr addr,
                            std::uint8_t value);
}

class Environment final : public Dynarmic::A32::UserCallbacks {
public:
  Dynarmic::A32::Jit *cpu = nullptr;
  touchHLE_Memory *mem = nullptr;

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
  void CallSVC(std::uint32_t) override { cpu->HaltExecution(); }
  void ExceptionRaised(std::uint32_t, Dynarmic::A32::Exception) override {
    abort(); // TODO
  }
  void AddTicks(std::uint64_t) override {}                 // TODO
  std::uint64_t GetTicksRemaining() override { return 2; } // TODO
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

  void run(touchHLE_Memory *mem) {
    env.mem = mem;
    cpu->Run();
    env.mem = nullptr;
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

void touchHLE_DynarmicWrapper_run(DynarmicWrapper *cpu, touchHLE_Memory *mem) {
  cpu->run(mem);
}
}

} // namespace touchHLE::cpu
