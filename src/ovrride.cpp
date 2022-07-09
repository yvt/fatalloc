#include <cstdio>
#include <cstdlib>
#include <new>

#if __cpp_aligned_new >= 201606
using my_align_val_t = std::align_val_t;
#else
enum class my_align_val_t : std::size_t {};
#endif

namespace {
void *new_inner(std::size_t size, my_align_val_t align, bool is_noexcept) {
    void *ptr = std::size_t(align) ? aligned_alloc(std::size_t(align), size)
                                   : malloc(size);
    if (!ptr && !is_noexcept) {
        // TODO: handle failure by throwing an exception
        fprintf(stderr, "fatalloc: allocation failed\n");
        abort();
    }
    return ptr;
}
} // namespace

void *operator new(std::size_t size) {
    return new_inner(size, my_align_val_t(0), false);
}
void *operator new[](std::size_t size) {
    return new_inner(size, my_align_val_t(0), false);
}
void *operator new(std::size_t size, const std::nothrow_t &) noexcept {
    return new_inner(size, my_align_val_t(0), true);
}
void *operator new[](std::size_t size, const std::nothrow_t &) noexcept {
    return new_inner(size, my_align_val_t(0), true);
}
void operator delete(void *ptr) noexcept { free(ptr); }

void operator delete[](void *ptr) noexcept { free(ptr); }
void operator delete(void *ptr, const std::nothrow_t &) noexcept { free(ptr); }
void operator delete[](void *ptr, const std::nothrow_t &) noexcept {
    free(ptr);
}

void operator delete(void *ptr, std::size_t) noexcept { free(ptr); }
void operator delete[](void *ptr, std::size_t) noexcept { free(ptr); }

#if __cpp_aligned_new >= 201606
void *operator new(std::size_t size, my_align_val_t align) {
    return new_inner(size, align, false);
}
void *operator new(std::size_t size, my_align_val_t align,
                   const std::nothrow_t &) noexcept {
    return new_inner(size, align, true);
}
void *operator new[](std::size_t size, my_align_val_t align) {
    return new_inner(size, align, false);
}
void *operator new[](std::size_t size, my_align_val_t align,
                     const std::nothrow_t &) noexcept {
    return new_inner(size, align, true);
}
void operator delete(void *ptr, my_align_val_t) noexcept { free(ptr); }
void operator delete(void *ptr, my_align_val_t,
                     const std::nothrow_t &) noexcept {
    free(ptr);
}
void operator delete(void *ptr, std::size_t, my_align_val_t) noexcept {
    free(ptr);
}
void operator delete[](void *ptr, my_align_val_t) noexcept { free(ptr); }
void operator delete[](void *ptr, my_align_val_t,
                       const std::nothrow_t &) noexcept {
    free(ptr);
}
void operator delete[](void *ptr, std::size_t, my_align_val_t) noexcept {
    free(ptr);
}
#endif
