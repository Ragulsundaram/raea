@spec
Feature: MLX: local models on Apple silicon's GPU (systems/mlx, raea-sys-mlx)
  Shane, 2026-11-10, told IDOL could run on MLX: "we should just do mlx anyway makes no sense not to support mlx
  generally going forward". Then, holding IDOL: "just have mlx support". On this M5 Pro Mac, ort's CPU kernels are the
  fastest ONNX path and too slow for big models. ort's Core ML provider has been slower still, and Core ML's GPU
  computes some models wrong (SigLIP: cosine 0.855 against the CPU's 0.98). MLX runs on the GPU directly.

  So raea-sys-mlx sits beside raea-sys-ml's ONNX Runtime:
  - Each model gets an MLX path on the Mac.
  - ONNX stays the path on Windows and Linux, and the cross-platform reference.
  - The facility reaches users only once a model and a verb use it.

  Background:
    Given mlx-rs =0.34.0 (crates.io, MIT OR Apache-2.0, maintained), which builds MLX v0.32.2 from source at build
      time. Its cmake clones MLX from GitHub, and Xcode's Metal toolchain compiles the kernels.
    And a model's MLX path is plain functions over its safetensors, keyed by the checkpoint's own tensor names, so it
      reads the same files as its PyTorch and ONNX paths

  Rule: the facility builds where MLX runs, and nowhere else
    @done
    Scenario: MX1. raea-sys-mlx links MLX on the Mac and is empty on Linux and Windows
      Then on macOS the crate links MLX and runs on the GPU (`on_gpu`)
      And on Linux and Windows it compiles to nothing and brings no MLX in: the dependency is macOS-only
      And 11-10, measured on the M5 Pro with renders sharing the GPU: a 4096^3 matmul runs at 9.2 TFLOPS in f32 and
        11.5 TFLOPS in f16

  Rule: its layers do PyTorch's maths, so a model's numbers can be matched
    @done
    Scenario: MX2. Each layer matches the same maths written out on the CPU
      Then linear, layer norm, attention, GELU (tanh) and SiLU agree with plain Rust on the CPU
        (systems/mlx/src/tests.rs)
      And the tolerance is 1e-5, or 3e-3 for an f32 matmul or attention, which MLX runs in TF32 on the GPU unless
        MLX_ENABLE_TF32=0 (with it set, all of them agree to 1e-5)
      And a Linear runs in its weight's dtype and a LayerNorm in f32, as PyTorch's do, so a model's mixed precision
        can be mirrored
      And weights load from safetensors by name, and a missing name is an error that names it

    Scenario: MX3. A model's MLX path is checked against its reference before it becomes the Mac default
      Given a model with a PyTorch or ONNX path
      Then each of its networks is compared with the reference on the same inputs (cosine and max error), against a bar
        written in the model's own feature file
      And the end-to-end output is compared the same way
      And the Mac default is picked by parity first, then speed, with both times stated

  Rule: the metallib reaches the user's Mac
    Scenario: MX4. A Mac without Xcode runs an MLX op from the signed raea
      Given MLX loads mlx.metallib (184 MB, 59 MB gzipped) at run time. It looks, in order: the path set by
        `use_metallib`, beside the binary, Resources/, then the build machine's path (~/.mlx/lib/<key>)
      Then with the first model, the metallib either:
        - arrives with that model's files on first use, sha256-pinned and from a models release, then `use_metallib`
        - or ships beside raea in the Mac tarball and the app
      And release.yml's Mac job builds the crate

  Rule: models use it
    Scenario: MX5. The first model runs on MLX on the Mac
      # IDOL (one image to a 3D Gaussian splat) was to be the first. It was held 11-10 (docs/handoff/idol.md, "Parked
      #   11-10"), with its six-network seam agreed.
      # Next candidates:
      #   - SigLIP (perception's embeddings), which Core ML's GPU computes wrong on this Mac
      #   - any local model whose ONNX run on the Mac's CPU is too slow to use
