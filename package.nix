{rustPlatform}:
rustPlatform.buildRustPackage {
  pname = "ristate";
  version = "unstable-2024-09-03";
  src = ./.;

  useFetchCargoVendor = true;
  cargoHash = "sha256-oyKgwVHcyRIQ4nRJaAX0ooibKp7hYJdmxyaH09cw1pA=";
}
