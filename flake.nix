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

            # Formal Verification
            (python311.withPackages (ps: with ps; [
              # Halmos for symbolic execution
              # Note: halmos may need to be installed via pip if not in nixpkgs
            ]))

            # Holochain (if available)
            # holochain.packages.${system}.holochain
            # holochain.packages.${system}.hc

            # General tools
            git
            jq
            yq-go
            curl

            # For criterion benchmarks
            gnuplot
          ];

          shellHook = ''
            echo "Mycelix v6.0 Living Protocol Development Environment"
            echo "======================================================"
            echo ""
            echo "Available commands:"
            echo "  cargo test --workspace             - Run all Rust tests"
            echo "  cargo run -p ws-server             - Start WebSocket RPC server"
            echo "  cd sdk/typescript && npm test      - Run TypeScript SDK tests"
            echo ""
            echo "Solidity / Formal Verification:"
            echo "  forge test --fuzz-runs 10000       - Run Solidity fuzz tests"
            echo "  forge test --match-path 'test/halmos/*' - Run Halmos invariant tests"
            echo "  pip install halmos && halmos --contract WoundEscrow - Symbolic execution"
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
