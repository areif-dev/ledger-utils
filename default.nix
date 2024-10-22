{ rustPlatform, version ? "git", lib }:

rustPlatform.buildRustPackage {
  pname = "pta-template-engine";
  inherit version;

  src = lib.cleanSource ./.;

  cargoLock.lockFile = ./Cargo.lock;
}
