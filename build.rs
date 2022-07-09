fn main() {
    cc::Build::new()
        .file("src/ovrride.cpp")
        .flag_if_supported("-std=c++17")
        .compile("fatalloc_ovrride_cpp");
}
