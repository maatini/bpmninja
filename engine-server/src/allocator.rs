// Only binaries opting into this feature replace the system allocator.
#[cfg(feature = "allocator-mimalloc")]
#[global_allocator]
static ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;
