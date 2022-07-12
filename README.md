<div align="center">

# `fatalloc`

**“Fault tolerant” memory allocator for Linux**

</div>

This library provides a drop-in replacement for the standard C allocation
functions. Add `libfatalloc.so` to `LD_PRELOAD` to “fix” minor heap overruns
in faulty software.

Using this library does not fix **C**VEs or improve security—if anything,
undermines it. This library bypasses various checks, which may be the first
line of defense against intrusion attempts, just to keep your programs alive
and your workflow uninterrupted as long as possible. Use at your own peril.

Inspired by Windows [Fault Tolerant Heap][1]. Written in Rust(🚀).

## Usage

### Nix [Flake][2]

```bash
export LD_PRELOAD=(nix build --no-link --print-out-paths github:yvt/fatalloc)/lib/libfatalloc.so)
faulty-program
```

To cross-build for x86 (32-bit) applications:

```bash
export LD_PRELOAD=(nix build --no-link --print-out-paths github:yvt/fatalloc#defaultPackage.i686-linux)/lib/libfatalloc.so)
```

### Traditional Linux

Go to [the Actions tab][3], select the latest CI run, and download a
precompiled binary from the Artifacts section.

*Note:* [You must be logged in to GitHub to download artifacts.][4]

## License

This program is licensed under the GNU Lesser General Public License version 3
or later.

[1]: https://docs.microsoft.com/en-us/windows/win32/win7appqual/fault-tolerant-heap
[2]: https://nixos.wiki/wiki/Flakes
[3]: https://github.com/yvt/fatalloc/actions/workflows/ci.yml
[4]: https://github.community/t/public-read-access-to-actions-artifacts/17363/11

