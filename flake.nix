{
  description = "prisoma (macOS-first): reproducible dev shell for Rust + Python (uv)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.11";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
      in
      {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            just

            # Rust toolchain (use nixpkgs version; pin via flake.lock).
            rustc
            cargo
            rustfmt
            clippy

            # Python + uv for dependency management and script execution.
            python311
            uv
          ];

          UV_NO_MANAGED_PYTHON = "1";
          UV_PYTHON_DOWNLOADS = "never";

          shellHook = ''
            echo "prisoma dev shell (Nix)"
            echo "  Rust:   $(rustc --version)"
            echo "  Cargo:  $(cargo --version)"
            echo "  Python: $(python --version)"
            echo "  uv:     $(uv --version)"
            echo ""
            echo "Next:"
            echo "  cargo test"
            echo "  cargo run --manifest-path pid-rs/crates/pid-core/Cargo.toml --features experimental-all --bin exp0"
            echo "  uv sync"
          '';
        };
      }
    );
}
