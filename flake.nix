{
  description = "Mycelix v6.0 Living Protocol Layer";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    foundry.url = "github:shazow/foundry.nix/monthly";
    holochain.url = "github:holochain/holochain";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, foundry, holochain }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) foundry.overlay ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
          targets = [ "wasm32-unknown-unknown" ];
        };
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            # Rust
            rustToolchain
            cargo-watch
            cargo-audit
            cargo-outdated

            # Node.js
            nodejs_20
            nodePackages.npm
            nodePackages.typescript

            # Solidity / Foundry
            foundry-bin

            # Holochain (if available)
            # holochain.packages.${system}.holochain
            # holochain.packages.${system}.hc

            # General tools
            git
            jq
            yq-go

            # For criterion benchmarks
            gnuplot
          ];

          shellHook = ''
            echo "Mycelix v6.0 Living Protocol Development Environment"
            echo "======================================================"
            echo ""
            echo "Available commands:"
            echo "  cargo test --workspace --features full  - Run all tests"
            echo "  cargo bench --features full             - Run benchmarks"
            echo "  forge test                              - Run Solidity tests"
            echo "  ./scripts/mycelix-cli.sh help           - CLI tool help"
            echo ""
            echo "Rust: $(rustc --version)"
            echo "Node: $(node --version)"
            command -v forge >/dev/null && echo "Forge: $(forge --version)" || echo "Forge: not available"
            echo ""
          '';

          RUST_BACKTRACE = "1";
          RUST_LOG = "info";
        };

        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "mycelix-living-protocol";
          version = "0.6.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;

          buildFeatures = [ "full" ];

          meta = with pkgs.lib; {
            description = "Mycelix v6.0 Living Protocol Layer";
            homepage = "https://github.com/mycelix/mycelix-v6-living";
            license = licenses.agpl3Plus;
          };
        };
      }
    );
}
