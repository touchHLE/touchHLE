void exit(int);

// This symbol must _not_ start with an underscore!
void fake_binding_helper() asm("dyld_stub_binding_helper");
// We don't use this.
void fake_binding_helper() {
    exit(-1);
}
