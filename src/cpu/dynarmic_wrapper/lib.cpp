#include <cstdint>

#include "dynarmic/interface/A32/a32.h"
#include "dynarmic/interface/A32/config.h"

namespace touchHLE::cpu {

using VAddr = std::uint32_t;

// FIXME: Everything. This is just a basic test that the build system and FFI
// are working.

static const uint32_t TEST_CODE[] = {
    0xE0800001, // add r0, r0, r1
    0xEF000001, // svc 0
};

class Environment final : public Dynarmic::A32::UserCallbacks {
public:
  Dynarmic::A32::Jit *cpu = nullptr;

private:
  std::uint8_t MemoryRead8(VAddr) override { return 0; }
  std::uint16_t MemoryRead16(VAddr) override { return 0; }
  std::uint32_t MemoryRead32(VAddr vaddr) override {
    return TEST_CODE[vaddr / 4];
  }
  std::uint64_t MemoryRead64(VAddr) override { return 0; }

  void MemoryWrite8(VAddr, std::uint8_t) override {}
  void MemoryWrite16(VAddr, std::uint16_t) override {}
  void MemoryWrite32(VAddr, std::uint32_t) override {}
  void MemoryWrite64(VAddr, std::uint64_t) override {}

  void InterpreterFallback(std::uint32_t, size_t) override {}
  void CallSVC(std::uint32_t) override { cpu->HaltExecution(); }
  void ExceptionRaised(std::uint32_t, Dynarmic::A32::Exception) override {}
  void AddTicks(std::uint64_t) override {}
  std::uint64_t GetTicksRemaining() override { return 2; }
};

extern "C" {

int32_t test_cpu_by_adding_numbers(int32_t a, int32_t b) {
  Environment env;
  Dynarmic::A32::UserConfig user_config;
  user_config.callbacks = &env;
  Dynarmic::A32::Jit cpu{user_config};
  env.cpu = &cpu;

  cpu.Regs()[0] = a;
  cpu.Regs()[1] = b;
  cpu.Regs()[15] = 0; // PC = 0

  cpu.Run();

  return cpu.Regs()[0];
}
}

} // namespace touchHLE::cpu
