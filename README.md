<div align="center">

# `fatalloc`

**“Fault tolerant” memory allocator for Linux**

</div>

This library provides a drop-in replacement for the standard C allocation
functions. Add `libfatalloc.so` to `LD_PRELOAD` to “fix” minor heap overruns
in faulty software.

Using this library has a negative impact on security and may lead to loss of
data, financial damage, strangelet creation, maximum overdrive, Xindi attacks,
or death. Use at your own peril.

Inspired by Windows [Fault Tolerant Heap][1]. Written in Rust(🚀).

<details>
<summary>Why the security impact?</summary>

While reducing the likelihood of application crashes may seem appealing to some
people, it doesn't necessarily mean bugs are actually fixed if done in a wrong
way. In fact, application crashes are symptoms of underlying bugs and meant to
stop the faulty program that is already straying from the designed behavior from
going even worse, e.g., incurring permanent damage to your files, [impeding
an Iranian nuclear program][7], [violating the right to privacy][8], or even
[taking human lives][6]. Modern binary exploit mitigation techniques, such as
[ShadowCallStack][10] and [Control Flow Guard][9], are often designed to
immediately abort the faulting program at the first sign of security violation.
The heap implementations in modern operating systems evolved as well to detect
heap usage errors and thwart potential heap-based exploits¹. All this library
does is to undo these efforts.

<sub>¹ Mark E. Russinovich, David A. Solomon, Alex Ionescu, *Windows Internals,
Part 2 (6th edition)*, pp 224–225.</sub>

</details>

## Features

- [x] Real-time memory allocator with good throughput (implemented by
  [`rlsf`][5])
- [x] Insert padding around allocations to mitigate heap overruns
- [x] Ignore invalid deallocation requests
- [ ] Delay deallocation to nullify brief use-after-free

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
[5]: https://github.com/yvt/rlsf
[6]: https://embeddedgurus.com/state-space/2014/02/are-we-shooting-ourselves-in-the-foot-with-stack-overflow/
[7]: https://en.wikipedia.org/wiki/Stuxnet
[8]: https://watchfulip.github.io/2021/09/18/Hikvision-IP-Camera-Unauthenticated-RCE.html
[9]: https://docs.microsoft.com/en-us/windows/win32/secbp/control-flow-guard
[10]: https://source.android.com/devices/tech/debug/shadow-call-stack
