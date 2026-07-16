{
  description = "prisoma (macOS-first): reproducible dev shell for Rust + Python (uv)";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";

  outputs = { nixpkgs, ... }:
    let
      systems = [
        "aarch64-darwin"
        "x86_64-darwin"
        "aarch64-linux"
        "x86_64-linux"
      ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
      toolArchives = {
        "aarch64-darwin" = {
          uvTarget = "aarch64-apple-darwin";
          uvHash = "sha256-M1QOt8iDq4V+/3m9WsKqMf4ntZWr7LSpwAOiyZhEcjI=";
          justTarget = "aarch64-apple-darwin";
          justHash = "sha256-81eY1LzcTbAg7vfShTrZi7+5ek0p7mlboELxjn/tzBE=";
        };
        "x86_64-darwin" = {
          uvTarget = "x86_64-apple-darwin";
          uvHash = "sha256-KteZgxJ//KfXe3fOaiQnjX5Pe4F6Gs9y/qX4EktKrF4=";
          justTarget = "x86_64-apple-darwin";
          justHash = "sha256-CbNf9tFwI/+uN85AjRp4qXbZ4AHK5UuI4jj39A25t4M=";
        };
        "aarch64-linux" = {
          uvTarget = "aarch64-unknown-linux-musl";
          uvHash = "sha256-2hDN+n2SISt6y2ICGg/WG8+FgMWMNjLskV0Qw6GnkGs=";
          justTarget = "aarch64-unknown-linux-musl";
          justHash = "sha256-yMHWVun0dWnsGuK/h3mvJiHN7qa7u6Owys1k+VHSXis=";
        };
        "x86_64-linux" = {
          uvTarget = "x86_64-unknown-linux-musl";
          uvHash = "sha256-8CFGs3HDXCh9hg8APs5zRchuNYo/1wqbY3AM0UHuf7Q=";
          justTarget = "x86_64-unknown-linux-musl";
          justHash = "sha256-+iqOwQFdnfUzCUGt4SQ3SI/EDTP5yfjNTrcKJt4Rtjk=";
        };
      };
    in
    {
      devShells = forAllSystems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
          toolArchive = toolArchives.${system};
          mkPinnedArchiveTool =
            {
              pname,
              version,
              archive,
              hash,
              binaries,
            }:
            pkgs.stdenvNoCC.mkDerivation {
              inherit pname version;
              src = pkgs.fetchurl {
                url = "https://github.com/${archive.repository}/releases/download/${version}/${archive.name}";
                inherit hash;
              };
              sourceRoot = ".";
              dontConfigure = true;
              dontBuild = true;
              installPhase = ''
                runHook preInstall
                mkdir -p "$out/bin"
                ${pkgs.lib.concatMapStringsSep "\n" (binary: ''
                  binary_path="$(find . -type f -name ${pkgs.lib.escapeShellArg binary} -perm -0100 -print -quit)"
                  test -n "$binary_path"
                  install -m 0755 "$binary_path" "$out/bin/${binary}"
                '') binaries}
                runHook postInstall
              '';
            };
          uvPinned = mkPinnedArchiveTool {
            pname = "uv";
            version = "0.11.28";
            archive = {
              repository = "astral-sh/uv";
              name = "uv-${toolArchive.uvTarget}.tar.gz";
            };
            hash = toolArchive.uvHash;
            binaries = [
              "uv"
              "uvx"
            ];
          };
          justPinned = mkPinnedArchiveTool {
            pname = "just";
            version = "1.56.0";
            archive = {
              repository = "casey/just";
              name = "just-1.56.0-${toolArchive.justTarget}.tar.gz";
            };
            hash = toolArchive.justHash;
            binaries = [ "just" ];
          };
        in
        {
          default =
            assert pkgs.lib.versionAtLeast pkgs.rustc.version "1.93.0";
            assert pkgs.python311.version == "3.11.15";
            pkgs.mkShell {
              packages = [
                justPinned

                # Rust toolchain (use nixpkgs version; pin via flake.lock).
                pkgs.rustc
                pkgs.cargo
                pkgs.rustfmt
                pkgs.clippy

                # Python + uv for dependency management and script execution.
                pkgs.python311
                uvPinned
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
                echo "  cargo test --locked"
                echo "  cargo run --locked --manifest-path pid-rs/crates/pid-core/Cargo.toml --features experimental-all --bin exp0"
                echo "  uv sync --locked"
              '';
            };
        }
      );
    };
}
