{
  description = "Ratatidy dev environment";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
  };

  outputs = { self, nixpkgs }:
  let
    system = "x86_64-linux";
    pkgs = nixpkgs.legacyPackages.${system};
    musl = pkgs.pkgsCross.musl64;
  in {
    packages.${system}.default = pkgs.rustPlatform.buildRustPackage {
      pname = "ratatidy";
      version = "0.1.0";
      src = self;
      cargoLock.lockFile = ./Cargo.lock;
      # Remote mode shells out to ssh and rsync
      nativeBuildInputs = [ pkgs.makeWrapper ];
      postInstall = ''
        wrapProgram $out/bin/ratatidy --suffix PATH : ${pkgs.lib.makeBinPath [ pkgs.openssh pkgs.rsync ]}
      '';
    };

    devShells.${system} = {
      default = pkgs.mkShell {
        buildInputs = with pkgs; [
          cargo
          rustc
          clippy
          rustfmt
          rust-analyzer
          rsync
          openssh
        ];
        RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
      };

      # Static build of the Agent for non-Nix servers:
      #   nix develop .#musl -c cargo build --release
      musl = musl.mkShell {
        nativeBuildInputs = [ musl.buildPackages.cargo musl.buildPackages.rustc ];
        CARGO_BUILD_TARGET = "x86_64-unknown-linux-musl";
        CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER = "${musl.stdenv.cc}/bin/${musl.stdenv.cc.targetPrefix}cc";
        # Without this the binary keeps a /nix/store interpreter and won't run on Debian
        CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_RUSTFLAGS = "-C target-feature=+crt-static";
      };
    };
  };
}
