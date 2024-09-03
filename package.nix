{rustPlatform}:
rustPlatform.buildRustPackage {
  pname = "ristate";
  version = "unstable-2024-09-03";
  src = ./.;

  cargoHash = "sha256-7hwgqVbaaQp4HNegGiFqDb7d2xZP9+99a+aq+6teHxw=";
}
